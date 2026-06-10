# Contributing to Kujo

Thanks for your interest in contributing to Kujo.

We welcome contributions from everyone: beginners, experienced Rust developers, compiler enthusiasts, language design experts, and curious explorers.

---

## Before You Start

1. Read the [README](README.md) for language overview and features
2. Check the [ROADMAP](ROADMAP.md) for planned features and priorities
3. Review the [INSTALLATION](INSTALLATION.md) guide to set up your development environment
4. Browse existing [Issues](https://github.com/kujolang/kujo/issues) to see what needs work

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
- Add language server features (future)

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
cargo run -- test

# Or use the binary directly
./target/debug/kujo test

# Run a specific example
cargo run -- run examples/hello.kujo
```

### 4. Verify Your Changes

```bash
# Run tests
cargo test

# Format code
cargo fmt

# Check for issues
cargo clippy

# Build release
cargo build --release
```

---

## Testing Guidelines

### Running Tests

Kujo uses snapshot testing with `.kujo` and `.kujo.out` files in the `tests/` directory.

```bash
# Run all tests
kujo test

# Update test snapshots after changes
kujo test --update
```

### Adding New Tests

1. Create a `.kujo` file in `tests/` directory:
   ```bash
   tests/test_my_feature.kujo
   ```

2. Run the test to generate output:
   ```bash
   kujo test --update
   ```

3. Verify the `.kujo.out` file is correct

4. Commit both `.kujo` and `.kujo.out` files

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

Always run `cargo fmt` before committing:

```bash
cargo fmt
```

### Linting

Address clippy warnings:

```bash
cargo clippy
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
   cargo test
   kujo test
   cargo clippy
   ```
5. **Keep commits clean** - squash fixup commits before merging
6. **Respond to feedback** - address review comments promptly

---

## Feature Development Checklist

When adding a new feature:

- [ ] Implement the feature in appropriate module(s)
- [ ] Add parser support if needed
- [ ] Add interpreter/evaluation logic
- [ ] Write comprehensive tests (`.kujo` files)
- [ ] Update documentation (README, ROADMAP)
- [ ] Add example usage to `examples/`
- [ ] Run all tests: `kujo test` and `cargo test`
- [ ] Format code: `cargo fmt`
- [ ] Check for issues: `cargo clippy`
- [ ] Update ROADMAP.md status if implementing a roadmap item

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

1. **Core Language Stability** - Fix bugs, improve error handling
2. **Error Messages** - Better diagnostics with line numbers
3. **Data Structures** - Arrays and dictionaries
4. **Control Flow** - Break/continue, for loops
5. **Module System** - Import/export functionality

See [ROADMAP](ROADMAP.md) for detailed feature list and implementation order.

---

## Questions or Need Help?

- **GitHub Issues**: [Open an issue](https://github.com/kujolang/kujo/issues)
- **Discussions**: Use GitHub Discussions for questions
- **Documentation**: Check README and ROADMAP for answers

---

## Code of Conduct

Be respectful, inclusive, and constructive. We're building this together.

---

## Recognition

All contributors will be recognized in release notes and the project README. Thank you for helping make Kujo better!

---

**Kujo is in active development — your contributions shape the language. Let's build something great together!**
