# Standard Library Reference (v1.0.0)

Status: stable v1.0.0 reference
Last updated: 2026-07-25

This is the canonical native standard library reference for the builtins that
are registered in the stable v1 release. The exhaustive
machine-checkable inventory remains [STANDARD_LIBRARY.md](STANDARD_LIBRARY.md);
this reference groups the same runtime surface by user-facing workflow and calls
out readiness boundaries, capability gates, aliases, and sharp edges.

Tier definitions:

- `stable`: expected to remain backward-compatible across v1 patch/minor releases
- `preview`: available and documented, but may evolve with additional edge-case hardening
- `experimental`: available for advanced workflows, with higher change risk before post-v1 hardening

v1 contract policy for tiers:

- `stable`: in-scope for v1 compatibility guarantees.
- `preview`: in-scope for v1 usage, but not frozen; behavior may tighten during v1 hardening and must be treated as non-guaranteed until promoted.
- `experimental`: explicitly non-guaranteed for v1 compatibility commitments; available for advanced workflows only and may change or be restricted without stability guarantees.

Release boundary: Kujo `v1.0.0` is stable, while `preview` and `experimental` tiers retain the narrower guarantees defined above.
Deferred/non-goal policy source: `docs/V1_SCOPE.md`.

Source of truth:

- runtime registration and dispatch are implemented in `src/interpreter/mod.rs`
- builtin name inventory is returned by `Interpreter::get_builtin_names()`
- as of 2026-07-25, the runtime registers 363 builtin names plus `PI`, `E`, and the legacy `null` value binding
- arity and capability metadata are contract-tested against [STANDARD_LIBRARY.md](STANDARD_LIBRARY.md)

Recently promoted helper surfaces worth calling out:

- `eprint(...)` for stderr-friendly output when you want normal stdout reserved for machine-readable data.
- `bit_and(...)`, `bit_or(...)`, `bit_xor(...)`, `bit_not(...)`, `bit_shl(...)`, and `bit_shr(...)` for masks, flags, and other low-level integer work.
- `type_of(...)` and `is_truthy(...)` for runtime introspection in dynamic scripts.
- `pad_start(...)` and `pad_end(...)` for aligned CLI output and human-readable reports.
- `read_file_lossy(...)`, `path_is_symlink(...)`, and `sha256_file(...)` for practical filesystem workflows that need tolerant reads, symlink checks, and integrity validation.

For wrapper-to-native mappings, see [HELPER_DISCOVERY_COOKBOOK.md](HELPER_DISCOVERY_COOKBOOK.md).

Readiness summary:

- Stable everyday surface: printing, formatting, string helpers, array/dict helpers, JSON, hashing, path inspection, filesystem basics, process result structs, deterministic vector math, token estimation, and core type/introspection helpers.
- Preview practical surface: host-effect helpers, HTTP, AI record/replay helpers, structured-data formats, regex, random/time helpers, assertions, image/archive helpers, collections, concurrency, async helpers, and database basics.
- Experimental/high-change surface: low-level TCP/UDP, HTTP server/listener APIs, database pooling internals, process pipelines, and asymmetric/symmetric crypto helpers beyond hashes/password verification.
- Internal surface: `__vm_for_iterable` is registered for VM/runtime lowering and appears in inventories for completeness; application code should not depend on it.

## Core IO and Formatting

| Function | Tier | Example |
| --- | --- | --- |
| `print` | stable | `print("hello")` |
| `println` | stable alias | `println("hello")` |
| `eprint` | stable | `eprint("warning")` |
| `input` | preview | `name := input("name: ")` |
| `format` | stable | `line := format("{}-{}", ["a", 1])` |

`print` and `println` write newline-terminated text to stdout. `eprint`
writes to stderr so scripts can keep stdout reserved for machine-readable JSON.
All three are variadic.

## Strings and Text

| Function | Tier | Example |
| --- | --- | --- |
| `len` | stable | `size := len("abc")` |
| `substring` | stable | `part := substring("abcdef", 1, 4)` |
| `substr` | stable alias | `part := substr("abcdef", 1, 4)` |
| `to_upper` | stable | `v := to_upper("hello")` |
| `upper` | stable alias | `v := upper("hello")` |
| `to_lower` | stable | `v := to_lower("HELLO")` |
| `lower` | stable alias | `v := lower("HELLO")` |
| `capitalize` | stable | `v := capitalize("kujo")` |
| `trim` | stable | `v := trim("  hi  ")` |
| `trim_start` | stable | `v := trim_start("  hi")` |
| `trim_end` | stable | `v := trim_end("hi  ")` |
| `escape_xml` | stable | `safe := escape_xml("<tag>")` |
| `render_markdown` | preview | `html := render_markdown("# Title")` |
| `render_listing_card` | preview | `html := render_listing_card(route, title, excerpt, image, terms, label)` |
| `render_layout_native` | preview | `html := render_layout_native(template, settings, route, title, description, navigation, content, meta)` |
| `contains` | stable | `ok := contains("abcdef", "cd")` |
| `replace_str` | stable | `v := replace_str("a-b", "-", "_")` |
| `replace` | stable alias | `v := replace("a-b", "-", "_")` |
| `split` | stable | `parts := split("a,b", ",")` |
| `join` | stable | `text := join(["a", "b"], ",")` |
| `starts_with` | stable | `ok := starts_with("abc", "a")` |
| `ends_with` | stable | `ok := ends_with("abc", "c")` |
| `index_of` | stable | `i := index_of("abc", "b")` |
| `repeat` | stable | `v := repeat("ha", 3)` |
| `char_at` | stable | `ch := char_at("kujo", 1)` |
| `is_empty` | stable | `ok := is_empty("")` |
| `count_chars` | stable | `n := count_chars("kujo")` |
| `pad_left` | stable | `v := pad_left("kujo", 6, "0")` |
| `pad_right` | stable | `v := pad_right("kujo", 6, ".")` |
| `pad_start` | stable alias | `v := pad_start("kujo", 6, "0")` |
| `pad_end` | stable alias | `v := pad_end("kujo", 6, ".")` |
| `lines` | stable | `rows := lines("a\nb")` |
| `words` | stable | `parts := words("hello kujo")` |
| `str_reverse` | stable | `v := str_reverse("abc")` |
| `slugify` | preview | `slug := slugify("Hello World")` |
| `truncate` | preview | `v := truncate("abcdef", 4, "...")` |
| `to_camel_case` | preview | `v := to_camel_case("hello_world")` |
| `to_snake_case` | preview | `v := to_snake_case("helloWorld")` |
| `to_kebab_case` | preview | `v := to_kebab_case("helloWorld")` |
| `ssg_render_pages` | preview | `pages := ssg_render_pages(content, template, settings)` |
| `ssg_build_output_paths` | preview | `paths := ssg_build_output_paths(routes, "dist")` |
| `ssg_render_and_write_pages` | preview | `receipt := ssg_render_and_write_pages(pages, "dist")` |
| `ssg_read_render_and_write_pages` | preview | `receipt := ssg_read_render_and_write_pages(cfg)` |

