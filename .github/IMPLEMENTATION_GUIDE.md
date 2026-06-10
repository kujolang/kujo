# Agent Instructions for Kujo Development

This document contains critical lessons learned and best practices for AI agents working on the Kujo programming language codebase.

---

## 🏗️ Architecture Overview

Kujo uses a **tree-walking interpreter** architecture:
1. **Lexer** (`src/lexer.rs`) - Tokenizes source code into `TokenKind` stream
2. **Parser** (`src/parser.rs`) - Converts tokens into Abstract Syntax Tree (AST)
3. **AST** (`src/ast.rs`) - Defines `Expr` and `Stmt` node types
4. **Type Checker** (`src/type_checker.rs`) - Infers and validates types (gradual typing)
5. **Interpreter** (`src/interpreter.rs`) - Evaluates AST and produces runtime `Value` results

---

## ⚠️ Critical Lessons Learned

### 1. Parser Ambiguity: Identifier + Brace

**Problem**: The sequence `Identifier {` is ambiguous:
- Could be struct instantiation: `Point { x: 5, y: 10 }`
- Could be identifier + block: `if x { ... }`

**Solution**: In `parse_call()`, use **lookahead** to check if `{` is followed by field syntax:
```rust
// Check if this looks like struct instantiation
if matches!(self.peek(), Some(TokenKind::Identifier(_))) {
    // Peek ahead to see if there's a colon (field syntax)
    // Only treat as struct if we see identifier: or empty braces
}
```

**When to watch**: Any time you add syntax that could be followed by `{` (if, while, match arms, etc.)

---

### 2. Adding New Value Types (Systematic Checklist)

When adding a new runtime type (like `Value::Bool`), you **MUST** update:

#### Core Type System
- [ ] `Value` enum in `src/interpreter.rs`
- [ ] `TokenKind` enum in `src/lexer.rs`
- [ ] `Expr` enum in `src/ast.rs`
- [ ] `TypeAnnotation` enum in `src/type_checker.rs`

#### Lexer Changes
- [ ] Add tokenization logic for new literals (e.g., `"true"/"false"`)

#### Parser Changes
- [ ] Update `parse_primary()` to handle new token types
- [ ] Update `parse_type_annotation()` if adding type syntax

#### Interpreter Changes
- [ ] Add `Value` variant with `Debug` implementation
- [ ] Update `eval_expr()` to handle new `Expr` variant
- [ ] Update `is_truthy()` function (for conditional contexts)
- [ ] Update `stringify_value()` for display/print
- [ ] Review **ALL built-in functions** that should return the new type
  - Example: File I/O functions should return `Bool`, not `Str("true")`
- [ ] Update comparison operators if they should return the new type

#### Type Checker Changes
- [ ] Add type inference in `infer_expr()` for new expression type
- [ ] Update binary operations if they should return/accept the new type

#### Testing
- [ ] Update existing tests that expect old representations
  - Search for old patterns (e.g., `Value::Str("true")` → `Value::Bool(true)`)
- [ ] Add comprehensive integration tests (10+ test cases)
- [ ] Test edge cases and interactions with other features

#### Documentation
- [ ] Add to `CHANGELOG.md` with detailed changes
- [ ] Update `ROADMAP.md` (mark complete, renumber)
- [ ] Update `README.md` features list
- [ ] Create or enhance example files in `examples/`

---

### 3. Test Update Patterns

When changing core value representations, **grep the test suite** for old patterns:

```bash
# Find tests expecting old string-based booleans
grep -r 'Str("true")' src/
grep -r 'Str("false")' src/

# Find tests expecting numeric comparison results
grep -r 'Value::Number(1.0)' tests/ examples/
```

**Common test failure pattern**: Test asserts `expected == Value::Str("true")` but now gets `Value::Bool(true)`.

---

### 4. Zero Warnings Policy

**Build must complete with ZERO warnings**:
```bash
cargo build 2>&1 | grep -i warning
# Should return nothing
```

**Before considering feature complete**:
1. Run `cargo build` - must be clean
2. Run `cargo test` - all tests must pass
3. Run example files - must execute correctly

---

### 5. Git Commit Practices

Use **emoji-prefixed commits** with clear categories:

- `:sparkles: FEAT:` - New features
- `:bug: FIX:` - Bug fixes
- `:white_check_mark: TEST:` - Test additions/updates
- `:book: DOC:` - Documentation updates
- `:recycle: REFACTOR:` - Code restructuring
- `:zap: PERF:` - Performance improvements
- `:construction: WIP:` - Work in progress (avoid in main)

**Commit after each major step**, not at the end:
1. Core implementation → commit
2. Fix tests + bugs → commit  
3. Add comprehensive tests → commit
4. Update documentation → commit

---

### 6. Documentation Requirements

Every feature MUST update:

1. **CHANGELOG.md**
   - Add entry under current version with date
   - Include detailed list of changes (Runtime, Lexer, Parser, etc.)
   - Show syntax examples
   - List breaking changes if any

2. **ROADMAP.md**
   - Mark feature as ✅ Complete in progress table
   - Remove from planned features section
   - Renumber remaining features
   - Update version milestone progress

3. **README.md**
   - Add to appropriate features section
   - Update version notes if milestone reached
   - Add to examples if significant feature
   - Update completion count (e.g., "Completed 7/14")

4. **Example Files**
   - Create or enhance `examples/*.kujo` demonstrating the feature
   - Show multiple use cases
   - Include edge cases and common patterns

---

### 7. Testing Philosophy

**Integration tests over unit tests**:
- Add tests in `src/interpreter.rs` under `#[cfg(test)]`
- Test complete workflows, not isolated functions
- Each test should be self-contained with clear intent
- Name tests descriptively: `test_bool_in_if_conditions`, not `test_bool_1`

