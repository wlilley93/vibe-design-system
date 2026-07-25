# Adopting VDS in a project

This file is engineering explanation. It binds nothing (VDS S-3(4)). Where it disagrees with
`VDS.md`, `VDS.md` wins.

**Nothing here can be done yet.** VDS is drafted and not commenced (VDS S-15), no designpack
exists, and the `vds` command described below is not built. This is the intended shape of
adoption, written now so the front door and the wall are designed together rather than the door
being retrofitted onto whatever the wall turned out to be. Every command in this file is in the
conditional voice for that reason.

---

## Preconditions

Adoption is refused, and should be, unless all of these hold.

| precondition | why it is a precondition |
|---|---|
| `VDS.md` has commenced under S-15 | until then there is nothing to grant a warrant under |
| a designpack exists and is published with a digest | you subscribe to a pack, not to a repository |
| the project's systems of record are named and stable | VDS derives from them; it cannot derive from a moving target |
| the project can run a deterministic check in CI | VDS S-7(2)(3). A local hook alone is an interim state |
| the Principal is available for W3 | acceptance is theirs alone and cannot be worked around |

The project's systems of record are fixed by [2026] VJS-CC-OPBOX 3 D1 and are not VDS's to
move: `app/globals.css` for what ships, the decided-target Figma file for what is decided, and
the committed snapshot as a derived one-way pin between them and nothing else.

---

## What `vds init` would write

Into the adopting project, at its repository root. Everything below is committed. Only
`.vds/cache/` and `.vds/private/` are ignored, because a governance record that is gitignored
is not a record.

```
designpack/v1/            vendored read-only from the published pack, digest-pinned.
                          statutes, regulations, invariants, obligations, orders,
                          judgments, specs, provenance, manifest.toml.
                          never edited in place: a change is a new pack version.

.vds/
  config.toml             the one fixed anchor. every other path is configurable from it.
  designpack.lock         designpack_id, designpack_version, digest, schema_version,
                          generated_at, locked_by.
  install.lock            schema_version, generated_at, config_digest, hooks[],
                          hook_digests[], adapters[]. a missing hook is a finding,
                          not a quiet absence.
  enforcement.lock        one entry per proof script: path, digest, kind, invoked_by,
                          proves, failing_direction_test, pinned_at, pinned_by.
                          absent on init. written when the first gate is pinned.

  register/               empty. one <id>.yaml per component, later.
  warrants/               empty. W1 to W4, later, and never on init.
  proofs/                 empty. written by checkers as a side effect of running.
  pins/                   empty. derived, byte-reproducible, digests only.
  ledgers/                empty. generated inventories, never hand-edited.
  submissions/
    draft/ filed/ docket/ empty, except the reserved-matter submissions below.
  court/convenings/       empty. VJS convening records, recorded back here.
  logs/
    decisions/            empty.
    breaches/             the two founding defects, filed on init (VDS S-12(4)).
  permits/                empty.
  cache/  private/        the only two ignored paths.
```

`vds init` would **not** write: any component record, any warrant, any proof result, any pin.
Those are earned, not scaffolded. An init that produced a warrant would have granted itself one,
which VDS S-1(3) forbids in terms.

### `.vds/config.toml`

The one file whose path is fixed. It carries no design value, only paths, identifiers and
globs.

```toml
version = 1
jurisdiction_id = "<project>"
repo_code = "<REPOCODE>"
designpack = "<pack-id>@<pack-version>"

[paths]
register     = ".vds/register"
warrants     = ".vds/warrants"
proofs       = ".vds/proofs"
pins         = ".vds/pins"
ledgers      = ".vds/ledgers"
submissions  = ".vds/submissions"
decisions    = ".vds/logs/decisions"
breaches     = ".vds/logs/breaches"
permits      = ".vds/permits"

[governance]
permit_required = [
  "app/globals.css",            # the system of record for what ships
  "<component library dirs>/**",
  "designpack/v1/**",
  ".vds/register/**",
  ".vds/config.toml",
  "<proof script dir>/**",      # the gates themselves
]
permit_exempt = [
  ".vds/logs/**",
  ".vds/permits/**",
  ".vds/proofs/**",
]
```

Two entries in `permit_required` are the ones people leave out, and both are load-bearing:
`.vds/config.toml` and the proof scripts. Omit them and the gate is editable without a permit
by the same hand it constrains, which is not a gate. `permit_exempt` covers the append-only
record directories, which must stay writable or the machinery deadlocks on its own audit trail.

---

## Order of operations

The order is the mechanism, in the same way the warrant order is. Doing these in a different
sequence produces artefacts that no gate ever reads, which is the state VDS exists to end.

### 1. Subscribe and pin

Vendor `designpack/v1/` read-only, write `.vds/designpack.lock` with the pack id, version,
digest and schema version. The runtime never fetches doctrine. A loader must refuse, loudly and
at load time, any pack whose `schema_version` exceeds what it understands.

A later digest bump is a deliberate recorded act. No doctrine flows downstream by silence.

### 2. Configure

Write `.vds/config.toml`. Declare the systems of record, the governed library directories and
the permit globs. This is where the project decides what "the design system" means for it, and
the answer is checkable from that point on rather than remembered.

### 3. Install the wall before the work

Install the hooks, write `.vds/install.lock`, wire the CI job. Do this **before** authoring a
single component record.

