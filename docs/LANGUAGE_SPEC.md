# Kujo Language Specification

Status: stable v1.0.0 contract
Spec version: 1.0.0
Last updated: 2026-08-08

## 1. Scope

This document defines the stable Kujo language and tooling compatibility contract for v1.0.0.

This specification describes the stable Kujo `v1.0.0` language contract. Explicit deferrals and non-goals remain documented in `docs/V1_SCOPE.md` and `docs/VM_INTERPRETER_PARITY_MATRIX.md`.

Release boundary: Kujo `v1.0.0` is the current stable release; explicit deferrals and compatibility guarantees are governed by `docs/V1_SCOPE.md`.

It is normative for:

- source-language syntax and parsing behavior
- runtime/evaluation semantics for core language constructs
- machine-readable contract rules for CLI and LSP output payloads
- breaking-change and versioning policy for language/tooling surfaces

Repository tests and fixtures remain the executable source of truth for implementation behavior, but any behavior that conflicts with this document requires either:

- an implementation fix, or
- a documented spec update with compatibility-policy classification

## 2. File Model

- Kujo source files use the `.kujo` extension.
- Source text is UTF-8.
- Line endings `\n` and `\r\n` are accepted by parser/lexer pathways.
- CLI parse entrypoints reject source files larger than `1,048,576` bytes with a parser diagnostic (`KUJOPARSE001`).

## 3. Lexical Model

The lexer tokenizes source into:

- identifiers
- keywords (`func`, `let`, `mut`, `const`, `if`, `else`, `for`, `while`, `loop`, `return`, `break`, `continue`, `async`, `await`, `match`, `case`, `try`, `except`, `throw`, `struct`, `test`, `test_group`, `test_setup`, `test_teardown`)
- literals (numeric, string, boolean, `null`)
- punctuation and operators
- comments (`#`, `//`, `/* ... */`, `///`)

Contextual constructors `Ok`, `Err`, `Some`, and `None` are identifiers in tokenization and parser flow (not lexer keywords).

Lexing failures are reported as structured diagnostics with source location metadata.
Malformed source must not be silently accepted as valid tokens.
Current lexer diagnostics include invalid character, null byte, unterminated string, unterminated block comment, invalid escape, malformed numeric literal, numeric overflow, and identifier/string/numeric token-length limit violations.

Lexical example:

```kujo
# line comment
// alternate line comment
/* block comment */

let escaped := "line\\nnext"
let quote := "say \\\"hi\\\""
let value := 42
```

## 4. Core Grammar Baseline (v0.14.0)

This section is an EBNF-style baseline for currently supported syntax.

