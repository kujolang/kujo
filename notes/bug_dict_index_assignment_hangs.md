# Critical Bug: Dict Index Assignment Causes Hang

**Date**: 2026-01-30  
**Status**: HISTORICAL - Superseded before v1.0 release readiness
**Severity**: Critical

## Current Status - Superseded 2026-06-19

This note records a historical failure investigation and should not be used
as current release status. Current Kujo supports dictionary index assignment
in both interpreter and VM paths.

Modern evidence:

- `docs/LANGUAGE_SPEC.md` defines assignment statements with `:=`, `=`,
  and compound assignment operators, with mutability controlled by
  `let`/`mut`/`const` binding rules.
- `tests/vm_interpreter_parity_surfaces.rs` covers successful local,
  nested, and captured map updates, plus invalid index-assignment targets.
- `tests/language_spec_contracts.rs` covers mutable binding reassignment and
  in-place mutation policy.
- On 2026-06-19, the historical repro `mut d := {"x": 0}; d["x"] = 5;
  print(d["x"])` printed `5` in both `kujo run` and `kujo run --interpreter`.

Use this file only as historical context for the January 2026 investigation.

## Summary

Dictionary index assignment (`d["key"] = value`) causes the VM to hang indefinitely. This is a pre-existing bug that blocks performance optimization work and affects all code that modifies dictionaries.

## Reproduction

### Minimal Test Case

```kujo
d := {"x": 0}
d["x"] = 5
```

This hangs indefinitely.

### Also Fails In:
- Functions: `fn() { d := {}; d["x"] = 1 }`
- Loops: `for i in range(3) { d["x"] = i }`  
- Both global and local scopes

### What DOES Work:
- Dict creation: `d := {"x": 0}` ✅
- Dict reassignment: `d = {"x": 1}` ✅  
- Dict reading: `x := d["x"]` ✅
- Array index assignment: `arr[0] = 5` (needs testing)

## Investigation

### Bytecode Sequence (for `d["x"] = 5`)

The compiler emits:
1. `LoadConst(5)` - push value
2. `LoadGlobal("d")` - push dict
3. `LoadConst("x")` - push key
4. `IndexSet` - mutate dict, push result
5. `StoreGlobal("d")` - peek and store back
6. `Pop` - clean stack

This bytecode sequence looks correct.

### VM Behavior

- No error message is produced
- Process hangs (100% CPU or deadlock - TBD)
- Happens even in simplest cases
- Pre-dates recent optimization attempts

### Potential Root Causes

1. **IndexSet Implementation**: Bug in vm.rs lines 1347-1376
   - Pops value, object, index
   - Mutates object
   - Should push modified object back
   - Maybe infinite loop inside match?

2. **StoreGlobal/StoreVar**: Lines 507-512, 472-505
   - Peek+clone for Store, pop for StorePop variants
   - Maybe issue with peek not finding value?

3. **Stack Corruption**: 
   - IndexSet might not be pushing result correctly
   - Subsequent StoreGlobal reads corrupted stack

4. **IP Management**:
   - Instruction pointer not advancing (unlikely - checked)
   - Some codepath missing IP increment

## Impact

- **Blocks**: Dict performance benchmarks
- **Blocks**: Any optimization work on dict operations
- **Blocks**: Real-world dict-heavy programs
- **Affects**: project_data_pipeline.kujo, project_log_analyzer.kujo, and many other examples

## Next Steps

1. Add debug logging to IndexSet opcode execution
2. Check if process is spinning (CPU 100%) or deadlocked
3. Use debugger to see exact point of hang
4. Consider array index assignment to see if issue is specific to dicts
5. Check git history to find when this broke (if it ever worked)

## Workaround

None available. Dict mutation is completely broken.

## Related Files

- `/Users/robertdevore/Documents/Kujolang/kujo-repos/kujo/src/vm.rs` - IndexSet implementation (lines 1347-1376)
- `/Users/robertdevore/Documents/Kujolang/kujo-repos/kujo/src/compiler.rs` - Assignment compilation (lines 997-1023)
- `/Users/robertdevore/Documents/Kujolang/kujo-repos/kujo/src/bytecode.rs` - OpCode definitions

## Test Files Created

- `/tmp/test_simple.kujo` - Basic dict index write
- `/tmp/test_basic.kujo` - Dict creation only (works)
- `/tmp/test_index_read.kujo` - Dict read (works)  
- `/tmp/test_global_dict.kujo` - Dict write in loop (hangs)
- `/tmp/test_func_dict.kujo` - Dict write in function (hangs)
