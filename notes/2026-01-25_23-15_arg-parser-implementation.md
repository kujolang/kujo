# Kujo Field Notes — Argument Parser Implementation

**Date:** 2026-01-25
**Session:** 23:15 local
**Branch/Commit:** main / abb5e10
**Scope:** Implemented comprehensive command-line argument parser (arg_parser) for building professional CLI tools. Part of ROADMAP item #23 (Standard Library Expansion - Phase 2).

---

## What I Changed

- **src/builtins.rs**:
  - Added `ArgumentDef` struct to represent argument definitions (long/short names, type, required, help, default)
  - Implemented `parse_arguments()` function to parse CLI arguments based on definitions
  - Implemented `generate_help()` function to create formatted help text
  - Added null handling for optional arguments without defaults
  - Updated `get_args()` to check `KUJO_SCRIPT_ARGS` environment variable for clap-parsed arguments

- **src/interpreter.rs**:
  - Registered `arg_parser()` native function in `register_builtins()`
  - Implemented ArgParser as a special Struct type ("ArgParser") with internal state
  - Added method handlers for:
    - `add_argument()` - Fluent API for adding argument definitions
    - `parse()` - Parse command-line arguments and return dictionary
    - `help()` - Generate formatted help text
  - Methods modify and return the ArgParser struct to enable chaining

- **src/main.rs**:
  - Added `script_args` field to `Run` command with `trailing_var_arg` and `allow_hyphen_values`
  - Implemented logic to store script arguments in `KUJO_SCRIPT_ARGS` environment variable
  - Enables passing arguments directly: `kujo run script.kujo --flag --option value`

- **tests/arg_parser.kujo**: Comprehensive test suite with 15 test cases
- **tests/arg_parser.out**: Expected output file
- **examples/arg_parser_demo.kujo**: Real-world CLI tool demonstration
- **CHANGELOG.md**, **ROADMAP.md**, **README.md**: Documentation updates

---

## Gotchas (Read This Next Time)

### Dictionary access for missing optional arguments returns 0, not null
- **Problem**: When optional arguments without defaults aren't provided, they weren't added to the result dict
- **Symptom**: Accessing missing keys returns 0 (default behavior) instead of a meaningful value
- **Solution**: Explicitly add `Value::Null` for optional non-bool arguments without defaults in `parse_arguments()`
- **Code location**: src/builtins.rs, parse_arguments() function (around line 1755)

### Clap intercepts script arguments without trailing_var_arg
- **Problem**: `kujo run script.kujo --flag value` fails because clap tries to parse `--flag` as a kujo option
- **Rule**: Must use `trailing_var_arg = true` and `allow_hyphen_values = true` on `script_args` field
- **Why**: Clap's default behavior is to parse all flags/options, even after positional arguments
- **Solution**: Add `script_args: Vec<String>` with proper annotations to Run command
- **Implication**: This enables natural CLI argument passing without requiring `--` separator

### Environment variable workaround for passing parsed args to builtins
- **Problem**: `env::args()` returns original process arguments, not clap-parsed script arguments
- **Solution**: Store script arguments in `KUJO_SCRIPT_ARGS` environment variable, check it first in `get_args()`
- **Why**: Can't directly modify `std::env::args()`, so we use env vars as a side channel
- **Pattern**: `get_args()` checks env var first, falls back to filtering `env::args()` if not set

### ArgParser uses Struct type, not a new Value variant
- **Problem**: Could add new `Value::ArgParser` variant, but that's heavy
- **Rule**: Use existing `Value::Struct { name: "ArgParser", fields }` with special name
- **Why**: Simpler implementation, reuses struct machinery, special methods handled by name check
- **Pattern**: Check `if name == "ArgParser"` in method call handler, implement methods there

---

## Things I Learned

### Fluent API pattern in Kujo
- Methods return modified struct to enable chaining
- Pattern: `let parser := parser.add_argument(...).add_argument(...)`
- Implementation: Clone struct fields, modify, return new struct
- Works well with Kujo's immutable-by-default variables

### Clap's trailing_var_arg feature
- Allows collecting all remaining arguments after positional ones
- Essential for implementing script runners that pass arguments through
- Combines well with `allow_hyphen_values` to accept flags

