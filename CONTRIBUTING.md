# Contributing to keyless

Thanks for your interest! This guide covers everything you need to contribute.

## Quick Start

```bash
# Fork on GitHub, then clone
git clone https://github.com/hate/keyless.git
cd keyless

# Build and test
cargo build
cargo test --workspace

# Run CI checks locally (format, clippy, build, tests, docs, doctests)
./check-ci.sh
```

---

## Development Workflow

### 1. Create a branch
```bash
git checkout -b feat/your-feature
# or fix/your-bugfix
```

### 2. Make changes

**Before committing:**
```bash
./check-ci.sh  # macOS/Linux (includes docs + doctests)
# or
.\check-ci.ps1  # Windows (PowerShell; includes docs + doctests)
```

### 3. Commit

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(whisper): add temperature fallback
fix(audio): prevent VAD flicker  
docs(readme): update installation steps
chore(ci): add Windows to matrix
```

**Format:** `<type>(<scope>): <description>`

**Commit Types:**

- **`feat`** - New user-facing functionality or features
  - Example: `feat(whisper): add temperature fallback`, `feat(tui): add config screen`
  - Use when: Adding new capabilities users can interact with

- **`fix`** - Bug fixes that resolve incorrect behavior
  - Example: `fix(audio): prevent VAD flicker`, `fix(runtime): handle PTT release edge case`
  - Use when: Correcting bugs or broken functionality

- **`docs`** - Documentation-only changes (README, comments, rustdoc)
  - Example: `docs(readme): update installation steps`, `docs(api): add safety notes`
  - Use when: Only changing documentation, no code changes

- **`style`** - Code style/formatting changes that don't affect logic
  - Example: `style: run cargo fmt`, `style(tui): fix indentation`
  - Use when: Whitespace, formatting, or style-only changes

- **`refactor`** - Code restructuring that improves structure without changing behavior
  - Example: `refactor(runtime): extract pipeline setup`, `refactor(audio): simplify VAD logic`
  - Use when: Improving code organization without fixing bugs or adding features

- **`perf`** - Performance improvements
  - Example: `perf(eq): cache FFT planner`, `perf(whisper): reduce allocations`
  - Use when: Optimizing speed, memory usage, or efficiency

- **`test`** - Adding or modifying tests
  - Example: `test(audio): add VAD threshold tests`, `test(models): cover download cancellation`
  - Use when: Test code changes only

- **`chore`** - Maintenance tasks (dependencies, CI, build, tooling)
  - Example: `chore(deps): update tokio`, `chore(ci): add Windows to matrix`, `chore: initial commit`
  - Use when: Infrastructure, tooling, or housekeeping that doesn't change functionality

**Scopes** (optional but recommended):
- Crate names: `whisper`, `audio`, `tui`, `runtime`, `models`, `output`, `core`, `logging`
- Areas: `ci`, `docs`, `build`, `deps`, `git`
- Leave out scope if change spans multiple areas: `chore: initial commit`

### 4. Push and create PR

```bash
git push origin feature/your-feature
```

Then open PR on GitHub.

---

## Code Standards

### Must Follow

- ✅ Run `cargo fmt --all` before committing
- ✅ No `clippy` warnings
- ✅ **No `unwrap()` or `expect()` in production code**
- ✅ Use `KeylessError` for errors
- ✅ Use `tracing` for logging
- ✅ Add tests for new features
- ✅ Document public APIs (rustdoc/JSDoc)
- ✅ Add inline comments for complex logic (Rust and TypeScript)

### Documentation Style

We enforce comprehensive documentation across the codebase:

**Rust (rustdoc):**
- Crate/module headers use `//!` with purpose and relationships
- All items (public and internal) use `///` with a first-line summary
- Include sections where applicable:
  - Examples, Errors, Panics, Safety (Send/Sync/FFI), Threading/Blocking, Performance, Invariants
- Prefer small compilable examples (doctests)

**Rust (inline comments):**
- Add inline comments (`//`) for complex logic, algorithms, and non-obvious code paths
- Document thread-safety considerations, state management, and async boundaries
- Explain business logic, state transitions, and error handling strategies

**TypeScript/React (JSDoc + inline):**
- File-level JSDoc headers (`/** */`) explaining component/hook purpose and usage
- Inline comments (`//`) for complex logic, state management, and event handling
- Document React hooks, state variables, effects, and event listeners
- Explain business logic, async operations, and error handling

Template:

```rust
/// One-line summary.
///
/// Longer context if needed.
///
/// # Examples
/// ```no_run
/// // minimal, compilable usage
/// ```
///
/// # Errors
/// // when and why errors occur
///
/// # Panics
/// // when and why panics occur (ideally never)
///
/// # Safety
/// // safety constraints or Send/Sync invariants
///
/// # Performance
/// // allocations, blocking, complexity, hot paths
```

### Error Handling

```rust
// ✅ Production code
pub fn do_work() -> KeylessResult<()> {
    something()?;  // Uses KeylessError
    Ok(())
}

// ✅ Tests (Box<dyn Error> is OK)
#[test]
fn test_something() -> Result<(), Box<dyn std::error::Error>> {
    do_work()?;
    Ok(())
}
```

### Logging

```rust
use tracing::{info, debug, error};

info!(model = %path, "model loaded");
error!(error = %e, "failed to start");
```

---

## Pull Request Guidelines

### Before Submitting

**Run this:**
```bash
./check-ci.sh   # macOS/Linux
.\check-ci.ps1  # Windows
```

**Must show:** All checks pass

## Testing

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p keyless-whisper

# Watch for failures
cargo test --workspace --no-fail-fast
```

**Tests with I/O** should return `Result<(), Box<dyn Error>>`:
```rust
#[test]
fn test_file_write() -> Result<(), Box<dyn std::error::Error>> {
    fs::write("test.txt", "data")?;
    Ok(())
}
```

---

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