```ebnf
program           = { declaration_or_statement } ;

declaration_or_statement
                  = function_decl
                  | struct_decl
                  | binding_stmt
                  | assignment_stmt
                  | control_stmt
                  | test_decl
                  | expression_stmt ;

function_decl     = [ "async" ] "func" identifier
                    "(" [ parameter_list ] ")"
                    [ "->" type_expr ]
                    block ;

parameter_list    = parameter { "," parameter } ;
parameter         = identifier [ ":" type_expr ] ;

struct_decl       = "struct" identifier "{" { struct_field } "}" ;
struct_field      = identifier [ ":" type_expr ] [ "=" expression ] ;

binding_stmt      = ( "let" | "mut" | "const" ) identifier
                    [ ":" type_expr ] ":=" expression ;

control_stmt      = if_stmt | while_stmt | loop_stmt | for_stmt
                    | return_stmt | break_stmt | continue_stmt
                    | match_stmt | try_except_stmt ;

if_stmt           = "if" expression block [ "else" ( if_stmt | block ) ] ;
while_stmt        = "while" expression block ;
loop_stmt         = "loop" block ;
for_stmt          = "for" identifier "in" expression block ;

match_stmt        = "match" expression "{" { case_clause } "}" ;
case_clause       = "case" pattern [ "if" expression ] block ;

try_except_stmt   = "try" block "except" [ identifier ] block ;

test_decl         = "test" string_literal block
                    | "test_group" string_literal block
                    | "test_setup" block
                    | "test_teardown" block ;

assignment_stmt   = assign_target assignment_op expression ;
assign_target     = identifier | field | index ;
expression_stmt   = expression ;

expression        = pipe_expr ;
assignment_op     = ":=" | "=" | "+=" | "-=" | "*=" | "/=" | "%=" ;

pipe_expr         = null_coalescing { "|>" null_coalescing } ;
null_coalescing   = logical_or { "??" logical_or } ;
logical_or        = logical_and { "||" logical_and } ;
logical_and       = equality { "&&" equality } ;
equality          = comparison { ( "==" | "!=" ) comparison } ;
comparison        = term { ( "<" | "<=" | ">" | ">=" ) term } ;
term              = factor { ( "+" | "-" ) factor } ;
factor            = unary { ( "*" | "/" | "%" ) unary } ;
unary             = ( "!" | "-" | "await" ) unary | postfix ;

postfix           = primary { call | index | field | method_call } ;
call              = "(" [ argument_list ] ")" ;
method_call       = "." identifier "(" [ argument_list ] ")" ;
index             = "[" expression "]" ;
field             = "." identifier ;

argument_list     = expression { "," expression } ;

primary           = literal
                  | identifier
                  | array_literal
                  | dict_literal
                  | function_expr
                  | spawn_expr
                  | "(" expression ")" ;

array_literal     = "[" [ array_elements ] "]" ;
array_elements    = array_element { "," array_element } ;
array_element     = expression | "..." expression ;

dict_literal      = "{" [ dict_elements ] "}" ;
dict_elements     = dict_element { "," dict_element } ;
dict_element      = ( string_literal | identifier ) ":" expression
                  | "..." expression ;

function_expr     = [ "async" ] "func" "(" [ parameter_list ] ")"
                    [ "->" type_expr ] block ;

spawn_expr        = "spawn" block ;

type_expr         = identifier { type_suffix } ;
type_suffix       = "?" | "[]" | "<" type_expr { "," type_expr } ">" ;
```

Notes:

- Spread (`...`) is valid in array/dictionary literal element positions.
- `Ok/Err/Some/None` pattern matching remains contextual and parser-driven.
- Parser safety limits: expression nesting depth is capped at `256` and statement-block nesting depth is capped at `128`. Inputs beyond either limit fail with parser diagnostics instead of recursing indefinitely.
- Assignment operators (`:=`, `=`, `+=`, `-=`, `*=`, `/=`, `%=`) are statement-level only. Chained assignments (for example `a := b := 1`) are rejected with parser diagnostics.

### 4.1 Operator Precedence And Associativity

From highest precedence to lowest:

| Level | Operators | Associativity |
| --- | --- | --- |
| Postfix | `()`, `[]`, `.`, method call `.` + `()` | Left |
| Unary | `!`, unary `-` | Right |
| Multiplicative | `*`, `/`, `%` | Left |
| Additive | `+`, `-` | Left |
| Comparison | `<`, `<=`, `>`, `>=` | Left |
| Equality | `==`, `!=` | Left |
| Logical AND | `&&` | Left |
| Logical OR | `||` | Left |
| Null coalescing | `??` | Left |
| Pipe | `|>` | Left |
| Assignment statements | `:=`, `=`, `+=`, `-=`, `*=`, `/=`, `%=` | Non-associative (chaining rejected) |

## 5. Runtime Semantics Baseline

### 5.1 Bindings and mutability

- `let` introduces immutable bindings.
  - Reassignment is rejected (`let x := 1; x := 2` fails).
  - In-place mutation through that binding is rejected (`let items := [1]; items[0] := 2` fails).
- `mut` introduces mutable bindings.
  - Reassignment is allowed.
  - In-place mutation through the binding is allowed.
- `const` introduces constant bindings.
  - Reassignment is rejected.
  - In-place mutation through that binding is rejected.
- Assignment without an explicit binding keyword preserves existing Kujo behavior:
  - `name := value` updates an existing mutable binding when present.
  - otherwise it creates a new mutable binding in the current scope.

Example:

```kujo
let immutable := 1
mut counter := 0
counter += 1
const build_id := "v1"
```

### 5.2 Scope model

