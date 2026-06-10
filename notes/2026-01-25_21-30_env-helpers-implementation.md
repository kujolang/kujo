# Kujo Field Notes — Environment Variable Helpers & CLI Arguments

**Date:** 2026-01-25
**Session:** 21:30 local
**Branch/Commit:** main / bc9da5e
**Scope:** Implemented comprehensive environment variable management functions (env_or, env_int, env_float, env_bool, env_required, env_set, env_list) and enhanced args() to filter kujo command/script from arguments. Fixed ErrorObject propagation in native functions for proper try/except handling.

---

## What I Changed

- `src/builtins.rs`:
  - Added 7 new environment variable helper functions with type parsing and validation
  - Enhanced `get_args()` to intelligently filter kujo executable, subcommand, and script filename
- `src/interpreter.rs`:
  - Registered 7 new native functions in interpreter initialization
  - Implemented handlers for env_or, env_int, env_float, env_bool, env_required, env_set, env_list
  - **Critical fix**: Modified Call expression handling to set `self.return_value` when native functions return ErrorObject
- `tests/env_and_args.kujo`: Comprehensive test suite covering all new functions
- `examples/env_config.kujo`: Database configuration pattern using environment variables
- `examples/cli_tool.kujo`: CLI tool demonstration
- `CHANGELOG.md`, `README.md`: Documentation updates

---

## Gotchas (Read This Next Time)

### Gotcha 1: ErrorObject Structure Is Specific
- **Symptom:** Compilation errors: `variant 'interpreter::Value::ErrorObject' has no field named 'details'`
- **Root cause:** ErrorObject has fixed fields: `message: String, stack: Vec<String>, line: Option<usize>, cause: Option<Box<Value>>` — NOT a generic HashMap
- **Fix:** Changed all ErrorObject creation to use correct fields:
  ```rust
  Value::ErrorObject {
      message: msg.clone(),
      stack: Vec::new(),
      line: None,
      cause: None,
  }
  ```
- **Prevention:** Always check `src/interpreter.rs` line ~260 for ErrorObject definition before creating new instances. Do NOT assume it has a "details" field.

### Gotcha 2: Native Function Errors Don't Auto-Propagate to try/except
- **Symptom:** Test showed "Should not reach here" instead of catching errors in try/except blocks
- **Root cause:** When native functions return `ErrorObject`, the value is returned normally to the caller. The `try/except` handler only checks `self.return_value`, not the actual returned Value
- **Fix:** Modified Call expression handling in `src/interpreter.rs` around line 6480:
  ```rust
  Value::NativeFunction(name) => {
      let res = self.call_native_function(&name, args);
      // Must set return_value for ErrorObject to trigger try/except
      match res {
          Value::ErrorObject { .. } | Value::Error(_) => {
              self.return_value = Some(res.clone());
              res
          }
          _ => res
      }
  }
  ```
- **Prevention:** ANY native function that can return ErrorObject or Error must have this pattern applied in the Call handler. Try/except ONLY checks `self.return_value`, not expression return values.

### Gotcha 3: args() Returns ALL Command-Line Arguments Including Kujo Itself
- **Symptom:** `args()` returned `["run", "examples/cli_tool.kujo"]` instead of script arguments
- **Root cause:** Original implementation used `env::args().skip(1)` which only skips the executable name, not the subcommand or script file
- **Fix:** Implemented smart filtering in `src/builtins.rs`:
  ```rust
  pub fn get_args() -> Vec<String> {
      let all_args: Vec<String> = env::args().collect();
      // Skip executable, detect subcommand (run/check/format), skip script file
      // Example: ["kujo", "run", "script.kujo", "arg1"] -> ["arg1"]
  }
  ```
- **Prevention:** When working with CLI arguments, remember the full structure is: `[kujo_binary, maybe_subcommand, script.kujo, ...user_args]`. User-facing `args()` should only return the last portion.

### Gotcha 4: Dictionary Field Access with Dot Notation Doesn't Work
- **Symptom:** Example printed `0` instead of actual dictionary values when using `db_config.host`
- **Root cause:** Dictionaries in Kujo (created with `{}` syntax) are `Value::Dict(HashMap)`, NOT structs. Field access (`obj.field`) only works on `Value::Struct`, not on dicts
- **Fix:** Changed from `db_config.host` to `db_config["host"]` (bracket notation)
- **Prevention:** 
  - Use `{}` syntax → `Value::Dict` → Access with `dict["key"]`
  - Use `struct Name { fields }` → `Value::Struct` → Access with `instance.field`
  - Never mix the two access patterns