Padding reaches a target width in Unicode characters and returns the original
string when it is already wide enough. Negative or non-finite widths are
errors; an empty pad string uses a space and a multi-character pad string uses
its first character. `slugify` lowercases text, retains Unicode alphanumerics,
maps whitespace/underscores to hyphens, removes other punctuation, and trims
edge hyphens.

`render_markdown(...)` escapes raw HTML text and replaces unsafe Markdown
link/image target schemes with `#`; use `html_response(...)` only for
intentionally raw HTML response bodies.

Predicate semantics note:

- `contains`, `starts_with`, `ends_with`, and `has_key` currently return `1`/`0`.
- Prefer explicit comparisons in control paths (for example `contains(text, "x") == 1`).

## Arrays and Collection Helpers

| Function | Tier | Example |
| --- | --- | --- |
| `push` | stable | `arr := push([1, 2], 3)` |
| `append` | stable alias | `arr := append([1, 2], 3)` |
| `pop` | stable | `last := pop([1, 2, 3])` |
| `insert` | stable | `arr := insert([1, 3], 1, 2)` |
| `remove` | stable | `arr := remove([1, 2, 3], 2)` |
| `remove_at` | stable | `arr := remove_at([1, 2, 3], 1)` |
| `clear` | stable | `arr := clear([1, 2])` |
| `slice` | stable | `part := slice([1, 2, 3, 4], 1, 3)` |
| `concat` | stable | `all := concat([1], [2, 3])` |
| `map` | stable | `out := map([1, 2], func (x) { return x * 2 })` |
| `filter` | stable | `out := filter([1, 2, 3], func (x) { return x > 1 })` |
| `reduce` | stable | `sum := reduce([1, 2, 3], 0, func (a, b) { return a + b })` |
| `find` | stable | `first := find(items, func (x) { return x > 1 })` |
| `sort` | preview | `out := sort([3, 1, 2])` |
| `reverse` | preview | `out := reverse([1, 2, 3])` |
| `unique` | preview | `out := unique([1, 1, 2])` |
| `sum` | preview | `total := sum([1, 2, 3])` |
| `any` | preview | `ok := any(items, func (x) { return x > 0 })` |
| `all` | preview | `ok := all(items, func (x) { return x > 0 })` |
| `chunk` | preview | `out := chunk([1, 2, 3, 4], 2)` |
| `flatten` | preview | `out := flatten([[1], [2, 3]])` |
| `zip` | preview | `out := zip([1, 2], ["a", "b"])` |
| `enumerate` | preview | `out := enumerate(["a", "b"])` |
| `take` | preview | `out := take([1, 2, 3], 2)` |
| `skip` | preview | `out := skip([1, 2, 3], 1)` |
| `windows` | preview | `out := windows([1, 2, 3], 2)` |
| `range` | stable | `nums := range(0, 5)` |

Collection update semantics:

- Helpers like `push`, `insert`, `remove_at`, `concat`, and `map` return updated values.
- Reassign the result when building arrays iteratively (`items = push(items, value)`).

## Output and Report Conventions

Kujo currently exposes low-level output primitives (`print`) rather than a built-in report DSL.
For scripts that emit many status lines, prefer local intent helpers to reduce repetitive mechanics:

```kujo
func section(title) { print(""); print("== " + title + " ==") }
func kv(label, value) { print("  " + label + ": " + value) }
func item(text) { print("  - " + text) }
```

Guideline:

- Small scripts: direct `print(...)` is fine.
- Multi-step CLI/report scripts: use local helpers for section/kv/list rendering.
- Machine-readable contracts: emit `to_json(...)` payloads on stdout and keep extra text minimal.

## Dicts and Structured Data

