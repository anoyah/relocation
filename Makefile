.PHONY: build
build:
	cargo build --release

.PHONY: debug
debug:
	cargo build

.PHONY: run
run:
	cargo run -- --help

.PHONY: test
test:
	cargo test

.PHONY: fmt
fmt:
	cargo fmt

.PHONY: check
check:
	cargo clippy --all-targets --all-features -- -D warnings
