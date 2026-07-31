# VDS. One command per thing a contributor actually does.
#
# `make check` is the same set the CI trust root runs, in the same order, so a
# green local run and a green remote run mean the same thing. Where they differ,
# CI wins: VDS S-7(3) holds that a hook is not CI, and the same is true of a
# Makefile.

CARGO  ?= cargo
VDS    ?= ./target/release/vds
PYTHON ?= python3

.PHONY: help
help:
	@echo 'make check     format, lint, test, and every VDS gate (what CI runs)'
	@echo 'make test      the test suite, including every failing-direction test'
	@echo 'make lint      clippy with warnings denied'
	@echo 'make fmt       format in place'
	@echo 'make build     the release binary at $(VDS)'
	@echo 'make schemas   regenerate schema/*.schema.json from the Rust types'
	@echo 'make hooks     point core.hooksPath at the committed pre-push hook'
	@echo 'make gates     the VDS gates alone, against this repository'
	@echo 'make gates-example  all eleven kinds over examples/storefront, no exemption'
	@echo 'make ci-ledger measure whether the CI workflow has EVER concluded (network)'
	@echo 'make prune     bound the proof working set (deletes, logs what it removed)'
	@echo 'make doctor    measure this project against the ten done criteria'

# The interim enforcement surface, and the reason it exists is BREACH-0011: the
# vds-enforce workflow has never had a successful run - fifty-three failures, zero
# successes, every one "The job was not started because recent account payments have
# failed" - and `.git/hooks/` held nothing but git's samples. So seventeen pinned gates
# had never been executed by anything except a person choosing to.
#
# `core.hooksPath` rather than copying into `.git/hooks/`: a hook under `.git/` is
# invisible to a clone, absent from every diff and covered by no digest, which is the
# same defect one level down. Pointing at the committed directory makes the hook
# reviewable and lets `.vds/enforcement.lock` pin it.
.PHONY: hooks
hooks:
	git config core.hooksPath scripts/githooks
	@chmod +x scripts/githooks/*
	@echo 'core.hooksPath -> scripts/githooks'
	@echo 'VDS S-7(3): a hook is NOT CI. `git push --no-verify` walks past it, so this'
	@echo 'is an interim surface and D4 stays UNMET until a real run concludes.'

.PHONY: fmt
fmt:
	$(CARGO) fmt --all

.PHONY: lint
lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

.PHONY: test
test:
	$(CARGO) test --workspace

# site-factory is JavaScript and has no cargo target, so `make test` never touched
# it and it shipped ungated. Run through tests/gate.js rather than `node --test`
# directly: a bare glob EXITS 0 WHEN IT MATCHES NOTHING, so a rename would turn the
# gate off and still report success. The gate asserts a floor on what actually ran.
#
# .PHONY matters here more than anywhere: this was the only target in the file without
# it, and a file or directory named `test-factory` at the repo root would make `make`
# decide the target is up to date and skip the gate silently. That is exactly the
# "green light wired to nothing" the comment above exists to prevent.
.PHONY: test-factory
test-factory:
	node site-factory/tests/gate.js

.PHONY: build
build:
	$(CARGO) build --release --bin vds

.PHONY: schemas
schemas: build
	$(VDS) schema emit

.PHONY: gates
gates: build
	$(VDS) schema check
	$(VDS) pack verify
	$(VDS) lock verify
	$(VDS) ledger screens
	$(VDS) proof --all --invoked-by package_script --allow-vacuous
	@# Without the exemption: the two kinds genuinely enforced against VDS itself.
	$(VDS) proof no_stored_values --invoked-by package_script
	$(VDS) proof ledger_staleness --invoked-by package_script

# The worked example, and the only place every implemented kind is enforced over
# rows that exist.
#
# VDS itself has no screens and no component library, so five of the seven come
# out vacuous above and the `--all` line needs --allow-vacuous to be non-blocking.
# A vacuous pass is not evidence (VDS S-7(2)(4)), so on its own the repository
# could not demonstrate that its own gates catch anything.
#
# `examples/storefront` is a real subject: three screens, six components, four
# registered screen arrangements, two themes and one deprecated record draining
# to zero. Every kind runs over real
# rows and NO EXEMPTION IS PASSED, so a vacuity here is a red build. This is the
# target that makes `vds doctor` D1, D2 and D3 answerable.
.PHONY: gates-example
gates-example: build
	$(VDS) ledger screens --root examples/storefront
	@# The figma ledger, regenerated from a response committed outside `.vds/`.
	@# Three proofs read it: `states` measures states.drawn against it,
	@# `reconciliation` resolves limb (c) against it, and `ledger_staleness` R3
	@# checks it is self-consistent and not older than the records that read it.
	$(VDS) figma pull --root examples/storefront \
	    --from examples/storefront/figma/file-SFDEMO.json
	@# The FRAME ledger, the other half of the Figma seam and the one the eleventh
	@# kind reads. `figma pull` records the file's COMPONENT sets; this records what
	@# its SCREEN frames draw. Both are derived from a response committed outside
	@# `.vds/`, because VDS S-7(2)(1) forbids a network call inside a proof.
	$(VDS) figma frames --root examples/storefront --file-key SFDEMO \
	    --from examples/storefront/figma/frames-SFDEMO.json
	@# Every kind, so each is exercised and recorded. --allow-vacuous is needed for
	@# exactly one of them and the next line is why the flag buys nothing here.
	$(VDS) proof --all --root examples/storefront --invoked-by package_script --allow-vacuous
	@# All eleven kinds, run with NO exemption, so a vacuity in any of them is a red
	@# build. `token_pin` comes last because it needs a pin GENERATED first, and
	@# that generation is out of band by design: one of the two records it compares
	@# is behind a network call VDS S-7(2)(1) forbids inside a proof.
	$(VDS) proof register_completeness --root examples/storefront --invoked-by package_script
	$(VDS) proof reconciliation        --root examples/storefront --invoked-by package_script
	$(VDS) proof composition           --root examples/storefront --invoked-by package_script
	$(VDS) proof contrast              --root examples/storefront --invoked-by package_script
	$(VDS) proof states                --root examples/storefront --invoked-by package_script
	$(VDS) proof parity                --root examples/storefront --invoked-by package_script
	$(VDS) proof retirement_drain      --root examples/storefront --invoked-by package_script
	$(VDS) proof ledger_staleness      --root examples/storefront --invoked-by package_script
	$(VDS) proof no_stored_values      --root examples/storefront --invoked-by package_script
	@# The eleventh kind, and the only one whose subject is a SCREEN. Four screens
	@# are registered here: three are SCORED against the arrangement their
	@# authoritative frame draws, and one is EXCLUDED because its frame disclaims
	@# itself. The coverage line says which, because a screen gate that measures
	@# what it happens to understand and prints a clean pass is the exact failure
	@# this kind was added to prevent (VDS S-5A(7)).
	$(VDS) proof screen_parity         --root examples/storefront --invoked-by package_script
	@# The tenth kind, and the one that was vacuous everywhere until a generator
	@# existed. The pin is REGENERATED from a response committed outside `.vds/`
	@# and then CHECKED, which is two different acts: generating it proves the
	@# derivation still runs, checking it proves the two records still agree.
	$(VDS) pin generate --root examples/storefront --file-key SFDEMO \
	    --from examples/storefront/figma/variables-export.json \
	    --subject "the storefront palette and spacing"
	$(VDS) proof token_pin             --root examples/storefront --invoked-by package_script

# The CI run ledger, and it is DELIBERATELY not part of `check`.
#
# It asks the forge whether the workflow ever concluded, which is a network call, and
# VDS S-7(2)(1) forbids one inside a proof - the same reason `figma pull` is out of band.
# Folding it into `check` would also make `check` fail on an aeroplane, which teaches
# people to skip the gate.
#
# Regenerate it rather than trusting the committed copy: a ledger nobody regenerates is a
# ledger that records an old answer confidently. D4 reads whatever is on disk.
.PHONY: ci-ledger
ci-ledger: build
	$(VDS) ledger ci

.PHONY: doctor
doctor: build
	$(VDS) doctor --report-only

# Bound the proof working set. Deliberately NOT part of `check`: a delete that
# runs as a side effect of a test command is a delete nobody chose.
.PHONY: prune
prune: build
	$(VDS) prune --apply
	$(VDS) prune --root examples/storefront --apply

.PHONY: check
check:
	$(CARGO) fmt --all --check
	$(MAKE) lint
	$(MAKE) test
	$(MAKE) test-factory
	$(MAKE) gates
	$(MAKE) gates-example
	$(MAKE) doctor