| Function | Tier | Example |
| --- | --- | --- |
| `keys` | stable | `k := keys({"a": 1, "b": 2})` |
| `values` | stable | `v := values({"a": 1, "b": 2})` |
| `items` | stable | `it := items({"a": 1})` |
| `has_key` | stable | `ok := has_key({"a": 1}, "a")` |
| `get` | stable | `v := get({"a": 1}, "a")` |
| `get_default` | stable | `v := get_default({"a": 1}, "b", 0)` |
| `merge` | preview | `m := merge({"a": 1}, {"b": 2})` |
| `update` | preview | `m := update({"a": 1}, {"a": 2})` |
| `invert` | preview | `m := invert({"a": 1, "b": 2})` |
| `parse_json` | stable | `obj := parse_json("{\"a\":1}")` |
| `to_json` | stable | `txt := to_json({"a": 1})` |
| `to_json_pretty` | stable | `txt := to_json_pretty({"a": 1})` |
| `json_schema_validate` | stable | `result := json_schema_validate(value, schema)` |
| `parse_toml` | preview | `cfg := parse_toml("x = 1")` |
| `to_toml` | preview | `txt := to_toml({"x": 1})` |
| `parse_yaml` | preview | `cfg := parse_yaml("x: 1")` |
| `to_yaml` | preview | `txt := to_yaml({"x": 1})` |
| `parse_csv` | preview | `rows := parse_csv("name\nkujo")` |
| `to_csv` | preview | `txt := to_csv([["name"], ["kujo"]])` |
| `encode_base64` | stable | `txt := encode_base64(bytes)` |
| `decode_base64` | stable | `bytes := decode_base64(txt)` |
| `decode_base64_utf8` | stable | `text := decode_base64_utf8(txt)` |
| `encode_uri_component` | stable | `part := encode_uri_component("café & tea")` |

Access semantics:

- Dictionary/map-like values use bracket access (`obj["key"]`).
- Runtime structs (for example `ProcessResult`) use dot fields (`result.exitcode`).

JSON serialization is deterministic for map-like values: string-key dictionaries
sort lexicographically, integer-key dictionaries sort numerically, and fixed or
dense dictionaries preserve declaration/index order. `to_json_pretty` shares
the exact conversion and ordering rules and only changes whitespace. Both
reject non-finite floats; `secret("value")` serializes as `"***"`. Runtime
structs, functions, bytes, and other unsupported values are errors. `parse_json`
accepts any JSON root value but caps input at 1 MiB and nesting at 64 levels.

Runnable contract example: [`examples/helper_hlp_015_canonical_json.kujo`](../examples/helper_hlp_015_canonical_json.kujo).

JSON Schema validation is local-only. It returns `{"valid": bool, "errors": [...]}`
for supported schemas and returns a runtime error for malformed schemas,
unsupported remote references, cyclic references, excessive recursion, excessive
validation nodes, oversized regex patterns, or arrays larger than the documented
limits in [STANDARD_LIBRARY.md](STANDARD_LIBRARY.md).

## Set, Queue, and Stack Collections

| Function | Tier | Example |
| --- | --- | --- |
| `Set` | preview | `s := Set()` |
| `set_add` | preview | `s := set_add(s, "a")` |
| `set_has` | preview | `ok := set_has(s, "a")` |
| `set_remove` | preview | `s := set_remove(s, "a")` |
| `set_union` | preview | `s := set_union(a, b)` |
| `set_intersect` | preview | `s := set_intersect(a, b)` |
| `set_difference` | preview | `s := set_difference(a, b)` |
| `set_to_array` | preview | `arr := set_to_array(s)` |
| `Queue` | preview | `q := Queue()` |
| `queue_enqueue` | preview | `q := queue_enqueue(q, "a")` |
| `queue_dequeue` | preview | `item := queue_dequeue(q)` |
| `queue_peek` | preview | `item := queue_peek(q)` |
| `queue_size` | preview | `n := queue_size(q)` |
| `queue_is_empty` | preview | `ok := queue_is_empty(q)` |
| `queue_to_array` | preview | `arr := queue_to_array(q)` |
| `Stack` | preview | `s := Stack()` |
| `stack_push` | preview | `s := stack_push(s, "a")` |
| `stack_pop` | preview | `item := stack_pop(s)` |
| `stack_peek` | preview | `item := stack_peek(s)` |
| `stack_size` | preview | `n := stack_size(s)` |
| `stack_is_empty` | preview | `ok := stack_is_empty(s)` |
| `stack_to_array` | preview | `arr := stack_to_array(s)` |

These constructors and helpers are ready for local scripts, examples, and test
fixtures, but remain preview because collection object shape and method aliases
may still be refined before a final frozen post-v1 compatibility policy.

## Types, Conversion, Assertions, and Secrets

| Function | Tier | Example |
| --- | --- | --- |
| `parse_int` | stable | `n := parse_int("42")` |
| `parse_float` | stable | `n := parse_float("3.14")` |
| `to_int` | stable | `n := to_int("42")` |
| `to_float` | stable | `n := to_float("3.14")` |
| `to_string` | stable | `s := to_string(42)` |
| `str` | stable alias | `s := str(42)` |
| `to_bool` | stable | `b := to_bool("true")` |
| `bytes` | preview | `b := bytes("hello")` |
| `dict` | stable | `d := dict()` |
| `array` | stable | `a := array(1, 2, 3)` |
| `error` | stable | `err := error("bad input")` |
| `type` | stable | `kind := type(value)` |
| `type_of` | stable alias | `kind := type_of(value)` |
| `is_truthy` | stable | `ok := is_truthy(value)` |
| `is_int` | stable | `ok := is_int(value)` |
| `is_float` | stable | `ok := is_float(value)` |
| `is_string` | stable | `ok := is_string(value)` |
| `is_secret` | stable | `ok := is_secret(value)` |
| `is_bool` | stable | `ok := is_bool(value)` |
| `is_array` | stable | `ok := is_array(value)` |
| `is_dict` | stable | `ok := is_dict(value)` |
| `is_null` | stable | `ok := is_null(value)` |
| `is_function` | stable | `ok := is_function(value)` |
| `assert` | stable | `assert(ok)` |
| `debug` | stable | `debug(value)` |
| `assert_equal` | preview | `assert_equal(actual, expected)` |
| `assert_true` | preview | `assert_true(ok)` |
| `assert_false` | preview | `assert_false(ok)` |
| `assert_contains` | preview | `assert_contains(text, "needle")` |
| `secret` | stable | `token := secret(env_required("TOKEN"))` |
| `reveal` | stable | `plain := reveal(token)` |

