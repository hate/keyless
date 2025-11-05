# Quick CI check script for Windows - run before pushing
# PowerShell version of check-ci.sh

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "KEYLESS CI CHECK"
Write-Host "────────────────────────────────────────────────"
Write-Host ""

# Step 1: Format
Write-Host -NoNewline "[1/11] Format check...           "
$null = cargo fmt --all -- --check 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      Fix with: cargo fmt --all"
    exit 1
}

# Step 2: Clippy
Write-Host -NoNewline "[2/11] Clippy (strict)...        "
$null = cargo clippy --all-targets -- -D warnings -D clippy::unwrap_used -D clippy::expect_used 2>&1 | Out-File -FilePath $env:TEMP\clippy.log
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\clippy.log | Select-String error"
    exit 1
}

# Step 3: Build
Write-Host -NoNewline "[3/11] Build (all targets)...    "
$null = cargo build --workspace --all-targets 2>&1 | Out-File -FilePath $env:TEMP\build.log
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\build.log | Select-String error"
    exit 1
}

# Step 4: Test
Write-Host -NoNewline "[4/11] Tests (workspace)...      "
$null = cargo test --workspace --no-fail-fast 2>&1 | Out-File -FilePath $env:TEMP\test.log
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\test.log | Select-String FAILED"
    exit 1
}

# Step 5: Docs build
Write-Host -NoNewline "[5/11] Docs build...             "
$env:RUSTDOCFLAGS = "-D warnings"
$null = cargo doc --workspace --no-deps 2>&1 | Out-File -FilePath $env:TEMP\docs.log
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\docs.log | Select-String error"
    exit 1
}

# Step 6: Doctests
Write-Host -NoNewline "[6/11] Doctests...               "
$null = cargo test --workspace --doc 2>&1 | Out-File -FilePath $env:TEMP\doctest.log
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\doctest.log | Select-String FAILED"
    exit 1
}

# Step 7: Clippy doc lints on private items
Write-Host -NoNewline "[7/11] Clippy (docs private)...  "
$null = cargo clippy --workspace -- -D clippy::missing_docs_in_private_items 2>&1 | Out-File -FilePath $env:TEMP\clippy_docs.log
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\clippy_docs.log | Select-String error"
    exit 1
}

# Step 8: Frontend typecheck (TS)
Write-Host -NoNewline "[8/11] Frontend typecheck...     "
$null = pnpm -C keyless-desktop install --frozen-lockfile 2>&1 | Out-File -FilePath $env:TEMP\frontend_install.log
if ($LASTEXITCODE -ne 0) {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\frontend_install.log | Select-String -Pattern 'ERR|error'"
    exit 1
}
$null = pnpm -C keyless-desktop exec tsc --noEmit 2>&1 | Out-File -FilePath $env:TEMP\frontend_typecheck.log
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\frontend_typecheck.log | Select-String -Pattern 'error'"
    exit 1
}

# Step 9: Frontend tests (Vitest)
Write-Host -NoNewline "[9/11] Frontend tests...         "
$null = pnpm -C keyless-desktop test 2>&1 | Out-File -FilePath $env:TEMP\frontend_test.log
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\frontend_test.log | Select-String -Pattern 'FAIL|Error'"
    exit 1
}

# Step 10: Cargo deny (security advisories, licenses, duplicates)
Write-Host -NoNewline "[10/11] Cargo deny check...       "
$null = cargo deny check 2>&1 | Out-File -FilePath $env:TEMP\cargo_deny.log
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\cargo_deny.log"
    Write-Host "      Install with: cargo install cargo-deny --locked"
    exit 1
}

# Step 11: Cargo audit (RustSec vulnerability database)
Write-Host -NoNewline "[11/11] Cargo audit...            "
$null = cargo audit 2>&1 | Out-File -FilePath $env:TEMP\cargo_audit.log
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓"
} else {
    Write-Host "✗"
    Write-Host "      See errors: Get-Content $env:TEMP\cargo_audit.log"
    Write-Host "      Install with: cargo install cargo-audit --locked"
    exit 1
}

Write-Host ""
Write-Host "────────────────────────────────────────────────"
Write-Host "All checks passed - ready to push"
Write-Host ""

