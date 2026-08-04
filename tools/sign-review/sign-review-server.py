#!/usr/bin/env python3
"""The sign-review service: an API, a small client, and a durable decision log.

    python3 sign-review-server.py --frames frames.json --renders ./renders \
        --shots ./shots --log ./decisions.jsonl [--host 0.0.0.0] [--port 8787]

Replaces the baked-snapshot page, which had three defects:
  1. it went stale silently when the ledger was re-derived;
  2. decisions lived in browser memory until a final submit, so a closed tab lost them;
  3. it could not resume or show what had already been decided.

FOUR PROPERTIES THIS SERVICE HAS AND THE SNAPSHOT DID NOT

  STALENESS IS FAIL-CLOSED. Every decision carries the ledger digest it was made against.
  The server rejects a decision whose digest is not the one it is currently serving. A
  page left open across a re-derivation cannot silently record against yesterday's reading.

  DECISIONS ARE DURABLE ON ARRIVAL. One POST per decision, appended to a JSONL log as it
  is made. Nothing is batched and nothing is held in a tab.

  THE LOG IS APPEND-ONLY. A changed mind is a new line that supersedes, never an edit. The
  history of what was decided and when is the point; a log that can be rewritten is not one.

  IT CREATES NO AUTHORITY. Order 16. This records decisions. The `vds` CLI turns the log
  into a recorded Principal act, and only that act creates authority.

AUTH. A token is required for every request and is minted at startup unless one is
supplied. This binds to a tailnet by default, where any device could otherwise post a
decision that becomes input to a Principal act. It is a shared secret over a private
network, not a credential system, and it is not a substitute for one.
"""

from __future__ import annotations

import argparse
import hashlib
import hmac
import json, os, sqlite3, tempfile, threading, urllib.request, urllib.error
import secrets
import sys
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse, parse_qs, unquote, quote

MAX_BODY = 4 * 1024 * 1024
VALID = {"sign", "refuse", "defer"}