Secret redaction contract:

- `secret(value)` wraps a string in a redacted runtime value.
- Printing, debug formatting, JSON/TOML/YAML/CSV serialization, errors, and AI cassette request metadata render secrets as `***` or `Secret(***)`.
- Secrets compare by inner string value and clone like ordinary runtime values; `reveal(secret_value)` is the documented escape hatch for plaintext.
- `options.api_key` for AI helpers accepts a plain string or a secret; cassettes and error-body excerpts redact configured keys and sensitive authorization headers.

## Math, Time, and Random

| Function | Tier | Example |
| --- | --- | --- |
| `abs` | stable | `v := abs(-1)` |
| `sqrt` | stable | `v := sqrt(9)` |
| `pow` | stable | `v := pow(2, 8)` |
| `floor` | stable | `v := floor(3.8)` |
| `ceil` | stable | `v := ceil(3.2)` |
| `round` | stable | `v := round(3.5)` |
| `min` | stable | `v := min(1, 2)` |
| `max` | stable | `v := max(1, 2)` |
| `sin` | stable | `v := sin(0)` |
| `cos` | stable | `v := cos(0)` |
| `tan` | stable | `v := tan(0)` |
| `log` | stable | `v := log(10)` |
| `exp` | stable | `v := exp(1)` |
| `bit_and` | stable | `mask := bit_and(6, 3)` |
| `bit_or` | stable | `mask := bit_or(4, 1)` |
| `bit_xor` | stable | `mask := bit_xor(6, 3)` |
| `bit_not` | stable | `mask := bit_not(0)` |
| `bit_shl` | stable | `mask := bit_shl(1, 3)` |
| `bit_shr` | stable | `mask := bit_shr(8, 1)` |
| `vec_dot` | stable | `score := vec_dot([1, 2], [3, 4])` |
| `vec_norm` | stable | `length := vec_norm([3, 4])` |
| `vec_normalize` | stable | `unit := vec_normalize([3, 4])` |
| `vec_cosine` | stable | `score := vec_cosine([1, 0], [0, 1])` |
| `vec_top_k` | stable | `matches := vec_top_k([1, 0], [[1, 0], [0, 1]], 1)` |
| `random` | preview | `v := random()` |
| `random_int` | preview | `v := random_int(1, 10)` |
| `random_choice` | preview | `v := random_choice(["a", "b"])` |
| `uuid_v4` | preview | `id := uuid_v4()` |
| `random_id` | preview | `id := random_id(12)` |
| `set_random_seed` | preview | `set_random_seed(42)` |
| `clear_random_seed` | preview | `clear_random_seed()` |
| `now` | stable | `t := now()` |
| `now_utc` | stable | `iso := now_utc()` |
| `now_unix` | stable | `seconds := now_unix()` |
| `now_utc_seconds` | stable alias | `seconds := now_utc_seconds()` |
| `current_timestamp` | stable | `ts := current_timestamp()` |
| `time` | stable alias | `ts := time()` |
| `time_us` | preview | `us := time_us()` |
| `time_ns` | preview | `ns := time_ns()` |
| `format_duration` | preview | `text := format_duration(1530)` |
| `format_date` | stable | `day := format_date(now_unix(), "YYYY-MM-DD")` |
| `parse_date` | stable | `seconds := parse_date("1970-01-01", "YYYY-MM-DD")` |
| `performance_now` | preview | `ms := performance_now()` |
| `elapsed` | preview | `dt := elapsed(start_ms, end_ms)` |

Time units are explicit: `now` and `now_unix` return Unix seconds,
`current_timestamp` returns Unix milliseconds, and `now_utc` returns a UTC
string in `YYYY-MM-DDTHH:mm:ssZ` form. `format_date` consumes Unix seconds;
use `now_unix()` rather than `current_timestamp()`. Its supported replacement
tokens are `YYYY`, `MM`, `DD`, `HH`, `mm`, and `ss`. `parse_date` currently
accepts only `YYYY-MM-DD` and returns Unix seconds.

Runnable contract example: [`examples/helper_hlp_007_text_time.kujo`](../examples/helper_hlp_007_text_time.kujo).

## File System and Paths

The examples in this section assume trusted mode. For untrusted scripts, start
with `kujo run --untrusted` and add only the required filesystem capability
flags (`--allow-fs-read`, `--allow-fs-write`, and/or `--allow-fs-delete`).

