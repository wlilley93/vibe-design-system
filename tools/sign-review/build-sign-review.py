#!/usr/bin/env python3
"""Build the signing review page: frame · contract · output · decision.

A VDS capability. Nothing here is specific to any subscriber estate: every input is a
path argument and every rule is derived from the frames ledger and the capture.

WHY THIS EXISTS. Reported by a subscriber estate on 2026-08-03 and generalised here. A
Principal signed 167 frames in one act. The evidence recorded scope, capture version and
aggregate digest, and not one layer name. Eleven of those frames had had the Principal's
own content renamed `LEGACY UNDERLAY` and hidden, with a machine-created clone given a
recognised authority marker. Nothing in front of the signer could have shown that. A
signature act that binds N frames without showing what is in them is a design defect, not
a discipline problem.

WHAT THIS IS NOT. This page has no authority and cannot create any (order 16: a machine
verdict creates no authority). It COLLECTS decisions and emits a JSON act; the `vds` CLI
records it. Nothing here writes a sign-off.

FIVE RULES, each paid for by a real defect:

1. DISCLOSURE BEFORE DECISION. The layer holding authority, every demotion marker, every
   hidden layer and every `cloned from` provenance string appears ABOVE the controls.

2. THE SIGNER CHOOSES THE LOCUS. Every candidate governing layer is listed with its
   metadata and the signer picks. The tool's own resolution is shown as a PROPOSAL and is
   never pre-accepted, because a default that is silently adopted is how a machine's choice
   became a contract in the first place. Choosing a locus IS the express label-resolution
   act; it needs no redraw.

3. ONE MOMENT. Frame, contract and output each carry their own digest, displayed. A
   missing one blocks the decision rather than being quietly skipped.

4. IT MUST BE ABLE TO SAY NO. Sign / Refuse / Defer, with a reason required for the
   latter two. A sign button with no refusal path is a check that cannot fail.

5. NO BULK. There is no sign-all. One act over N rows is what this exists to stop.

A missing artefact renders as an explicit NOT CAPTURED / NOT RENDERED panel, never blank:
"the frame draws nothing here" and "we did not look" must never look the same.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import html
import json
import re
import sys
from pathlib import Path

DEFAULT_CURRENT = ("CURRENT SOURCE", "SOURCE AUTHORITY", "CURRENT CODE")
DEFAULT_DEMOTION = ("LEGACY UNDERLAY", "REFERENCE", "NOT SOURCE-CURRENT", "DEPRECATED", "QUARANTINE")
CONTAINERS = {"FRAME", "GROUP", "COMPONENT", "COMPONENT_SET", "INSTANCE", "SECTION"}


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()


def parse_frames_ledger(path: Path) -> dict:
    """Minimal read of a VDS frames ledger. Deliberately dependency-free."""
    text = path.read_text()
    header = {}
    for key in ("capture_depth", "captured_at", "generated_at", "content_digest", "file_key",
                "truncated_leaves"):
        m = re.search(rf"^{key}:\s*(.*)$", text, re.M)
        if m:
            header[key] = m.group(1).strip()
    rows = {}
    for block in re.split(r"\n- node_id: ", text)[1:]:
        nid = block.split("\n")[0].strip()
        get = lambda k: (re.search(rf"^\s*{k}:\s*(.*)$", block, re.M) or [None, ""])[1].strip()
        rows[nid] = {
            "node_id": nid,
            "frame_name": get("frame_name"),
            "authority_layer": get("authority_layer"),
            "authority_by": get("authority_by"),
            "columns": get("columns"),
            "content_digest": get("content_digest"),
            "disclaimed": get("disclaimed") == "true",
            "truncated": get("truncated") == "true",
            "regions": re.findall(r"^\s*- (\w+)\s*$", block, re.M),
        }
    return {"header": header, "rows": rows}


def measure(n: dict) -> tuple[int, int]:
    nodes = 1
    texts = 1 if n.get("type") == "TEXT" and n.get("characters") else 0
    for c in n.get("children") or []:
        a, b = measure(c)
        nodes += a
        texts += b
    return nodes, texts


def classify(name: str, current: tuple[str, ...], demotion: tuple[str, ...]) -> str | None:
    u = (name or "").upper()
    if any(u.startswith(m) for m in current):
        return "current"
    if any(m in u for m in demotion):
        return "demoted"
    return None


def candidates(doc: dict, regions: list[str], current, demotion) -> list[dict]:
    """Every locus that could lawfully govern this frame, with the metadata to choose.

    Deliberately generous: a signer cannot select a locus the tool declined to show, and
    the incident this exists for is precisely a governing layer that no reader surfaced."""
    out = []

    def add(node, depth, note):
        nodes, texts = measure(node)
        b = node.get("absoluteBoundingBox") or {}
        out.append({
            "id": node.get("id") or "",
            "name": node.get("name") or "(unnamed)",
            "type": node.get("type") or "",
            "depth": depth,
            "visible": node.get("visible", True),
            "nodes": nodes,
            "texts": texts,
            "w": int(b.get("width") or 0),
            "h": int(b.get("height") or 0),
            "marker": classify(node.get("name") or "", current, demotion),
            "cloned_from": (re.search(r"cloned from (\d+[:\-]\d+)", node.get("name") or "") or [None, None])[1]
                           if re.search(r"cloned from", node.get("name") or "") else None,
            "note": note,
        })

    add(doc, 0, "the frame itself")
    for c in doc.get("children") or []:
        why = []
        if (c.get("name") or "") in regions:
            why.append("declares a shell region")
        k = classify(c.get("name") or "", current, demotion)
        if k == "current":
            why.append("carries a current-source marker")
        if k == "demoted":
            why.append("carries a demotion marker")
        if (c.get("type") or "") in CONTAINERS:
            why.append("container")
        add(c, 1, ", ".join(why) or "direct child")

    def deep(node, depth):
        for c in node.get("children") or []:
            if classify(c.get("name") or "", current, demotion) and depth + 1 > 1:
                add(c, depth + 1, "marker below the top level")
            deep(c, depth + 1)

    deep(doc, 0)
    seen, uniq = set(), []
    for c in out:
        key = (c["id"], c["name"])
        if key in seen:
            continue
        seen.add(key)
        uniq.append(c)
    return uniq


def load_capture(cap_dir: Path) -> dict[str, dict]:
    nodes = {}
    for f in sorted(cap_dir.glob("nodes-*.json")):
        try:
            data = json.loads(f.read_text())
        except Exception as exc:
            print(f"  WARNING: {f.name} did not parse ({exc}). Treated as ABSENT, not empty.",
                  file=sys.stderr)
            continue
        for k, v in (data.get("nodes") or {}).items():
            nodes[k.replace("-", ":")] = v.get("document")
    return nodes


def data_uri(p: Path | None) -> str | None:
    if not p or not p.exists():
        return None
    return "data:image/png;base64," + base64.b64encode(p.read_bytes()).decode()


def asset_src(p: Path | None, base: str | None, kind: str) -> str | None:
    """A served URL when a base is given, otherwise an inline data URI.

    Inlining is correct for a handful of frames and ruinous for a whole estate: 167
    frames inline to a ~41MB document that a phone cannot parse."""
    if not p or not p.exists():
        return None
    if base is not None:  # "" is a VALID base (same-origin). Falsy != absent.
        return f"{base.rstrip('/')}/{kind}/{p.name}"
    return data_uri(p)


def _canon(s: str) -> str:
    """Reduce a route or a filename stem to comparable letters and digits.

    The capture pipeline collapses every non-alphanumeric run to one underscore and
    leaves a trailing one after a `]`, so `/c/[token]/calendar/[stepId]` is written
    `c_token_calendar_stepId_`. Matching on a hand-built slug missed 55 of 60 screenshots
    that were present on disk, and reported them to the signer as NOT CAPTURED. Normalise
    both sides instead of guessing one side's convention."""
    return re.sub(r"[^a-z0-9]", "", s.lower())