APP = r"""<!doctype html><html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<meta name="color-scheme" content="light dark">
<!-- The token is in this page's URL. The Figma links navigate off-site; without this the
     browser would hand figma.com the referring URL and the shared secret with it. -->
<meta name="referrer" content="no-referrer"><title>Sign review</title><style>
:root{--bg:#fff;--fg:#111;--mute:#4d4d4d;--line:rgba(0,0,0,.11);--warn:#fc0035;--ok:#28a948;--accent:#006bff;--surface:#fafafa;--rail:220px}
@media(prefers-color-scheme:dark){:root{--bg:#0a0a0a;--fg:#ededed;--mute:#a1a1a1;--line:rgba(255,255,255,.13);--surface:#131313}}
*{box-sizing:border-box}html,body{height:100%}
body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.5 ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif;display:flex;min-height:100vh}
.mono{font-family:ui-monospace,Menlo,monospace;font-variant-numeric:tabular-nums}
.sub{color:var(--mute);font-size:12px}
button{font:inherit;padding:8px 13px;min-height:38px;border-radius:7px;border:1px solid var(--line);background:var(--bg);color:var(--fg);cursor:pointer}
button:hover{border-color:var(--fg)}
button.p{background:var(--fg);color:var(--bg);border-color:var(--fg);font-weight:600}
button:disabled{opacity:.35;cursor:not-allowed;background:var(--bg);color:var(--mute);border-color:var(--line)}
button.chip{min-height:0;padding:4px 10px;font-size:12px;border-radius:20px}
button.chip[aria-pressed=true]{background:var(--fg);color:var(--bg);border-color:var(--fg)}
button.nav{min-width:42px;padding:8px 11px;font-size:17px;line-height:1}

/* ---- rail: 220px flat, upper/lower groups, 1px divider. The canonical shell. ---- */
nav{width:var(--rail);flex:none;border-right:1px solid var(--line);background:var(--surface);
 display:flex;flex-direction:column;position:sticky;top:0;height:100vh;overflow-y:auto}
nav .brand{padding:14px 14px 10px;font-weight:600;font-size:14px;border-bottom:1px solid var(--line)}
nav .grp{padding:10px 8px 4px;font-size:10.5px;letter-spacing:.07em;text-transform:uppercase;color:var(--mute)}
nav a{display:flex;gap:8px;align-items:center;padding:7px 12px;margin:1px 6px;border-radius:6px;
 cursor:pointer;text-decoration:none;color:var(--fg);font-size:13.5px}
nav a:hover{background:color-mix(in srgb,var(--fg) 6%,transparent)}
nav a[aria-current=true]{background:var(--fg);color:var(--bg);font-weight:600}
nav a .n{flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
nav a .c{font-size:11px;opacity:.7;font-variant-numeric:tabular-nums}
nav .foot{margin-top:auto;padding:10px 14px;border-top:1px solid var(--line);font-size:11px;color:var(--mute)}

main{flex:1;min-width:0;display:flex;flex-direction:column}
header{position:sticky;top:0;z-index:9;background:var(--bg);border-bottom:1px solid var(--line);
 padding:10px 16px;display:flex;gap:9px;align-items:center;flex-wrap:wrap}
header h1{font-size:15px;margin:0;font-weight:600}
.body{padding:16px;max-width:1400px;width:100%}

/* ---- cards ---- */
.cards{display:grid;gap:12px;grid-template-columns:repeat(auto-fill,minmax(230px,1fr))}
.card{border:1px solid var(--line);border-radius:9px;background:var(--bg);overflow:hidden;
 cursor:pointer;text-align:left;padding:0;display:flex;flex-direction:column;min-height:0}
.card:hover{border-color:var(--fg)}
.card .thumb{aspect-ratio:14/9;background:var(--surface);border-bottom:1px solid var(--line);
 display:flex;align-items:center;justify-content:center;overflow:hidden}
.card .thumb img{width:100%;height:100%;object-fit:cover;object-position:top left;display:block}
.card .thumb .none{font-size:11px;color:var(--mute);padding:8px;text-align:center}
.card .meta{padding:9px 11px;display:flex;flex-direction:column;gap:5px;flex:1}
.card .rt{font-size:13.5px;font-weight:500;word-break:break-all;line-height:1.35}
.card .tl{font-size:11.5px;color:var(--mute)}
.card .tags{display:flex;gap:4px;flex-wrap:wrap;margin-top:auto;padding-top:4px}
.tag{font-size:10.5px;padding:2px 6px;border-radius:4px;border:1px solid var(--line);color:var(--mute);white-space:nowrap}
.tag.bad{border-color:var(--warn);color:var(--warn)}.tag.ok{border-color:var(--ok);color:var(--ok)}

/* ---- detail ---- */
.bar{display:flex;gap:8px;align-items:center;margin-bottom:12px;flex-wrap:wrap}
.detail{display:grid;grid-template-columns:minmax(0,1fr);gap:14px;align-items:start}
@media(min-width:1080px){.detail{grid-template-columns:minmax(0,1fr) 372px}}
.signrail{position:sticky;top:62px;max-height:calc(100vh - 74px);overflow-y:auto;
 border:1px solid var(--line);border-radius:9px;background:var(--surface);padding:12px}
.signrail h2{margin:0 0 10px;font-size:12px;letter-spacing:.06em;text-transform:uppercase;color:var(--mute)}
.signrail fieldset{background:var(--bg);margin-bottom:10px}
.signrail fieldset:last-of-type{margin-bottom:10px}
@media(max-width:1079px){.signrail{position:static;max-height:none}}
.bar .t{flex:1;min-width:0}.bar .t b{display:block;font-size:18px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.panes{display:grid;grid-template-columns:1fr;gap:1px;background:var(--line);border:1px solid var(--line);border-radius:9px;overflow:hidden;margin-bottom:12px}
@media(min-width:640px){.panes{grid-template-columns:1fr 1fr}}
a.fig{color:var(--accent);text-decoration:none;word-break:break-all}
a.fig:hover{text-decoration:underline}
.panes.swap .pane:first-child{order:2}.panes.swap .pane:last-child{order:1}
.pane{background:var(--bg);padding:12px}
.pane h2{margin:0 0 8px;font-size:11px;letter-spacing:.06em;text-transform:uppercase;color:var(--mute)}
.pane img{width:100%;height:auto;border:1px solid var(--line);border-radius:5px;display:block;cursor:zoom-in;background:#fff}
.pane img:hover{outline:2px solid var(--accent);outline-offset:1px}
.zoomhint{font-size:11px;color:var(--mute);margin:5px 0 0}
.missing{border:1px dashed var(--warn);color:var(--warn);border-radius:5px;padding:28px 12px;text-align:center;font-size:13px}
.kvwrap,fieldset{border:1px solid var(--line);border-radius:9px;padding:12px 14px;margin:0 0 12px}
table.kv{width:100%;font-size:13px}table.kv td{padding:2px 0}table.kv td:first-child{color:var(--mute);width:46%}
.warn{border:1px solid var(--warn);background:color-mix(in srgb,var(--warn) 7%,transparent);border-radius:9px;padding:11px 13px;margin-bottom:12px;font-size:13px}
.warn b{display:block;margin-bottom:3px}
legend{font-size:11px;letter-spacing:.06em;text-transform:uppercase;color:var(--mute);padding:0 5px}
.help{margin:0 0 10px;font-size:13px;color:var(--mute);line-height:1.45}
label.opt{display:flex;gap:10px;align-items:flex-start;padding:9px;border-bottom:1px solid var(--line);cursor:pointer;border-radius:6px}
label.opt:last-child{border-bottom:0}
label.opt:hover{background:color-mix(in srgb,var(--accent) 5%,transparent)}
label.opt:has(input:checked){background:color-mix(in srgb,var(--accent) 9%,transparent)}
label.opt input{width:20px;height:20px;flex:none;margin:1px 0 0}
.opt .nm{word-break:break-word}.opt .meta{font-size:12px;color:var(--mute)}
textarea{width:100%;min-height:64px;padding:9px;border-radius:7px;border:1px solid var(--line);background:var(--bg);color:var(--fg);font:inherit}
.acts{display:flex;gap:8px;flex-wrap:wrap}.acts button{flex:1 1 auto}
.msg{font-size:13px;padding:9px 12px;border-radius:7px;border:1px solid var(--line);margin-top:10px}
.msg.err{border-color:var(--warn);color:var(--warn)}.msg.ok{border-color:var(--ok);color:var(--ok)}
#lb{position:fixed;inset:0;background:rgba(0,0,0,.93);display:none;align-items:center;justify-content:center;z-index:99;cursor:zoom-out;padding:10px}
#lb.on{display:flex}#lb img{max-width:100%;max-height:100%;object-fit:contain}
#lb.full{align-items:flex-start;overflow:auto}#lb.full img{max-height:none;width:100%;object-fit:fill}
#lbbar{position:fixed;top:8px;right:10px;z-index:100;display:flex;gap:6px}
#lbbar button{min-height:32px;padding:4px 11px;font-size:12.5px}

@media(max-width:820px){
 body{flex-direction:column}
 nav{width:100%;height:auto;position:static;flex-direction:row;overflow-x:auto;border-right:0;border-bottom:1px solid var(--line)}
 nav .brand,nav .grp,nav .foot{display:none}
 nav a{margin:6px 3px;white-space:nowrap}
 .body{padding:11px}.cards{grid-template-columns:repeat(auto-fill,minmax(150px,1fr));gap:9px}
 button{min-height:42px}
}
</style></head><body>
<nav id="rail"></nav>
<main>
 <header><h1 id="crumb">Sign review</h1><span id="hdr" class="sub"></span>
  <span style="flex:1"></span><span id="filters"></span></header>
 <div class="body" id="app">loading&hellip;</div>
</main>
<div id="lb" onclick="if(event.target.id==='lb')closeLb()"><span id="lbbar">
 <button onclick="event.stopPropagation();document.getElementById('lb').classList.toggle('full')">fit / actual size</button>
 <button onclick="event.stopPropagation();closeLb()">close</button></span><img id="lbi" alt=""></div>
<script>
const K=new URLSearchParams(location.search).get('k')||sessionStorage.getItem('k')||'';
if(K)sessionStorage.setItem('k',K);
const api=async(p,o={})=>{const r=await fetch(p,{...o,headers:{'X-Auth':K,'Content-Type':'application/json',...(o.headers||{})}});
 if(r.status===401)throw new Error('unauthorised - use the URL the server printed');
 if(!r.ok){let m;try{m=(await r.json()).error}catch(e){m='HTTP '+r.status}throw new Error(m)}return r.json()};
// Escapes for BOTH contexts this interpolates into: HTML text/attributes, and JS string
// literals inside onclick="...('...')". The apostrophe is the one that matters for the second.
const esc=s=>String(s==null?'':s).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const fam=f=>(f&&f.family)||'unindexed';
// Figma resolves /design/<key>/?node-id=<a-b> without the name slug, and selects the node.
const figUrl=id=>{const k=STATE.header&&STATE.header.file_key;if(!k||!id)return null;
 return `https://www.figma.com/design/${k}/?node-id=${encodeURIComponent(String(id).replace(':','-'))}`};
let STATE={},FRAMES=[],DEC={},filter='all',SECT='__all',CUR=null;
let SWAP=localStorage.getItem('swapSides')==='1';
function swapSides(){SWAP=!SWAP;localStorage.setItem('swapSides',SWAP?'1':'0');
 const el=document.getElementById('panes');if(el)el.classList.toggle('swap',SWAP);}

const decidable=f=>f.kind!=='no-frame';
const passFilter=f=>filter==='all'||(filter==='flagged'&&f.flagged)||(filter==='blocked'&&(f.blocked||[]).length)
 ||(filter==='noframe'&&f.kind==='no-frame')||(filter==='todo'&&!DEC[f.node_id]&&f.kind!=='no-frame')
 ||(filter==='decided'&&!!DEC[f.node_id]);
const inSect=f=>SECT==='__all'||fam(f)===SECT;
const shown=()=>FRAMES.filter(f=>passFilter(f)&&inSect(f));
// Every decidable route across ALL sections, in rail order (family, then route). This is the
// sequence auto-advance walks, so the end of a section is not the end of the work.
const allOrder=()=>FRAMES.filter(f=>passFilter(f)&&decidable(f))
 .slice().sort((a,b)=>fam(a).localeCompare(fam(b))||String(a.route).localeCompare(String(b.route)));

function rail(){
 const fams={};FRAMES.filter(passFilter).forEach(f=>{const k=fam(f),o=(fams[k]=fams[k]||{n:0,d:0,x:0});
 if(!decidable(f)){o.x++;return}o.n++;if(DEC[f.node_id])o.d++});
 const keys=Object.keys(fams).sort();
 const tot=allOrder().length, done=allOrder().filter(f=>DEC[f.node_id]).length;
 document.getElementById('rail').innerHTML=
  `<div class="brand">Sign review</div><div class="grp">All</div>`+
  `<a onclick="go('__all')" aria-current="${SECT==='__all'}"><span class="n">Every route</span><span class="c">${done}/${tot}</span></a>`+
  `<div class="grp">Families</div>`+
  keys.map(k=>{const o=fams[k],done=o.n&&o.d===o.n;
   return `<a onclick="go('${esc(k)}')" aria-current="${SECT===k}"><span class="n">${done?'&check; ':''}${esc(k)}</span>`+
    `<span class="c">${o.d}/${o.n}${o.x?` <span title="${o.x} not decidable">+${o.x}</span>`:''}</span></a>`}).join('')+
  `<div class="foot">${FRAMES.length} routes<br>${esc(STATE.header&&STATE.header.captured_at||'')}</div>`;
}
function go(k){SECT=k;CUR=null;rail();list();window.scrollTo(0,0)}
function chips(){const n=FRAMES.length,fl=FRAMES.filter(x=>x.flagged).length,
 bl=FRAMES.filter(x=>(x.blocked||[]).length).length,nf=FRAMES.filter(x=>x.kind==='no-frame').length,
 td=FRAMES.filter(x=>!DEC[x.node_id]&&x.kind!=='no-frame').length;
 const dn=FRAMES.filter(x=>DEC[x.node_id]).length;
 document.getElementById('filters').innerHTML=[['all','All '+n],['todo','To do '+td],['decided','Decided '+dn],['flagged','Machine '+fl],['blocked','Blocked '+bl],['noframe','No frame '+nf]]
  .map(([k,l])=>`<button class="chip" aria-pressed="${filter===k}" onclick="filter='${k}';chips();rail();list()">${l}</button>`).join(' ')}
function tags(f){const t=[];if(DEC[f.node_id])t.push(`<span class="tag ok">${esc(DEC[f.node_id].decision)}</span>`);
 if(f.kind==='no-frame'){t.push(`<span class="tag">${esc(f.tracker_tier||'no frame')}</span>`);
  // Say which picture this is. An unlabelled shipped page in a grid of frames reads as a frame.
  if(f.shot_url)t.push('<span class="tag">shipped page only</span>');
  return t.join('')}
 if(f.flagged)t.push('<span class="tag bad">machine</span>');
 if((f.blocked||[]).length)t.push('<span class="tag bad">blocked</span>');return t.join('')}
function thumb(f){
 if(f.kind==='no-frame')return f.shot_url
  ?`<img loading="lazy" alt="" src="${esc(f.shot_url)}">`
  :`<div class="none">no frame<br><span class="sub">${esc(f.tracker_tier||'')}</span></div>`;
 // The server sends the real filename. Re-deriving it here would be a second copy of a naming
 // rule, and a divergence would show as a blank card rather than as the bug it is.
 if(!f.thumb_url)return '<div class="none">not rendered</div>';
 return `<img loading="lazy" alt="" src="${esc(f.thumb_url)}"
  onerror="this.replaceWith(Object.assign(document.createElement('div'),{className:'none',textContent:'not rendered'}))">`;
}
function list(){CUR=null;
 const f=shown();
 document.getElementById('crumb').textContent = SECT==='__all'?'Every route':SECT;
 document.getElementById('app').innerHTML =
  `<p class="sub" style="margin:0 0 12px">${f.length} route${f.length===1?'':'s'}${f.length<FRAMES.length?` &middot; filtered from ${FRAMES.length}`:''}</p>`+
  (f.length?`<div class="cards">`+f.map(x=>`<button class="card" onclick="open_('${esc(x.node_id)}')">
    <span class="thumb">${thumb(x)}</span>
    <span class="meta"><span class="rt">${esc(x.route)}</span>
     ${x.title?`<span class="tl">${esc(x.title)}</span>`:''}
     <span class="tags">${tags(x)}</span></span></button>`).join('')+`</div>`
   :'<p class="sub">Nothing matches this filter.</p>');
}
// Move to the next UNDECIDED route, crossing into the next section when this one is done.
// Scans forward, then wraps and scans the head, so "all done" is only ever said when nothing
// anywhere is undecided - not merely when the tail happens to be complete.
function advance(fromId){
 const o=allOrder(); if(!o.length)return allDone();
 const i=o.findIndex(x=>x.node_id===fromId);
 const order=i<0?o:o.slice(i+1).concat(o.slice(0,i+1));
 const nx=order.find(x=>!DEC[x.node_id]);
 if(!nx)return allDone();
 if(fam(nx)!==SECT&&SECT!=='__all'){SECT=fam(nx);rail()}
 open_(nx.node_id);
}
function allDone(){
 CUR=null;rail();
 const d=allOrder(),c={sign:0,refuse:0,defer:0};
 d.forEach(f=>{const v=DEC[f.node_id];if(v)c[v.decision]=(c[v.decision]||0)+1});
 const nf=FRAMES.filter(f=>passFilter(f)&&!decidable(f)).length;
 document.getElementById('crumb').textContent='All done';
 document.getElementById('app').innerHTML=`<div class="kvwrap" style="text-align:center;padding:36px 18px">
  <p style="font-size:26px;margin:0 0 6px">&check; All done</p>
  <p class="sub" style="margin:0 0 18px">Every route in scope has a decision.</p>
  <table class="kv" style="max-width:320px;margin:0 auto;text-align:left">
   <tr><td>signed</td><td class="mono">${c.sign||0}</td></tr>
   <tr><td>refused</td><td class="mono">${c.refuse||0}</td></tr>
   <tr><td>deferred</td><td class="mono">${c.defer||0}</td></tr>
   <tr><td>decided in total</td><td class="mono">${d.filter(f=>DEC[f.node_id]).length} / ${d.length}</td></tr>
   ${nf?`<tr><td>not decidable (no frame)</td><td class="mono">${nf}</td></tr>`:''}</table>
  <p style="margin:20px 0 0"><button onclick="SECT='__all';filter='all';chips();rail();list()">Review everything</button></p></div>`;
}
// The left pane shows either the whole frame or ONE candidate layer, rendered on demand from
// Figma and cached. Repainted on its own so selecting a radio never disturbs the rest of the form.
let LAYER=null;
function paintFrame(){
 const f=window._f,el=document.getElementById('figpane');if(!f||!el)return;
 if(!LAYER){
  el.innerHTML=`<h2>Frame &mdash; what Figma draws</h2>`+
   (f.render_url?`<img src="${esc(f.render_url)}" alt="frame" loading="lazy" onclick="big('${esc(f.render_url)}')">
    <p class="zoomhint">click to enlarge &middot; select a layer below to view it alone</p>`
    :'<div class="missing">This frame has not been rendered from Figma.</div>');
  return}
 const c=(f.candidates||[]).find(x=>x.id===LAYER)||{};
 const u=`/layers/${encodeURIComponent(LAYER)}.png?k=${encodeURIComponent(K)}`;
 el.innerHTML=`<h2>Layer &mdash; <span class="mono">${esc(c.name||LAYER)}</span></h2>
  <div id="figwait" class="missing" style="border-style:solid;border-color:var(--line);color:var(--mute)">rendering this layer&hellip;</div>
  <img id="figimg" src="${u}" alt="layer" style="display:none"
   onload="this.style.display='block';const w=document.getElementById('figwait');if(w)w.remove()"
   onclick="big('${u}')" onerror="layerFailed('${esc(LAYER)}')">
  <p class="zoomhint">${c.visible===false?'<b style="color:var(--warn)">this layer is hidden in Figma</b> &middot; ':''}
   <a href="#" onclick="LAYER=null;paintFrame();return false">show the whole frame</a></p>`;
}
// An <img> cannot read a JSON body, so on failure ask the same URL again to get the stated
// reason. A blank pane that says nothing is the thing worth avoiding.
async function layerFailed(id){
 const el=document.getElementById('figpane');let why='could not be rendered';
 try{await api(`/layers/${encodeURIComponent(id)}.png`)}catch(e){why=e.message}
 const c=(window._f.candidates||[]).find(x=>x.id===id)||{};
 el.innerHTML=`<h2>Layer &mdash; <span class="mono">${esc(c.name||id)}</span></h2>
  <div class="missing">${esc(why)}</div>
  <p class="zoomhint"><a href="#" onclick="LAYER=null;paintFrame();return false">show the whole frame</a></p>`;
}
function pickLocus(el){LAYER=el.dataset.nid||null;paintFrame()}
function big(u){if(!u)return;document.getElementById('lbi').src=u;
 const lb=document.getElementById('lb');lb.classList.remove('full');lb.classList.add('on')}
function closeLb(){document.getElementById('lb').classList.remove('on','full')}

async function open_(id){
 const f=await api('/api/frames/'+encodeURIComponent(id));CUR=id;
 const sib=shown(),i=sib.findIndex(x=>x.node_id===id);
 document.getElementById('crumb').textContent=f.route;
 const nav=`<button onclick="list()">&larr; ${esc(fam(f))}</button>
  <span class="t"><b>${esc(f.route)}</b><span class="sub mono">${esc(f.node_id)} &middot; ${esc(f.authority_by)}</span></span>
  <button class="nav" ${i<=0?'disabled':''} onclick="open_('${i>0?esc(sib[i-1].node_id):''}')">&lsaquo;</button>
  <span class="sub">${i+1}/${sib.length}</span>
  <button class="nav" ${i<0||i>=sib.length-1?'disabled':''} onclick="open_('${i<sib.length-1?esc(sib[i+1].node_id):''}')">&rsaquo;</button>`;
 if(f.kind==='no-frame'){
  document.getElementById('app').innerHTML=`<div class="bar">${nav}</div>
   <div class="warn"><b>There is no frame for this route</b>${esc(f.reason)}</div>
   ${f.shipped_url?`<div class="panes"><div class="pane">
     <h2>Output &mdash; what the app renders today</h2>
     <img src="${esc(f.shipped_url)}" alt="served page" loading="lazy" onclick="big('${esc(f.shipped_url)}')">
     <p class="zoomhint">click to enlarge &middot; there is no frame to compare this against, but
      what the app renders is what decides whether a frame is owed at all</p></div></div>`
    :`<div class="kvwrap"><p class="sub" style="margin:0">No shipped screenshot was captured for
      this route either${f.shot_refusal?`: ${esc(f.shot_refusal)}`:''}.</p></div>`}
   <fieldset><legend>Why it is not reviewable</legend><table class="kv">
    <tr><td>tracker status</td><td class="mono">${esc(f.tracker_status||'-')}</td></tr>
    <tr><td>tracker tier</td><td class="mono">${esc(f.tracker_tier||'-')}</td></tr>
    <tr><td>self-disclaims</td><td class="mono">${f.disclaimed?'yes':'no'}</td></tr></table></fieldset>
   <fieldset><legend>What happens instead</legend><p style="margin:0">${esc(f.remedy||'')}</p></fieldset>
   <p class="sub">Nothing can be signed here. It is listed so its absence is visible and explained.</p>`;
  window._f=f;return}
 const prev=DEC[id],dem=f.candidates.filter(c=>c.marker==='demoted'),cl=f.candidates.filter(c=>c.cloned_from);
 // "Not available" is not a reason. Say which artefact is absent and why, so a gap is
 // chaseable rather than just blank.
 const img=(u,a,why)=>u?`<img src="${u}" alt="${esc(a)}" loading="lazy" onclick="big('${u}')">`
  :`<div class="missing">${esc(why)}</div>`;
 const fu=figUrl(f.node_id);
 document.getElementById('app').innerHTML=`
  <div class="bar">${nav}<button class="chip" onclick="swapSides()">&#8646; swap sides</button></div>
  ${dem.length||cl.length?`<div class="warn"><b>Machine-authored content in this frame</b>
   ${dem.length?`Demoted: ${dem.map(c=>esc(c.name)).join(', ')}.`:''}
   ${cl.length?`Cloned: ${cl.map(c=>esc(c.name)).join(', ')}.`:''}</div>`:''}
  <div class="detail">
   <div>
    <div class="panes${SWAP?' swap':''}" id="panes">
     <div class="pane" id="figpane"></div>
     <div class="pane"><h2>Output &mdash; what the app renders</h2>${img(f.shipped_url,'served page',
      f.shot_refusal||'No screenshot of this route was captured, so there is nothing to compare the frame against.')}
      ${f.shipped_url?'<p class="zoomhint">click to enlarge</p>':''}</div></div>
    <div class="kvwrap"><table class="kv">
     <tr><td>node in Figma</td><td>${fu?`<a class="fig mono" href="${fu}" target="_blank" rel="noopener noreferrer">${esc(f.node_id)} &nearr;</a>`:`<span class="mono">${esc(f.node_id)}</span>`}</td></tr>
     <tr><td>frame name</td><td class="mono">${esc(f.frame_name||'-')}</td></tr>
     <tr><td>authority resolves by</td><td class="mono">${esc(f.authority_by)}</td></tr>
     <tr><td>regions the frame declares</td><td class="mono">${f.regions.join(', ')||'none'}</td></tr>
     <tr><td>content columns</td><td class="mono">${esc(f.columns||'-')}</td></tr>
     <tr><td>frame self-disclaims</td><td class="mono">${f.disclaimed?'yes':'no'}</td></tr>
     <tr><td>frame digest</td><td class="mono" style="word-break:break-all;font-size:11px">${esc(f.content_digest||'-')}</td></tr>
     <tr><td>Figma file</td><td class="mono" style="font-size:11px">${esc((STATE.header&&STATE.header.file_key)||'-')}</td></tr></table></div>
   </div>
   <form class="signrail" onsubmit="return submitForm(event)">
    <h2>Sign this route</h2>
    <fieldset><legend>1 &middot; Design contract</legend>
     <p class="help">Pick the layer that <b>is the design</b> for this route. Your choice is the
      declaration; the tool's reading is only <span class="tag">proposed</span> and never pre-selected.</p>
     ${f.candidates.map((c,n)=>{const cu=figUrl(c.id),was=prev&&prev.selectedLocus&&prev.selectedLocus.id===c.id;
      return `<label class="opt"><input type="radio" name="loc" value="${n}" required
       data-nid="${esc(c.id)}" onchange="pickLocus(this)" ${was?'checked':''}>
      <span><span class="nm mono">${esc(c.name)}</span>
      ${c.name===f.authority_layer?'<span class="tag">proposed</span>':''}
      ${was?'<span class="tag ok">your last choice</span>':''}
      ${c.visible?'':'<span class="tag bad">hidden</span>'}
      ${c.marker==='demoted'?'<span class="tag bad">demoted</span>':''}
      ${c.cloned_from?'<span class="tag bad">cloned</span>':''}
      <br><span class="meta">${c.nodes} nodes &middot; ${c.texts} texts &middot; ${c.w}&times;${c.h}
      ${cu?` &middot; <a class="fig" href="${cu}" target="_blank" rel="noopener noreferrer" onclick="event.stopPropagation()">open &nearr;</a>`:''}</span></span></label>`}).join('')}
    </fieldset>
    <fieldset><legend>2 &middot; Decision</legend>
     <p class="help"><b>Sign</b> adopts it as the contract. <b>Refuse</b> rejects it. <b>Defer</b> parks
      it. Refuse and Defer need a comment.</p>
     <label class="opt"><input type="radio" name="dec" value="sign" ${(f.blocked||[]).length?'disabled':''} required ${prev&&prev.decision==='sign'?'checked':''}><span>Sign</span></label>
     <label class="opt"><input type="radio" name="dec" value="refuse" ${prev&&prev.decision==='refuse'?'checked':''}><span>Refuse</span></label>
     <label class="opt"><input type="radio" name="dec" value="defer" ${prev&&prev.decision==='defer'?'checked':''}><span>Defer</span></label>
     ${(f.blocked||[]).length?`<p class="sub" style="color:var(--warn)">Cannot sign: ${esc(f.blocked.join('; '))}.</p>`:''}
    </fieldset>
    <fieldset><legend>3 &middot; Comment</legend>
     <textarea name="cm" placeholder="optional when signing, required to refuse or defer">${prev&&prev.comment?esc(prev.comment):''}</textarea></fieldset>
    ${prev?`<p class="msg">Currently <b>${esc(prev.decision)}</b> as decision #${esc(prev.seq)}
      &middot; <a href="#" onclick="showHistory('${esc(f.node_id)}');return false">history</a><br>
      <span class="sub">Your previous choice is pre-selected. Recording again supersedes it;
      the earlier decision stays in the record.</span></p>`:''}
    <div class="acts"><button class="p" type="submit">${prev?'Update decision':'Record decision'}</button></div>
    <div id="out"></div><div id="hist"></div></form></div>`;
 LAYER=(prev&&prev.selectedLocus&&prev.selectedLocus.id)||null;
 window._f=f;paintFrame()}

let BUSY=false;
async function showHistory(id){
 const h=document.getElementById('hist');h.innerHTML='<p class="sub">loading&hellip;</p>';
 try{const r=await api('/api/history/'+encodeURIComponent(id));
  h.innerHTML='<table class="kv" style="margin-top:8px">'+r.history.map(x=>
   `<tr><td>#${esc(x.seq)} ${esc((x.recordedAt||'').slice(0,16).replace('T',' '))}</td>
    <td><b>${esc(x.decision)}</b>${x.selectedLocus?` <span class="mono sub">${esc(x.selectedLocus.name)}</span>`:''}
    ${x.comment?`<br><span class="sub">${esc(x.comment)}</span>`:''}</td></tr>`).join('')+'</table>';
 }catch(err){h.innerHTML='<p class="msg err">'+esc(err.message)+'</p>'}}

async function submitForm(e){
 e.preventDefault();if(BUSY)return false;
 const f=window._f,fd=new FormData(e.target),out=document.getElementById('out');
 const btn=e.target.querySelector('button[type=submit]');
 const dec=fd.get('dec'),cm=(fd.get('cm')||'').trim(),loc=f.candidates[+fd.get('loc')];
 const prevDec=DEC[f.node_id];
 if(dec!=='sign'&&!cm){out.innerHTML='<p class="msg err">Comment required.</p>';return false}
 out.innerHTML='<p class="msg">Recording&hellip;</p>';BUSY=true;if(btn)btn.disabled=true;
 try{const r=await api('/api/decisions',{method:'POST',body:JSON.stringify({
   nodeId:f.node_id,route:f.route,decision:dec,comment:cm||null,
   selectedLocus:{id:loc.id,name:loc.name,depth:loc.depth,nodes:loc.nodes,visible:loc.visible},
   toolProposedLocus:f.authority_layer||null,
   overridesToolProposal:!!(f.authority_layer&&loc.name!==f.authority_layer),
   figmaFileKey:(STATE.header&&STATE.header.file_key)||null,
   figmaNodeUrl:figUrl(f.node_id),figmaLocusUrl:figUrl(loc.id),
   frameContentDigest:f.content_digest,ledgerDigest:STATE.ledgerDigest})});
  const amending=!!prevDec;
  DEC[f.node_id]={nodeId:f.node_id,route:f.route,decision:dec,comment:cm,
                  selectedLocus:{id:loc.id,name:loc.name},seq:r.seq};
  chips();rail();
  if(amending){
   // Came back on purpose. Confirm in place rather than jumping to the next undecided route.
   BUSY=false;if(btn){btn.disabled=false;btn.textContent='Update decision'}
   out.innerHTML=`<p class="msg ok">Updated. Decision #${r.seq} now stands for this route.</p>`;
   showHistory(f.node_id);
  }else{
   out.innerHTML=`<p class="msg ok">Recorded #${r.seq}.</p>`;
   setTimeout(()=>{BUSY=false;advance(f.node_id)},420);
  }
 }catch(err){BUSY=false;if(btn)btn.disabled=false;
  // An identical resubmit is not a failure, it is a no-op. Saying "error" for it teaches
  // the signer to distrust the messages that DO matter.
  const dup=/^identical to decision #/.test(err.message);
  out.innerHTML=`<p class="msg${dup?'':' err'}">`+
   (dup?'No change: '+esc(err.message):'Not recorded: '+esc(err.message))+'</p>'}
 return false}

addEventListener('keydown',e=>{if(!CUR||/INPUT|TEXTAREA/.test(e.target.tagName))return;
 const s=shown(),i=s.findIndex(x=>x.node_id===CUR);
 if(e.key==='ArrowLeft'&&i>0)open_(s[i-1].node_id);
 if(e.key==='ArrowRight'&&i<s.length-1)open_(s[i+1].node_id);
 if(e.key==='Escape'){const lb=document.getElementById('lb');lb.classList.contains('on')?closeLb():list()}});
(async()=>{try{STATE=await api('/api/state');FRAMES=(await api('/api/frames')).frames;
 (await api('/api/decisions')).decisions.forEach(d=>DEC[d.nodeId]=d);
 document.getElementById('hdr').innerHTML=`captured ${esc(STATE.header.captured_at||'-')} &middot; depth ${esc(STATE.header.capture_depth||'-')}`;
 chips();rail();list()}catch(e){document.getElementById('app').innerHTML='<p class="msg err">'+esc(e.message)+'</p>'}})();
</script></body></html>"""