| Function | Tier | Example |
| --- | --- | --- |
| `read_file` | stable | `txt := read_file("notes.txt")` |
| `read_file_lossy` | stable | `txt := read_file_lossy("legacy.txt")` |
| `write_file` | stable | `write_file("notes.txt", "hello")` |
| `write_file_atomic` | stable | `write_file_atomic("notes.txt", "hello", true)` |
| `append_file` | stable | `append_file("notes.txt", "more")` |
| `file_exists` | stable | `ok := file_exists("notes.txt")` |
| `read_lines` | stable | `rows := read_lines("notes.txt")` |
| `jsonl_query` | stable | `rows := jsonl_query("evidence.jsonl", {"filter_field": "provider", "filter_equals": "gsc", "max_rows": 100})` |
| `list_dir` | stable | `entries := list_dir(".")` |
| `create_dir` | stable | `create_dir("tmp")` |
| `file_size` | stable | `bytes := file_size("notes.txt")` |
| `delete_file` | stable | `delete_file("old.txt")` |
| `rename_file` | stable | `rename_file("a.txt", "b.txt")` |
| `copy_file` | preview | `copy_file("a.txt", "b.txt")` |
| `read_binary_file` | preview | `blob := read_binary_file("in.bin")` |
| `write_binary_file` | preview | `write_binary_file("out.bin", blob)` |
| `io_read_bytes` | preview | `blob := io_read_bytes("in.bin")` |
| `io_write_bytes` | preview | `io_write_bytes("out.bin", blob)` |
| `io_append_bytes` | preview | `io_append_bytes("out.bin", blob)` |
| `io_read_at` | preview | `chunk := io_read_at("in.bin", 0, 16)` |
| `io_write_at` | preview | `io_write_at("out.bin", 0, blob)` |
| `io_seek_read` | preview | `chunk := io_seek_read("in.bin", 128, 32)` |
| `io_file_metadata` | preview | `meta := io_file_metadata("in.bin")` |
| `io_set_permissions` | preview | `io_set_permissions("secret.txt", 384)` |
| `io_write_private_file` | preview | `io_write_private_file("secret.txt", text)` |
| `io_private_spool_open` | preview | `spool := io_private_spool_open("message.eml", 26214400, 384)` |
| `io_private_spool_write` | preview | `io_private_spool_write(spool, chunk)` |
| `io_private_spool_finish` | preview | `receipt := io_private_spool_finish(spool)` |
| `io_private_spool_abort` | preview | `io_private_spool_abort(spool)` |
| `io_truncate` | preview | `io_truncate("out.bin", 1024)` |
| `io_copy_range` | preview | `io_copy_range("in.bin", "out.bin", 0, 1024)` |
| `join_path` | stable | `p := join_path("a", "b")` |
| `dirname` | stable | `d := dirname("a/b.txt")` |
| `basename` | stable | `b := basename("a/b.txt")` |
| `path_exists` | stable | `ok := path_exists("notes.txt")` |
| `path_join` | stable alias | `p := path_join("a", "b")` |
| `path_absolute` | preview | `p := path_absolute(".")` |
| `path_is_dir` | stable | `ok := path_is_dir(".")` |
| `path_is_file` | stable | `ok := path_is_file("a.txt")` |
| `path_is_symlink` | stable | `ok := path_is_symlink("link")` |
| `path_extension` | stable | `ext := path_extension("a.txt")` |
| `os_getcwd` | stable | `cwd := os_getcwd()` |
| `os_chdir` | preview | `os_chdir("work")` |
| `os_rmdir` | preview | `os_rmdir("empty-dir")` |
| `os_environ` | preview | `vars := os_environ()` |

Write-file overwrite contract:

- `write_file(path, content)` errors if `path` already exists.
- To replace an existing file, pass options with overwrite enabled:
	- `write_file(path, content, {"overwrite": true})`

## Environment, Process, and Concurrency

The process/env examples assume trusted mode. For untrusted scripts, grant only
the required capability (`--allow-process-exec`, `--allow-shell-exec`,
`--allow-env-read`, or `--allow-env-write`), and prefer argv-array process APIs
over shell strings when handling user input.

| Function | Tier | Example |
| --- | --- | --- |
| `env` | stable | `home := env("HOME")` |
| `env_or` | stable | `mode := env_or("MODE", "dev")` |
| `env_int` | stable | `port := env_int("PORT", 8080)` |
| `env_float` | stable | `timeout := env_float("TIMEOUT", 1.5)` |
| `env_bool` | stable | `enabled := env_bool("ENABLED", false)` |
| `env_required` | stable | `token := env_required("TOKEN")` |
| `kv_set` | preview | `kv_set("key", "value")` |
| `kv_get` | preview | `value := kv_get("key")` |
| `env_set` | preview | `env_set("MODE", "dev")` |
| `env_list` | preview | `vars := env_list()` |
| `args` | stable | `argv := args()  # user args only; script name excluded` |
| `arg_parser` | preview | `opts := arg_parser(spec)` |
| `exit` | stable | `exit(0)` |
| `sleep` | stable | `sleep(100)` |
| `execute` | preview | `out := execute("echo hi", {"timeout_ms": 1000})` |
| `execute_status` | preview | `r := execute_status("echo hi")` |
| `spawn_process` | experimental | `r := spawn_process(["echo", "hi"], {"max_output_bytes": 4096})` |
| `pipe_commands` | experimental | `out := pipe_commands([["echo", "hi"], ["cat"]], {"timeout_ms": 1000})` |
| `channel` | preview | `ch := channel()` |
| `shared_set` | preview | `shared_set("count", 1)` |
| `shared_get` | preview | `v := shared_get("count")` |
| `shared_has` | preview | `ok := shared_has("count")` |
| `shared_delete` | preview | `shared_delete("count")` |
| `shared_add_int` | preview | `shared_add_int("count", 1)` |
| `async_sleep` | preview | `await async_sleep(100)` |
| `async_timeout` | preview | `result := await async_timeout(task, 1000)` |
| `async_http_get` | preview | `res := await async_http_get(url)` |
| `async_http_post` | preview | `res := await async_http_post(url, body)` |
| `async_read_file` | preview | `text := await async_read_file(path)` |
| `async_read_files` | preview | `texts := await async_read_files(paths)` |
| `async_write_file` | preview | `await async_write_file(path, text)` |
| `async_write_files` | preview | `await async_write_files(files)` |
| `spawn_task` | preview | `task := spawn_task(func () { return 1 })` |
| `await_task` | preview | `value := await_task(task)` |
| `cancel_task` | preview | `cancel_task(task)` |
| `Promise.all` | preview | `values := Promise.all(tasks)` |
| `promise_all` | preview alias | `values := promise_all(tasks)` |
| `await_all` | preview alias | `values := await_all(tasks)` |
| `parallel_map` | preview | `out := parallel_map([1,2], func (x) { return x + 1 })` |
| `promise_wait` | preview | `out := promise_wait(parallel_map(items, mapper, 4))` |
| `par_map` | preview alias | `out := par_map(items, func (x) { return x })` |
| `par_each` | preview | `par_each(items, func (x) { print(x) })` |
| `set_task_pool_size` | preview | `set_task_pool_size(8)` |
| `get_task_pool_size` | preview | `n := get_task_pool_size()` |

