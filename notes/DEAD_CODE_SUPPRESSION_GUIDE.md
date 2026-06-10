# Dead Code Suppression Guide

This guide explains how to handle `#[allow(dead_code)]` annotations in the Kujo codebase.

---

## Understanding Dead Code Suppression

Not all `#[allow(dead_code)]` annotations are bugs. There are three distinct categories:

### ✅ Category 1: Intentionally Unused by Design (KEEP FOREVER)

**Description:** Code that exists for architectural completeness but is semantically never used.

**Characteristics:**
- Has detailed documentation explaining the design decision
- Often mentioned in `notes/GOTCHAS.md`
- Removing it would break the architecture or require major refactoring
- No TODO or FIXME comments

**Current Examples:**
```rust
/// Spread expression: ...expr
/// NOTE: This variant exists in the AST for completeness but is NEVER constructed
/// as a standalone expression. Spread is only valid within ArrayElement::Spread
/// and DictElement::Spread contexts. The warning is suppressed because this design
/// is intentional - spread semantics depend on container context.
#[allow(dead_code)]
Spread(Box<Expr>),
```

**Why `Expr::Spread` must stay:**
- The `Expr` enum represents all possible expression types
- Logically, `...arr` looks like an expression
- But it's **semantically invalid** outside array/dict contexts
- Spread is only valid as `ArrayElement::Spread(expr)` or `DictElement::Spread(expr)`
- Removing `Expr::Spread` would require a complete redesign of how spread is handled
- Current design is clean: spread exists in the enum but is never instantiated

**Action:** Keep suppressed, ensure documentation is clear.

---

### ⚠️ Category 2: Temporary Scaffolding (FIX IN FUTURE VERSION)

**Description:** Code that will be used in upcoming features but isn't ready yet.

**Characteristics:**
- Has TODO or FIXME comments
- Explains what feature will use it
- Should be addressed within 1-3 versions
- Clear path to completion

**Current Examples:**
```rust
/// Get all available variable names in current scope
/// TODO: This will be used when adding "Did you mean?" suggestions to interpreter
/// runtime errors (currently only used in type checker for undefined function errors)
#[allow(dead_code)]
fn get_available_variables(&self) -> Vec<String> {
    self.variables.keys().cloned().collect()
}
```

**Why `get_available_variables` exists:**
- Phase 1: Type checker has "Did you mean?" suggestions (✅ Complete)
- Phase 2: Interpreter runtime needs same suggestions (⚠️ Not yet done)
- Method is scaffolding for Phase 2
- Should be used in v0.8.1 or v0.9.0

**Action:** Track in ROADMAP or TODO list, remove suppression when feature is implemented.

---

### ❌ Category 3: Actually Dead Code (DELETE)

**Description:** Leftover code from refactoring with no clear purpose.

**Characteristics:**
- No documentation explaining why it exists
- No TODO/FIXME comments
- No clear future use
- Just has `#[allow(dead_code)]` with no context

**Action:** Delete it. If you're unsure:
1. Remove the code
2. Run `cargo build`
3. Run `cargo test`
4. If everything works, commit the deletion

---

## How to Review Dead Code Suppression

### Step 1: Find All Suppressions

```bash
rg "#\[allow\(dead_code\)\]" --type rust
```

### Step 2: For Each Suppression, Ask:

1. **Is there a comment explaining why?**
   - No comment → Probably Category 3 (delete)
   - Has comment → Check next questions

2. **Is it a design decision or temporary scaffolding?**
   - Design decision → Category 1 (keep forever)
   - Temporary → Category 2 (track for future work)

3. **Does GOTCHAS.md or session notes mention it?**
   - If yes → Probably Category 1 (intentional design)
   - If no → Could be Category 2 or 3

4. **Can I safely delete it?**
   - Remove the code and `#[allow(dead_code)]`
   - Run tests
   - If tests pass → Was Category 3, safe to delete
   - If tests fail → Investigate why it's needed

### Step 3: Take Action

| Category | Action | Timeline |
|----------|--------|----------|
| Category 1 | Keep suppressed, improve docs | N/A |
| Category 2 | Add to TODO, implement feature | 1-3 versions |
| Category 3 | Delete immediately | Now |

---

## Current Suppressions in Kujo (v0.8.0)

### Intentional Design (Category 1)

| Location | Code | Reason | Action |
|----------|------|--------|--------|
| `src/ast.rs` | `Expr::Spread` | Spread only valid in array/dict contexts, not standalone | Keep forever |

### Temporary Scaffolding (Category 2)

| Location | Code | Reason | Target Version |
|----------|------|--------|----------------|
| `src/type_checker.rs` | `get_available_variables()` | Will be used for runtime "Did you mean?" suggestions | v0.8.1 or v0.9.0 |

### Actually Dead (Category 3)

| Location | Code | Reason | Action |
|----------|------|--------|--------|
| *(None currently)* | - | - | - |

---

## Best Practices

### When Adding New `#[allow(dead_code)]`

**ALWAYS add a comment explaining why:**

```rust
// ✅ GOOD - Clear explanation
/// This method will be used when implementing feature X in v0.9.0
/// TODO: Remove #[allow(dead_code)] when feature X is complete
#[allow(dead_code)]
fn future_method() { }

// ❌ BAD - No explanation
#[allow(dead_code)]
fn some_method() { }
```

### Prefer This Order:

1. **Best:** Don't write the code until you need it
2. **Good:** Write it with TODO comment and timeline
3. **Acceptable:** Write it with design justification
4. **Bad:** Write it with `#[allow(dead_code)]` and no comment

### Regular Cleanup

- Review all `#[allow(dead_code)]` every major version
- Delete Category 3 immediately
- Convert Category 2 to working code or delete
- Improve documentation for Category 1

---

## Example Review Session

```bash
# Find all suppressions
$ rg "#\[allow\(dead_code\)\]" --type rust -A 3

# For each result:
# 1. Read the surrounding code and comments
# 2. Check if it's in GOTCHAS.md or session notes
# 3. Categorize it (1, 2, or 3)
# 4. Take appropriate action
```

---

## Related Documents

- `notes/GOTCHAS.md` - Documents intentional design decisions
- `.github/AGENT_INSTRUCTIONS.md` - "Zero Warnings" policy
- Session notes - May explain why code was added

---

## Summary

- `#[allow(dead_code)]` is **not always a bug**
- **Category 1** (design) → Keep forever with good docs
- **Category 2** (scaffolding) → Track and complete within 1-3 versions
- **Category 3** (actual dead code) → Delete immediately
- **Always document why** code is suppressed
- **Review regularly** to prevent accumulation

Last updated: 2026-01-25