def index_shots(shots: Path | None) -> dict[str, list[Path]]:
    idx: dict[str, list[Path]] = {}
    if shots and shots.is_dir():
        for p in sorted(shots.glob("*.png")):
            idx.setdefault(_canon(p.name.split(".")[0]), []).append(p)
    return idx


def find_shipped(idx: dict[str, list[Path]], route: str) -> tuple[Path | None, str | None]:
    """Return (path, refusal_reason). AMBIGUITY IS A REFUSAL, not a coin toss.

    Two routes resolving to one screenshot is a real defect on this estate (21 duplicate
    screenshots once collapsed onto two cards). Picking one silently would hand the signer
    a picture of a different page and call it this one."""
    hits = idx.get(_canon(route.strip("/")) or "root", [])
    if not hits:
        return None, None
    if len(hits) > 1:
        return None, ("ambiguous screenshot: " + ", ".join(p.name for p in hits))
    return hits[0], None


CSS = """
:root{--bg:#fff;--fg:#111;--mute:#4d4d4d;--line:rgba(0,0,0,.12);--warn:#fc0035;--ok:#28a948;--accent:#006bff;--surface:#fafafa}
@media(prefers-color-scheme:dark){:root{--bg:#0a0a0a;--fg:#ededed;--mute:#a1a1a1;--line:rgba(255,255,255,.14);--surface:#141414}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:14px/1.55 ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif}
code,.mono{font-family:ui-monospace,"SF Mono",Menlo,monospace;font-variant-numeric:tabular-nums}
header{position:sticky;top:0;z-index:9;background:var(--bg);border-bottom:1px solid var(--line);padding:12px 20px;display:flex;gap:16px;align-items:baseline;flex-wrap:wrap}
h1{font-size:15px;margin:0;font-weight:600}.sub{color:var(--mute);font-size:12px}
main{padding:20px;max-width:1700px;margin:0 auto}
.frame{border:1px solid var(--line);border-radius:8px;margin-bottom:28px;overflow:hidden;background:var(--surface)}
.fh{padding:12px 16px;border-bottom:1px solid var(--line);display:flex;gap:12px;align-items:baseline;flex-wrap:wrap}
.fh .route{font-weight:600;font-size:15px}
.badge{font-size:11px;padding:2px 7px;border-radius:4px;border:1px solid var(--line);color:var(--mute)}
.badge.bad{border-color:var(--warn);color:var(--warn)}
.sec{padding:14px 16px;border-bottom:1px solid var(--line)}
.sec.alert{background:color-mix(in srgb,var(--warn) 8%,transparent);border-left:3px solid var(--warn)}
.sec h3{margin:0 0 8px;font-size:12px;letter-spacing:.06em;text-transform:uppercase;color:var(--mute)}
.sec ul{margin:4px 0 0;padding-left:18px}.sec li{font-size:13px}
.panes{display:grid;grid-template-columns:repeat(auto-fit,minmax(320px,1fr));gap:1px;background:var(--line)}
.pane{background:var(--bg);padding:14px 16px;min-width:0}
.pane h2{margin:0 0 4px;font-size:12px;letter-spacing:.06em;text-transform:uppercase;color:var(--mute)}
.pane .digest{font-size:11px;color:var(--mute);word-break:break-all;margin-bottom:10px}
.pane img{max-width:100%;height:auto;border:1px solid var(--line);border-radius:4px;display:block}
.missing{border:1px dashed var(--warn);color:var(--warn);border-radius:4px;padding:22px 14px;text-align:center;font-size:13px}
table.kv{width:100%;border-collapse:collapse;font-size:13px}
table.kv td{padding:3px 0;vertical-align:top}
table.kv td:first-child{color:var(--mute);width:44%;padding-right:10px}
.cands{overflow-x:auto}
table.c{width:100%;border-collapse:collapse;font-size:13px;min-width:640px}
table.c th{text-align:left;font-size:11px;text-transform:uppercase;letter-spacing:.05em;color:var(--mute);font-weight:500;padding:4px 8px 4px 0;border-bottom:1px solid var(--line)}
table.c td{padding:5px 8px 5px 0;border-bottom:1px solid var(--line);vertical-align:top}
table.c tr.proposed td{background:color-mix(in srgb,var(--accent) 7%,transparent)}
.decide{padding:14px 16px;display:flex;gap:10px;align-items:center;flex-wrap:wrap}
button{font:inherit;padding:7px 15px;border-radius:6px;border:1px solid var(--line);background:var(--bg);color:var(--fg);cursor:pointer}
button.sign{border-color:var(--fg);background:var(--fg);color:var(--bg);font-weight:600}
button.sign:disabled{opacity:.35;cursor:not-allowed;background:var(--bg);color:var(--mute);border-color:var(--line)}
button[aria-pressed=true]{outline:2px solid var(--accent);outline-offset:1px}
input.reason{flex:1;min-width:220px;padding:7px 10px;border-radius:6px;border:1px solid var(--line);background:var(--bg);color:var(--fg)}
.state{font-size:12px;color:var(--mute)}
.filters{display:flex;gap:6px;flex-wrap:wrap}
.filters .f{padding:4px 10px;font-size:12px;min-height:0;border-radius:20px}
.filters .f[aria-pressed=true]{background:var(--fg);color:var(--bg);border-color:var(--fg);outline:none}.blocked{color:var(--warn);font-size:13px;padding:0 16px 14px}
footer{position:sticky;bottom:0;background:var(--bg);border-top:1px solid var(--line);padding:12px 20px;display:flex;gap:14px;align-items:center;flex-wrap:wrap;padding-bottom:max(12px,env(safe-area-inset-bottom))}
img{max-width:100%;height:auto}

/* --- narrow screens ------------------------------------------------------
   The candidate table is the hard part: it is the control the signer decides
   with, so it may not be the thing that gets truncated. Drop the columns that
   inform (type, depth, size) and keep the ones that decide (layer, weight,
   flags), rather than letting the row scroll off and be missed. */
@media (max-width:720px){
  main{padding:10px}
  header,footer{padding:10px 12px;gap:8px}
  .frame{margin-bottom:16px;border-radius:6px}
  .fh,.sec,.pane,.decide{padding:11px 12px}
  .panes{grid-template-columns:1fr}
  table.c{min-width:0;font-size:12px}
  table.c th:nth-child(3),table.c td:nth-child(3),
  table.c th:nth-child(4),table.c td:nth-child(4),
  table.c th:nth-child(7),table.c td:nth-child(7){display:none}
  table.c td:nth-child(2){word-break:break-word}
  .cands{overflow-x:visible}
  input[type=radio]{width:22px;height:22px}
  table.c td{padding:9px 8px 9px 0}
  button{min-height:44px;padding:10px 16px;flex:1 1 auto}
  .decide{gap:8px}
  input.reason{flex:1 0 100%;min-height:44px}
  .state,#submitState{flex:1 0 100%}
  header h1{flex:1 0 100%}
}
@media (max-width:400px){
  table.c th:nth-child(5),table.c td:nth-child(5),
  table.c th:nth-child(6),table.c td:nth-child(6){display:none}
}
"""

