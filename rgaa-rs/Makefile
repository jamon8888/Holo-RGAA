.PHONY: build build-release test check fmt fmt-check clippy ci clean

build:
	cargo build --workspace

build-release:
	cargo build --workspace --release

test:
	cargo nextest run --workspace

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

check: fmt-check clippy test

ci: check build-release

clean:
	cargo clean
