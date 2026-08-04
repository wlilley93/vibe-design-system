#!/usr/bin/env python3
"""Serve the sign-review site and receive submitted decisions.

Stdlib only. Binds to loopback by default: this surface displays an estate's unreleased
design and accepts decisions that become input to a Principal act, so it is not something
to expose on a network interface by accident.

    python3 serve-sign-review.py --page /path/to/sign-review.html --outdir ./decisions

POST /submit writes one timestamped JSON act per submission and never overwrites: a
resubmission is a new record, because a decision log that can be silently replaced is not
a log. The response carries the path back so the page can show where it landed rather than
claiming success in the abstract.

This server records NOTHING as authority. It writes a decisions file. The `vds` CLI turns
that into a recorded Principal act, and only a Principal act creates authority (order 16).
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

MAX_BODY = 32 * 1024 * 1024


def make_handler(page: Path, outdir: Path, assets: dict[str, Path]):
    class Handler(BaseHTTPRequestHandler):
        server_version = "vds-sign-review/1"

        def log_message(self, fmt, *args):  # quieter than the default
            sys.stderr.write("  %s\n" % (fmt % args))

        def _send(self, code, body: bytes, ctype="application/json"):
            self.send_response(code)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("X-Content-Type-Options", "nosniff")
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            if self.path in ("/", "/index.html"):
                if not page.exists():
                    self._send(500, b"the page has not been built", "text/plain")
                    return
                self._send(200, page.read_bytes(), "text/html; charset=utf-8")
            elif self.path == "/health":
                self._send(200, json.dumps({"ok": True, "page": str(page),
                                            "assets": {k: str(v) for k, v in assets.items()}}).encode())
            elif self.path.startswith("/renders/") or self.path.startswith("/shots/"):
                kind, _, name = self.path.lstrip("/").partition("/")
                root = assets.get(kind)
                # Resolve and confine. A served directory that accepts ".." serves the disk.
                if not root:
                    self._send(404, b"no such asset root", "text/plain"); return
                try:
                    target = (root / name).resolve()
                    target.relative_to(root.resolve())
                except (ValueError, OSError):
                    self._send(403, b"outside the asset root", "text/plain"); return
                if not target.is_file():
                    self._send(404, b"not found", "text/plain"); return
                self.send_response(200)
                self.send_header("Content-Type", "image/png")
                self.send_header("Content-Length", str(target.stat().st_size))
                self.send_header("Cache-Control", "public, max-age=3600")
                self.end_headers()
                with target.open("rb") as f:
                    while chunk := f.read(1 << 16):
                        self.wfile.write(chunk)
            else:
                self._send(404, b"not found", "text/plain")

        def do_POST(self):
            if self.path != "/submit":
                self._send(404, b"not found", "text/plain")
                return
            try:
                n = int(self.headers.get("Content-Length") or 0)
            except ValueError:
                n = 0
            if n <= 0 or n > MAX_BODY:
                self._send(413, json.dumps({"error": "bad or oversized body"}).encode())
                return
            raw = self.rfile.read(n)
            try:
                act = json.loads(raw)
            except Exception as exc:
                self._send(400, json.dumps({"error": f"not JSON: {exc}"}).encode())
                return

            # Refuse to record something that is not what it claims to be. A receiver that
            # accepts any shape makes the store unreadable later.
            if act.get("documentType") != "vds.sign-review.decisions":
                self._send(400, json.dumps({"error": "unexpected documentType"}).encode())
                return
            decisions = act.get("decisions")
            if not isinstance(decisions, list) or not decisions:
                self._send(400, json.dumps({"error": "no decisions in submission"}).encode())
                return
            missing = [d.get("nodeId") for d in decisions
                       if d.get("decision") == "sign" and not d.get("selectedLocus")]
            if missing:
                self._send(400, json.dumps({
                    "error": "a sign decision must name the governing locus it signs",
                    "nodes": missing}).encode())
                return

            outdir.mkdir(parents=True, exist_ok=True)
            stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
            path = outdir / f"sign-review-decisions-{stamp}.json"
            i = 1
            while path.exists():  # never overwrite a prior submission
                path = outdir / f"sign-review-decisions-{stamp}-{i}.json"
                i += 1
            act["receivedAt"] = datetime.now(timezone.utc).isoformat()
            act["receivedBy"] = "tools/sign-review/serve-sign-review.py"
            act["authority"] = ("none. This file records decisions only. Authority arises when "
                                "the vds CLI records a Principal act citing it.")
            path.write_text(json.dumps(act, indent=2))
            counts: dict[str, int] = {}
            for d in decisions:
                counts[d.get("decision", "?")] = counts.get(d.get("decision", "?"), 0) + 1
            over = sum(1 for d in decisions if d.get("overridesToolProposal"))
            print(f"  recorded {len(decisions)} decisions -> {path}")
            print(f"    {counts}  ({over} overriding the tool's proposed locus)")
            self._send(200, json.dumps({"ok": True, "path": str(path),
                                        "counts": counts, "overrides": over}).encode())

    return Handler


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--page", required=True, type=Path)
    ap.add_argument("--outdir", type=Path, default=Path("./sign-review-decisions"))
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=8787)
    ap.add_argument("--renders", type=Path, help="dir served at /renders/")
    ap.add_argument("--shots", type=Path, help="dir served at /shots/")
    a = ap.parse_args()

    if not a.page.exists():
        print(f"page not found: {a.page}\nBuild it first with build-sign-review.py", file=sys.stderr)
        return 1
    if a.host not in ("127.0.0.1", "localhost", "::1"):
        print(f"  WARNING: binding {a.host}, not loopback. This surface shows unreleased design "
              f"and accepts decisions. Be deliberate about that.", file=sys.stderr)

    assets = {k: v for k, v in (("renders", a.renders), ("shots", a.shots)) if v}
    for k, v in assets.items():
        if not v.is_dir():
            print(f"  WARNING: --{k} {v} is not a directory; those images will 404 and the "
                  f"page will show the artefact as missing.", file=sys.stderr)
    srv = ThreadingHTTPServer((a.host, a.port), make_handler(a.page, a.outdir, assets))
    print(f"sign review on http://{a.host}:{a.port}/")
    print(f"  page        {a.page}")
    print(f"  submissions {a.outdir.resolve()}")
    for k, v in assets.items():
        print(f"  /{k}/      {v}")
    print("  this server creates no authority; it records decisions for the vds CLI")
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        print("\nstopped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
