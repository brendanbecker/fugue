.PHONY: build install install-release test check clean

# Development build
build:
	cargo build

# Release build
release:
	cargo build --release

# Install to ~/.cargo/bin (release mode)
install:
	cargo install --path fugue-client
	cargo install --path fugue-server
	cargo install --path fugue-compat
	cargo install --path fugue-sandbox

# Run all tests
test:
	cargo test

# Quick check (no codegen)
check:
	cargo check

# Lint
lint:
	cargo clippy -- -D warnings

# Format check
fmt:
	cargo fmt --check

# Format fix
fmt-fix:
	cargo fmt

# Clean build artifacts
clean:
	cargo clean

# Full CI check
ci: fmt check lint test

# Build and install (convenience target)
all: release install
