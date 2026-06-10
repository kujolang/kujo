# Test Failure Analysis & Game Plan

**Date**: 2026-01-26  
**Analyst**: AI Agent (with robertdevore)  
**Context**: Analyzing 37 failing tests (out of 140 total) to create remediation strategy

## Executive Summary

**Current Status**: 103/140 tests passing (73.6%)  
**Failing Tests**: 37 tests (26.4%)  
**Root Cause**: Primary issue is missing `print()` function in type checker causing widespread false failures  
**Quick Win**: Fix type checker registration → expect ~25-30 tests to pass immediately  
**Remaining Work**: 7-12 tests need actual feature implementation

---

## Critical Discovery: Type Checker Bug

**Problem**: `print()` function is NOT registered in `src/type_checker.rs` builtins  
**Impact**: Type checker throws "Undefined function 'print'" errors on virtually every test  
**Evidence**: All tests showing this error actually work fine when type checking is disabled

**Verification**:
```bash
# These tests show "Undefined Function: Undefined function 'print'" 
./target/release/kujo run tests/destructuring.kujo 2>&1 | head -20
./target/release/kujo run tests/test_regex.kujo 2>&1 | head -20  
./target/release/kujo run tests/vm_native_functions_test.kujo 2>&1 | head -20
```

**Files to Fix**:
- `src/type_checker.rs` line ~60-150 (register_builtins function)
- Need to add: `print`, `input`, `debug`, and other I/O functions

---

## Failing Tests Categorized

### Category 1: Type Checker False Failures (Est. ~25-30 tests)

Tests that fail ONLY because `print()` isn't registered in type checker:

1. `tests/destructuring.kujo`
2. `tests/test_regex.kujo`
3. `tests/test_regex_simple.kujo`
4. `tests/vm_native_functions_test.kujo`
5. `tests/test_json_parse.kujo`
6. `tests/test_json_serialize.kujo`
7. `tests/test_json_edge_cases.kujo`
8. `tests/test_collections.kujo`
9. `tests/test_enhanced_collections.kujo`
10. `tests/dict_methods_test.kujo`
11. `tests/test_http_headers.kujo`
12. `tests/test_binary_simple.kujo`
13. `tests/test_binary_files.kujo`
14. `tests/test_simple_random.kujo`
15. `tests/simple_image_test.kujo`
16. `tests/image_processing_test.kujo`
17. `tests/stdlib_crypto_test.kujo`
18. `tests/stdlib_test.kujo`
19. `tests/test_stdlib_system.kujo`
20. `tests/test_stdlib_datetime.kujo`
21. `tests/test_stdlib_paths.kujo`
22. `tests/stdlib_os_path_test.kujo`
23. `tests/stdlib_io_test.kujo`
24. `tests/arg_parser.kujo`
25. `tests/env_and_args.kujo`
26. `tests/spread_operator.kujo`

**Quick Fix**: Add missing builtin function signatures to type checker

### Category 2: VM Feature Gaps (Est. ~5-7 tests)

Tests requiring VM implementation work (per ROADMAP Phase 1 Weeks 5-8):

1. `tests/bytecode_vm.kujo` - Needs VM completion
2. `tests/test_transaction_simple.kujo` - Needs exception handling  
3. `tests/test_trans_newvar.kujo` - Needs exception handling
4. `tests/test_trans_nostr.kujo` - Needs exception handling
5. `tests/test_trans_vars.kujo` - Needs exception handling
6. `tests/test_trans_debug.kujo` - Needs exception handling
7. `tests/test_trans_minimal.kujo` - Needs exception handling
8. `tests/test_transactions_working.kujo` - Needs exception handling
9. `tests/test_database_transactions.kujo` - Needs exception handling + DB support

**Status**: These are documented in ROADMAP.md Task #28 Phase 1 Week 6 (Exception Handling)  
**Priority**: HIGH - Core language feature

### Category 3: Unknown/Need Investigation (Est. ~2-5 tests)

Tests that may have other issues requiring deeper investigation:

- Some tests may have actual bugs in implementation
- Some may need additional stdlib functions
- Some may be obsolete/duplicate tests

---

## Game Plan: 3-Phase Approach

### Phase 0: Quick Win - Fix Type Checker (1-2 hours)

**Objective**: Fix the type checker bug causing false failures  
**Impact**: Expect 25-30 tests to pass immediately  
**New Pass Rate**: ~93% (130/140 tests)

**Tasks**:
1. Open `src/type_checker.rs`
2. Find `register_builtins()` function (around line 60)
3. Add missing I/O functions:
   ```rust
   // I/O functions
   self.functions.insert(
       "print".to_string(),
       FunctionSignature {
           param_types: vec![None], // Accepts any type
           return_type: None,        // Returns null
       },
   );
   
   self.functions.insert(
       "input".to_string(),
       FunctionSignature {
           param_types: vec![Some(TypeAnnotation::String)],
           return_type: Some(TypeAnnotation::String),
       },
   );
   
   self.functions.insert(
       "debug".to_string(),
       FunctionSignature {
           param_types: vec![None], // Variadic - accepts any args
           return_type: None,
       },
   );
   ```