SCHEMA = """
CREATE TABLE IF NOT EXISTS decisions (
  seq            INTEGER PRIMARY KEY AUTOINCREMENT,
  node_id        TEXT NOT NULL,
  route          TEXT,
  decision       TEXT NOT NULL,
  comment        TEXT,
  locus_id       TEXT,
  locus_name     TEXT,
  tool_proposed  TEXT,
  overrides      INTEGER NOT NULL DEFAULT 0,
  frame_digest   TEXT,
  ledger_digest  TEXT,
  figma_node_url TEXT,
  recorded_at    TEXT NOT NULL,
  recorded_by    TEXT NOT NULL,
  payload        TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_decisions_node ON decisions(node_id, seq);
-- The operative decision for a node is its LATEST row. Earlier rows are history, not error:
-- changing your mind is a lawful act, and the record must show both the change and what it replaced.
CREATE VIEW IF NOT EXISTS current_decisions AS
  SELECT d.* FROM decisions d
  JOIN (SELECT node_id, MAX(seq) AS seq FROM decisions GROUP BY node_id) m
    ON d.node_id = m.node_id AND d.seq = m.seq;
"""


def asset_url(kind: str, name: str | None, token: str) -> str | None:
    """Quote the filename. It is a path segment, and an unquoted `#` or `?` would truncate it."""
    return f"/{kind}/{quote(name)}?k={quote(token)}" if name else None