This is the step people invert, and inverting it is the founding defect in a different costume.
In the motivating project, `component-map.json` held 56 component entries while
`src/components/ui` and `src/components/onyx` together held 90 `.tsx` files (measured:
`json.load(...)['components']` length, and `ls <dir>/*.tsx | wc -l` on each directory). Those
are not necessarily contradictory, since one entry may legitimately cover several files. The
problem is that no command derived either number from the other, so nobody could say which was
wrong. A register authored before anything reconciles it is a document, and documents decay.

### 4. Wire `no_stored_values` first

Of the ten proof kinds, wire this one before any other. It scans `.vds/**` for colour literals,
length literals, font families, durations and easing curves, and a hit is fatal.

Wire it first because it is the only proof that guards the authoring you are about to do. Author
two hundred component records with a hex value in each and you will rewrite two hundred records.
Wire the scan first and the first one fails.

### 5. Make each gate satisfy all five limbs, then pin it

For each proof kind, before it counts as a proof at all (VDS S-7(2)):

1. one named command, deterministic, no network call, no model call
2. a named test that seeds a violation against a fixture and asserts the non-zero exit
3. an invocation by something that is not the author choosing to run it
4. `rows_considered` and `rows_enforced` reported, with a zero enforced count recorded as
   `status: vacuous` and never as `passed`
5. the result record written by the checker itself, `capture_mode: automatic`

Then add the entry to `.vds/enforcement.lock`. The lock entry cannot be written without naming
the failing-direction test, which is how limb 2 is enforced rather than requested.

On limb 3, be honest about where you actually are. In the motivating project the two existing
design gates are invoked from `.githooks/pre-push` at lines 106 and 123, and from 0 of the 10
files in `.github/workflows/` (measured: `grep -n` over the hook, `grep -rln` over the workflow
directory). That satisfies the hook limb and not the CI limb. `git commit --no-verify` bypasses
a local hook, so a hook-only state is an interim state and must be recorded as one.

### 6. Build the register

One record per component, per `schema/component-record.schema.json`. The nine states are fixed
(`default`, `hover`, `focus`, `active`, `selected`, `disabled`, `loading`, `error`, `success`)
and a record may require a subset but may not invent a tenth.

`demand` is measured, never estimated: the record carries the command that measured it and the
timestamp.

This is the expensive part, and it costs the same whether or not VDS exists, because it is just
"write down every component and what it must do".

### 7. Run reconciliation and expect it to fail

It will fail on day one and keep failing until the register is genuinely complete: entries with
no resolvable code counterpart, code in the governed directories with no entry, Figma node ids
that do not resolve, prop and state contracts that disagree. Every one of those is a real
finding that was previously invisible. Driving that list to zero is the work.

### 8. W1, and only then design

Refer W1 REGISTER-COMPLETE to VJS on the evidence: `register_completeness` and `reconciliation`
both exit 0 and both non-vacuous over the declared surface. Design may begin when it is granted
and not before, because every drift defect measured in the motivating project was authored
before anyone asked whether the thing being used was registered.

Note the RESERVED clause here. Whether W1 may be granted provisionally on a greenfield surface
is unsettled (`SUBMISSION-VDS-001`), and until VJS answers, W1 is strict. On a genuinely
greenfield surface that is a real constraint and you should file the submission rather than
route around it.

### 9. W2, W3, W4

- **W2 DESIGN-COMPLETE** on `composition`, `states` and `contrast` over every declared screen.
  Who may grant it is RESERVED (`SUBMISSION-VDS-002`); until answered it is referred to VJS,
  and a proof-only candidate may be recorded but not treated as granted.
- **W3 PRINCIPAL-ACCEPTED** by the Principal alone, as a dated digest-pinned acceptance event.
  No proof substitutes for it, no bench may grant it, and acceptance is never inferred from
  silence.
- **W4 PARITY** on `parity` for every registered component, plus `token_pin` and `contrast`
  re-run against the shipped CSS.

Every warrant names `case_file_digest` and every proof it relies on by id and digest. A warrant
carrying no evidence entry is a signature on nothing and is void on its face.

### 10. File the reserved matters

Five clauses depend on unsettled points (VDS S-13). Each needs a submission on file, naming its
`reserved_clause` and its `fail_closed_interim`, whether or not you expect to hit it. A clause
depending on an unsettled point with no submission behind it is how a reserved matter quietly
becomes a settled one by default.

---

## After adoption: the two things that rot

**Warrants go stale silently.** A warrant is spent when the surface it was granted over
changes. Adding a screen after W2, or a component after W1, does not inherit the warrant: the
proof re-runs and the warrant is re-granted or refused, and `status: spent` is recorded rather
than deleted. If nothing in the project notices a surface change, the warrant chain becomes
decorative.

**Proof records fall behind decision logs.** Measured in VJS at VDS drafting time: 173 decision
logs against 3 proof records. Decision logs are cheap and proofs are not, so the ratio drifts
in exactly one direction. VDS's whole value sits on the proof surface, which is why capture is
wired into the checker rather than left to be written by hand, and why `docs/GOAL.md` D9 makes
the ratio a watched number rather than an assumed one.

## What adoption does not give you

- It does not make the design good. W3 exists because that judgement is the Principal's.
- It does not prove "no unregistered component anywhere". The `composition` proof covers the
  declared screen set; a screen outside that set is outside the proof, which is why every
  warrant names its surface by digest.
- It does not make the enforcement surface tamper-proof. An author with write access can edit a
  gate and re-pin it in the same act. The lock makes that visible in a diff. It does not
  prevent it, and no VDS document may claim otherwise.