- Top-level script bindings resolve in the global scope.
- Function bodies introduce lexical scope boundaries.
- `if`/`else`, `while`, and `loop` bodies execute in nested lexical scopes.
- `for ... in` introduces a loop-variable scope; the loop variable does not leak after the loop completes.
- Duplicate declarations in the same lexical scope are rejected with `Duplicate declaration in the same scope: <name>`.
- Inner-scope shadowing is allowed and resolved by nearest lexical definition.
- Closures capture the nearest visible lexical binding.
- Referencing an identifier with no visible binding is a runtime error of the form `Undefined variable: <name>`. Kujo does not convert unknown identifiers into strings; quote string literals explicitly.

Example:

```kujo
mut outer := 10
if true {
    let outer := 20
    seen_inner := outer
}
seen_outer := outer
```

### 5.3 Function execution

- Functions support positional parameters.
- Function body fallthrough (reaching the end of the body without an explicit `return`) yields `null`.
- Return without explicit value yields `null`.
- `async func` values produce awaitable handles in runtime modes that support async scheduling.

Example:

```kujo
func add(a, b) {
    return a + b
}

func no_value() {
    let local := 1
}

let total := add(2, 3)
let fallback := no_value()
```

### 5.4 Control flow

- `if`/`else` branches evaluate condition truthiness using runtime truthiness rules.
- `for ... in` iterates over iterable runtime values.
- `break` and `continue` are valid only within loop contexts.

Truthiness rules are centralized across interpreter and VM:

- Falsey values: `false`, `null`, integer `0`, float `0.0`, empty string `""`, empty array `[]`, empty dictionary `{}`.
- Truthy values: every other runtime value, including non-empty strings/arrays/dictionaries, functions, structs/objects, and native handles.
- String values are not parsed as boolean keywords in truthiness checks (`"false"` is a non-empty string and therefore truthy).

Logical operators use these same rules:

- `a && b` short-circuits when `a` is falsey; `b` is evaluated only when `a` is truthy.
- `a || b` short-circuits when `a` is truthy; `b` is evaluated only when `a` is falsey.
- Both operators return boolean results (`true`/`false`), not the original operand values.

Example:

```kujo
mut falsey_hits := 0
if 0 { falsey_hits += 1 }
if "" { falsey_hits += 1 }

mut truthy_hits := 0
if 1 { truthy_hits += 1 }
if "false" { truthy_hits += 1 }

let and_value := true && 5
let or_value := false || 5
```

### 5.5 Error flow

- `throw(value)` signals runtime exceptions.
- `try`/`except` catches exceptions thrown in protected regions.
- parse/compile/runtime error pathways must produce deterministic message shapes for machine-readable mode.

### 5.6 Data structures

- Arrays preserve insertion order.
- Dictionaries preserve key/value associations; merge/spread behavior is right-biased for duplicate keys.
- Dictionary indexing with a missing key is a runtime error. Programs that need fallback behavior should use explicit dictionary helpers such as `has_key`, `get`, or `get_default`.
- Dictionary indexing accepts string keys and integer keys. Other key types are invalid index operations.
- Array/string indexing outside bounds is a runtime error (`Index out of bounds: <index>`), not a sentinel-value fallback.
- Invalid index assignment targets (for example assigning through index access on non-indexable values) are runtime errors.
- Unsupported unary/binary operations are runtime errors; Kujo does not silently coerce invalid operations to `Int(0)` or empty-string values.
- Struct fields are resolved by declared field names.
- Struct method behavior and runtime-path parity are tracked in `docs/VM_INTERPRETER_PARITY_MATRIX.md`.

Example:

```kujo
mut profile := {"name": "kujo", "visits": 1}
profile["visits"] += 1

mut items := [1, 2, 3]
items[0] := 9
```

### 5.7 Concurrency and await

- `await` blocks expression completion on pending async values.
- `spawn { ... }` schedules detached async work where supported by runtime mode.
- Current VM/interpreter parity and capability notes for `spawn`, spread/destructuring, and match-binding surfaces are tracked in `docs/VM_INTERPRETER_PARITY_MATRIX.md`.

### 5.8 Numeric semantics

- Kujo integers are signed 64-bit values (`i64`).
- Integer arithmetic (`+`, `-`, `*`, `/`, `%`) uses checked execution:
  - overflow is a runtime error (`Integer overflow: <left> <op> <right>`),
  - division by zero is a runtime error (`Division by zero`),
  - modulo by zero is a runtime error (`Modulo by zero`).