4. Add other commonly used functions:
   - `assert(condition, message)` → None
   - `type(value)` → String
   - `parse_json(str)` → None (returns any)
   - `to_json(value)` → String
   - File I/O: `read_file`, `write_file`, `file_exists`
   - Many others from interpreter/native_functions/*.rs

**Success Criteria**: 
- Run `kujo test` and see ~130/140 tests passing
- Zero "Undefined function 'print'" errors

---

### Phase 1: Complete VM Exception Handling (1-2 weeks)

**Objective**: Implement try/catch/finally/throw in bytecode VM  
**Impact**: 7-9 transaction/exception tests pass  
**New Pass Rate**: ~98% (137/140 tests)

**Tasks** (from ROADMAP Phase 1 Week 6):

1. **Add Exception Opcodes**:
   - `Try` - Push exception handler onto stack
   - `Catch` - Begin catch block
   - `Finally` - Begin finally block  
   - `Throw` - Throw exception
   - `EndTry` - Pop exception handler

2. **Implement Stack Unwinding**:
   - Exception handler stack in VM
   - Unwind call frames on throw
   - Restore stack state to try block entry
   - Execute catch block with exception value

3. **Exception Propagation**:
   - Search exception handler stack
   - Propagate through call frames
   - Handle uncaught exceptions gracefully
   - Preserve stack traces

4. **Compiler Updates**:
   - Compile try/catch/finally statements
   - Generate exception handler metadata
   - Track try block boundaries
   - Emit cleanup code for finally blocks

**Referenced Files**:
- `src/vm.rs` - Add exception handling to VM
- `src/compiler.rs` - Compile exception statements
- `src/bytecode.rs` - Add new opcodes

**Success Criteria**:
- All 9 transaction tests pass
- Exception propagation works correctly
- Finally blocks execute even on exception
- Stack traces preserved

---

### Phase 2: Investigate Remaining Failures (1 day)

**Objective**: Debug the 3-5 remaining failing tests  
**Impact**: 100% test pass rate (140/140 tests)

**Tasks**:
1. Run each remaining failing test individually
2. Identify specific missing features or bugs
3. Create GitHub issues for each category
4. Either fix immediately or document for future work

**Possible Issues**:
- Missing stdlib functions (easy fix)
- Feature gaps in VM (document for later)
- Obsolete tests (remove or update)
- Test bugs/incorrect expectations (fix test)

---

## Priority Recommendations

### Immediate Action (Today/Tomorrow):
✅ **Phase 0: Fix Type Checker** - Massive immediate impact, minimal effort

### Next Sprint (This Week):
🔄 **Phase 1: VM Exception Handling** - Core feature, well-documented in ROADMAP

### Follow-up (Next Week):
🔍 **Phase 2: Investigate Remaining** - Cleanup work, document findings

---

## Success Metrics

| Milestone | Tests Passing | Pass Rate | ETA |
|-----------|--------------|-----------|-----|
| **Baseline** | 103/140 | 73.6% | Current |
| **After Type Checker Fix** | ~130/140 | ~93% | +2 hours |
| **After Exception Handling** | ~137/140 | ~98% | +2 weeks |
| **After Investigation** | 140/140 | 100% | +3 weeks |

---

## ROADMAP Alignment

All work aligns with existing ROADMAP.md documentation:

- **Type Checker Fix**: Not explicitly documented (bug fix)
- **Exception Handling**: ✅ ROADMAP Task #28 Phase 1 Week 6
- **VM Completion**: ✅ ROADMAP Task #28 Phase 1 Weeks 1-8

**No new ROADMAP tasks needed** - all work fits within existing plans.

---

## Risk Assessment

### Low Risk:
- ✅ Type checker fix (straightforward, well-understood)
- ✅ Exception handling (documented, clear requirements)

### Medium Risk:
- ⚠️ Time estimates may be optimistic
- ⚠️ Some tests may reveal deeper issues

### Mitigation:
- Start with quick wins (type checker)
- Document issues as we find them
- Don't let perfect be enemy of good (98% is excellent)

---

## Next Steps

1. **Confirm approach with robertdevore**: Does this game plan make sense?
2. **Start Phase 0**: Fix type checker builtins registration
3. **Validate impact**: Run full test suite after fix
4. **Proceed to Phase 1**: Implement VM exception handling if time permits
5. **Document findings**: Update this document as we progress

---

## Notes

- The type checker bug is a **major discovery** - it's masking the true test status
- Real pass rate is likely ~93% already, we just need to fix false negatives
- Exception handling is a well-defined task with clear requirements
- We're in better shape than the raw numbers suggest

---

## Questions for Discussion

1. Should we fix type checker immediately or continue with VM Phase 1?
2. Are the 9 transaction tests critical for v0.8.0 or can they wait?
3. Should we add a `--skip-type-check` flag for testing?
4. Do we want 100% pass rate or is 98% acceptable?

---

**End of Analysis**