Environment reads require `env-read` in restricted mode. Typed accessors fail
when a variable is absent or malformed; `env_bool` accepts only the documented
case-insensitive true/false spellings. `env` returns an empty string when a
variable is absent, while `env_required` distinguishes absence with an error.
Load and validate configuration once, then pass the resulting dictionary to
the rest of the program.

Runnable contract example: [`examples/helper_hlp_011_env_config.kujo`](../examples/helper_hlp_011_env_config.kujo).

Process result contracts:

- `execute_status` and `spawn_process` return a `ProcessResult` runtime struct
  with dot fields `exitcode`, `stdout`, `stderr`, `success`, `timed_out`,
  `cancelled`, `stdout_truncated`, and `stderr_truncated`.
- `spawn_process` takes explicit argv and is gated by `process-exec`; use it
  for user-controlled arguments. `execute_status` and `execute` take shell
  command strings and are gated by `shell-exec`.
- Process options include `timeout_ms`, `max_output_bytes`, `inherit_env`,
  `env_allow`, `env_deny`, `env`, `stream_channel`, `stream_stdout_path`,
  `stream_stderr_path`, `redact_values`, `cancel_file`, and optional `cwd`.
  Defaults are 30
  seconds and 1 MiB per output stream; 16 MiB per stream is the maximum.
- `execute` returns a stdout string on successful, non-truncated exit and
  returns a runtime error on timeout, output-limit overflow, or non-zero exit.
- A `ProcessResult` cannot be passed directly to `to_json`; copy selected dot
  fields into a dictionary first.

CLI/Process semantics notes:

- `args()` returns only user-provided arguments after the script path. Example: `kujo run tool.kujo -- summarize --format json` becomes `args() == ["summarize", "--format", "json"]`.
- `execute(...)` accepts a single shell command string (not an argv array).
- Use `execute_status(...)` when you need exit code and stderr without exception-style control flow.

Runnable contract example: [`examples/helper_hlp_013_process_result.kujo`](../examples/helper_hlp_013_process_result.kujo).

Type taxonomy quick reference:

- Scalar literals: `"int"`, `"float"`, `"string"`, `"bool"`, `"null"`
- Containers: `"array"`, `"dict"`
- Parsed-document containers can appear as `"list"` in some parser-backed flows.
- Error values return `"Error"`.
- Use tolerant checks for parsed collections when needed (for example `type(x) == "array" || type(x) == "list"`).

## Network, HTTP, and Auth

Network examples assume trusted mode. In untrusted mode, outbound clients need
`--allow-net-client`, listeners need `--allow-net-server`, and private/local
destinations remain subject to the outbound destination policy described in
`docs/NATIVE_API_SECURITY_POSTURE.md`.

| Function | Tier | Example |
| --- | --- | --- |
| `regex_match` | stable | `ok := regex_match(text, "^[a-z]+$")` |
| `regex_find_all` | stable | `matches := regex_find_all(text, "\\w+")` |
| `regex_replace` | stable | `out := regex_replace(text, "\\s+", " ")` |
| `regex_split` | stable | `parts := regex_split(text, ",\\s*")` |
| `http_get` | preview | `res := http_get("https://example.com")` |
| `http_request` | preview | `res := http_request({"method":"GET", "url": url, "destination_policy":"deny_private", "pin_dns":true, "redirects":"none", "max_response_bytes":65536})` |
| `http_post` | preview | `res := http_post("https://example.com", {"x":1})` |
| `http_put` | preview | `res := http_put("https://example.com", {"x":1})` |
| `http_delete` | preview | `res := http_delete("https://example.com")` |
| `http_get_binary` | preview | `blob := http_get_binary("https://example.com/a.bin")` |
| `http_download_file` | preview | `receipt := http_download_file(url, "artifact.bin", {"max_bytes": 1073741824})` |
| `http_upload_file` | preview | `receipt := http_upload_file(url, "artifact.bin", {"method": "PUT", "headers": {"Content-Type": "application/octet-stream"}})` |
| `parallel_http` | preview | `all := parallel_http(["https://a", "https://b"])` |
| `ai_request_hash` | stable | `hash := ai_request_hash(request)` |
| `ai_chat` | preview | `res := ai_chat(messages, options)` |
| `ai_stream_chat` | preview | `res := ai_stream_chat(messages, options, callback)` |
| `ai_embedding` | preview | `vec := ai_embedding("hello", options)` |
| `ai_tool_loop` | preview | `res := ai_tool_loop(messages, tools, options)` |
| `http_get_stream` | experimental | `ch := http_get_stream(url, options)` |
| `http_server` | experimental | `srv := http_server(8080)`; 8 MiB body bound, bounded socket read deadline, 413/408 pre-dispatch errors, and socket-derived peer fields |
| `http_listen` | experimental | `srv := http_listen("127.0.0.1", 8080)`; uses the same body, deadline, and peer-identity policy |
| `http_response` | preview | `res := http_response(200, "ok")` |
| `json_response` | preview | `res := json_response({"ok": true})` |
| `html_response` | preview | `res := html_response("<p>ok</p>")` |
| `redirect_response` | preview | `res := redirect_response("/next")` |
| `set_header` | preview | `res := set_header(res, "X-Trace", "1")` |
| `set_headers` | preview | `res := set_headers(res, {"X-Trace": "1"})` |

