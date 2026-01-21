.PHONY: build install clean test dev hardening increment-version

# Increment version number
increment-version:
	@echo "Incrementing version..."
	@VERSION=$$(grep -A 1 "^\[workspace.package\]" Cargo.toml | grep "^version" | sed 's/version = "\(.*\)"/\1/'); \
	MAJOR=$$(echo $$VERSION | cut -d. -f1); \
	MINOR=$$(echo $$VERSION | cut -d. -f2); \
	PATCH=$$(echo $$VERSION | cut -d. -f3); \
	NEW_PATCH=$$((PATCH + 1)); \
	NEW_VERSION="$$MAJOR.$$MINOR.$$NEW_PATCH"; \
	sed -i.bak "/^\[workspace.package\]/,/^\[/ s/version = \".*\"/version = \"$$NEW_VERSION\"/" Cargo.toml && rm -f Cargo.toml.bak; \
	echo "Version updated: $$VERSION -> $$NEW_VERSION"

# Build all workspace crates (auto-increments version)
build: increment-version
	cargo build --workspace --release

# Development build (also increments version)
dev: increment-version
	cargo build --workspace

# Install system-wide (requires root)
install: build
	cd install && sudo ./install.sh

# Run OS hardening scripts
hardening:
	cd install && sudo ./harden_os.sh

# Clean build artifacts
clean:
	cargo clean

# Run tests
test:
	cargo test --workspace

# Check code without building
check:
	cargo check --workspace

# Format code
fmt:
	cargo fmt --all

# Lint code
lint:
	cargo clippy --workspace -- -D warnings

# Build documentation
docs:
	cargo doc --workspace --no-deps

# Complete development check
verify: fmt lint test
	@echo "All checks passed"
