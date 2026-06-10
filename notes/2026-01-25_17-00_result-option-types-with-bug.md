# Session Notes: Result & Option Types Implementation

**Date**: 2026-01-25  
**Time**: ~17:00  
**Feature**: Result<T, E> and Option<T> types (ROADMAP item #22, P1)

---

## Summary

Implemented Result and Option types for Kujo with Ok, Err, Some, None constructors and the `?` try operator. All AST, lexer, parser, and interpreter changes compile successfully. However, there is a **runtime hanging issue in pattern matching** that needs debugging.

---

## What Was Completed

### 1. AST Changes (`src/ast.rs`)
- ✅ Added `TypeAnnotation::Result { ok_type, err_type }` and `TypeAnnotation::Option { inner_type }`
- ✅ Added `Expr::Ok(Box<Expr>)`, `Expr::Err(Box<Expr>)`, `Expr::Some(Box<Expr>)`, `Expr::None`, `Expr::Try(Box<Expr>)`
- ✅ Updated `TypeAnnotation::matches()` to handle Result and Option types

### 2. Value Representation (`src/interpreter.rs`)
- ✅ Added `Value::Result { is_ok: bool, value: Box<Value> }`
- ✅ Added `Value::Option { is_some: bool, value: Box<Value> }`  
- ✅ Updated `Debug` impl for Value to pretty-print Result/Option
- ✅ Updated `stringify_value()` to format Ok/Err/Some/None properly

### 3. Lexer (`src/lexer.rs`)
- ✅ Added keywords: `Result`, `Option`, `Ok`, `Err`, `Some`, `None`
- ✅ `?` operator already existed for optional chaining/null coalescing

### 4. Parser (`src/parser.rs`)
- ✅ Added `parse_type_annotation_inner()` helper for nested generic types
- ✅ Updated `parse_type_annotation()` to parse `Result<T, E>` and `Option<T>` syntax
- ✅ Added parsing for `Ok(value)`, `Err(error)`, `Some(value)`, `None` in `parse_primary()`
- ✅ Added `Try(Box<Expr>)` parsing for `expr?` in `parse_call()` loop

### 5. Interpreter Evaluation (`src/interpreter.rs`)
- ✅ Implemented `Expr::Ok/Err/Some/None` evaluation to create corresponding `Value` variants
- ✅ Implemented `Expr::Try` to unwrap `Ok` values or early return on `Err`
- ❌ **BROKEN**: Pattern matching on Result/Option hangs (see Issues section)

### 6. Type Checker (`src/type_checker.rs`)
- ✅ Added type inference for `Ok`, `Err`, `Some`, `None` expressions
- ✅ Added type checking for `Try` operator (enforces Result type)
- ✅ Generates proper type errors for invalid `?` usage

### 7. Built-ins (`src/builtins.rs`)
- ✅ Updated `format_debug_value()` to handle Result/Option

### 8. Tests
- ✅ Created `tests/result_option.kujo` with 20 comprehensive test cases
- ✅ Created simpler debug tests: `simple_result_test.kujo`, `minimal_match_test.kujo`, etc.
- ❌ **FAIL**: All tests hang when trying to pattern match on Result/Option values

---

## Critical Bug: Pattern Matching Hangs

### Symptoms
- Creating `Ok(42)`, `Err("msg")`, `Some(100)`, `None` works fine and prints correctly
- Program hangs indefinitely when entering a `match` statement with Result/Option value
- Hangs even with:
  - Empty match body: `case Ok: {}`
  - No pattern variable: `case Ok:` instead of `case Ok(v):`
  - Simple single-case match

### What Works
- Regular enum matching (tested with `tests/test_enum_ok.kujo`) works perfectly
- Result/Option construction and printing works
- Non-matching code executes fine

### Attempted Fixes
1. ✅ Cloned `cases` and `default` vectors to avoid borrow conflicts during `eval_stmts`
2. ✅ Extracted tag and value data before entering match logic (avoiding complex nested borrows)
3. ❌ Still hangs - issue is deeper in match logic

### Investigation Needed
- The hang happens **before** `eval_stmts(body)` is called (verified with empty body test)
- Likely an infinite loop in the pattern matching comparison logic
- Check if `pattern.find('(')` or `pattern.as_str() == tag_str` behaves unexpectedly
- May need to add debug prints or use a debugger to trace execution flow
- Possible issue with how `&cases_clone` iteration interacts with match logic

### Code Location
**File**: `src/interpreter.rs`  
**Function**: `eval_stmt()` - `Stmt::Match` branch  
**Lines**: ~4869-4960  

The problematic code structure:
```rust
if is_result_or_option {
    for (pattern, body) in &cases_clone {
        if let Some(open_paren) = pattern.find('(') {
            // ... extract pattern variable ...
            if tag_str == enum_tag.trim() {
                // Should execute body and return here
                self.eval_stmts(body);
                return;
            }
        } else if pattern.as_str() == tag_str {
            // Or here for simple patterns
            self.eval_stmts(body);
            return;
        }
    }
}
```

**Hypothesis**: The pattern matching conditions might not be evaluating correctly, causing the loop to continue indefinitely or the return statements to not execute.

---

## Next Steps (In Order)

1. **Debug the match hang** (CRITICAL)
   - Add explicit `eprintln!` debug statements throughout match logic
   - Print: `tag_str` value, `pattern` values, comparison results
   - Use `cargo run` with `--` `--debug` flag or debugger (lldb/gdb)
   - Check if iteration over `&cases_clone` is the issue
   
2. **Fix pattern matching** once root cause is found

3. **Run full test suite** (`cargo test`)

4. **Update documentation**:
   - CHANGELOG.md (add Result & Option to [Unreleased])
   - ROADMAP.md (mark item #22 as Complete)
   - README.md (add to features list)

5. **Create examples/**
   - `examples/result_option_demo.kujo` with practical use cases
   
6. **Final commit and push**

---

## Lessons Learned / Gotchas

### Pattern Matching Implementation is Complex
- Simply converting Result/Option to Tagged values doesn't work due to borrow checker issues
- Needed to extract tag and value data upfront before entering match logic
- Cloning vectors is necessary when eval_stmts needs mutable access to interpreter

### Try Operator Semantics
- `?` unwraps Ok values and returns Err early via `self.return_value`
- Must set return_value to propagate error up the call stack
- Type checker correctly enforces Result type for `?` operator

### Type Annotation Parsing
- Generic types need recursive parsing (Result<T, E> contains nested types)
- Created `parse_type_annotation_inner()` to handle type parameters inside `<>`
- Must check for `<` and `>` operators explicitly in parser

---

## Files Modified

**Core Language**:
- `src/ast.rs` - AST types
- `src/lexer.rs` - Keywords  
- `src/parser.rs` - Parsing logic
- `src/interpreter.rs` - Evaluation + pattern matching (**BUG HERE**)
- `src/type_checker.rs` - Type inference
- `src/builtins.rs` - Debug formatting

**Tests** (all currently fail due to match hang):
- `tests/result_option.kujo` - 20 comprehensive tests
- `tests/simple_result_test.kujo` - Basic Result/Option creation
- `tests/minimal_match_test.kujo` - Minimal match test
- `tests/match_no_param.kujo` - Match without pattern variable
- `tests/match_empty_body.kujo` - Match with empty body

---

## Git Commits

1. `:package: NEW: add Result and Option types to AST and Value enum`
2. `:package: NEW: add Result, Option, Ok, Err, Some, None keywords to lexer`
3. `:package: NEW: parse Result/Option type annotations and Ok/Err/Some/None/Try expressions`
4. `:package: NEW: implement Result/Option evaluation, pattern matching, and try operator`
5. `:bug: BUG: Result/Option types implemented but match statement has hang issue - needs debugging`

---

## Example Usage (Once Fixed)

```kujo
# Function returning Result
func divide(a: float, b: float) -> Result<float, string> {
    if b == 0.0 {
        return Err("Division by zero")
    }
    return Ok(a / b)
}

# Pattern matching
match divide(10, 2) {
    case Ok(value): print("Result: " + to_string(value))
    case Err(error): print("Error: " + error)
}

# Try operator  
func complex_operation() -> Result<Data, Error> {
    let data1 := fetch_data()?      # Returns early if Err
    let data2 := process(data1)?     # Chains operations
    return Ok(finalize(data2))
}

# Option type
func find_user(id: int) -> Option<User> {
    if exists {
        return Some(user)
    }
    return None
}

match find_user(1) {
    case Some(user): print("Found: " + user.name)
    case None: print("Not found")
}
```

---

## Justifications

### Why not use Tagged values directly?
- Result/Option are special-cased built-in types, not user-defined enums
- Need first-class support for type checking and the `?` operator
- Tagged values would require awkward `Result::Ok` syntax instead of clean `Ok(value)`

### Why clone cases and default in match?
- `eval_stmts()` requires `&mut self`, which conflicts with borrowing from the Stmt
- Cloning avoids lifetime issues and borrow checker errors
- Performance impact is minimal - match statements aren't in hot loops

### Why separate Result/Option match logic?
- Attempted to use existing Tagged infrastructure but hit borrow checker issues
- Extracting values upfront avoids nested borrow problems
- Cleaner separation makes debugging easier (though still has bugs!)

