# Agent Instructions for Kujo Language Development

This document provides strict guidelines for AI coding agents working on the Kujo programming language codebase. Following these rules ensures consistent, high-quality contributions and proper documentation.

---

## 🎯 Core Principles

1. **No Fluff, Only Facts** - Documentation must be actionable and focused
2. **Complete the Work** - Don't stop at partial implementations
3. **Zero Warnings** - All code must compile without warnings
4. **Test Everything** - Add tests for new features and bug fixes
5. **Document Changes** - Always update CHANGELOG, ROADMAP, and README
6. **Commit Incrementally** - Commit after each feature/fix, not all at once

---

## 🔄 Git Workflow (MANDATORY)

### Critical Rule: Incremental Commits

**YOU MUST commit code to git after each major step. DO NOT wait until everything is done.**

### When to Commit

Create a commit after completing each of:
- ✅ Single feature implementation (e.g., "add lexical scoping")
- ✅ Single bug fix (e.g., "fix field assignment crash")
- ✅ Test suite addition (e.g., "add 10 scoping tests")
- ✅ Documentation updates (e.g., "update CHANGELOG for v0.3.0")
- ✅ Compiler warning elimination (e.g., "silence dead code warnings")

### Commit Message Standards

Use emoji-prefixed commit messages for clear visual categorization:

```bash
git commit -m ":package: NEW: <description>"
git commit -m ":ok_hand: IMPROVE: <description>"
git commit -m ":bug: BUG: <description>"
git commit -m ":book: DOC: <description>"
git commit -m ":rocket: RELESE: v<description>"
```

**Commit Types:**
- `:package: NEW:` - New feature implementation
- `:ok_hand: IMPROVE:` - Improvements, updates, refactoring
- `:bug: BUG:` - Bug fixes
- `:book: DOC:` - Documentation changes
- `:rocket: RELEASE:` - Version releases

**Good Commit Messages:**
```bash
:package: NEW: implement lexical scoping with environment stack
:bug: BUG: field assignment now works with array[0].field syntax
:ok_hand: IMPROVE: add 10 integration tests for boolean conditions
:book: DOC: update CHANGELOG with v0.2.0 features
:ok_hand: IMPROVE: eliminate all compiler warnings
:ok_hand: IMPROVE: extract field mutation logic into helper function
:rocket: RELEASE: v0.2.0
```

**Bad Commit Messages:**
```bash
# Too vague
fix stuff
update code
improvements

# Too broad (should be multiple commits)
:package: NEW: add scoping, input function, and file I/O

# Wrong emoji
:package: NEW: fix bug  # Should be ":bug: BUG:"
:book: DOC: implement feature  # Should be ":package: NEW:"
```

### Git Commands to Use

After completing a logical unit of work:

```bash
# Stage all changes
git add .

# Or stage specific files
git add src/interpreter.rs tests/scoping_test.kujo

# Commit with descriptive message
git commit -m "feat: implement lexical scoping with parent scope lookup"

# Push to remote
git push origin main
```

### Example Workflow for a Feature

Implementing "Lexical Scoping" feature:

```bash
# Step 1: Implement core logic
# ... edit src/interpreter.rs ...
git add src/interpreter.rs
git commit -m ":package: NEW: implement environment stack for lexical scoping"
git push origin main

# Step 2: Add tests
# ... add tests in src/interpreter.rs ...
git add src/interpreter.rs
git commit -m ":ok_hand: IMPROVE: add 10 integration tests for nested scopes"
git push origin main

# Step 3: Update documentation
# ... edit CHANGELOG.md, ROADMAP.md ...
git add CHANGELOG.md ROADMAP.md
git commit -m ":book: DOC: document lexical scoping in CHANGELOG and ROADMAP"
git push origin main

# Step 4: Update README if needed
# ... edit README.md ...
git add README.md
git commit -m ":book: DOC: add lexical scoping to README features"
git push origin main
```

### Why Incremental Commits Matter

1. **Checkpoint Recovery** - Roll back to specific working states
2. **Code Review** - Each commit is reviewable on its own
3. **History Clarity** - Understand what changed and why
4. **Debugging** - Use git bisect to find when bugs were introduced
5. **Team Coordination** - Other developers see progress in real-time

### Atomic Commits