For untrusted callback destinations, `http_request` supports three composable
request policies. `destination_policy: "deny_private"` rejects any DNS answer
in loopback, private, link-local, multicast, or unspecified ranges, pins the
validated answers, cannot be relaxed by the process-wide private-network
override, and requires `redirects: "none"`. `pin_dns: true` independently
installs the validated address set into the HTTP client so the request does not
perform a second DNS resolution. `redirects: "none"` returns 3xx responses
without following `Location`. Use all three explicitly for webhooks and similar
SSRF-sensitive callbacks. The backward-compatible defaults are `"inherit"`,
`false`, and `"follow"` respectively. `max_response_bytes` may lower the global
8 MiB response ceiling for a specific request and must be a positive integer.
| `jwt_encode` | preview | `tok := jwt_encode({"sub":"user"}, "secret")` |
| `jwt_decode` | preview | `payload := jwt_decode(tok, "secret")` |
| `jwt_verify` | preview | `ok := jwt_verify(tok, "secret")` |
| `oauth2_auth_url` | preview | `url := oauth2_auth_url(cfg)` |
| `oauth2_get_token` | preview | `tok := oauth2_get_token(cfg)` |

AI helper details live in [AI_RUNTIME.md](AI_RUNTIME.md). `ai_request_hash`,
`ai_count_tokens`, `ai_fit_context`, `ai_text`, `ai_image_url`, and
`ai_message` are local deterministic helpers. `ai_chat`, `ai_stream_chat`,
`ai_embedding`, and `ai_tool_loop` require the AI/network capability path and
should use replay cassettes in tests.

Low-level socket APIs are available for systems scripts, but remain
experimental because server lifecycle, nonblocking behavior, and cross-platform
edge cases are higher-risk than the HTTP helper surface.

| Function | Tier | Example |
| --- | --- | --- |
| `tcp_listen` | experimental | `srv := tcp_listen("127.0.0.1", 9000)` |
| `tcp_accept` | experimental | `conn := tcp_accept(srv)` |
| `tcp_connect` | experimental | `conn := tcp_connect("127.0.0.1", 9000)` |
| `tcp_connect_bound` | preview | `conn := tcp_connect_bound("mx.example.net", 25, "192.0.2.10")` |
| `tcp_send` | experimental | `tcp_send(conn, "hello")` |
| `tcp_receive` | experimental | `data := tcp_receive(conn, 1024)` |
| `tcp_close` | experimental | `tcp_close(conn)` |
| `tcp_set_nonblocking` | experimental | `tcp_set_nonblocking(conn, true)` |
| `tcp_info` | preview | `info := tcp_info(conn)` |
| `tcp_set_timeouts` | preview | `tcp_set_timeouts(conn, 30000, 30000)` |
| `ip_classify` | preview | `info := ip_classify("192.0.2.1")` |
| `tcp_bind_probe` | preview | `probe := tcp_bind_probe("127.0.0.1")` |
| `udp_bind` | experimental | `sock := udp_bind("127.0.0.1", 9001)` |
| `udp_send_to` | experimental | `udp_send_to(sock, "hi", "127.0.0.1", 9002)` |
| `udp_receive_from` | experimental | `packet := udp_receive_from(sock, 1024)` |
| `udp_close` | experimental | `udp_close(sock)` |
| `dns_lookup_mx` | preview | `mx := dns_lookup_mx("example.com")` |
| `dns_lookup_txt` | preview | `txt := dns_lookup_txt("_dmarc.example.com")` |
| `dns_lookup_ptr` | preview | `ptr := dns_lookup_ptr("192.0.2.1")` |
| `dns_lookup_tlsa` | preview | `tlsa := dns_lookup_tlsa("_25._tcp.mail.example.com")` |
| `tls_connect` | preview | `tls := tls_connect("mail.example.com", 465)` |
| `tls_upgrade_client` | preview | `tls := tls_upgrade_client(tcp, "mail.example.com")` |
| `tls_acceptor` | preview | `acceptor := tls_acceptor("cert.pem", "key.pem")` |
| `tls_upgrade_server` | preview | `tls := tls_upgrade_server(tcp, acceptor)` |
| `tls_send` | preview | `tls_send(tls, "EHLO sender.example\r\n")` |
| `tls_receive` | preview | `reply := tls_receive(tls, 4096)` |
| `tls_close` | preview | `tls_close(tls)` |
| `tls_info` | preview | `info := tls_info(tls)` |

DNS lookups require `network-client`. Their optional dictionary accepts bounded
`timeout_ms` (250-30000), `attempts` (1-5), `max_records` (1-256), and `dnssec`
(boolean, default `true`). Results use the `kujo.dns.lookup.v1` envelope and
distinguish `OK` from `NO_RECORDS`; resolver failures remain errors. When DNSSEC
checking is enabled, the envelope reports `SECURE`, `INSECURE`, `BOGUS`, or
`INDETERMINATE`. Callers that require DANE must accept only `SECURE` TLSA data.

