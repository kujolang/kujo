# Test Failures Tracking

**Last Updated**: 2026-01-26  
**Total Tests**: 134  
**Passing**: 119-121 (varies due to randomness)  
**Failing**: 13 (documented below)

## Summary

All 13 failing tests are for **unimplemented or incomplete features**. None are regressions from the generator implementation work.

## Failing Tests List

1. `tests/destructuring.kujo` - ✅ Documented in ROADMAP
2. `tests/spread_operator.kujo` - ✅ Documented in ROADMAP  
3. `tests/test_try_except.kujo` - ⚠️ NOT in ROADMAP
4. `tests/bytecode_vm.kujo` - ✅ Documented in ROADMAP
5. `tests/test_stdlib_random.kujo` - ⚠️ Partially documented
6. `tests/test_simple_random.kujo` - ⚠️ Partially documented
7. `tests/test_stdlib_datetime.kujo` - ⚠️ Partially documented
8. `tests/test_json_parse.kujo` - ⚠️ Partially documented
9. `tests/test_json_serialize.kujo` - ⚠️ Partially documented
10. `tests/dict_methods_test.kujo` - ⚠️ NOT specifically documented
11. `tests/test_enhanced_collections.kujo` - ⚠️ NOT specifically documented
12. `tests/stdlib_test.kujo` - ✅ Documented (general stdlib)
13. `tests/arg_parser.kujo` - ✅ Documented in ROADMAP

## Detailed Breakdown

### Destructuring & Spread (Tests 1-2)
- **ROADMAP Status**: Listed in v0.7.0 as "complete" but only parsing is done
- **Issue**: Parser works, interpreter doesn't execute the AST nodes
- **Priority**: HIGH - Marked as complete but broken

### Try/Except (Test 3)
- **ROADMAP Status**: ⚠️ NOT DOCUMENTED as a feature
- **Issue**: Only appears in async/await example, not a standalone feature
- **Priority**: HIGH - Core language feature for error handling
- **Action**: Add dedicated section to ROADMAP

### Bytecode VM (Test 4)
- **ROADMAP Status**: ✅ In v0.8.0 Remaining Work
- **Issue**: VM incomplete, many operations not supported
- **Priority**: MEDIUM - Documented as in-progress

### Standard Library Modules (Tests 5-9, 12)
- **ROADMAP Status**: ⚠️ Generally mentioned but not specific
- **Modules**: random, datetime, JSON, general stdlib
- **Issue**: Functions incomplete or buggy
- **Priority**: MEDIUM - Part of v0.8.0 "Standard Library Expansion"
- **Action**: Add specific sections for random, datetime, JSON modules

### Collections (Tests 10-11)  
- **ROADMAP Status**: ⚠️ NOT specifically documented
- **Issue**: Enhanced dict methods and collection operations missing
- **Priority**: LOW-MEDIUM - Enhancement rather than core feature
- **Action**: Document specific dict/collection methods

### Argument Parser (Test 13)
- **ROADMAP Status**: ✅ Mentioned in v0.9.0+ stdlib
- **Issue**: Not yet implemented
- **Priority**: LOW - Planned for later version

## Action Items

### Critical - Add to ROADMAP
These features have tests but aren't documented in ROADMAP:

1. **Try/Except Error Handling** - Core language feature
2. **JSON Module** - Essential stdlib (parse, serialize, validate)
3. **Random Module** - Common stdlib (random, random_int, choice, etc.)
4. **Datetime Module** - Common stdlib (now, format, parse, diff)

### Critical - Fix Documentation
These are marked "complete" but don't work:

1. **Destructuring** - Update ROADMAP: parsing ✅, interpreter ❌
2. **Spread Operator** - Update ROADMAP: parsing ✅, interpreter ❌

### To Investigate
Need to check what specific functionality is missing:

1. Dict methods - Which methods are tested/missing?
2. Enhanced collections - What operations are expected?
3. JSON issues - Parsing? Serialization? Both?
4. Random/datetime bugs - Are functions present but buggy?

## Notes

- **Test flakiness**: 119-121 passing is due to random number tests
- **No regressions**: All failures pre-existed generator work
- **Generator tests**: All 24 pass consistently ✅
- **Improvement**: Added +24 tests, net -1 failure from baseline

## Next Steps

1. Document the 4 missing features in ROADMAP
2. Update destructuring/spread status (parsing done, interpreter needed)
3. Run individual failing tests to understand specific issues
4. Create GitHub issues for each failure category
5. Prioritize fixes based on ROADMAP v0.8.0 goals
