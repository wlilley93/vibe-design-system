# Adopting VDS

This file is engineering explanation. It binds nothing (VDS S-3(4)).

Every command below has been run against a real project. Where a step cannot be completed
with the current build, it says so and says what to do instead. The previous version of this
document listed ten steps of which eight could not be executed, which is the defect VDS
exists to prevent, committed by VDS against its own users.

---

## Before you start: what this costs

**The register is the expensive part, and it costs the same whether or not you adopt VDS.**
It is "write down every component and what it must do". Skipping VDS does not avoid that
cost; it decides not to write it down, which is the state that produced both defects at
VDS S-1(4). Read VDS S-14 before committing to this.

**The reconciliation proof will fail on day one and keep failing** until the register is
genuinely complete. That is a feature and will not feel like one.

---

## 1. Build the binary

```bash
git clone https://github.com/wlilley93/vibe-design-system
cd vibe-design-system
cargo build --release --bin vds
# put ./target/release/vds on PATH, or call it by path below
```

Rust 1.95.0, pinned in `rust-toolchain.toml`. The one network call VDS makes shells out to
`curl`, and it is not on the proof path.

## 2. Scaffold the record

```bash
cd /path/to/your/project
vds init
```

This writes `.vds/config.toml`, `.vds/designpack.lock`, a `.gitignore` ignoring exactly
`cache/` and `private/` (VDS S-3(9): a governance record that is gitignored is not a record),
and the record directories.

**Commit `.vds/` now**, before it holds anything. The record is committed, not scratch.

## 3. Declare your surface

Open `.vds/config.toml` and set three things. Everything VDS ever claims is bounded by them,
so getting them wrong makes every proof narrower or wider than you think.

```toml
[surface]
# Every screen VDS will reason about. A screen outside these globs is outside every proof.
screen_globs = ["app/**/page.tsx"]
# An import starting with one of these is IN SCOPE. Anything else is counted, not enforced,
# and the count is printed, so the carve-out is visible rather than assumed.
governed_import_prefixes = ["@/components/"]
# Directories the register is expected to cover. `reconciliation` walks them to find code
# with no register entry. A directory that is not there makes the proof REFUSE rather than
# report a narrowing, so leave this empty rather than aspirational.
library_dirs = ["src/components/ui"]
```

Only `*` (within a segment), `**` (across segments) and `?` are supported. A brace or a
character class is REFUSED rather than half-understood, because a glob that silently matches
less than you meant makes every proof narrower and nothing says so.

## 4. Generate the declared surface

```bash
vds ledger screens
```

Read the output. It reports how many screens matched, how many component references and bare
elements it found, and how many imports it could not resolve. **If it says the surface
matched no file, stop and fix the globs**: every proof will be vacuous.

If it REFUSES, naming a file it could not scan completely, fix that file. A reference the
scanner did not see is not skipped and not counted; it does not exist, and a ledger built
from it would make every proof narrower than it looks.

## 5. Register components, before designing them

```bash
vds register add --name Button \
  --import-path @/components/ui \
  --source-file src/components/ui/button.tsx \
  --export-name Button \
  --require default,hover,focus,disabled \
  --drawn default \
  --role button \
  --keyboard 'Enter=activates' \
  --floor 'control-border:surface:3.0:WCAG 2.2 SC 1.4.11:control_boundary'
```

`add` mints only at `proposed` or `designed`. To go further, advance one step at a time:

```bash
vds register set-status CMP-0001 designed
vds register set-status CMP-0001 registered
```

The lifecycle is a directed path and skipping is forbidden (VDS S-5(4)). This is not
ceremony: VDS S-6(2) calls the ordering "the entire mechanism", because every drift defect
measured in the motivating project was authored before anyone asked whether the thing being
used was registered.

A floor is a **requirement** drawn from a standard, never a colour. `3.0` is lawful because
WCAG 2.2 SC 1.4.11 says so; `#ebebeb` has nowhere to go.

## 6. Run the proofs

```bash
vds proof --list          # the closed registry, and why three kinds are unbuilt
vds proof --all
```

Expect failures. `register_completeness` will name every component your screens use that the
register does not know, with the file and the line. That list is the work.

Exit codes: `0` passed, `1` a violation, `2` a precondition failed and the proof DID NOT RUN,
`3` vacuous. A vacuous run is not a pass: it means no row was in an enforceable state, and it
is never evidence for a warrant (VDS S-7(2)(4)).

## 7. Wire the gates into CI, and pin them

A hook is not CI. `git commit --no-verify` bypasses a local hook, so the invocation limb at
VDS S-7(2)(3) is satisfied by a remote check and only interim-satisfied by a hook
(VDS S-7(3)). Copy `.github/workflows/vds-enforce.yml` from this repository and adapt it,
then pin each gate:

```bash
vds lock add crates/vds-proof/src/composition.rs \
  --proves composition \
  --invoked-by 'ci_workflow=.github/workflows/vds-enforce.yml job:enforce=blocking' \
  --test-path crates/vds-proof/src/composition.rs \
  --test-name composition_fails_on_an_unregistered_component \
  --seeds 'a screen importing a governed component with no register record'
vds lock verify
```

An entry cannot be written without naming the test that proves the gate's FAILING direction.
That is how VDS S-7(2)(2) is made structural rather than requested: a check whose failing
direction is asserted nowhere has proven only its happy path.

**Make the CI job a required status check.** Without that its verdict is advisory, and D4 is
not met however green it goes.

## 8. Measure yourself

```bash
vds doctor
```

Ten criteria, each naming the command that settled it. It will not flatter you, and a
criterion this build cannot settle reports NOT CHECKED and is counted separately, because a
report listing only what it can check reads as a clean bill of health.

## 9. Wire the design round trip

See [`FIGMA-ROUND-TRIP.md`](FIGMA-ROUND-TRIP.md). Briefly:

```bash
vds brief > brief.md            # hand to a generating agent BEFORE it draws
export FIGMA_TOKEN=...
vds figma pull                  # measure what it actually drew
vds impl CMP-0001               # hand to an implementing agent BEFORE it writes
```

## 10. Warrants, when you are ready

**VDS grants nothing.** W1, W2 and W4 are VJS's on a referred submission, and W3 is the
Principal's alone (VDS S-1(3), S-6(7)). `vds warrant record` writes down a grant that
happened elsewhere and pins the evidence it was made on. If no such grant happened, the file
it writes is a false statement of the record and not a warrant.

```bash
vds warrant status              # where the chain stops, and whether the surface has moved
```

A stage cannot be recorded before its predecessor is granted, and a predecessor that is
granted but SPENT does not count: the surface has moved, and the warrant has to be re-earned
over the current one.

---

## What is not implemented, so you do not look for it

- **The permit lifecycle** (VDS S-12(1)) and `install.lock` (VDS S-11(4)). The
  `[governance]` globs `vds init` writes are read by nothing yet. They are recorded so the
  surface a permit will cover is declared before the machinery exists.
- **Decision logs and breach reports as commands.** The directories exist; write the files by
  hand against the VJS schemas.
- **A `submit` command.** Submissions are hand-authored into `.vds/submissions/filed/` and
  validated on read. See this repository's own six for the shape.
- **Three of the ten proof kinds**: `contrast`, `parity` and `token_pin`, each of which needs
  a named record VDS reads and does not own. `vds proof --list` prints the reason for each.

A command written in a doc is not a command that runs. Where the machinery does not exist, do
the equivalent by hand and say so in the log.
