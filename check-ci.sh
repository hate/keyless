#!/bin/bash
# Quick CI check script - run before pushing to ensure CI will pass

set -e  # Exit on first error

echo ""
echo "KEYLESS CI CHECK"
echo "────────────────────────────────────────────────"
echo ""

# Step 1: Format
printf "[1/9] Format check...            "
if cargo fmt --all -- --check > /dev/null 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      Fix with: cargo fmt --all"
    exit 1
fi

# Step 2: Clippy
printf "[2/9] Clippy (strict)...         "
if cargo clippy --all-targets -- -D warnings -D clippy::unwrap_used -D clippy::expect_used > /tmp/clippy.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/clippy.log | grep error"
    exit 1
fi

# Step 3: Build
printf "[3/9] Build (all targets)...     "
if cargo build --workspace --all-targets > /tmp/build.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/build.log | grep error"
    exit 1
fi

# Step 4: Test
printf "[4/9] Tests (workspace)...       "
if cargo test --workspace --no-fail-fast > /tmp/test.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/test.log | grep FAILED"
    exit 1
fi

# Step 5: Docs build (warnings as errors)
printf "[5/9] Docs build...              "
if RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps > /tmp/docs.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/docs.log | grep error"
    exit 1
fi

# Step 6: Doctests
printf "[6/9] Doctests...                "
if cargo test --workspace --doc > /tmp/doctest.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/doctest.log | grep FAILED"
    exit 1
fi

# Step 7: Clippy doc lints on private items
printf "[7/9] Clippy (docs private)...   "
if cargo clippy --workspace -- -D clippy::missing_docs_in_private_items > /tmp/clippy_docs.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/clippy_docs.log | grep error"
    exit 1
fi

# Step 8: Cargo deny (security advisories, licenses, duplicates)
printf "[8/9] Cargo deny check...        "
if cargo deny check > /tmp/cargo_deny.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/cargo_deny.log"
    echo "      Install with: cargo install cargo-deny --locked"
    exit 1
fi

# Step 9: Cargo audit (RustSec vulnerability database)
printf "[9/9] Cargo audit...             "
if cargo audit > /tmp/cargo_audit.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/cargo_audit.log"
    echo "      Install with: cargo install cargo-audit --locked"
    exit 1
fi

echo ""
echo "────────────────────────────────────────────────"
echo "All checks passed - ready to push"
echo ""