TLS client verification cannot be disabled. `tls_connect` and
`tls_upgrade_client` use SNI and hostname verification, trust platform roots,
and may add a bounded public CA bundle with `ca_pem`. Both upgrade calls consume
the source `TcpStream`; reuse returns a deterministic closed/upgraded error.
Server private keys must be owner-only on Unix, and TLS info exposes only public
negotiation metadata and SHA-256 fingerprints. TLS 1.0 and 1.1 are rejected.

## Database, Compression, Crypto, and Image

Database, archive, and image examples assume trusted mode. In untrusted mode,
database calls need `--allow-database`; archive writes/extraction and image
conversion need filesystem write permission; and image loading needs
filesystem read permission.

| Function | Tier | Example |
| --- | --- | --- |
| `db_connect` | preview | `db := db_connect("sqlite://local.db")` |
| `db_query` | preview | `rows := db_query(db, "select 1")` |
| `db_execute` | preview | `n := db_execute(db, "delete from t")` |
| `db_close` | preview | `db_close(db)` |
| `db_pool` | experimental | `pool := db_pool(cfg)` |
| `db_pool_acquire` | experimental | `db := db_pool_acquire(pool)` |
| `db_pool_release` | experimental | `db_pool_release(pool, db)` |
| `db_pool_stats` | experimental | `stats := db_pool_stats(pool)` |
| `db_pool_close` | experimental | `db_pool_close(pool)` |
| `db_begin` | preview | `db_begin(db)` |
| `db_commit` | preview | `db_commit(db)` |
| `db_rollback` | preview | `db_rollback(db)` |
| `db_last_insert_id` | preview | `id := db_last_insert_id(db)` |
| `zip_create` | preview | `z := zip_create("out.zip")` |
| `zip_add_file` | preview | `zip_add_file(z, "a.txt")` |
| `zip_add_dir` | preview | `zip_add_dir(z, "assets")` |
| `zip_close` | preview | `zip_close(z)` |
| `unzip` | preview | `unzip("in.zip", "out")` |
| `gzip_decompress` | preview | `plain := gzip_decompress(compressed, 4194304)` |
| `zip_single_file_read` | preview | `entry := zip_single_file_read(archive, 4194304)` |
| `sha256` | stable | `h := sha256("hello")` |
| `hmac_sha256` | stable | `signature := hmac_sha256(secret, payload)` |
| `hmac_sha256_verify` | stable | `valid := hmac_sha256_verify(secret, payload, signature)` |
| `sha256_file` | stable | `h := sha256_file("artifact.bin")` |
| `md5` | stable | `h := md5("hello")` |
| `md5_file` | stable | `h := md5_file("artifact.bin")` |
| `hash_password` | preview | `h := hash_password("secret")` |
| `verify_password` | preview | `ok := verify_password("secret", h)` |
| `aes_encrypt` | experimental | `c := aes_encrypt("msg", key)` |
| `aes_decrypt` | experimental | `p := aes_decrypt(c, key)` |
| `aes_encrypt_bytes` | experimental | `c := aes_encrypt_bytes(blob, key)` |
| `aes_decrypt_bytes` | experimental | `p := aes_decrypt_bytes(c, key)` |
| `aes_encrypt_file_stream` | preview | `receipt := aes_encrypt_file_stream("in.bin", "out.aead", key, 1048576)` |
| `aes_decrypt_file_stream` | preview | `receipt := aes_decrypt_file_stream("out.aead", "restored.bin", key)` |
| `rsa_generate_keypair` | experimental | `kp := rsa_generate_keypair()` |
| `rsa_encrypt` | experimental | `c := rsa_encrypt(data, public_key)` |
| `rsa_decrypt` | experimental | `p := rsa_decrypt(c, private_key)` |
| `rsa_sign` | experimental | `sig := rsa_sign(data, key)` |
| `rsa_verify` | experimental | `ok := rsa_verify(data, sig, key)` |
| `load_image` | preview | `img := load_image("photo.png")` |
| `gif_to_webp` | preview | `out := gif_to_webp("in.gif", "out.webp")` |

Image values expose bounded, constant-allocation pixel access through methods:

| Method | Returns | Notes |
| --- | --- | --- |
| `img.width()` | integer | Pixel width. |
| `img.height()` | integer | Pixel height. |
| `img.format()` | string | Lowercase source filename extension. |
| `img.get_pixel(x, y)` | `[r, g, b, a]` | Coordinates are zero-based integers; channels are `0..255`. |
| `img.set_pixel(x, y, r, g, b)` | boolean | Mutates one pixel and preserves its current alpha channel. |
| `img.set_pixel(x, y, r, g, b, a)` | boolean | Mutates one RGBA pixel; all channels must be integers in `0..255`. |
| `img.save(path)` | boolean | Saves using the output filename extension. |

Pixel methods return runtime errors for invalid types, channels, or coordinates.
They intentionally operate on one pixel at a time and do not allocate a full-image
pixel array. Loading requires filesystem read permission; saving requires filesystem
write permission.

## Dispatch and Coverage Guarantees

This reference is validated by tests to stay aligned with runtime dispatch:

- `tests/interpreter_tests.rs::test_builtin_names_include_release_hardening_contract_entries`
- `tests/interpreter_tests.rs::test_builtin_names_do_not_contain_duplicates`
- `tests/stdlib_reference_contract.rs::stdlib_reference_documents_runtime_builtins`

When adding/removing/renaming builtins, update all of:

- `src/interpreter/mod.rs` builtin registration/dispatch
- this document
- the reference contract tests
