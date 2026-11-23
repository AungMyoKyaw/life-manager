.PHONY: dev build release bundle test lint fmt check validate-design

dev:
	cargo run -p life-gpui

build:
	cargo build -p life-gpui -p life-cli

release:
	cargo build --release -p life-gpui -p life-cli

bundle:
	./scripts/release.sh build

test:
	cargo test -p life-core -p life-cli -p life-gpui

lint:
	cargo clippy -p life-core -p life-cli -p life-gpui --all-targets -- -D warnings

fmt:
	cargo fmt --all

check:
	cargo fmt --all -- --check
	cargo clippy -p life-core -p life-cli -p life-gpui --all-targets -- -D warnings
	cargo test -p life-core -p life-cli -p life-gpui

validate-design:
	python3 design/scripts/validate_design.py