- Float arithmetic keeps IEEE results for non-zero divisors, with explicit zero-divisor guards:
  - `x / 0.0` is a runtime error (`Division by zero`),
  - `x % 0.0` is a runtime error (`Modulo by zero`).
- Float equality is deterministic:
  - `NaN` is never equal to any value (including itself),
  - `!=` against `NaN` is true by complement of equality,
  - infinities compare by IEEE sign/value (`+inf == +inf`, `+inf != -inf`),
  - finite float equality retains epsilon-based comparison.
- Float ordering (`<`, `<=`, `>`, `>=`) follows IEEE behavior:
  - comparisons against `NaN` evaluate to false,
  - infinities compare as expected (`+inf` greater than finite values, `-inf` less than finite values).

### 5.9 Equality and comparison semantics

- `==` and `!=` always return boolean results.
- Primitive equality rules:
  - `null == null` is `true`; `null` compared to non-`null` is unequal.
  - booleans compare only against booleans.
  - strings compare only against strings.
  - bytes compare by exact byte sequence.
  - numeric equality is cross-type for `int`/`float` using Kujo float-equality policy (`1 == 1.0` is `true`).
- Collection and structured equality rules:
  - arrays compare deeply and order-sensitively.
  - dictionaries compare deeply by key/value pairs; runtime dictionary encodings (`Dict`, fixed dictionaries, and optimized integer-key variants) are treated as semantic equals when their effective key/value content matches.
  - tagged values, structs, struct definitions, `Result`, and `Option` compare structurally by matching metadata plus recursively equal contained values.
- Callable equality rules:
  - native functions compare by function name identity.
  - interpreter closures and async closures compare by function-body identity plus captured-environment identity.
  - VM bytecode closures compare by compiled chunk equality plus captured-binding identity.
  - generator definitions compare by parameter list and function-body identity.
- Non-data runtime handles that do not satisfy one of the rules above compare unequal (`==` returns `false`, `!=` returns `true`).
- Ordering operators (`<`, `<=`, `>`, `>=`) are only defined for:
  - numeric pairs (`int`/`int`, `float`/`float`, `int`/`float`, `float`/`int`),
  - string pairs (`string`/`string`, lexicographic ordering).
- Unsupported ordering type pairs are runtime errors:
  - `Invalid binary operation: <left_type> <op> <right_type>`.

### 5.10 Module import resolution and caching semantics

- Kujo supports `import module_name`, `from module_name import symbol1, symbol2`, and dotted `from` module paths such as `from src.util import value` and `from src.core.math import add`.
- Dotted `from` paths must use identifier segments separated by `.`; malformed paths (for example `from src..util import value`) are parse errors.
- Import resolution searches for module files in deterministic order:
  - the importing module's package root (for nested imports),
  - then the loader's configured module search paths.
- For each search root Kujo checks candidates in this exact precedence order:
  - `<module_name>.kujo` (legacy flat behavior),
  - for dotted module names only, `<seg1>/<seg2>/.../<segN>.kujo` (directory-backed dotted resolution).
- Module names are normalized as relative paths and must not contain unsafe traversal components:
  - parent traversal (`..`) is rejected,
  - absolute/drive-prefixed paths are rejected,
  - symlink-resolved canonical targets must remain inside the active search root.
- Import cycles are rejected with deterministic runtime diagnostics that include the full cycle chain (for example `Circular import detected: a -> b -> a`).
- Module cache behavior:
  - cache keys are scoped by package-root context plus canonical module path,
  - cached exports are reused only while module source metadata is unchanged,
  - when source metadata changes (mtime/size), the module is re-evaluated and cache state is refreshed.

Example:

```kujo
import math_helpers
from metrics import average, total
from src.util import value
from src.core.math import add
```

### 5.11 Package workflow and lockfile determinism

- `kujo init` creates a package manifest (`kujo.toml`) and a starter source layout.
- `kujo package-add <name> --version <range>` updates dependency declarations in the manifest.
- `kujo package-install` regenerates the deterministic `kujo.lock` snapshot derived from `kujo.toml`.
- `kujo package-install --frozen` verifies that the manifest and lockfile remain in sync without rewriting either file.
- Package workflow imports use the same package-root-aware module resolution rules as ordinary runtime imports, so nested layouts under `src/` remain available on the default VM path.
- Installations may add shared first-party source-module roots with the platform path-list environment variable `KUJO_MODULE_PATH`; local entry-file and project `modules/` roots take precedence.

