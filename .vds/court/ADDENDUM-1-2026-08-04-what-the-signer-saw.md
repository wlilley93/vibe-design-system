# Addendum 1 to the locus-bearing registration case

**Supplements** `SUBMISSION-2026-08-04-074921`, whose case file is
`CASE-2026-08-04-locus-bearing-registration.md`,
sha256 `fa7be09c541bd619dc67f1c018293780d5c96a62b6c2e2863cb74515fdf10a05`. **That file is NOT
amended.** Its bytes are the filed case file and they stand; this is a separate instrument, so
the digest the submission pins remains true.

**Occasion.** After filing and before the bench convened, the Principal stated, unprompted:

> "I signed off or rejected based on the frame in front of me, which I presumed to have been the
> full frame."

That is evidence about the evidential basis of all 167 decisions and it is not for the advocate
to weigh privately. It is measured here and put before the court.

## A1. What the signer was actually shown

The service displayed the **whole-frame render** by default. A per-layer view (selecting a locus
radio repaints the pane with that layer alone) did not exist until commit `9ffef18`,
`2026-08-04T07:02:24Z`. Split by that moment:

```
sqlite3 ~/Backups/opbox-sign-decisions/sign-decisions.sqlite \
  "SELECT decision, COUNT(*) FROM current_decisions WHERE recorded_at < '2026-08-04T07:02:24Z' GROUP BY decision"
-> before the layer view existed: 32   (sign 27, refuse 4, defer 1)
-> after:                        135   (sign 100, refuse 32, defer 3)
```

So for 32 decisions the whole-frame image was the **only** image available. The Principal's
account of what he was looking at is therefore correct as to those, and correct as to the
default view throughout.

## A2. The presumption is sound, and this is the decisive measurement

```
sqlite3 ~/Backups/opbox-sign-decisions/sign-decisions.sqlite \
  "SELECT COUNT(*) FROM current_decisions WHERE locus_id = node_id"      -> 166
  "SELECT COUNT(*) FROM current_decisions WHERE locus_id <> node_id"     ->   1
```

**In 166 of 167 decisions the locus chosen IS the frame's own node.** The whole-frame image was
not merely what the signer saw; it is the thing he adopted. There is no gap between the picture
and the act.

The single exception is seq 33, `/documents/packs/[id]`: frame `675:74319`, locus `1007:89086`
(`SOURCE AUTHORITY - /documents/packs/[id] - cloned from 29:524`). **Its decision is `refuse`.**
It therefore produces no register row on any construction and cannot carry the defect the
Principal was worried about.

**Consequence: every one of the 127 `sign` decisions adopts the frame's own node.** No sign
decision adopts anything narrower than what was displayed.

## A3. What the 18 overrides actually are

Pleaded at F4 as "18 chosen loci differ from the tool's proposal". Measured, they are all the
same act:

```
/dashboards               tool proposed: /dashboards - current source matter ma...
                          Principal chose: Screen - /dashboards
/documents/[id]           tool proposed: SOURCE AUTHORITY - /documents/[id] - cloned from...
                          Principal chose: Screen - /documents/[id] - editor
/files                    tool proposed: SOURCE AUTHORITY - /files - cloned from...
                          Principal chose: Screen - /files - Filecloud current
```

In each, the tool proposed the **machine-implanted clone** and the Principal chose the frame's
**own** content. These eighteen are the Principal rejecting, route by route, the implant that
produced CC-OPBOX 6 and 7.

## A4. Why this SHARPENS rather than answers the filed question

It might be said that if the locus is the frame's own node in every signed case, then `node_id`
alone records it and no locus field is needed. That does not follow, and the eighteen are why.
A row holding only `node_id` records `674:26005` identically whether the Principal adopted the
frame's own content or the `SOURCE AUTHORITY` clone sitting inside it. The two are different
adoptions with different consequences, and the register cannot tell them apart. The Principal's
rejection of the implant is exactly the fact that a frame-only row destroys.

## A5. Disclosure, measured

```
127 sign decisions; 127 carry a disclosedAtSigning block
 14 enumerate quarantined layers on the face of the record, naming 44 layers
  2 enumerate machine-cloned layers
```

The 44 are the frozen `LEGACY UNDERLAY` layers of CC-OPBOX 7 D1. D8 requires that a sign-off for
such a frame enumerate each with recursive node and text counts on its face; the counts are in
the payloads. Note the frame render excludes hidden layers, so the picture the signer saw did
**not** show the demoted content - which is why the enumeration above the controls, and not the
image, is what discharges D8.

## A6. The advocate's position, unchanged

Nothing here asks the court to prefer a construction. It removes a doubt the Principal raised
about the basis of his own act, and it corrects the weight of F4: the eighteen are not a
scattering of disagreements about layer names, they are a consistent rejection of the implant.
