#!/usr/bin/env python3
"""Fetch Figma frame renders for the sign-review page.

    FIGMA_TOKEN=... python3 fetch-renders.py --ledger .vds/ledgers/frames.yaml \
        --out ./renders [--scale 1] [--batch 25]

Writes <node-id-with-dashes>.png per frame plus a manifest recording, for every requested
node, whether a render arrived and why not if it did not.

WHY THE MANIFEST. A frame with no render must be DISTINGUISHABLE from a frame that was
never requested. The sign-review page blocks signing on a missing render, so "we did not
ask" and "Figma could not draw it" have to be told apart or the block is unexplainable.

TRUNCATION. `vds-figma` documents that the Figma API can answer HTTP 200 with a body that
simply stops, and that curl exits 0 on it. Every JSON response here is therefore parsed
before it is believed, and a body that is not valid JSON is an error rather than an empty
result. The same applies per-image: a PNG that does not start with the PNG signature is
rejected rather than written.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

PNG_MAGIC = b"\x89PNG\r\n\x1a\n"
API = "https://api.figma.com/v1"


def get_json(url: str, token: str, timeout: int = 120) -> dict:
    req = urllib.request.Request(url, headers={"X-Figma-Token": token})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        raw = r.read()
    body = raw.decode("utf-8", "replace").strip()
    if not (body.startswith("{") and body.endswith("}")):
        raise RuntimeError(
            f"response is not a complete JSON document ({len(raw)} bytes, "
            f"ends {body[-40:]!r}). The request may have SUCCEEDED and been truncated."
        )
    return json.loads(body)


def parse_nodes(ledger: Path) -> list[str]:
    return [b.split("\n")[0].strip()
            for b in re.split(r"\n- node_id: ", ledger.read_text())[1:]]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--ledger", required=True, type=Path)
    ap.add_argument("--file-key", help="defaults to file_key in the ledger header")
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--scale", default="1")
    ap.add_argument("--batch", type=int, default=25)
    ap.add_argument("--only", nargs="*")
    a = ap.parse_args()

    token = os.environ.get("FIGMA_TOKEN")
    if not token:
        print("no FIGMA_TOKEN in the environment", file=sys.stderr)
        return 1

    text = a.ledger.read_text()
    key = a.file_key or (re.search(r"^file_key:\s*(\S+)$", text, re.M) or [None, None])[1]
    if not key:
        print("no file key: pass --file-key", file=sys.stderr)
        return 1

    nodes = a.only or parse_nodes(a.ledger)
    a.out.mkdir(parents=True, exist_ok=True)
    manifest: dict[str, dict] = {}
    got = failed = skipped = 0

    for i in range(0, len(nodes), a.batch):
        chunk = nodes[i:i + a.batch]
        url = f"{API}/images/{key}?ids={','.join(chunk)}&format=png&scale={a.scale}"
        print(f"  batch {i // a.batch + 1}: {len(chunk)} nodes", flush=True)
        try:
            data = get_json(url, token)
        except Exception as exc:
            for n in chunk:
                manifest[n] = {"rendered": False, "reason": f"batch request failed: {exc}"}
                failed += 1
            print(f"    FAILED: {exc}", file=sys.stderr)
            continue
        if data.get("err"):
            for n in chunk:
                manifest[n] = {"rendered": False, "reason": f"figma error: {data['err']}"}
                failed += 1
            print(f"    figma error: {data['err']}", file=sys.stderr)
            continue

        for n in chunk:
            src = (data.get("images") or {}).get(n)
            dst = a.out / f"{n.replace(':', '-')}.png"
            if dst.exists() and dst.stat().st_size > 0:
                manifest[n] = {"rendered": True, "reason": "already present", "path": dst.name}
                skipped += 1
                continue
            if not src:
                manifest[n] = {"rendered": False,
                               "reason": "figma returned no image url for this node"}
                failed += 1
                continue
            try:
                with urllib.request.urlopen(src, timeout=180) as r:
                    blob = r.read()
                if not blob.startswith(PNG_MAGIC):
                    raise RuntimeError(f"not a PNG ({blob[:12]!r})")
                dst.write_bytes(blob)
                manifest[n] = {"rendered": True, "bytes": len(blob), "path": dst.name}
                got += 1
            except Exception as exc:
                manifest[n] = {"rendered": False, "reason": f"download failed: {exc}"}
                failed += 1
        time.sleep(0.4)  # be a good citizen against the render queue

    (a.out / "renders-manifest.json").write_text(json.dumps({
        "fileKey": key, "scale": a.scale, "requested": len(nodes),
        "rendered": got + skipped, "failed": failed, "nodes": manifest}, indent=2))
    print(f"\n  rendered {got} (+{skipped} already present), {failed} failed, "
          f"of {len(nodes)} requested")
    if failed:
        print("  failures are recorded in renders-manifest.json; the sign page will show "
              "NOT RENDERED for those and block signing, which is correct.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