### 5.12 Diagnostics and CLI exit codes

- CLI diagnostics are emitted on `stderr`.
- Successful program output and successful `--json` payloads are emitted on `stdout`.
- Exit-code categories are stable for automation:
  - `0`: success
  - `1`: command failure or unmet gate (`format --check`, lint/test failure)
  - `2`: command-line usage or argument parse error
  - `3`: lexer/parser diagnostic error
  - `4`: runtime semantic/execution error
  - `5`: IO failure
  - `6`: internal/tooling failure
- Machine-readable diagnostic payload shape contracts are documented in:
  - `docs/CLI_MACHINE_READABLE_CONTRACTS.md`
  - `docs/PROTOCOL_CONTRACTS.md`

## 6. Tooling Contract Compatibility Guarantees

The following compatibility classes apply to language and tooling behavior:

1. Syntax compatibility
- No previously valid v0.13.0 syntax may become invalid in a patch release.
- Grammar additions must be backward compatible within a minor line unless explicitly documented as breaking.

2. Runtime compatibility
- Existing successful programs should preserve observable behavior across patch releases.
- Error-message wording may be improved, but machine-readable error fields must remain compatible.

3. CLI/LSP machine-readable compatibility
- For documented `--json` or JSON-RPC payloads, field names and top-level shape are stable within a minor line.
- Adding optional fields is non-breaking.
- Removing fields, renaming fields, or changing field types is breaking.

4. Native builtin compatibility
- Existing builtin names and documented argument contracts are stable within a minor line unless a security fix requires immediate breakage.
- High-level AI HTTP helpers (`ai_chat`, `ai_stream_chat`, `ai_embedding`, `ai_tool_loop`) follow deterministic contracts:
  - invalid argument/options shapes return `Value::Error` contract messages;
  - transport/provider failures return `Result(Err("<message>"))` by default;
  - `options.structured_errors: true` returns typed `Result(Err(<dictionary payload>))` failures;
  - successful responses return `Result(Ok(<dictionary payload>))` with additive AI metadata such as `usage`, `finish_reason`, `tool_calls`, and `provider` when available.
  - `ai_stream_chat` may receive an optional callback `(delta, raw_chunk)`; returning `false` cancels later chunk delivery while preserving the aggregate return shape.
  - `ai_text`, `ai_image_url`, and `ai_message` build pure, capability-free message dictionaries accepted by AI helpers.

```kujo
opts := {"endpoint": "http://127.0.0.1:8080/v1/chat/completions", "model": "gpt-mock"}
chat := ai_chat("Hello", opts)
```

## 7. Breaking-Change Policy

A change is breaking if it does any of the following:

- makes previously valid syntax invalid
- changes runtime behavior for existing valid programs in non-bug-fix scenarios
- removes or renames machine-readable fields in CLI/LSP payloads
- changes command exit code meaning for established command/result classes
- removes or renames stable builtin APIs

Breaking changes require:

- a minor (or major) version bump consistent with semantic versioning intent
- explicit changelog entry under a compatibility-impact note
- migration guidance in release documentation

Emergency exception rule:

- security and correctness hotfixes may break behavior in patch releases when required to prevent data loss, corruption, or exploitation
- such exceptions must include rationale and follow-up migration guidance in release notes

## 8. Versioning Rules For Language/Tooling Contracts

- Patch (`x.y.Z`): backward-compatible fixes, performance work, diagnostics improvements, additive optional output fields.
- Minor (`x.Y.z`): additive, backward-compatible features and planned contract updates.
- Major (`X.y.z`): contract-reset level changes requiring migration treatment.

For the stable 1.x line, Kujo follows strict contract discipline:

- breaking changes require a major version except for the documented emergency exception above
- any intentional break must be documented in the changelog and release evidence
- machine-readable outputs should preserve stability expectations used by automation and IDE integrations

## 9. Specification Governance

- Any change to this document must be reviewed alongside relevant tests/fixtures.
- Spec updates that alter compatibility guarantees require matching changelog notes.
- If implementation and spec disagree, either code or spec must be updated in the same development cycle before release sign-off.