def open_db(path: Path, import_from: Path | None) -> sqlite3.Connection:
    """Open the record. WAL so a reader never blocks a write that is a Principal act, and
    synchronous=FULL because losing an acknowledged decision is the failure that matters here."""
    path.parent.mkdir(parents=True, exist_ok=True)
    db = sqlite3.connect(path, check_same_thread=False, isolation_level=None)
    db.row_factory = sqlite3.Row
    db.execute("PRAGMA journal_mode=WAL")
    db.execute("PRAGMA synchronous=FULL")
    db.executescript(SCHEMA)
    if db.execute("SELECT COUNT(*) c FROM decisions").fetchone()["c"] == 0 \
            and import_from and import_from.exists():
        rows = [json.loads(x) for x in import_from.read_text().splitlines() if x.strip()]
        for r in rows:
            insert_decision(db, r, seq=r.get("seq"))
        print(f"  migrated {len(rows)} rows from {import_from}")
    return db


def insert_decision(db: sqlite3.Connection, d: dict, seq: int | None = None) -> int:
    loc = d.get("selectedLocus") or {}
    cur = db.execute(
        "INSERT INTO decisions (seq,node_id,route,decision,comment,locus_id,locus_name,"
        "tool_proposed,overrides,frame_digest,ledger_digest,figma_node_url,recorded_at,"
        "recorded_by,payload) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        (seq, d.get("nodeId"), d.get("route"), d.get("decision"), d.get("comment"),
         loc.get("id"), loc.get("name"), d.get("toolProposedLocus"),
         1 if d.get("overridesToolProposal") else 0, d.get("frameContentDigest"),
         d.get("ledgerDigest"), d.get("figmaNodeUrl"),
         d.get("recordedAt") or datetime.now(timezone.utc).isoformat(),
         d.get("recordedBy") or "tools/sign-review/sign-review-server.py", json.dumps(d)))
    return cur.lastrowid


