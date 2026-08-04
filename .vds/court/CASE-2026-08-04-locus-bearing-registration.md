# Case file: can a label-resolving Principal act be registered?

**Lane** VDS. **Filed by** Lexby, as advocate and as the party whose own work created the gap.
**Prepared** 2026-08-04. **Every count below is derived at filing time by the command shown**
(CC-OPBOX 7 D11), and the reading instrument is named (D10).

---

## I. The reading instrument

```
vds path:    /home/jellytot/.local/bin/vds
vds version: vds 0.1.0
repo:        vibe-design-system @ 6b0ed95, branch feat/proof-kinds-visual-gap
```

A measurement of a record is a measurement of a reader (CC-OPBOX 7, R4). Everything below is
stated as read by that build and no other.

## II. The facts, each with the command that produced it

**F1. The `SignOff` row cannot name a locus.** `crates/vds-core/src/types/signoff.rs:41`.

```
python3 -c "import re,pathlib; t=pathlib.Path('crates/vds-core/src/types/signoff.rs').read_text(); \
import sys; m=re.search(r'pub struct SignOff \{(.*?)\n\}',t,re.S); \
print(', '.join(re.findall(r'pub (\w+):',m.group(1))))"
-> id, file_key, node_id, frame_digest, signed_by, signed_at, notes
```

The struct carries `#[serde(deny_unknown_fields)]`, so the absence is not an omission a caller
can route around. It is a closed shape.

**F2. The `DirectionRecord` row exists and has a different shape.**

```
-> id, log_id, decision_digest, surface, direction, magnitude, directed_at, notes
```

`surface` is `DirectedSurface::Frame { file_key, node_id }` or `Route { route }`. It likewise
names no layer within a frame.

**F3. There are 167 operative decisions, each naming exactly one locus.**

```
sqlite3 ~/Backups/opbox-sign-decisions/sign-decisions.sqlite \
  "SELECT decision, COUNT(*) FROM current_decisions GROUP BY decision"
-> sign 127 | refuse 36 | defer 4      (167 total, 167 naming a locus, 167 distinct loci)
```

**F4. In eighteen of them the locus the Principal chose is NOT the locus the tool proposed.**

```
sqlite3 ~/Backups/opbox-sign-decisions/sign-decisions.sqlite \
  "SELECT COUNT(*) FROM current_decisions WHERE overrides = 1"
-> 18
```

This fact is load-bearing and is the reason the matter is brought. Where the chosen locus equals
the proposed one, a later reader could in principle re-derive it from the frame. For these
eighteen it cannot: the choice exists only in the Principal's act, and a register row that
records the frame alone silently discards it.

**F5. The register today holds 19 sign-off rows and 1 direction row.**

The register for this estate lives in the parity worktree, not at the vibe-design-system root,
and naming the wrong root would report zero and read as "the estate holds none":

```
ls /var/tmp/claude/parity-wt/.vds/signoffs/*.yaml   | wc -l  -> 19
ls /var/tmp/claude/parity-wt/.vds/directions/*.yaml | wc -l  -> 1
ls /home/jellytot/Projects/vibe-design-system/.vds/signoffs/*.yaml | wc -l -> 0
```

The third line is included deliberately. A NOT FOUND is a claim about where the reader looked and
never a fact about the corpus (CC-OPBOX 7, R4).

## III. The law said to bear

- **`[2026] VJS-CC-OPBOX 6`** (ratio): registration requires a recognised current-source
  declaration on the selected locus **or** an express, verified, hash-bound Principal act
  resolving that label first.
- **`[2026] VJS-CC-OPBOX 7`, R5**: a hash-bound attestation over a ledger is not a
  label-resolution act on a locus. Cure (b) requires an act that is express, verified,
  hash-bound, Principal **and** label-resolving on a named locus. *"The conjunction is the whole
  point of it."*
- **`[2026] VJS-CA-VDS 1`**, at the passage recorded as the buildability gap: *"An unregistered
  direction is not instrument-readable authority: authority the instruments cannot read is
  authority the estate does not have."* That court met a structurally identical gap - `SignOff`
  could not carry a direction's log id and decision digest - and met it by **adding a row kind**
  (`DirectionRecord`) rather than by widening `SignOff` or by using `notes`.
- **`[2026] VJS-CC-OPBOX 7`, forbidden list**: no file under `vibe-design-system` may be edited
  on the authority of that judgment. Hence this filing; the question needs its own basis.
- **Order 16**: a machine verdict creates no authority. The sign-review service is an input.

## IV. The question