Each commit should be:
- **Self-contained** - Complete one logical change
- **Functional** - Code compiles and tests pass after commit
- **Descriptive** - Clear what changed and why
- **Focused** - One purpose per commit

### Creating Releases

When releasing a new version:

```bash
# 1. Update version in Cargo.toml (e.g., 0.1.0 -> 0.2.0)

# 2. Run tests to verify
cargo test

# 3. Commit the version bump
git add Cargo.toml
git commit -m ":rocket: RELEASE: 0.2.0"
git push origin main

# 4. Create annotated tag (NO 'v' prefix - just numbers)
git tag -a 0.2.0 -m "Release 0.2.0 - Brief description

See CHANGELOG.md for full details."

# 5. Push the tag
git push origin 0.2.0

# 6. Create GitHub release
# Go to: https://github.com/kujolang/kujo/releases/new
# - Select tag: 0.2.0
# - Release title: "Kujo 0.2.0 - Brief Title"
# - Description: Copy content from CHANGELOG.md for that version
# - Click "Publish release"
```

**Important:** Tag format is `0.2.0` NOT `v0.2.0` (no "v" prefix)

---

## 📋 Mandatory Workflow for Every Task

### Phase 1: Understand and Plan

1. **Read existing documentation first**:
   - **ALWAYS start by reading this file** (.github/AGENT_INSTRUCTIONS.md)
   - Check README.md for project overview
   - Review ROADMAP.md for planned features (focus on High Priority section)
   - Check CHANGELOG.md for recent changes
   - Read relevant source files

2. **Create a todo list**:
   - Break the task into specific, actionable steps
   - Use the manage_todo_list tool to track progress
   - Include: implementation, testing, documentation, and git commits
   - Mark items in-progress as you work, completed when done

3. **Verify you understand the request**:
   - If unclear, ask specific questions
   - Don't guess at requirements
   - Confirm the scope before starting

4. **Search for related code**:
   - Use semantic_search to find similar implementations
   - Use grep_search to find relevant functions/types
   - Read enough context to understand the full picture

### Phase 2: Implement Changes

1. **Make incremental changes**:
   - Fix one issue at a time
   - Test after each change
   - Don't create half-implemented features

2. **Follow Rust best practices**:
   - Use meaningful variable names
   - Add comments for complex logic
   - Handle errors properly (Result/Option types)
   - Avoid unwrap() - use proper error handling

3. **Maintain code quality**:
   - Run `cargo build` - must have 0 warnings
   - Run `cargo test` - all tests must pass
   - Run `cargo fmt` - code must be formatted
   - Add `#[allow(dead_code)]` only for legitimate infrastructure code

### Phase 3: Test Thoroughly

1. **Add integration tests**:
   - Create test cases in `src/interpreter.rs` (mod tests)
   - Use the full lexer/parser pipeline
   - Test both success and error cases
   - Test edge cases and boundary conditions

2. **Add example files if appropriate**:
   - Create `.kujo` files in `examples/` or `examples/projects/`
   - Show real-world usage of new features
   - Include comments explaining the example

3. **Validate everything works**:
   - Run all existing tests: `cargo test`
   - Run examples manually: `cargo run -- run examples/file.kujo`
   - Verify error messages are clear
   - Check that warnings are gone

4. **Commit after tests pass**:
   ```bash
   git add .
   git commit -m ":ok_hand: IMPROVE: add integration tests for [feature]"
   git push origin main
   ```

### Phase 4: Update Documentation and Commit (CRITICAL)

**YOU MUST UPDATE ALL THREE FILES FOR EVERY SIGNIFICANT CHANGE:**

#### 1. CHANGELOG.md - Document What Was Done

**Rules:**
- Add entry under `[Unreleased]` section (or current version)
- Use categories: Added, Changed, Fixed, Removed, Security, Performance
- Be specific about what changed and why it matters
- Include code examples for new syntax
- Link to related issues if applicable

**Example Entry:**
```markdown
### Added
- **Field Assignment**: Full support for mutating struct fields
  - Direct: `person.age := 26`
  - Nested: `todos[0].done := true`
  - Fixes issue where struct fields couldn't be modified
```

**What NOT to write:**
```markdown
### Added
- Field assignment support
```
*(Too vague - what exactly is supported? How do you use it?)*

