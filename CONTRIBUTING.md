# Contributing to Kujo

Thanks for your interest in contributing to Kujo.

We welcome contributions from everyone: beginners, experienced Rust developers, compiler enthusiasts, language design experts, and curious explorers.

---

## Before You Start

1. Read the [README](README.md) for language overview and features
2. Check the [ROADMAP](ROADMAP.md) for planned features and priorities
3. Review the [INSTALLATION](INSTALLATION.md) guide to set up your development environment
4. For language/runtime changes, read the [language spec](docs/LANGUAGE_SPEC.md), [standard library inventory](docs/STANDARD_LIBRARY.md), and [architecture guide](docs/ARCHITECTURE.md)
5. For CLI, LSP, security, or release-facing work, read the [machine-readable CLI contracts](docs/CLI_MACHINE_READABLE_CONTRACTS.md), [native API security posture](docs/NATIVE_API_SECURITY_POSTURE.md), and [release process](docs/RELEASE_PROCESS.md)
6. Browse existing [Issues](https://github.com/kujolang/kujo/issues) to see what needs work

---

## Ways to Contribute

### Bug Fixes
- Fix parsing errors or interpreter crashes
- Improve error messages
- Resolve edge cases in pattern matching or error handling

### Language Features
- Implement features from [ROADMAP](ROADMAP.md)
- Enhance existing features (loops, functions, enums)
- Add new operators or control flow constructs

### Documentation
- Improve code comments and documentation
- Write tutorials or guides
- Create example `.kujo` programs demonstrating language features

### Testing
- Add test cases for edge cases
- Improve test coverage
- Create integration tests

### Tooling
- Enhance CLI functionality
- Build REPL features
- Improve error reporting
- Improve language server, editor adapter, DocGen, and machine-readable output surfaces

### Examples & Demos
- Create example programs showcasing Kujo features
- Write practical demos (file I/O, data processing, etc.)
- Document best practices

---

## Development Setup

### 1. Fork and Clone

```bash
# Fork the repo on GitHub, then:
git clone https://github.com/YOUR_USERNAME/kujo.git
cd kujo
```

### 2. Build the Project

```bash
# Development build (faster compile, slower runtime)
cargo build

# Release build (optimized)
cargo build --release
```

### 3. Run Tests

```bash
# Run all tests
cargo run -- test --runtime vm
cargo run -- test --runtime dual

# Or use the binary directly
./target/debug/kujo test --runtime vm

# Run a specific example
cargo run -- run examples/hello.kujo
```

### 4. Verify Your Changes

```bash
# Format and lint Rust code
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings

# Run Rust/unit/integration tests
cargo test

# Run Kujo fixture tests
cargo run -- test --runtime vm
cargo run -- test --runtime dual

# Run the lightweight release gate
bash scripts/release_gate.sh --minimal
```

---

## Testing Guidelines

### Running Tests

Kujo uses a mix of Rust contract tests, `.kujo` snapshot fixtures, diagnostics goldens, docs/example smoke tests, and release-gate scripts. The fixture runner supports `--runtime vm|dual|interpreter`; the current default is `dual`.

```bash
# Run fixture snapshots through the VM path
cargo run -- test --runtime vm

# Run VM-primary compatibility sweep with interpreter fallback visibility
cargo run -- test --runtime dual

# Run a Kujo file that contains test "..." declarations
cargo run -- test-run tests/generators_test.kujo

# Run focused Rust contract suites
cargo test --test cli_contracts
cargo test --test docs_examples
cargo test --test native_api_security_boundaries
```

### Adding New Tests

1. Create a `.kujo` fixture in `tests/` when the behavior belongs in the snapshot corpus:
   ```bash
   tests/test_my_feature.kujo
   ```

2. Run the fixture and inspect actual versus expected output:
   ```bash
   cargo run -- test --runtime vm -v
   ```

3. Update snapshots only after confirming the behavior is intentional:
   ```bash
   cargo run -- test --runtime vm --update
   ```

4. Commit both `.kujo` and `.kujo.out` files.

5. If the file uses Kujo's `test "..." {}` declaration style instead of snapshot output, mark it with a top-of-file note such as `Run with: kujo test-run tests/my_file.kujo` and validate it with `cargo run -- test-run`.

### Test Naming Convention

Use descriptive names that indicate what's being tested:
- `test_enum_ok.kujo` - Tests enum with Ok variant
- `test_try_except.kujo` - Tests try/except error handling
- `test_arithmetic.kujo` - Tests arithmetic operations

---

## Code Style Guidelines

### Rust Code Style

Follow standard Rust conventions:

```rust
// ✅ Good - idiomatic Rust
pub fn eval_expr(&mut self, expr: &Expr) -> Value {
    match expr {
        Expr::Number(n) => Value::Number(*n),
        Expr::String(s) => Value::String(s.clone()),
        _ => Value::Error("Unsupported expression".to_string()),
    }
}

// ❌ Avoid - unclear names, poor structure
pub fn e(&mut self, x: &Expr) -> Value {
    if let Expr::Number(n) = expr { return Value::Number(*n); }
    if let Expr::String(s) = expr { return Value::String(s.clone()); }
    Value::Error("err".to_string())
}
```

### Formatting

Always check formatting before committing, and use `cargo fmt` to apply fixes:

```bash
cargo fmt --check
```

### Linting

Address clippy warnings with the same strict settings used by release gates:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Documentation

Document public APIs and complex logic:

```rust
/// Evaluates an expression and returns the resulting value.
///
/// # Arguments
/// * `expr` - The expression to evaluate
///
/// # Returns
/// The value produced by evaluating the expression, or an error value if evaluation fails.
pub fn eval_expr(&mut self, expr: &Expr) -> Value {
    // Implementation
}
```

### Error Handling Best Practices

Kujo has a structured error system in `src/errors.rs`. When adding features that can fail:

```rust
use crate::errors::{KujoError, ErrorKind, SourceLocation};

// Create structured errors
let error = KujoError::undefined_variable(
    var_name.clone(),
    SourceLocation::new(line, column)
);

// Report errors with context
self.report_error(error);
```

**Guidelines**:
- Use `KujoError` for structured errors with location info
- Provide clear, actionable error messages
- Include source location when available
- Add source line context for better debugging
- Use appropriate ErrorKind for different error types

### Documentation and Contract Updates

Keep docs and tests synchronized with the surface you change:

- Language syntax or semantics: update [docs/LANGUAGE_SPEC.md](docs/LANGUAGE_SPEC.md) and relevant semantic tests.
- Native APIs or builtins: update [docs/STANDARD_LIBRARY.md](docs/STANDARD_LIBRARY.md), [docs/STANDARD_LIBRARY_REFERENCE.md](docs/STANDARD_LIBRARY_REFERENCE.md), and standard-library/security contract tests.
- AI runtime helpers: update [docs/AI_RUNTIME.md](docs/AI_RUNTIME.md), [docs/SECURE_AI_SCRIPTING.md](docs/SECURE_AI_SCRIPTING.md), replay fixtures, and AI/security tests as appropriate.
- Host-effect or untrusted-execution behavior: update [docs/NATIVE_API_SECURITY_POSTURE.md](docs/NATIVE_API_SECURITY_POSTURE.md).
- CLI JSON, exit codes, diagnostics, DocGen JSON, or LSP helper payloads: update [docs/CLI_MACHINE_READABLE_CONTRACTS.md](docs/CLI_MACHINE_READABLE_CONTRACTS.md) and the matching contract tests.
- Release workflow or compatibility policy: update [docs/RELEASE_PROCESS.md](docs/RELEASE_PROCESS.md) and the relevant release-gate tests.

---

## Git Workflow

### Branch Naming

Use descriptive branch names:
- `feature/add-for-loops` - New features
- `fix/parser-crash` - Bug fixes
- `docs/improve-readme` - Documentation
- `test/add-enum-tests` - Tests

### Commit Messages

Write clear, concise commit messages in present tense:

```bash
# ✅ Good
git commit -m "Add support for for-in loops"
git commit -m "Fix parser crash on nested match expressions"
git commit -m "Update README with enum examples"

# ❌ Avoid
git commit -m "changes"
git commit -m "Fixed stuff"
git commit -m "WIP"
```

### Pull Request Process

1. **Create a PR** with a clear title and description
2. **Link related issues** using "Fixes #123" or "Closes #456"
3. **Describe your changes**:
   - What problem does this solve?
   - How does it work?
   - Any breaking changes?
4. **Ensure tests pass**:
   ```bash
   cargo fmt --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test
   cargo run -- test --runtime vm
   cargo run -- test --runtime dual
   ```
5. **Keep commits clean** - squash fixup commits before merging
6. **Respond to feedback** - address review comments promptly

---

## Feature Development Checklist

When adding a new feature:

- [ ] Implement the feature in appropriate module(s)
- [ ] Add parser support if needed
- [ ] Add compiler/VM and interpreter coverage when the feature touches runtime semantics
- [ ] Write comprehensive tests (Rust contracts, `.kujo` fixtures, or `kujo test-run` tests as appropriate)
- [ ] Update documentation ([README](README.md), [ROADMAP](ROADMAP.md), and the relevant docs under `docs/`)
- [ ] Add example usage to `examples/`
- [ ] Run targeted tests plus `cargo test` and the relevant `cargo run -- test --runtime ...` sweep
- [ ] Format code: `cargo fmt --check`
- [ ] Check for issues: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Update [ROADMAP.md](ROADMAP.md) status if implementing a roadmap item

---

## Bug Report Guidelines

When filing a bug report, include:

1. **Kujo version**: Output of `kujo --version`
2. **Operating system**: macOS, Linux, Windows (include version)
3. **Rust version**: Output of `rustc --version`
4. **Minimal reproduction**:
   ```kujo
   # Paste the smallest code that reproduces the bug
   ```
5. **Expected behavior**: What should happen
6. **Actual behavior**: What actually happens
7. **Error messages**: Full error output if applicable

---

## Feature Request Guidelines

When proposing a new feature:

1. **Check the roadmap** - Is it already planned?
2. **Describe the use case** - Why is this needed?
3. **Provide syntax examples** - How would it look?
   ```kujo
   # Example of proposed syntax
   ```
4. **Consider alternatives** - Are there other approaches?
5. **Estimate complexity** - Small, Medium, Large?

---

## Development Priorities

Current focus areas (in order):

1. **Release Candidate Evidence** - Close the [official v1.0 checklist](docs/V1_0_OFFICIAL_RELEASE_CHECKLIST.md), release artifacts, and tag-time validation evidence
2. **Runtime Parity and VM Readiness** - Keep VM/interpreter behavior intentional and documented in the [parity matrix](docs/VM_INTERPRETER_PARITY_MATRIX.md)
3. **Security and Host-Effect Boundaries** - Keep capability gates, AI egress, filesystem/process/network behavior, and untrusted-mode docs current
4. **CLI, LSP, DocGen, and Agent Contracts** - Preserve machine-readable output contracts for tools and editor integrations
5. **Documentation and Example Accuracy** - Keep README, specs, examples, generated inventories, and docs/example smoke tests aligned

See [ROADMAP](ROADMAP.md) for detailed feature list and implementation order.

---

## Questions or Need Help?

- **GitHub Issues**: [Open an issue](https://github.com/kujolang/kujo/issues)
- **Discussions**: Use GitHub Discussions for questions
- **Documentation**: Start with the [README](README.md), [ROADMAP](ROADMAP.md), [language spec](docs/LANGUAGE_SPEC.md), [installation guide](INSTALLATION.md), and [release process](docs/RELEASE_PROCESS.md)

---

## Code of Conduct

Be respectful, inclusive, and constructive. We're building this together.

---

## Recognition

All contributors will be recognized in release notes and the project README. Thank you for helping make Kujo better!

---

**Kujo is in active development — your contributions shape the language. Let's build something great together!**
