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
	$(VDS) proof --all --invoked-by package_script

.PHONY: doctor
doctor: build
	$(VDS) doctor --report-only

.PHONY: check
check:
	$(CARGO) fmt --all --check
	$(MAKE) lint
	$(MAKE) test
	$(MAKE) gates
	$(MAKE) doctor