#### 2. ROADMAP.md - Update Future Plans

**Rules:**
- NEVER add completed features to ROADMAP
- ONLY show future planned work
- Move completed items to CHANGELOG instead
- Update progress tracking table
- Adjust version milestones if priorities changed

**When a feature is completed:**
1. Remove its detailed description from ROADMAP.md
2. Update progress table to mark it complete and reference CHANGELOG
3. Add full details to CHANGELOG.md
4. Update README.md if it's a major feature

**Example ROADMAP Update:**
```markdown
## Progress Tracking

| Feature | Priority | Target Version | Status |
|---------|----------|----------------|--------|
| Field Assignment | High | v0.2.0 | ✅ Complete (see CHANGELOG) |
| Lexical Scoping | Critical | v0.3.0 | Planned |
```

#### 3. README.md - Update User-Facing Info

**Rules:**
- Update feature list if new capability added
- Update examples if syntax changed
- Update installation/usage if needed
- Keep it concise - link to other docs for details
- No fluff phrases like "powerful", "elegant", "beautiful"

**Update README when:**
- New syntax is available
- Major feature completed
- Installation process changes
- Usage commands change

**After updating documentation:**
```bash
git add CHANGELOG.md ROADMAP.md README.md
git commit -m ":book: DOC: update documentation for [feature/fix]"
git push origin main
```

---

## 🚫 Common Mistakes to Avoid

### Documentation Anti-Patterns

❌ **DON'T: Add fluff to ROADMAP**
```markdown
## Future Plans
We're excited to bring you amazing new features that will make Kujo 
the most powerful language ever created...
```

✅ **DO: Be specific and actionable**
```markdown
## Planned for v0.3.0
- Lexical scoping: Fix variable shadowing in nested blocks
- File I/O: Add read_file(), write_file(), list_dir()
```

---

❌ **DON'T: Vague changelog entries**
```markdown
### Fixed
- Fixed bugs
- Improved performance
```

✅ **DO: Specific, measurable changes**
```markdown
### Fixed
- Field assignment now works with nested access: `arr[0].field := value`
- Boolean conditions properly evaluate "true"/"false" strings
- Variable assignment always updates existing variables instead of shadowing
```

---

❌ **DON'T: Keep completed work in ROADMAP**
```markdown
## Roadmap
1. ✅ Structs (v0.2.0) - Full implementation with...
2. ✅ Arrays (v0.2.0) - Complete array support...
3. 🔜 Scoping (v0.3.0) - Planned lexical scoping
```

✅ **DO: Remove completed items**
```markdown
## Roadmap
### High Priority (v0.3.0)
1. Lexical Scoping - Fix nested block variable handling
2. User Input - Add input() function for interactive programs
```

---

### Implementation Anti-Patterns

❌ **DON'T: Leave warnings**
```
warning: unused variable `x`
warning: field `name` is never read
```

✅ **DO: Fix or silence with explanation**
```rust
#[allow(dead_code)]  // Infrastructure for future type checking
pub enum TypeAnnotation { ... }
```

---

❌ **DON'T: Skip testing**
"I implemented the feature, looks good!"

✅ **DO: Add comprehensive tests**
```rust
#[test]
fn test_field_assignment() {
    let code = r#"
        struct Point { x, y }
        p := Point { x: 1, y: 2 }
        p.x := 10
        print(p.x)
    "#;
    // ... test implementation
}
```

---

❌ **DON'T: Stop at partial implementations**
"Field assignment works for direct access but not nested - that's good enough"

✅ **DO: Complete the feature**
"Field assignment works for: obj.field, arr[0].field, dict["key"].field"

---

## 📊 Quality Checklist

Before marking any task complete, verify:

- [ ] Code compiles: `cargo build` succeeds
- [ ] Zero warnings: No compiler warnings remain
- [ ] Tests pass: `cargo test` shows all green
- [ ] New tests added: Feature has test coverage
- [ ] CHANGELOG updated: Changes documented with examples
- [ ] ROADMAP updated: Progress table reflects completion
- [ ] README updated: If user-facing changes made
- [ ] Examples work: Manually tested relevant examples
- [ ] Error messages clear: Users can understand failures
- [ ] Code formatted: `cargo fmt` applied
- [ ] **Changes committed**: Each milestone has a git commit
- [ ] **Changes pushed**: All commits pushed to origin/main