---

## Things I Learned

### Type Checker vs Runtime Behavior
- Type checker shows warnings for undefined functions (env_or, env_int, etc.) because these are registered at runtime, not in the type checker's function registry
- This is expected behavior and safe to ignore during development
- To fix: Would need to register native functions in type checker (deferred)

### Boolean Parsing Convention
- Chose to match shell/config file conventions: "true", "1", "yes", "on" (case insensitive) → true
- Everything else (including "false", "0", "no", "off") → false
- This is more forgiving than strict true/false and matches user expectations

### Environment Variable Error Messages
- Specific error messages are critical for debugging: 
  - `"Environment variable 'PORT' value 'abc' is not a valid integer"`
  - `"Required environment variable 'API_KEY' is not set"`
- Always include variable name AND problematic value in error messages

### Smart Args Filtering Pattern
- Need to handle multiple invocation styles:
  - `kujo run script.kujo arg1` → filter "kujo", "run", "script.kujo"
  - `kujo script.kujo arg1` → filter "kujo", "script.kujo"
  - Detection: Check if first arg after binary is a subcommand keyword or ends with `.kujo`

---

## Debug Notes

### Initial ErrorObject Compilation Failure
- **Failing error:** `error[E0559]: variant 'interpreter::Value::ErrorObject' has no field named 'details'`
- **Repro steps:** Added ErrorObject creation with `details: HashMap::new()` field
- **Breakpoints/logs used:** Searched for `ErrorObject {` pattern in interpreter.rs to find correct structure
- **Final diagnosis:** ErrorObject is not a generic error container - it has specific fields for structured error reporting with stack traces

### Try/Except Not Catching Native Function Errors
- **Failing test:** `try { env_int("INVALID") } except { }` printed "Should not reach here"
- **Repro steps:** Run env_and_args.kujo test
- **Investigation:** 
  - Checked how TryExcept statement works (line ~5590 in interpreter.rs)
  - Found it only checks `self.return_value`, not expression return values
  - Compared with how user-defined functions set `return_value` on errors
- **Final diagnosis:** Native function errors are "returned" as values but don't set the interpreter's return_value flag, so try/except doesn't see them

---

## Follow-ups / TODO (For Future Agents)

- [ ] Register native functions in type checker to eliminate "undefined function" warnings
- [ ] Consider adding `env_or_int(key, default)`, `env_or_float(key, default)` for one-liner defaults with type parsing
- [ ] Add CLI support for passing arguments to scripts (currently kujo CLI doesn't support: `kujo run script.kujo --flag value`)
- [ ] Consider moving environment helpers to a dedicated `env` module/namespace when module system is enhanced
- [ ] Add `env_remove(key)` function for completeness
- [ ] Performance: env_list() could be expensive for large environments - consider adding caching or lazy evaluation

---

## Links / References

- Files touched:
  - `src/builtins.rs` (env functions implementation)
  - `src/interpreter.rs` (native function registration and ErrorObject fix)
  - `tests/env_and_args.kujo` (comprehensive tests)
  - `examples/env_config.kujo` (database config pattern)
  - `examples/cli_tool.kujo` (CLI demonstration)
  - `CHANGELOG.md` (feature documentation)
  - `README.md` (System section update)
- Related docs:
  - `ROADMAP.md` (Item #23 - Standard Library Expansion)
  - `notes/2026-01-25_21-45_env-helpers-complete.md` (completion summary)

---

## Assumptions I Almost Made

- **Almost assumed** ErrorObject would have a generic "details" field for custom metadata
  - Reality: It has a structured format specifically for error reporting (message/stack/line/cause)
  
- **Almost assumed** returning ErrorObject from a native function would automatically trigger error handling
  - Reality: Must explicitly set `self.return_value` for try/except to detect the error

- **Almost assumed** dictionary field access would work like struct field access
  - Reality: They are completely different: dicts use bracket notation, structs use dot notation