JS = """
const DEC={};
function filt(mode,btn){
  document.querySelectorAll('.filters .f').forEach(b=>b.setAttribute('aria-pressed',String(b===btn)));
  document.querySelectorAll('section.frame').forEach(sec=>{
    const id=sec.id.slice(2);
    const show = mode==='all' ? true
      : mode==='flagged' ? sec.dataset.flagged==='1'
      : mode==='blocked' ? sec.dataset.blocked==='1'
      : !DEC[id];
    sec.style.display = show ? '' : 'none';
  });
}
function card(id){return document.getElementById('f-'+id);}
function locus(id){const r=card(id).querySelector('input[name="loc-'+id+'"]:checked');return r?JSON.parse(r.value):null;}
function setDec(id,v,btn){
  const l=locus(id);
  if(v==='sign'&&!l){card(id).querySelector('.state').textContent='choose the governing layer first';return;}
  DEC[id]={decision:v,locus:l,comment:(card(id).querySelector('.reason')||{}).value||''};
  card(id).querySelectorAll('.decide button[data-v]').forEach(b=>b.setAttribute('aria-pressed',String(b===btn)));
  card(id).querySelector('.state').textContent =
    v==='sign' ? ('signing with locus: '+l.name) : (v+' — comment required');
  sync();
}
function sync(){
  let signed=0,other=0,bad=0;
  for(const [id,d] of Object.entries(DEC)){
    const c=(card(id).querySelector('.reason')||{}).value||'';
    d.comment=c;
    if(d.decision==='sign'){signed++; d.locus=locus(id);}
    else{other++; if(!c.trim())bad++;}
  }
  document.getElementById('tally').textContent =
    signed+' to sign · '+other+' refused or deferred'+(bad?(' · '+bad+' MISSING A COMMENT'):'');
  document.getElementById('emit').disabled=(signed+other)===0||bad>0;
}
function buildAct(){
  return {documentType:'vds.sign-review.decisions',schemaVersion:1,
    generatedBy:'tools/sign-review/build-sign-review.py',
    fileKey:FILE_KEY,captureVersion:CAPTURE,ledgerDigest:LEDGER_DIGEST,
    decidedAt:new Date().toISOString(),
    note:'Collected decisions including an express choice of governing locus per frame. Creates NO authority by itself; input to the vds CLI, which records the Principal act.',
    decisions:Object.entries(DEC).map(([nodeId,d])=>({nodeId,route:ROUTES[nodeId],
      frameContentDigest:DIGESTS[nodeId],decision:d.decision,
      selectedLocus:d.locus||null,toolProposedLocus:PROPOSED[nodeId]||null,
      overridesToolProposal:!!(d.locus&&PROPOSED[nodeId]&&d.locus.name!==PROPOSED[nodeId]),
      comment:d.comment||null}))};
}
function download(){
  const b=new Blob([JSON.stringify(buildAct(),null,2)],{type:'application/json'});
  const a=document.createElement('a');a.href=URL.createObjectURL(b);
  a.download='sign-review-decisions.json';a.click();
}
async function submit(){
  const el=document.getElementById('submitState');
  const btn=document.getElementById('emit');
  btn.disabled=true; el.textContent='submitting…';
  try{
    const r=await fetch('/submit',{method:'POST',headers:{'Content-Type':'application/json'},
      body:JSON.stringify(buildAct())});
    if(!r.ok) throw new Error('HTTP '+r.status);
    const j=await r.json();
    el.textContent='recorded to '+j.path+' — not yet authority; run the vds CLI to record the act';
  }catch(err){
    // Never silently swallow: a submit that failed must not look like one that worked.
    el.textContent='SUBMIT FAILED ('+err.message+'). Falling back to a file download so nothing is lost.';
    download();
    btn.disabled=false;
  }
}
document.addEventListener('input',e=>{
  if(e.target.classList.contains('reason'))sync();
  if(e.target.type==='radio'){const id=e.target.name.slice(4);if(DEC[id])setDec(id,DEC[id].decision,
    card(id).querySelector('.decide button[aria-pressed=true]'));}
});
"""


