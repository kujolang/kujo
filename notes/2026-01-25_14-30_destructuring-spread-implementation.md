# Kujo Field Notes — Destructuring Patterns & Spread Operators

**Date:** 2026-01-25
**Session:** 14:30 local
**Branch/Commit:** main / f676f2f
**Scope:** Implemented two major v0.8.0 features - destructuring patterns (arrays, dicts, nested, rest, ignore) and spread operators (array/dict spreading). Complete AST, parser, interpreter, and type checker updates.

---

## What I Changed

- Added `Pattern` enum to AST (`src/ast.rs`) with variants: `Identifier`, `Array`, `Dict`, `Ignore`
- Added `ArrayElement` enum (Single/Spread) and `DictElement` enum (Pair/Spread) to AST
- Added `Expr::Spread` variant (intentionally unused as standalone expression)
- Modified lexer (`src/lexer.rs`) to tokenize `...` as `Operator("...")`
- Added `parse_pattern()` method to parser (`src/parser.rs`) for recursive pattern parsing
- Updated `parse_let()` to use patterns instead of simple identifiers
- Modified `parse_stmt()` to recognize pattern-based assignments starting with `[` or `{`
- Updated array/dict literal parsing to handle spread elements
- Implemented `bind_pattern()` in interpreter (`src/interpreter.rs`) for recursive pattern matching
- Updated array/dict literal evaluation to handle spread elements
- Fixed type checker (`src/type_checker.rs`) to work with Pattern-based system
- Created 15 destructuring tests (`tests/destructuring.kujo`)
- Created 15 spread operator tests (`tests/spread_operator.kujo`)
- Created example files: `examples/destructuring_demo.kujo`, `examples/spread_operator_demo.kujo`
- Updated `CHANGELOG.md`, `README.md`, `ROADMAP.md`

---

## Gotchas (Read This Next Time)

- **Gotcha:** Compiler warning about `Expr::Spread` being unused as an `Expr`
  - **Symptom:** `cargo build` emits warning: `variant `Spread` is never constructed` for `Expr` enum
  - **Root cause:** `Spread` is **intentionally NOT** a standalone expression. It only exists within `ArrayElement::Spread` and `DictElement::Spread` contexts
  - **Fix:** This is **expected behavior** - do not suppress the warning, it's harmless
  - **Prevention:** Do NOT refactor `Spread` into being a valid standalone `Expr`. The spread operator (`...`) is **context-dependent** and only valid inside array/dict literals and (future) function calls. Making it a standalone expression would break semantic validation.

- **Gotcha:** Parser initially failed to recognize patterns starting with `[` or `{`
  - **Symptom:** Parse errors when trying `[a, b] := [1, 2]` syntax
  - **Root cause:** `parse_stmt()` only looked for identifier tokens to start a pattern, not punctuation
  - **Fix:** Added pattern recognition in `parse_stmt()` for `TokenKind::Punctuation('[')` and `TokenKind::Punctuation('{')`
  - **Prevention:** When adding new syntax forms, check ALL token types that could start the construct, not just identifiers

- **Gotcha:** Lexer didn't tokenize `...` as a single operator
  - **Symptom:** Parser couldn't recognize spread syntax, got three separate `.` tokens
  - **Root cause:** Lexer's `.` case only emitted a single `Punctuation('.')` token
  - **Fix:** Modified lexer to peek ahead when seeing `.`, check for `...` sequence, emit `Operator("...")` if matched
  - **Prevention:** Multi-character operators require explicit lookahead in lexer. Search for similar patterns when adding new operators.

- **Gotcha:** Type checker unit tests broke after AST changes
  - **Symptom:** Compilation errors: `no field `name` on type `Stmt``, expected `name: String` 
  - **Root cause:** Tests were constructing `Stmt::Let { name: "x".to_string(), ... }` but the field changed to `pattern: Pattern`
  - **Fix:** Updated all 6 test cases to use `pattern: Pattern::Identifier("x".to_string())`
  - **Prevention:** When changing AST structure, grep for test usages: `rg "Stmt::Let" tests/` or `rg "name:" src/`

- **Gotcha:** Rest elements require explicit tracking in patterns
  - **Symptom:** Needed to know if pattern has rest element to handle remaining values
  - **Root cause:** Rest patterns (`...rest`) consume all remaining elements, but parser needs to reject multiple rest elements
  - **Fix:** Track whether a rest element has been seen during pattern parsing, reject duplicate rest patterns
  - **Prevention:** When implementing collection patterns, consider "consume remaining" semantics upfront

---

## Things I Learned

- **AST Design Principle:** Use dedicated enum variants for context-specific syntax (e.g., `ArrayElement::Spread` vs `Expr::Spread`) rather than overloading general expression types. This makes invalid states unrepresentable.

- **Parser Lookahead:** Kujo's recursive descent parser can peek ahead with `self.lexer.peek()`. Use this for multi-token operator recognition (like `...`) or disambiguating syntax forms.

- **Pattern Binding is Recursive:** The `bind_pattern()` implementation naturally mirrors the `Pattern` enum structure - each variant recursively binds sub-patterns. This makes nested destructuring "free" once the base cases work.

- **Type Checker Independence:** Type checking patterns requires checking the pattern structure matches the value type, but doesn't need runtime values. Pattern::Array expects array type, Pattern::Dict expects dict type, etc.

- **Spread Semantics:** Spread in arrays is simple concatenation. Spread in dicts is merge-with-override (right side wins). This matches JavaScript/Python behavior.

- **Test Organization:** Separate test files for each feature (`destructuring.kujo`, `spread_operator.kujo`) makes it easy to verify all cases. Each test file should have 10-15 focused test cases.

---

## Debug Notes

- **Failing test:** Type checker tests failed with "no field 'name'" errors
- **Repro steps:** Run `cargo test` after changing `Stmt::Let` from `name: String` to `pattern: Pattern`
- **Breakpoints / logs used:** Compiler error messages pointed directly to test construction sites in `src/type_checker.rs`
- **Final diagnosis:** Tests were using old AST structure. Simple find-replace of `name: "x".to_string()` → `pattern: Pattern::Identifier("x".to_string())`

---

## Follow-ups / TODO (For Future Agents)

- [ ] Add spread operator support in function calls: `func(...args)` - requires parser updates to handle spread in argument lists
- [ ] Consider adding pattern matching in function parameters: `func process_user({name, email}) { ... }`
- [ ] Investigate if `Expr::Spread` warning can be eliminated (maybe remove variant entirely and only use ArrayElement/DictElement?)
- [ ] Add destructuring in `for` loops: `for [key, value] in pairs { ... }`
- [ ] Performance testing: measure overhead of pattern binding vs simple assignment

---

## Links / References

Files touched:
- `src/ast.rs` - Pattern enum, ArrayElement/DictElement enums, Expr::Spread
- `src/lexer.rs` - `...` operator tokenization  
- `src/parser.rs` - parse_pattern method, pattern recognition, spread handling
- `src/interpreter.rs` - bind_pattern method, spread evaluation
- `src/type_checker.rs` - Pattern type checking
- `tests/destructuring.kujo` - 15 test cases
- `tests/spread_operator.kujo` - 15 test cases
- `examples/destructuring_demo.kujo` - Practical examples
- `examples/spread_operator_demo.kujo` - Practical examples

Related docs:
- `README.md` - Updated with feature descriptions
- `ROADMAP.md` - Marked features as complete
- `CHANGELOG.md` - Added v0.8.0 section with feature descriptions
- `.github/AGENT_INSTRUCTIONS.md` - Followed incremental commit workflow