### Dictionary access gotchas
- Missing keys return 0 in Kujo (not an error)
- String interpolation with dict values works: `"Value: ${dict["key"]}"`
- Direct access works: `let val := dict["key"]`
- Always initialize all expected keys, even with Null

---

## Debug Notes

### Testing arg_parser with real arguments
Initial attempt failed because clap intercepted arguments:
```bash
cargo run -- run script.kujo --input file.txt  # ERROR: unexpected argument '--input'
```

After adding `trailing_var_arg`:
```bash
cargo run -- run script.kujo --input file.txt  # Works!
```

### Null values vs missing keys
Initial implementation didn't add missing optional args to result dict.
Accessing them returned 0, which was confusing:
```kujo
print("Input: ${args["input"]}")  # Printed "Input: 0" instead of "Input: null"
```

Fixed by explicitly adding `Value::Null` for missing optional non-bool arguments.

---

## Follow-ups / TODO (For Future Agents)

### Positional argument handling
- Current implementation collects positional args in `_positional` key
- Consider adding proper positional argument definitions with names and types
- Example: `parser.add_positional("files", type="array", help="Files to process")`

### Help text formatting
- Current help text is basic but functional
- Could add:
  - Usage line: `Usage: program [OPTIONS] FILES`
  - Argument grouping: Required/Optional sections
  - Better alignment of descriptions
  - Examples section

### Subcommands
- Many CLI tools have subcommands (git clone, git push, etc.)
- Could add `parser.add_subcommand("name", description, callback)`
- Each subcommand would have its own argument parser

### Argument validation
- Add min/max for int/float arguments
- Add regex patterns for string validation
- Add choices: `type="string", choices=["json", "xml", "yaml"]`

### Mutual exclusivity
- Add `mutually_exclusive_with` option
- Example: `--verbose` and `--quiet` shouldn't both be true

---

## Links / References

- **ROADMAP**: Standard Library Expansion (#23) - arg_parser marked as complete
- **Similar implementations**:
  - Python's argparse
  - Rust's clap
  - Node.js's commander/yargs

---

## Assumptions I Almost Made

### "ArgParser needs a new Value variant"
- **Almost did**: Add `Value::ArgParser { definitions: Vec<ArgumentDef> }`
- **Actually**: Used `Value::Struct` with special name "ArgParser"
- **Why better**: Simpler, reuses existing infrastructure, less code to maintain

### "Arguments must be passed with -- separator"
- **Almost did**: Document that users must use `kujo run script.kujo -- --arg`
- **Actually**: Implemented `trailing_var_arg` so `kujo run script.kujo --arg` works directly
- **Why better**: More natural UX, matches how other script runners work

### "Each add_argument call needs all parameters"
- **Almost did**: Make all parameters required in function call
- **Actually**: Used key-value pairs: `add_argument("--flag", "type", "bool", "help", "Help text")`
- **Why better**: Flexible, optional parameters just omitted, reads like named arguments

---

## Verification

All tests passing:
```bash
$ cargo test
# All 208+ tests pass
```

Manual testing:
```bash
$ cargo run -- run examples/arg_parser_demo.kujo --input data.csv --verbose --limit 25
# Works perfectly, parses all arguments correctly
```

Help generation:
```kujo
let parser := arg_parser()
let parser := parser.add_argument("--verbose", "short", "-v", "type", "bool", "help", "Verbose mode")
print(parser.help())
# Prints formatted help text
```

---

## Commits Made

1. `:package: NEW: implement arg_parser for CLI argument parsing`
   - ArgumentDef struct and parsing logic in builtins.rs
   - ArgParser struct handling in interpreter.rs
   - CLI argument passing in main.rs

2. `:ok_hand: IMPROVE: add comprehensive tests and examples for arg_parser`
   - tests/arg_parser.kujo with 15 test cases
   - examples/arg_parser_demo.kujo real-world example

3. `:book: DOC: document arg_parser in CHANGELOG, ROADMAP, and README`
   - CHANGELOG: Added Argument Parser section with examples
   - ROADMAP: Marked arg_parser as complete in Standard Library Expansion
   - README: Added arg_parser to System/CLI features list

All commits pushed to main successfully.
