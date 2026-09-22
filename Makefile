.PHONY: all fmt fmt-check size-check lint test build package print-rustfmt-nightly

# Load .env if present, for anything a local toolchain wants configured.
-include .env
export

# Default: the whole gate, in the order CI runs it.
default: all

all: fmt-check size-check lint test build

# The largest a Rust source file may get before it has to be split. See
# scripts/check-file-size.sh for why this is enforced rather than advised.
RUST_FILE_LINE_LIMIT ?= 700

size-check:
	@./scripts/check-file-size.sh $(RUST_FILE_LINE_LIMIT)

# The single source of truth for the rustfmt toolchain. rustfmt.toml enables
# unstable options, so formatting is only reproducible against one exact
# nightly. CI installs whatever this names (via print-rustfmt-nightly) and then
# runs these same targets, so a bump here is picked up everywhere -- just run
# `make fmt` and commit the reformat alongside it.
RUSTFMT_NIGHTLY ?= nightly-2026-09-21

# Used by CI to install the pinned toolchain before running fmt-check.
print-rustfmt-nightly:
	@echo $(RUSTFMT_NIGHTLY)

fmt:
	# --all to match fmt-check; run twice because rustfmt is not
	# idempotent in one pass under indent_style = "Visual".
	rustup run $(RUSTFMT_NIGHTLY) cargo fmt --all
	rustup run $(RUSTFMT_NIGHTLY) cargo fmt --all

fmt-check:
	rustup run $(RUSTFMT_NIGHTLY) cargo fmt --all --check

lint:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

build:
	cargo build --workspace

# Builds Knot.app and the DMG with cargo-packager, configured in
# crates/knot/Cargo.toml under [package.metadata.packager]. Needs
# `cargo install cargo-packager --locked`. cargo-packager resolves that config
# from the manifest in the current directory, hence the cd.
package:
	cd crates/knot && cargo packager --release --formats app,dmg
