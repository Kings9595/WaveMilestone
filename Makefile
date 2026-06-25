.PHONY: build test lint fmt clean setup help

help:
	@echo "Usage: make <target>"
	@echo ""
	@echo "  build   Build the Soroban contract WASM"
	@echo "  test    Run all tests"
	@echo "  lint    Run clippy and fmt check"
	@echo "  fmt     Format code"
	@echo "  clean   Remove build artifacts"
	@echo "  setup   Bootstrap Soroban dependencies"

build:
	./scripts/build.sh release

test:
	./scripts/test.sh

lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

clean:
	cargo clean

setup:
	./scripts/bootstrap.sh
