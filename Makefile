.PHONY: lint lint-fix format test check

lint:
	cargo clippy --all-features --all-targets -- -D warnings

lint-fix:
	cargo clippy --fix --allow-staged --allow-dirty --all-features --all-targets
	cargo +nightly fmt --all

format:
	cargo +nightly fmt --all

test:
	cargo test --all-features

check:
	cargo check --all-features --all-targets

