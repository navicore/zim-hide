# Zimhide Build System
#
# This is the SOURCE OF TRUTH for all build/test/lint operations.
# GitHub Actions calls these recipes directly - no duplication!

# Default recipe: show available commands
default:
    @just --list

# Build the release binary
build:
    @echo "Building zimhide..."
    cargo build --release
    @echo "✅ Built: target/release/zimhide"

# Run all Rust tests (unit + integration)
test:
    @echo "Running tests..."
    cargo test --all-targets

# Run clippy with opinionated lints
lint:
    @echo "Running clippy..."
    cargo clippy --all-targets -- \
        -D warnings \
        -D clippy::all \
        -A clippy::missing_errors_doc \
        -A clippy::missing_panics_doc \
        -A clippy::module_name_repetitions \
        -A clippy::must_use_candidate \
        -A clippy::redundant_pub_crate \
        -A clippy::significant_drop_tightening \
        -A clippy::cast_possible_truncation \
        -A clippy::cast_precision_loss \
        -A clippy::cast_sign_loss \
        -A clippy::cast_lossless \
        -A clippy::needless_pass_by_value \
        -A clippy::uninlined_format_args \
        -A clippy::missing_const_for_fn \
        -A clippy::option_if_let_else \
        -A clippy::cast_possible_wrap

# Format all code
fmt:
    @echo "Formatting code..."
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    @echo "Checking code formatting..."
    cargo fmt --all -- --check

# Run all CI checks (same as GitHub Actions!)
# This is what developers should run before pushing
ci: fmt-check lint test build
    @echo ""
    @echo "✅ All CI checks passed!"
    @echo "   - Code formatting ✓"
    @echo "   - Clippy lints ✓"
    @echo "   - Tests ✓"
    @echo "   - Build ✓"
    @echo ""
    @echo "Safe to push to GitHub - CI will pass."

# Development: quick format + build + test
dev: fmt build test

# Show test output (verbose)
test-verbose:
    cargo test -- --nocapture

# Clean all build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean
    @echo "✅ Clean complete"

# Generate documentation
doc:
    cargo doc --no-deps --open

# Install to ~/.cargo/bin
install:
    @echo "Installing zimhide..."
    cargo install --path .
    @echo "✅ Installed to ~/.cargo/bin/zimhide"

# Check for outdated dependencies
outdated:
    cargo outdated

# Run security audit
audit:
    cargo audit

# Verify crate can be published
publish-dry-run:
    @echo "Checking if crate can be published..."
    cargo publish --dry-run
    @echo "✅ Crate is ready to publish"