def build(frames, header, out: Path, file_key: str) -> None:
    cards, routes_js, digests_js, proposed_js = [], {}, {}, {}
    for fr in frames:
        nid = fr["node_id"]
        routes_js[nid] = fr["route"]
        digests_js[nid] = fr["content_digest"]
        proposed_js[nid] = fr["authority_layer"]

        cands = fr["candidates"]
        alert = any(c["marker"] == "demoted" or c["cloned_from"] for c in cands)
        code_traced = any((c["name"] or "").upper().startswith("CURRENT CODE") for c in cands)

        bits = [f"<h3>What the tool resolved, as a proposal only</h3>"
                f"<p class='mono'>{html.escape(fr['authority_layer'] or '(none)')}"
                f" <span class='badge'>{html.escape(fr['authority_by'])}</span></p>"
                f"<p class='sub'>Nothing is pre-selected. The tool's reading is a proposal; the "
                f"choice below is the act.</p>"]
        dem = [c for c in cands if c["marker"] == "demoted"]
        if dem:
            bits.append("<h3>Content demoted out of authority</h3><ul>" + "".join(
                f"<li class='mono'>{html.escape(c['name'])}"
                + ("" if c["visible"] else " <span class='badge bad'>hidden</span>")
                + f" <span class='sub'>{c['nodes']} nodes, {c['texts']} texts</span></li>"
                for c in dem) + "</ul>")
        cl = [c for c in cands if c["cloned_from"]]
        if cl:
            bits.append("<h3>Machine provenance</h3><ul>" + "".join(
                f"<li class='mono'>{html.escape(c['name'])}</li>" for c in cl) + "</ul>")
        if code_traced:
            bits.append("<p class='badge bad'>CURRENT CODE — traced from the shipped "
                        "application. Selecting it makes the code its own contract.</p>")

        rows = []
        for i, c in enumerate(cands):
            val = html.escape(json.dumps({"id": c["id"], "name": c["name"], "depth": c["depth"],
                                          "nodes": c["nodes"], "visible": c["visible"]}), quote=True)
            is_prop = c["name"] == fr["authority_layer"]
            flags = " ".join(filter(None, [
                "<span class='badge'>proposed</span>" if is_prop else "",
                "<span class='badge bad'>hidden</span>" if not c["visible"] else "",
                "<span class='badge bad'>demoted</span>" if c["marker"] == "demoted" else "",
                "<span class='badge'>marker</span>" if c["marker"] == "current" else "",
                f"<span class='badge bad'>cloned from {html.escape(c['cloned_from'])}</span>" if c["cloned_from"] else "",
            ]))
            rows.append(
                f"<tr class='{'proposed' if is_prop else ''}'>"
                f"<td><input type='radio' name='loc-{html.escape(nid)}' value=\"{val}\" id='r{i}-{html.escape(nid)}'></td>"
                f"<td><label for='r{i}-{html.escape(nid)}' class='mono'>{html.escape(c['name'])}</label><br>"
                f"<span class='sub'>{html.escape(c['note'])}</span></td>"
                f"<td class='mono'>{html.escape(c['type'])}</td><td class='mono'>{c['depth']}</td>"
                f"<td class='mono'>{c['nodes']}</td><td class='mono'>{c['texts']}</td>"
                f"<td class='mono'>{c['w']}&times;{c['h']}</td><td>{flags}</td></tr>")

        blockers = []
        if fr["truncated"]:
            blockers.append("This frame's capture was truncated; what is not shown may exist.")
        if fr["shipped"] is None:
            blockers.append("No shipped screenshot: nothing to compare the frame against.")
        if fr["render"] is None:
            blockers.append("The frame is not rendered: you would sign a drawing you cannot see.")

        def pane(t, dg, body):
            return (f"<div class='pane'><h2>{t}</h2>"
                    f"<div class='digest mono'>{html.escape(dg or '—')}</div>{body}</div>")

        rb = (f"<img alt='Frame for {html.escape(fr['route'])}' src='{fr['render']}'>" if fr["render"]
              else "<div class='missing'>NOT RENDERED<br><span class='sub'>no image fetched for this node</span></div>")
        sb = (f"<img alt='Served page {html.escape(fr['route'])}' src='{fr['shipped']}'>" if fr["shipped"]
              else "<div class='missing'>NOT CAPTURED<br><span class='sub'>no screenshot of the served build</span></div>")
        kv = "".join(f"<tr><td>{html.escape(k)}</td><td class='mono'>{html.escape(str(v))}</td></tr>"
                     for k, v in [("regions declared", ", ".join(fr["regions"]) or "none"),
                                  ("content columns", fr["columns"] or "—"),
                                  ("resolved by", fr["authority_by"]),
                                  ("self-disclaims?", "yes" if fr["disclaimed"] else "no"),
                                  ("capture depth", header.get("capture_depth", "—")),
                                  ("captured at", header.get("captured_at", "—"))])

        cards.append(f"""
<section class="frame" id="f-{html.escape(nid)}" data-flagged="{'1' if alert else '0'}" data-blocked="{'1' if blockers else '0'}">
  <div class="fh"><span class="route">{html.escape(fr['route'])}</span>
    <span class="badge mono">{html.escape(nid)}</span>
    <span class="badge">{html.escape(fr['authority_by'])}</span>
    {"<span class='badge bad'>machine-authored content present</span>" if alert else ""}</div>
  <div class="sec{' alert' if alert else ''}">{''.join(bits)}</div>
  <div class="panes">{pane("Frame", fr["frame_image_digest"], rb)}
    {pane("Contract", fr["content_digest"], f"<table class='kv'>{kv}</table>")}
    {pane("Output", fr["shipped_digest"], sb)}</div>
  <div class="sec"><h3>Which layer governs this frame?</h3>
    <div class="cands"><table class="c"><thead><tr><th></th><th>layer</th><th>type</th>
      <th>depth</th><th>nodes</th><th>texts</th><th>size</th><th></th></tr></thead>
      <tbody>{''.join(rows)}</tbody></table></div></div>
  {"<div class='blocked'>Decision blocked: " + " ".join(html.escape(b) for b in blockers) + "</div>" if blockers else ""}
  <div class="decide">
    <button class="sign" data-v="sign" {"disabled" if blockers else ""}
      onclick="setDec('{html.escape(nid)}','sign',this)">Sign with the chosen layer</button>
    <button data-v="refuse" onclick="setDec('{html.escape(nid)}','refuse',this)">Refuse</button>
    <button data-v="defer" onclick="setDec('{html.escape(nid)}','defer',this)">Defer</button>
    <input class="reason" placeholder="comment — optional when signing, required to refuse or defer">
    <span class="state"></span></div>
</section>""")

    page = f"""<!doctype html>
<html lang="en"><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<meta name="color-scheme" content="light dark">
<title>Sign review — {html.escape(file_key)}</title>
<style>{CSS}</style>
</head><body>
<header><h1>Sign review</h1>
  <span class="sub mono">file {html.escape(file_key)} · captured {html.escape(header.get('captured_at','—'))} · depth {html.escape(header.get('capture_depth','—'))} · {html.escape(header.get('truncated_leaves','?'))} truncated leaves</span>
  <span class="sub">{len(frames)} frames, each decided on its own. There is no sign-all.</span>
  <span class="filters">
    <button class="f" data-f="all" aria-pressed="true" onclick="filt('all',this)">All {len(frames)}</button>
    <button class="f" data-f="flagged" onclick="filt('flagged',this)">Machine-authored {sum(1 for f in frames if any(c["marker"]=="demoted" or c["cloned_from"] for c in f["candidates"]))}</button>
    <button class="f" data-f="blocked" onclick="filt('blocked',this)">Blocked {sum(1 for f in frames if f["render"] is None or f["shipped"] is None or f["truncated"])}</button>
    <button class="f" data-f="todo" onclick="filt('todo',this)">Undecided</button>
  </span></header>
<main>{''.join(cards)}</main>
<footer><span id="tally" class="state">nothing decided yet</span>
  <button id="emit" class="sign" onclick="submit()" disabled>Submit decisions</button>
  <button onclick="download()">Download instead</button>
  <span id="submitState" class="sub"></span>
  <span class="sub">This page creates no authority. It submits a JSON act for <code>vds</code> to record.</span></footer>
<script>
const ROUTES={json.dumps(routes_js)};const DIGESTS={json.dumps(digests_js)};
const PROPOSED={json.dumps(proposed_js)};const FILE_KEY={json.dumps(file_key)};
const CAPTURE={json.dumps(header.get('captured_at',''))};
const LEDGER_DIGEST={json.dumps(header.get('content_digest',''))};
{JS}
</script>
</body></html>"""
    out.write_text(page)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--ledger", required=True, type=Path)
    ap.add_argument("--capture", required=True, type=Path, help="dir of nodes-*.json")
    ap.add_argument("--audit", type=Path, help="json with frames[].nodeId/.route")
    ap.add_argument("--shots", type=Path)
    ap.add_argument("--renders", type=Path, help="dir of <node-id>.png")
    ap.add_argument("--current-markers", nargs="*", default=list(DEFAULT_CURRENT))
    ap.add_argument("--demotion-markers", nargs="*", default=list(DEFAULT_DEMOTION))
    ap.add_argument("--only", nargs="*")
    ap.add_argument("--tracker", type=Path,
                    help="route tracker json. Routes present here but absent from the ledger are "
                         "emitted as no-frame entries WITH A REASON, so a route that is not "
                         "reviewable is visibly not reviewable rather than silently absent.")
    ap.add_argument("--disclaimed-from", type=Path,
                    help="signature evidence json naming excludedSelfDisclaimingArtifacts")
    ap.add_argument("--asset-base", help="serve images from this URL prefix instead of inlining "
                                         "them (use with serve-sign-review.py)")
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--format", choices=("html", "json"), default="html",
                    help="json emits frames.json for the API server; html bakes a snapshot")
    a = ap.parse_args()

    led = parse_frames_ledger(a.ledger)
    nodes = load_capture(a.capture)
    routes = {}
    if a.audit and a.audit.exists():
        routes = {str(r["nodeId"]): r.get("route", "")
                  for r in json.loads(a.audit.read_text()).get("frames", [])}

    shot_idx = index_shots(a.shots)
    frames = []
    for nid in (a.only or list(led["rows"])):
        row = led["rows"].get(nid)
        if not row:
            print(f"  skip {nid}: not in ledger", file=sys.stderr)
            continue
        doc = nodes.get(nid)
        route = routes.get(nid) or row["frame_name"] or nid
        shot, shot_refusal = find_shipped(shot_idx, route)
        rend = (a.renders / f"{nid.replace(':', '-')}.png") if a.renders else None
        rend = rend if rend and rend.exists() else None
        frames.append({
            **row, "route": route, "shot_refusal": shot_refusal,
            "candidates": candidates(doc, row["regions"], tuple(a.current_markers),
                                     tuple(a.demotion_markers)) if doc else [],
            "shipped": asset_src(shot, a.asset_base, "shots"),
            "shipped_digest": sha256_file(shot) if shot else None,
            "render": asset_src(rend, a.asset_base, "renders"),
            "frame_image_digest": sha256_file(rend) if rend else None,
        })

    # Every route the estate knows about, not merely every route with a frame. An absence
    # that is not on the page reads as "nothing to see"; an absence with its reason on the
    # page reads as what it is.
    if a.tracker and a.tracker.exists():
        tr = json.loads(a.tracker.read_text())
        tr = tr if isinstance(tr, list) else tr.get("routes", [])
        # The tracker already groups these into 17 real families. Grouping by first path
        # segment instead produced 47 groups, 26 of them a single route, and scattered
        # the marketing site across five of them.
        meta = {r.get("route"): r for r in tr if r.get("route")}
        for f in frames:
            m = meta.get(f["route"]) or {}
            f["family"] = m.get("family") or "unindexed"
            f["title"] = m.get("title") or ""
            f["tracker_status"] = m.get("status")
            f["tracker_tier"] = m.get("tier")
        disclaimed = set()
        if a.disclaimed_from and a.disclaimed_from.exists():
            sig = json.loads(a.disclaimed_from.read_text())
            disclaimed = {x.get("route") for x in sig.get("excludedSelfDisclaimingArtifacts", [])}
            ineligible = (sig.get("parityIneligibleFrame") or {}).get("route")
            if ineligible:
                disclaimed.add(ineligible)
        have = {f["route"] for f in frames}
        for row in tr:
            route = row.get("route")
            if not route or route in have:
                continue
            if route in disclaimed:
                why = ("The frame draws itself as NOT SOURCE-CURRENT, so it states no contract. "
                       "Comparing a page to it would measure a difference that means nothing.")
                todo = "Redraw the frame as current, then it becomes reviewable."
            elif str(row.get("status", "")).upper() == "RETIRED" or str(row.get("tier", "")).upper() == "RETIRE":
                why = "The route is retired. No frame is drawn for it and none is owed."
                todo = "Nothing. It leaves the estate; it is here so its absence is visible."
            else:
                why = "No frame node is recorded for this route in the frames ledger."
                todo = "Draw the frame, capture it, and re-derive, or record why it needs none."
            frames.append({
                "node_id": f"noframe:{route}", "route": route, "kind": "no-frame",
                "reason": why, "remedy": todo,
                "tracker_status": row.get("status"), "tracker_tier": row.get("tier"),
                "family": row.get("family"), "title": row.get("title"),
                "candidates": [], "regions": [], "columns": None, "authority_by": "no frame",
                "authority_layer": None, "content_digest": None, "disclaimed": route in disclaimed,
                "truncated": False, "shot_refusal": None,
                "render": None, "shipped": None,
                "frame_image_digest": None, "shipped_digest": None,
            })

    frames.sort(key=lambda f: f["route"])
    if a.format == "json":
        # The API serves this. Images are referenced by FILE NAME, never inlined: the
        # server owns how they are addressed, so the data outlives any URL scheme.
        for f in frames:
            f["render_file"] = f"{f['node_id'].replace(':', '-')}.png" if f["render"] else None
            f["shipped_file"] = (f["shipped"].rsplit("/", 1)[-1] if f["shipped"] else None)
            f.pop("render", None)
            f.pop("shipped", None)
            f["kind"] = f.get("kind") or "frame"
            f["flagged"] = any(c["marker"] == "demoted" or c["cloned_from"] for c in f["candidates"])
            f["blocked"] = ([f["reason"]] if f["kind"] == "no-frame" else []) + [b for b in [
                "capture truncated" if f["truncated"] else None,
                f["shot_refusal"] or ("no shipped screenshot" if not f["shipped_file"] else None),
                "frame not rendered" if not f["render_file"] else None] if b]
        a.out.write_text(json.dumps({
            "documentType": "vds.sign-review.frames", "schemaVersion": 1,
            "header": led["header"], "builtFrom": str(a.ledger),
            "frames": frames}, indent=1))
        print(f"wrote {a.out}  ({len(frames)} frames · "
              f"{sum(1 for f in frames if f['flagged'])} flagged · "
              f"{sum(1 for f in frames if f['blocked'])} blocked)")
        return 0
    build(frames, led["header"], a.out, led["header"].get("file_key", "?"))
    alert = sum(1 for f in frames if any(c["marker"] == "demoted" or c["cloned_from"]
                                         for c in f["candidates"]))
    blocked = sum(1 for f in frames if f["render"] is None or f["shipped"] is None or f["truncated"])
    print(f"wrote {a.out}  ({len(frames)} frames · {alert} carrying machine-authored content · "
          f"{blocked} with signing BLOCKED on a missing artefact)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
