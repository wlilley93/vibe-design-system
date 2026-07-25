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
	@echo 'make test-py   the Python tooling suite under tools/tests (no install step)'
	@echo 'make lint      clippy with warnings denied'
	@echo 'make fmt       format in place'
	@echo 'make build     the release binary at $(VDS)'
	@echo 'make schemas   regenerate schema/*.schema.json from the Rust types'
	@echo 'make gates     the VDS gates alone, against this repository'
	@echo 'make doctor    measure this project against the ten done criteria'

.PHONY: fmt
fmt:
	$(CARGO) fmt --all

.PHONY: lint
lint:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

.PHONY: test
test:
	$(CARGO) test --workspace

# The Python tooling under tools/ has its own failing-direction suite, and it is
# kept separate from `make test` because it needs no toolchain at all: python3
# and nothing else. VDS S-7(2)(2) is why it exists. Pass T=<module> for one file,
# and VDS_TEST_PROTECT=<tree> to fence an adopting repository's .vds/ as well.
.PHONY: test-py
test-py:
	@tools/run-tests.sh $(T)

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
# `examples/storefront` is a real subject: three screens, six components, two
# themes and one deprecated record draining to zero. Every kind runs over real
# rows and NO EXEMPTION IS PASSED, so a vacuity here is a red build. This is the
# target that makes `vds doctor` D1, D2 and D3 answerable.
.PHONY: gates-example
gates-example: build
	$(VDS) ledger screens --root examples/storefront
	@# Every kind, so each is exercised and recorded. --allow-vacuous is needed for
	@# exactly one of them and the next line is why the flag buys nothing here.
	$(VDS) proof --all --root examples/storefront --invoked-by package_script --allow-vacuous
	@# The nine kinds that are REAL against this subject, run with NO exemption, so
	@# a vacuity in any of them is a red build. `token_pin` is the tenth and is
	@# absent from this list on purpose: it CHECKS a pin and nothing in this build
	@# PRODUCES one, because one of the two records it compares is behind a network
	@# call VDS S-7(2)(1) forbids inside a proof. Hand-authoring a pin to fill the
	@# gap would put a `generated_by` in the record naming a command that does not
	@# exist, which is a record that lies about itself.
	$(VDS) proof register_completeness --root examples/storefront --invoked-by package_script
	$(VDS) proof reconciliation        --root examples/storefront --invoked-by package_script
	$(VDS) proof composition           --root examples/storefront --invoked-by package_script
	$(VDS) proof contrast              --root examples/storefront --invoked-by package_script
	$(VDS) proof states                --root examples/storefront --invoked-by package_script
	$(VDS) proof parity                --root examples/storefront --invoked-by package_script
	$(VDS) proof retirement_drain      --root examples/storefront --invoked-by package_script
	$(VDS) proof ledger_staleness      --root examples/storefront --invoked-by package_script
	$(VDS) proof no_stored_values      --root examples/storefront --invoked-by package_script

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
	$(MAKE) gates
	$(MAKE) gates-example
	$(MAKE) doctor