def redundant(db: sqlite3.Connection, d: dict) -> int | None:
    """Is this the SAME decision, on the same locus, with the same comment, as the row already
    standing for this node? Then it adds nothing and is refused with the seq that already says it.

    This is the structural cure for the incident that produced seqs 17 and 18: the client's
    auto-advance had no next route at the end of a section, so the form sat on the last frame and
    re-recorded it on each press. The client is fixed, but a guard that lives only in the client
    is a guard one refresh can lose."""
    prev = db.execute("SELECT seq, decision, comment, locus_id FROM current_decisions "
                      "WHERE node_id = ?", (d.get("nodeId"),)).fetchone()
    if not prev:
        return None
    loc = (d.get("selectedLocus") or {}).get("id")
    same = (prev["decision"] == d.get("decision")
            and (prev["comment"] or "").strip() == (d.get("comment") or "").strip()
            and (prev["locus_id"] or "") == (loc or ""))
    return prev["seq"] if same else None


def export_jsonl(db: sqlite3.Connection, out: Path) -> None:
    """Rewrite the JSONL from the database. It is a DERIVED VIEW and never a second record:
    two files that can disagree about one fact are one file and an argument."""
    rows = db.execute("SELECT payload FROM decisions ORDER BY seq").fetchall()
    tmp = out.with_suffix(out.suffix + ".tmp")
    tmp.write_text("".join(r["payload"] + "\n" for r in rows))
    # Set the mode on the TEMP file. replace() moves a new inode into place, so a chmod on the
    # destination is undone by the first write - which is exactly what happened here.
    os.chmod(tmp, 0o600)
    tmp.replace(out)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--frames", required=True, type=Path)
    ap.add_argument("--renders", type=Path)
    ap.add_argument("--shots", type=Path)
    ap.add_argument("--db", type=Path,
                    default=Path.home() / "Backups" / "opbox-sign-decisions" / "sign-decisions.sqlite",
                    help="the record. Durable by default: /var/tmp is disposable by house rule "
                         "and by D3 of [2026] VJS-CC-OPBOX 7.")
    ap.add_argument("--log", type=Path, default=Path("./sign-review-decisions.jsonl"),
                    help="JSONL EXPORT, derived from --db after every write. Not the record.")
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=8787)
    ap.add_argument("--token", help="shared secret; one is minted if omitted. Prefer "
                                    "--token-file: an argv secret is visible in ps to every "
                                    "user on the box.")
    ap.add_argument("--token-file", type=Path, help="read the shared secret from this file")
    # Relative to the system temp dir, not a hardcoded absolute path. This is a VDS capability
    # and nothing in it may be specific to one operator's box: an absolute default works
    # forever here and nowhere else, and it fails by silently caching somewhere unwritable
    # rather than by saying so.
    ap.add_argument("--layer-cache", type=Path,
                    default=Path(tempfile.gettempdir()) / "vds-sign-review-layers",
                    help="where on-demand per-layer renders are cached "
                         "(default: <tmp>/vds-sign-review-layers)")
    ap.add_argument("--no-prewarm", action="store_true",
                    help="do not warm the layer cache in the background at startup")
    ap.add_argument("--figma-token-file", type=Path,
                    help="Figma PAT for on-demand layer renders; falls back to $FIGMA_TOKEN")
    a = ap.parse_args()

    data = json.loads(a.frames.read_text())
    frames = {f["node_id"]: f for f in data["frames"]}
    header = data.get("header", {})
    ledger_digest = header.get("content_digest", "")
    token = (a.token_file.read_text().strip() if a.token_file
             else a.token) or secrets.token_urlsafe(18)
    assets = {k: v for k, v in (("renders", a.renders), ("shots", a.shots)) if v}
    a.log.parent.mkdir(parents=True, exist_ok=True)
    try:
        sys.stdout.reconfigure(line_buffering=True)   # else the banner below never reaches a log file
    except (AttributeError, ValueError):
        pass
    db = open_db(a.db, import_from=a.log)
    dblock = threading.Lock()
    # The record holds Principal acts; the export sits in a disposable dir. Neither is for
    # other users of this box.
    for f in (a.db, a.log):
        try:
            os.chmod(f, 0o600)
        except OSError:
            pass
    # Re-derive the export at boot, so "derived from the db" is true from the first request and
    # not only after the first write.
    export_jsonl(db, a.log)
    print(f"  record:   {a.db}  ({db.execute('SELECT COUNT(*) c FROM decisions').fetchone()['c']} rows)")
    print(f"  export:   {a.log}  (derived, rewritten after each write)")

    a.layer_cache.mkdir(parents=True, exist_ok=True)
    figma_token = (a.figma_token_file.read_text().strip() if a.figma_token_file
                   else os.environ.get("FIGMA_TOKEN") or "")
    file_key = header.get("file_key") or ""
    # Only ever render a node this ledger already names as a candidate locus. Without this the
    # service is an open proxy to any node in any file the PAT can reach.
    layer_meta = {c["id"]: c for f in frames.values()
                  for c in (f.get("candidates") or []) if c.get("id")}
    hidden = sum(1 for c in layer_meta.values() if c.get("visible") is False)
    print(f"  layers:   {len(layer_meta)} loci ({hidden} hidden) -> {a.layer_cache}"
          f"{'' if figma_token else '  (NO FIGMA TOKEN - layer view will report why)'}")

    # Per-node de-duplication rather than one global lock: a prewarm and a click wanting the
    # SAME layer must not each fetch it, but wanting different layers must not queue either.
    inflight: dict[str, threading.Event] = {}
    inflight_lock = threading.Lock()

    def cache_path(node_id: str) -> Path:
        return a.layer_cache / (node_id.replace(":", "-") + ".png")

    def cached_ok(pth: Path) -> bool:
        return pth.is_file() and pth.stat().st_size > 0

    def image_urls(ids: list[str]) -> tuple[dict, str | None]:
        """One images call for up to 25 nodes. Batching is the point: the round trip dominates,
        so 25 layers cost about what one does."""
        try:
            url = (f"https://api.figma.com/v1/images/{file_key}"
                   f"?ids={quote(','.join(ids))}&format=png&scale=2")
            req = urllib.request.Request(url, headers={"X-Figma-Token": figma_token})
            with urllib.request.urlopen(req, timeout=180) as r:
                body = json.loads(r.read())
            if body.get("err"):
                return {}, f"Figma refused: {body['err']}"
            return body.get("images") or {}, None
        except (urllib.error.URLError, OSError, ValueError) as exc:
            return {}, f"could not reach Figma: {exc}"

    def download(src: str, dest: Path) -> str | None:
        try:
            with urllib.request.urlopen(src, timeout=120) as r:
                png = r.read()
        except (urllib.error.URLError, OSError) as exc:
            return f"could not download the render: {exc}"
        if not png.startswith(b"\x89PNG"):
            return "the fetched bytes are not a PNG"
        tmp = dest.with_suffix(".part")
        tmp.write_bytes(png)
        tmp.replace(dest)
        return None

    def layer_png(node_id: str) -> tuple[Path | None, str | None]:
        """Cached-or-fetched PNG for one layer. Returns (path, refusal). Never raises at the
        caller: a Figma outage must read as a stated reason in the pane, not a blank box."""
        pth = cache_path(node_id)
        if cached_ok(pth):
            return pth, None
        # Answer from what we already know. Figma renders nothing for a fully hidden node, and
        # many loci here ARE hidden (the frozen LEGACY UNDERLAY layers), so asking would spend
        # seconds per click to be told nothing. Say the real reason immediately.
        c = layer_meta.get(node_id) or {}
        if c.get("visible") is False:
            return None, (f"\u201c{c.get('name', node_id)}\u201d is hidden in Figma, so Figma "
                          f"renders no image for it. It is still selectable as the contract; "
                          f"you just cannot preview it.")
        if not figma_token:
            return None, "no Figma token on this server, so layers cannot be rendered"
        if not file_key:
            return None, "the capture header names no file_key"
        with inflight_lock:
            ev = inflight.get(node_id)
            mine = ev is None
            if mine:
                ev = inflight[node_id] = threading.Event()
        if not mine:
            ev.wait(120)
            return (pth, None) if cached_ok(pth) else (None, "still rendering; try again")
        try:
            imgs, why = image_urls([node_id])
            if why:
                return None, why
            src = imgs.get(node_id)
            if not src:
                return None, ("Figma returned no image for this node "
                              "(usually: it is empty, or fully hidden)")
            why = download(src, pth)
            return (None, why) if why else (pth, None)
        finally:
            with inflight_lock:
                inflight.pop(node_id, None)
            ev.set()

    def prewarm() -> None:
        """Warm every renderable locus in the background, 25 per call. Without this, selecting a
        locus radio - which is REQUIRED in order to sign - costs a cold round trip to Figma every
        single time, so the mandatory path through the form is the slow one. That is the bug this
        exists to fix, and it was introduced by adding the layer view in the first place."""
        todo = [n for n, c in layer_meta.items()
                if c.get("visible") is not False and not cached_ok(cache_path(n))]
        if not todo:
            print("  prewarm:  layer cache already complete")
            return
        print(f"  prewarm:  {len(todo)} layers to warm in the background")
        done = fail = 0
        for i in range(0, len(todo), 25):
            chunk = [n for n in todo[i:i + 25] if not cached_ok(cache_path(n))]
            if not chunk:
                continue
            imgs, why = image_urls(chunk)
            if why:
                fail += len(chunk)
                continue
            for nid in chunk:
                src = imgs.get(nid)
                if not src or download(src, cache_path(nid)):
                    fail += 1
                else:
                    done += 1
            if (done + fail) % 200 < 25:
                print(f"  prewarm:  {done + fail}/{len(todo)}")
        print(f"  prewarm:  done - {done} cached, {fail} unavailable")

    if figma_token and file_key and not a.no_prewarm:
        threading.Thread(target=prewarm, daemon=True, name="prewarm").start()

    def decisions() -> list[dict]:
        """The operative decision per node, from the database view."""
        with dblock:
            return [json.loads(r["payload"])
                    for r in db.execute("SELECT payload FROM current_decisions "
                                        "ORDER BY seq").fetchall()]

    class H(BaseHTTPRequestHandler):
        server_version = "vds-sign-review/2"
        # Keep-alive. The card grid requests ~27 thumbnails per section; under HTTP/1.0 each one
        # is a new TCP connection, which over Tailscale looks like the page hanging. Every
        # response below sets Content-Length, which is what makes 1.1 safe here.
        protocol_version = "HTTP/1.1"

        def log_message(self, fmt, *args):
            # The token rides in the query string because an <img> cannot send a header, so
            # every request line contains the shared secret. Redact before it reaches a file.
            line = fmt % args
            if token and token in line:
                line = line.replace(token, "<redacted>")
            sys.stderr.write("  %s\n" % line)

        def _j(self, code, obj):
            b = json.dumps(obj).encode()
            self.send_response(code)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(b)))
            self.send_header("Cache-Control", "no-store")
            self.send_header("X-Content-Type-Options", "nosniff")
            self.end_headers()
            self.wfile.write(b)

        def _auth(self, q) -> bool:
            got = self.headers.get("X-Auth") or (q.get("k", [""])[0])
            return hmac.compare_digest(got or "", token)

        def do_GET(self):
            try:
                return self._get()
            except (BrokenPipeError, ConnectionResetError):
                self.close_connection = True     # the tab navigated away mid-image; not an error
            except Exception as exc:
                self.close_connection = True     # keep-alive must not carry on over a torn reply
                sys.stderr.write(f"  GET {self.path.split('?')[0]} failed: {exc!r}\n")

        def _get(self):
            u = urlparse(self.path)
            q = parse_qs(u.query)
            if u.path == "/health":
                return self._j(200, {"ok": True, "frames": len(frames)})
            if not self._auth(q):
                return self._j(401, {"error": "unauthorised"})
            if u.path in ("/", "/index.html"):
                b = APP.encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(len(b)))
                # The client is inline in this document. Cache it and a fixed bug looks unfixed.
                self.send_header("Cache-Control", "no-store, must-revalidate")
                self.send_header("X-Content-Type-Options", "nosniff")
                self.end_headers()
                return self.wfile.write(b)
            if u.path == "/api/state":
                return self._j(200, {"header": header, "ledgerDigest": ledger_digest,
                                     "count": len(frames)})
            if u.path == "/api/frames":
                return self._j(200, {"frames": [
                    dict({k: f.get(k) for k in ("node_id", "route", "authority_by", "flagged",
                                                "blocked", "frame_name", "kind", "reason",
                                                "tracker_status", "tracker_tier", "family",
                                                "title")},
                         thumb_url=asset_url("renders", f.get("render_file"), token),
                         shot_url=asset_url("shots", f.get("shipped_file"), token))
                    for f in frames.values()]})
            if u.path.startswith("/api/frames/"):
                # URL-DECODE. A node id is `674:26005`; the browser's encodeURIComponent
                # sends `674%3A26005`, and a no-frame id carries a slash too. curl sends
                # the colon literally, which is why every hand test passed while every
                # real click 404'd.
                f = frames.get(unquote(u.path.split("/api/frames/", 1)[1]))
                if not f:
                    return self._j(404, {"error": "no such frame"})
                out = dict(f)
                out["render_url"] = asset_url("renders", f.get("render_file"), token)
                out["shipped_url"] = asset_url("shots", f.get("shipped_file"), token)
                return self._j(200, out)
            if u.path == "/api/decisions":
                return self._j(200, {"decisions": decisions()})
            if u.path.startswith("/api/history/"):
                nid = unquote(u.path.split("/api/history/", 1)[1])
                with dblock:
                    rows = db.execute("SELECT payload FROM decisions WHERE node_id = ? "
                                      "ORDER BY seq DESC", (nid,)).fetchall()
                return self._j(200, {"history": [json.loads(r["payload"]) for r in rows]})
            if u.path.startswith("/layers/"):
                nid = unquote(u.path.split("/layers/", 1)[1]).removesuffix(".png")
                if nid not in layer_meta:
                    return self._j(404, {"error": "not a candidate locus in this ledger"})
                png, why = layer_png(nid)
                if not png:
                    return self._j(503, {"error": why})
                b = png.read_bytes()
                self.send_response(200)
                self.send_header("Content-Type", "image/png")
                self.send_header("Content-Length", str(len(b)))
                self.send_header("Cache-Control", "private, max-age=86400, immutable")
                self.end_headers()
                return self.wfile.write(b)
            if u.path.startswith(("/renders/", "/shots/")):
                kind, _, name = unquote(u.path).lstrip("/").partition("/")
                root = assets.get(kind)
                if not root:
                    return self._j(404, {"error": "no such asset root"})
                try:
                    t = (root / name).resolve()
                    t.relative_to(root.resolve())
                except (ValueError, OSError):
                    return self._j(403, {"error": "outside the asset root"})
                if not t.is_file():
                    return self._j(404, {"error": "not found"})
                self.send_response(200)
                self.send_header("Content-Type", "image/png")
                self.send_header("Content-Length", str(t.stat().st_size))
                # Immutable: the filename is the node id and a re-render changes the ledger,
                # which the staleness guard already catches on the API side.
                self.send_header("Cache-Control", "private, max-age=86400, immutable")
                self.end_headers()
                with t.open("rb") as fh:
                    while chunk := fh.read(1 << 16):
                        self.wfile.write(chunk)
                return
            return self._j(404, {"error": "not found"})

        def do_POST(self):
            try:
                return self._post()
            except (BrokenPipeError, ConnectionResetError):
                self.close_connection = True
            except Exception as exc:
                self.close_connection = True
                sys.stderr.write(f"  POST failed: {exc!r}\n")
                try:
                    self._j(500, {"error": "the server failed to record this; it is NOT saved"})
                except Exception:
                    pass

        def _post(self):
            u = urlparse(self.path)
            if not self._auth(parse_qs(u.query)):
                return self._j(401, {"error": "unauthorised"})
            if u.path != "/api/decisions":
                return self._j(404, {"error": "not found"})
            try:
                n = int(self.headers.get("Content-Length") or 0)
            except ValueError:
                n = 0
            if n <= 0 or n > MAX_BODY:
                return self._j(413, {"error": "bad or oversized body"})
            try:
                d = json.loads(self.rfile.read(n))
            except Exception as exc:
                return self._j(400, {"error": f"not JSON: {exc}"})

            nid = d.get("nodeId")
            if nid not in frames:
                return self._j(400, {"error": "unknown frame"})
            if d.get("decision") not in VALID:
                return self._j(400, {"error": f"decision must be one of {sorted(VALID)}"})
            # Fail closed on staleness. A tab left open across a re-derivation must not
            # record a decision against a reading the server no longer serves.
            if d.get("ledgerDigest") != ledger_digest:
                return self._j(409, {"error": "the ledger has changed since this page loaded; "
                                              "reload before deciding"})
            if frames[nid].get("kind") == "no-frame":
                return self._j(400, {"error": "this route has no frame to sign: "
                                              + (frames[nid].get("reason") or "no reason recorded")})
            if d["decision"] == "sign":
                if not d.get("selectedLocus"):
                    return self._j(400, {"error": "a sign decision must name the governing locus"})
                if frames[nid].get("blocked"):
                    return self._j(400, {"error": "signing is blocked: "
                                                  + "; ".join(frames[nid]["blocked"])})
            elif not (d.get("comment") or "").strip():
                return self._j(400, {"error": "refuse and defer require a comment"})

            # VI.3, the largest bloc (Ravensmere J and Thornbury J): "The signer may still sign
            # a demoted frame; he may not sign one without the demotion appearing on the face of
            # the record he signs." Enriched SERVER-SIDE so no client can omit it, and recorded
            # verbatim with counts rather than as a flag, because a boolean cannot be audited.
            fr = frames[nid]
            d["disclosedAtSigning"] = {
                "quarantinedLayers": [
                    {"name": c["name"], "nodes": c["nodes"], "texts": c["texts"],
                     "visible": c["visible"], "depth": c["depth"]}
                    for c in fr.get("candidates", []) if c.get("marker") == "demoted"],
                "machineProvenance": [
                    {"name": c["name"], "clonedFrom": c["cloned_from"]}
                    for c in fr.get("candidates", []) if c.get("cloned_from")],
                "toolResolvedAuthorityLayer": fr.get("authority_layer"),
                "authorityBy": fr.get("authority_by"),
                "frameSelfDisclaims": fr.get("disclaimed"),
                "captureDepth": header.get("capture_depth"),
                "truncatedLeavesInCapture": header.get("truncated_leaves"),
            }
            with dblock:
                dup = redundant(db, d)
                if dup is not None:
                    return self._j(409, {
                        "error": f"identical to decision #{dup} already standing for this route "
                                 f"({d['decision']}, same locus, same comment). Nothing to add.",
                        "seq": dup, "duplicateOf": dup})
                d |= {"recordedAt": datetime.now(timezone.utc).isoformat(),
                      "recordedBy": "tools/sign-review/sign-review-server.py",
                      "authority": "none; input to a vds-recorded Principal act (order 16)"}
                # One transaction. The seq is only known after the INSERT, so the payload is
                # completed by an UPDATE; under autocommit a crash between the two would leave a
                # stored payload that does not carry the seq it was filed under.
                db.execute("BEGIN IMMEDIATE")
                try:
                    seq = insert_decision(db, d)
                    d["seq"] = seq
                    db.execute("UPDATE decisions SET payload = ? WHERE seq = ?",
                               (json.dumps(d), seq))
                    db.execute("COMMIT")
                except Exception:
                    db.execute("ROLLBACK")
                    raise
                export_jsonl(db, a.log)
            print(f"  {d['decision']:>7}  {d.get('route')}"
                  + ("  [overrides the tool's proposal]" if d.get("overridesToolProposal") else ""))
            return self._j(200, {"ok": True, "seq": seq})

    if a.host not in ("127.0.0.1", "localhost", "::1"):
        print(f"  binding {a.host}. The token below is the only thing standing between this "
              f"surface and anything else on that network.", file=sys.stderr)
    print(f"\n  http://{a.host}:{a.port}/?k={token}\n")
    print(f"  frames {len(frames)}  ·  log {a.log.resolve()}")
    print(f"  ledger digest {ledger_digest[:24]}…  (decisions against any other digest are refused)")
    print("  this service creates no authority; it records decisions for the vds CLI\n")
    try:
        ThreadingHTTPServer((a.host, a.port), H).serve_forever()
    except KeyboardInterrupt:
        print("\nstopped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