**Test coverage for new types should include**:
- Literal values
- Variable assignments
- Function returns
- Struct fields
- Array elements
- Dictionary values
- Conditional contexts (if/while)
- Comparison operations
- Built-in function integration
- Print/stringify behavior

---

### 8. Parser Best Practices

**Lookahead Strategy**:
```rust
// Good: Check next token without consuming
if matches!(self.peek(), Some(TokenKind::LeftBrace)) {
    // Decide what to do
}

// Bad: Consuming tokens prematurely
self.advance(); // Now you can't go back!
```

**Common Ambiguities**:
- `Identifier {` - struct vs block
- `Identifier (` - function call vs grouping
- `-` - binary minus vs unary negation

**Resolution**: Use context (previous tokens) or lookahead (next tokens) to disambiguate.

---

### 9. Built-in Function Updates

When adding new types, **audit ALL built-ins** in `src/builtins.rs` and `src/interpreter.rs`:

```rust
// Before: Inconsistent boolean representation
Value::Str("true".to_string())  // ❌ Wrong

// After: Use proper type
Value::Bool(true)  // ✅ Correct
```

**Functions to check**:
- File I/O: `file_exists()`, `write_file()`, `create_dir()`
- Comparison helpers
- Type conversion: `to_bool()`, `is_bool()` (if added)
- Any function returning success/failure

---

### 10. Scope and Environment Management

**Current scoping model** (as of v0.3.0):
- Environment stack with linked scopes
- `let` creates new binding (shadows outer)
- `:=` updates existing binding up the scope chain
- Functions have lexical scope (closures work)
- For-loop variables are scoped to loop body

**When adding control flow**:
- Push new environment for block scope
- Pop when exiting block
- Use `update_variable()` for `:=`, `define_variable()` for `let`

---

## 🧪 Pre-Commit Checklist

Before committing ANY feature:

```bash
# 1. Build check
cargo build 2>&1 | grep -i warning

# 2. Run all tests
cargo test

# 3. Run example files
cargo run --quiet -- run examples/test_bool.kujo
cargo run --quiet -- run examples/showcase.kujo

# 4. Check formatting (if using rustfmt)
cargo fmt --check

# 5. Verify documentation updates
git diff CHANGELOG.md ROADMAP.md README.md
```

**All must pass** before committing.

---

## 📋 Feature Implementation Workflow

### Phase 1: Planning
1. Read ROADMAP.md to identify next feature
2. Create comprehensive todo list with all steps
3. Review existing code to understand integration points

### Phase 2: Core Implementation
1. Update AST nodes (`src/ast.rs`)
2. Update lexer tokenization (`src/lexer.rs`)
3. Update parser rules (`src/parser.rs`)
4. Update interpreter evaluation (`src/interpreter.rs`)
5. Update type checker (`src/type_checker.rs`)
6. **Commit**: `:sparkles: FEAT: implement [feature name]`

### Phase 3: Bug Fixes
1. Build and identify warnings/errors
2. Run existing tests, fix failures
3. Test example files, fix issues
4. **Commit**: `:bug: FIX: [describe fixes]`

### Phase 4: Testing
1. Add 10+ integration tests covering all use cases
2. Create or enhance example file
3. Verify all tests pass
4. **Commit**: `:white_check_mark: TEST: add comprehensive tests for [feature]`

### Phase 5: Documentation
1. Update CHANGELOG.md with detailed entry
2. Update ROADMAP.md (mark complete, renumber)
3. Update README.md features list
4. **Commit**: `:book: DOC: update documentation for [feature]`

---

## 🎯 Common Pitfalls to Avoid

1. **Don't forget `is_truthy()`** - Must handle new types for conditional contexts
2. **Don't skip `stringify_value()`** - Needed for print() and debug output
3. **Don't miss built-in functions** - Grep for old patterns (e.g., returning strings instead of bools)
4. **Don't ignore parser ambiguity** - Always consider what `Identifier {` or `Identifier (` could mean
5. **Don't forget Debug trait** - Add it to all new Value/Expr variants
6. **Don't batch commits** - Commit incrementally after each major step
7. **Don't skip example files** - They're critical for demonstrating features

---

## 🔍 Useful Grep Patterns

```bash
# Find all Value enum matches (to update for new type)
grep -n "Value::" src/interpreter.rs

# Find all stringify/print locations
grep -n "stringify_value" src/

# Find comparison operators
grep -n "TokenKind::(EqualEqual\|Greater\|Less)" src/

# Find where variables are defined
grep -n "define_variable\|update_variable" src/

# Find test assertions
grep -n "assert_eq!" src/interpreter.rs
```

---

## 📚 Code Style Notes

**Rust Conventions** (already followed):
- Use `snake_case` for functions and variables
- Use `PascalCase` for types and enum variants
- Use 4-space indentation (or tabs as per rustfmt.toml)
- Add doc comments `///` for public APIs
- Group related functionality with comments

**Error Messages**:
- Use colored output via `colored` crate
- Include line/column information
- Show source context when possible
- Be specific about what went wrong

---

## 🚀 Next Feature Priorities (from ROADMAP)

When selecting next feature to implement:

1. **Loop Control** (break/continue/while) - High priority, v0.3.0
2. **String Interpolation** - Medium priority, v0.3.x
3. **Enhanced Strings** - Medium priority, v0.4.0
4. **Array Higher-Order Fns** - Medium priority, v0.4.0

Follow the workflow above for each implementation.

---

*Last Updated: January 22, 2026 (Boolean Type Implementation)*