---

## 🔍 Verification Commands

Run these before completing any task:

```bash
# Build with zero warnings
cargo build 2>&1 | grep warning
# Should output nothing

# Run all tests
cargo test
# All tests should pass

# Format code
cargo fmt

# Test examples
cargo run --quiet -- run examples/hello.kujo
cargo run --quiet -- run examples/projects/todo_manager.kujo
```

---

## 📝 Documentation Standards

### CHANGELOG.md Format

```markdown
## [VERSION] - YYYY-MM-DD

### Added
- **Feature Name**: Brief description
  - Specific syntax example
  - What it enables users to do
  - Related features if applicable

### Changed
- **What Changed**: Why it changed
  - Old behavior vs new behavior
  - Migration guide if breaking change

### Fixed
- **Bug Description**: What was broken
  - How it manifests to users
  - What now works correctly

### Performance
- Specific improvement with metrics if available

### Security
- Security issue fixed (be specific without exposing exploit)
```

### ROADMAP.md Format

```markdown
# Development Roadmap

Current Version: vX.Y.Z
Next Planned: vX.Y+1.Z

## High Priority (vX.Y+1)

### Feature Name
**Status**: Planned
**Effort**: Small/Medium/Large (X days)
**Priority**: Critical/High/Medium/Low

**Description**: What this feature does

**Planned Syntax**:
```kujo
code_example()
```

**Implementation Steps**:
1. Specific step 1
2. Specific step 2
```

### README.md Standards

- **Be concise**: No marketing fluff
- **Show code**: Examples over explanations
- **Link out**: Don't duplicate docs from other files
- **Update promptly**: Outdated README is worse than no README

---

## 🎓 Learning from This Codebase

### Code Organization

- **src/lexer.rs**: Tokenization (text → tokens)
- **src/parser.rs**: Parsing (tokens → AST)
- **src/ast.rs**: AST node definitions
- **src/interpreter.rs**: Execution (AST → values)
- **src/builtins.rs**: Native Rust functions
- **src/errors.rs**: Error types and formatting
- **src/module.rs**: Module loading system

### Testing Approach

Integration tests in `src/interpreter.rs`:
```rust
#[test]
fn test_feature() {
    let code = r#"
        # Kujo code using lexer/parser pipeline
        x := 5
        print(x)
    "#;
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    // ... run interpreter and assert results
}
```

### Error Handling Patterns

```rust
// Return Result for recoverable errors
fn parse_thing() -> Result<Thing, ParseError> {
    if invalid {
        return Err(ParseError::new("reason"));
    }
    Ok(thing)
}

// Use KujoError for user-facing errors
return Err(KujoError::runtime(
    format!("Cannot divide by zero"),
    SourceLocation::new("file.kujo", 10, 5)
));
```

---

## 🚀 When Adding New Features

1. **Plan the syntax first**
   - Write example code showing how it will look
   - Consider edge cases
   - Check for conflicts with existing syntax

2. **Update the AST**
   - Add new node types to `ast.rs`
   - Update parser to recognize new syntax
   - Update interpreter to handle new nodes

3. **Implement incrementally**
   - Basic case first
   - Then edge cases
   - Then optimizations

4. **Test comprehensively**
   - Happy path
   - Error cases
   - Edge cases
   - Integration with other features

5. **Document completely**
   - CHANGELOG with examples
   - ROADMAP updated (remove if completed)
   - README if user-facing
   - Code comments for complex logic

---

# When creating temporary files

❗ All files MUST be written inside the workspace directory.
❗ Do NOT write to /tmp, /var, or absolute paths.
❗ Use ./scratch/, ./notes/, or ./checkpoints/ instead.

---

## 💡 Final Reminders

- **Completed features belong in CHANGELOG, not ROADMAP**
- **ROADMAP shows where we're going, not where we've been**
- **Every change needs a CHANGELOG entry**
- **Zero warnings is non-negotiable**
- **Test before marking complete**
- **Be specific, never vague**
- **Users read this documentation - respect their time**

---

*These guidelines were established during v0.2.0 development to ensure consistent, high-quality contributions. Follow them strictly.*
