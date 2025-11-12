#!/bin/bash
# Quick CI check script - run before pushing to ensure CI will pass

set -e  # Exit on first error

echo ""
echo "KEYLESS CI CHECK"
echo "────────────────────────────────────────────────"
echo ""

# Step 1: Format
printf "[1/11] Format check...            "
if cargo fmt --all -- --check > /dev/null 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      Fix with: cargo fmt --all"
    exit 1
fi

# Step 2: Clippy
printf "[2/11] Clippy (strict)...         "
if cargo clippy --all-targets -- -D warnings -D clippy::unwrap_used -D clippy::expect_used > /tmp/clippy.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/clippy.log | grep error"
    exit 1
fi

# Step 3: Build
printf "[3/11] Build (all targets)...     "
if cargo build --workspace --all-targets > /tmp/build.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/build.log | grep error"
    exit 1
fi

# Step 4: Test
printf "[4/11] Tests (workspace)...       "
if cargo test --workspace --no-fail-fast > /tmp/test.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/test.log | grep FAILED"
    exit 1
fi

# Step 5: Docs build (warnings as errors)
printf "[5/11] Docs build...             "
if RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps > /tmp/docs.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/docs.log | grep error"
    exit 1
fi

# Step 6: Doctests
printf "[6/11] Doctests...               "
if cargo test --workspace --doc > /tmp/doctest.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/doctest.log | grep FAILED"
    exit 1
fi

# Step 7: Clippy doc lints on private items
printf "[7/11] Clippy (docs private)...  "
if cargo clippy --workspace -- -D clippy::missing_docs_in_private_items > /tmp/clippy_docs.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/clippy_docs.log | grep error"
    exit 1
fi

# Step 8: Frontend typecheck (TS)
printf "[8/11] Frontend typecheck...     "
if pnpm -C keyless-desktop install --frozen-lockfile > /tmp/frontend_install.log 2>&1 && pnpm -C keyless-desktop exec tsc --noEmit > /tmp/frontend_typecheck.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      Install log: sed -n '1,80p' /tmp/frontend_install.log"
    echo "      See errors: sed -n '1,200p' /tmp/frontend_typecheck.log"
    exit 1
fi

# Step 9: Frontend tests (Vitest)
printf "[9/11] Frontend tests...         "
if pnpm -C keyless-desktop test > /tmp/frontend_test.log 2>&1; then
    echo "✓"
else
    # Check if the failure is due to no test files found (which is acceptable)
    if grep -q "No test files found" /tmp/frontend_test.log; then
        echo "✓ (no tests)"
    else
        echo "✗"
        echo "      See errors: sed -n '1,200p' /tmp/frontend_test.log"
        exit 1
    fi
fi

# Step 10: Cargo deny (security advisories, licenses, duplicates)
printf "[10/11] Cargo deny check...       "
if cargo deny check > /tmp/cargo_deny.log 2>&1; then
    echo "✓"
else
    echo "✗"
    echo "      See errors: cat /tmp/cargo_deny.log"
    echo "      Install with: cargo install cargo-deny --locked"
    exit 1
fi

# Step 11: Cargo audit (RustSec vulnerability database)
printf "[11/11] Cargo audit...            "
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