The Principal has made 167 express, per-frame, per-locus, hash-bound decisions. 127 are `sign`.
**By what lawful route, if any, do those 127 enter the register?**

## V. The constructions, argued at their strongest

### Construction A - the locus goes in `notes`, and nothing needs to change

`SignOff.notes` is a free `Option<String>`. Writing `locus: SOURCE AUTHORITY - /pipelines
(983:1028)` there records the layer on the face of the row. The act is then express (the
Principal chose), verified (the service refused to pre-select), hash-bound (`frame_digest`), and
Principal. The conjunction CC-OPBOX 7 R5 demands is satisfied in substance, and no schema change,
no submission and no delay is required. Nothing in R5 says the locus must occupy a typed field;
it says the act must be label-resolving, and a row whose notes name the label resolves it.

*Against:* `notes` is unparsed and unvalidated. `frame_authority()` at `signoff.rs:195` reads
`file_key`, `node_id` and `frame_digest` and nothing else, so no instrument can read the locus and
no gate can be made to depend on it. That is precisely the condition CA-VDS 1 called authority the
estate does not have. It also cannot be checked: two rows for one frame naming different loci in
prose would both parse, and the register could not say which governs.

### Construction B - register them as `DirectionRecord` rows

A direction is *"taste exercised AT the register, hash-bound, by the only person entitled to
exercise it"*. The row already carries `surface`, a free-text `direction`, a `magnitude`, and
binds to a logged decision by digest rather than to frame content. The 167 decisions are logged,
digestible and Principal. This uses an enacted row kind exactly as designed and needs no
amendment.

*Against:* a direction row *"confers authority for its own terms only, later in time, and carries
a live duty to redraw so the frame record converges on the directed state"*. A locus resolution is
not a direction to change anything; the frame is already correct and the Principal is saying which
part of it governs. Recording 127 adoptions as 127 directions would create 127 redraw duties that
nobody owes and would make the register assert the opposite of what happened.

### Construction C - amend S-7D so a sign-off row can name its locus

Add a locus to `SignOff` (node id and layer name, and on one view the layer's own digest), and make
`frame_authority()` refuse a row that omits it where the frame has more than one candidate. This
puts the thing the authority actually attaches to inside the typed record, where an instrument can
read it and a gate can depend on it.

*Against:* it is a schema change to an enacted specification with 19 live rows, and a required
field breaks all 19 (which name no locus) unless it is optional - and an optional locus is a field
that will be omitted, which returns the estate to where it started. It also asserts, without the
question having been argued, that the locus belongs on the sign-off rather than in a record of its
own.

### Construction D - a new row kind for the resolution itself

Follow CA-VDS 1's own method: where the register could not express a thing, it grew a row kind for
that thing. A `LocusResolution` row would record file key, node id, the resolved layer's node id
and name, the frame digest at resolution, the signer and the time. A `SignOff` would then be
lawful only where a resolution row covers the same frame at the same digest.

*Against:* two rows per act is more ceremony than the estate has shown it needs, and it splits one
Principal moment across two records that can go stale independently. It is also the most work, and
the gap it closes may be closable by C at a fraction of the cost.

## VI. What the advocate says plainly

The eighteen at F4 are the whole matter. On any construction that does not put the locus somewhere
an instrument can read, those eighteen Principal choices are recorded nowhere and the estate will,
within a month, be unable to say which layer it adopted for those routes. That is the same failure
mode as the original 167-frame act which produced CC-OPBOX 6 and 7, at smaller scale and with the
evidence already in hand.

I do not ask the court to prefer my construction. I ask it to decide, because until it does the
127 decisions sit outside the register and the parity programme cannot resume.

## VII. Matters expressly NOT before the court

- The 36 refusals and 4 deferrals. They create no register row on any construction.
- The reserved question of whether an agent-authored marker layer can constitute the Principal's
  Order 25 declaration (CC-OPBOX 7, reserved).
- `675:25422` (`/matters`), which CC-OPBOX 7 D12 puts beyond re-registration until its
  competing-current-layers question is determined. **It is excluded from the 127, but the court
  should know the operative reason is not D12**: the Principal REFUSED it (decision #75, locus
  `Screen - /matters - Hub - source contract + target layout reference`), so it produces no
  sign-off row on any construction. Verified at filing time:

```
sqlite3 ~/Backups/opbox-sign-decisions/sign-decisions.sqlite \
  "SELECT seq, decision FROM current_decisions WHERE node_id='675:25422'"
-> 75 | refuse
```

  I had first pleaded this as an exclusion effected by D12. That was wrong and is corrected here
  rather than left to be discovered: D12 would have bitten had the answer been `sign`, and it did
  not need to.
