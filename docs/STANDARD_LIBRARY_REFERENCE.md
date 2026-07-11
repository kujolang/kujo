# Standard Library Reference (v1.0.0)

Status: v1.0.0 baseline draft (active)
Last updated: 2026-06-09

This is the canonical native standard library reference for major builtin categories in v1.

Tier definitions:

- `stable`: expected to remain backward-compatible across v1 patch/minor releases
- `preview`: available and documented, but may evolve with additional edge-case hardening
- `experimental`: available for advanced workflows, with higher change risk before post-v1 hardening

v1 contract policy for tiers:

- `stable`: in-scope for v1 compatibility guarantees.
- `preview`: in-scope for v1 usage, but not frozen; behavior may tighten during pre-v1 hardening and must be treated as non-guaranteed until promoted.
- `experimental`: explicitly non-guaranteed for v1 compatibility commitments; available for advanced workflows only and may change or be restricted without stability guarantees.

Canonical readiness boundary: Kujo remains a pre-tag `1.0.0` release candidate until `ROADMAP.md`, `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md`, and tag-time artifact evidence are closed.
Deferred/non-goal policy source: `docs/V1_SCOPE.md`.

Source of truth:

- runtime registration and dispatch are implemented in `src/interpreter/mod.rs`
- builtin name inventory is returned by `Interpreter::get_builtin_names()`

Recently promoted helper surfaces worth calling out:

- `eprint(...)` for stderr-friendly output when you want normal stdout reserved for machine-readable data.
- `bit_and(...)`, `bit_or(...)`, `bit_xor(...)`, `bit_not(...)`, `bit_shl(...)`, and `bit_shr(...)` for masks, flags, and other low-level integer work.
- `type_of(...)` and `is_truthy(...)` for runtime introspection in dynamic scripts.
- `pad_start(...)` and `pad_end(...)` for aligned CLI output and human-readable reports.
- `read_file_lossy(...)`, `path_is_symlink(...)`, and `sha256_file(...)` for practical filesystem workflows that need tolerant reads, symlink checks, and integrity validation.

For wrapper-to-native mappings, see [HELPER_DISCOVERY_COOKBOOK.md](HELPER_DISCOVERY_COOKBOOK.md).

## Core IO and Formatting

| Function | Tier | Example |
| --- | --- | --- |
| `print` | stable | `print("hello")` |
| `input` | preview | `name := input("name: ")` |
| `format` | stable | `line := format("{}-{}", ["a", 1])` |

## Strings and Text

| Function | Tier | Example |
| --- | --- | --- |
| `len` | stable | `size := len("abc")` |
| `substring` | stable | `part := substring("abcdef", 1, 4)` |
| `to_upper` | stable | `v := to_upper("hello")` |
| `to_lower` | stable | `v := to_lower("HELLO")` |
| `trim` | stable | `v := trim("  hi  ")` |
| `escape_xml` | stable | `safe := escape_xml("<tag>")` |
| `render_markdown` | preview | `html := render_markdown("# Title")` |
| `render_listing_card` | preview | `html := render_listing_card(route, title, excerpt, image, terms, label)` |
| `render_layout_native` | preview | `html := render_layout_native(template, settings, route, title, description, navigation, content, meta)` |
| `contains` | stable | `ok := contains("abcdef", "cd")` |
| `replace_str` | stable | `v := replace_str("a-b", "-", "_")` |
| `split` | stable | `parts := split("a,b", ",")` |
| `join` | stable | `text := join(["a", "b"], ",")` |
| `starts_with` | stable | `ok := starts_with("abc", "a")` |
| `ends_with` | stable | `ok := ends_with("abc", "c")` |
| `pad_left` | stable | `v := pad_left("kujo", 6, "0")` |
| `pad_right` | stable | `v := pad_right("kujo", 6, ".")` |
| `pad_start` | stable alias | `v := pad_start("kujo", 6, "0")` |
| `pad_end` | stable alias | `v := pad_end("kujo", 6, ".")` |
| `slugify` | preview | `slug := slugify("Hello World")` |
| `to_camel_case` | preview | `v := to_camel_case("hello_world")` |
| `to_snake_case` | preview | `v := to_snake_case("helloWorld")` |
| `to_kebab_case` | preview | `v := to_kebab_case("helloWorld")` |

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
| `pop` | stable | `last := pop([1, 2, 3])` |
| `insert` | stable | `arr := insert([1, 3], 1, 2)` |
| `remove_at` | stable | `arr := remove_at([1, 2, 3], 1)` |
| `slice` | stable | `part := slice([1, 2, 3, 4], 1, 3)` |
| `concat` | stable | `all := concat([1], [2, 3])` |
| `map` | stable | `out := map([1, 2], func (x) { return x * 2 })` |
| `filter` | stable | `out := filter([1, 2, 3], func (x) { return x > 1 })` |
| `reduce` | stable | `sum := reduce([1, 2, 3], 0, func (a, b) { return a + b })` |
| `sort` | preview | `out := sort([3, 1, 2])` |
| `reverse` | preview | `out := reverse([1, 2, 3])` |
| `chunk` | preview | `out := chunk([1, 2, 3, 4], 2)` |
| `flatten` | preview | `out := flatten([[1], [2, 3]])` |
| `zip` | preview | `out := zip([1, 2], ["a", "b"])` |
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
| `parse_toml` | preview | `cfg := parse_toml("x = 1")` |
| `to_toml` | preview | `txt := to_toml({"x": 1})` |
| `parse_yaml` | preview | `cfg := parse_yaml("x: 1")` |
| `to_yaml` | preview | `txt := to_yaml({"x": 1})` |
| `parse_csv` | preview | `rows := parse_csv("name\nkujo")` |
| `to_csv` | preview | `txt := to_csv([["name"], ["kujo"]])` |

Access semantics:

- Dictionary/map-like values use bracket access (`obj["key"]`).
- Runtime structs (for example `ProcessResult`) use dot fields (`result.exitcode`).

## Math, Time, and Random

| Function | Tier | Example |
| --- | --- | --- |
| `abs` | stable | `v := abs(-1)` |
| `sqrt` | stable | `v := sqrt(9)` |
| `pow` | stable | `v := pow(2, 8)` |
| `min` | stable | `v := min(1, 2)` |
| `max` | stable | `v := max(1, 2)` |
| `random` | preview | `v := random()` |
| `random_int` | preview | `v := random_int(1, 10)` |
| `set_random_seed` | preview | `set_random_seed(42)` |
| `now` | stable | `t := now()` |
| `now_utc` | stable | `iso := now_utc()` |
| `now_unix` | stable | `seconds := now_unix()` |
| `now_utc_seconds` | stable alias | `seconds := now_utc_seconds()` |
| `current_timestamp` | stable | `ts := current_timestamp()` |
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
| `write_file` | stable | `write_file("notes.txt", "hello")` |
| `append_file` | stable | `append_file("notes.txt", "more")` |
| `file_exists` | stable | `ok := file_exists("notes.txt")` |
| `list_dir` | stable | `entries := list_dir(".")` |
| `create_dir` | stable | `create_dir("tmp")` |
| `delete_file` | stable | `delete_file("old.txt")` |
| `rename_file` | stable | `rename_file("a.txt", "b.txt")` |
| `copy_file` | preview | `copy_file("a.txt", "b.txt")` |
| `join_path` | stable | `p := join_path("a", "b")` |
| `dirname` | stable | `d := dirname("a/b.txt")` |
| `basename` | stable | `b := basename("a/b.txt")` |
| `path_absolute` | preview | `p := path_absolute(".")` |
| `path_is_dir` | stable | `ok := path_is_dir(".")` |
| `path_is_file` | stable | `ok := path_is_file("a.txt")` |

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
| `env_int` | stable | `port := env_int("PORT")` |
| `env_float` | stable | `timeout := env_float("TIMEOUT")` |
| `env_bool` | stable | `enabled := env_bool("ENABLED")` |
| `env_required` | stable | `token := env_required("TOKEN")` |
| `args` | stable | `argv := args()  # user args only; script name excluded` |
| `sleep` | stable | `sleep(100)` |
| `execute` | preview | `out := execute("echo hi", {"timeout_ms": 1000})` |
| `execute_status` | preview | `r := execute_status("echo hi")` |
| `spawn_process` | experimental | `r := spawn_process(["echo", "hi"], {"max_output_bytes": 4096})` |
| `pipe_commands` | experimental | `out := pipe_commands([["echo", "hi"], ["cat"]], {"timeout_ms": 1000})` |
| `channel` | preview | `ch := channel()` |
| `shared_set` | preview | `shared_set("count", 1)` |
| `shared_get` | preview | `v := shared_get("count")` |
| `shared_add_int` | preview | `shared_add_int("count", 1)` |
| `parallel_map` | preview | `out := parallel_map([1,2], func (x) { return x + 1 })` |
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

- `execute_status` and `spawn_process` return `ProcessResult` fields: `exitcode`, `stdout`, `stderr`, `success`, `timed_out`, `stdout_truncated`, `stderr_truncated`
- `execute` returns a stdout string on success and raises a deterministic error object on timeout, output-limit overflow, or non-zero exit

CLI/Process semantics notes:

- `args()` returns only user-provided arguments after the script path. Example: `kujo run tool.kujo -- summarize --format json` becomes `args() == ["summarize", "--format", "json"]`.
- `execute(...)` accepts a single shell command string (not an argv array).
- Use `execute_status(...)` when you need exit code and stderr without exception-style control flow.

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
| `http_get` | preview | `res := http_get("https://example.com")` |
| `http_post` | preview | `res := http_post("https://example.com", {"x":1})` |
| `http_get_binary` | preview | `blob := http_get_binary("https://example.com/a.bin")` |
| `parallel_http` | preview | `all := parallel_http(["https://a", "https://b"])` |
| `http_server` | experimental | `srv := http_server(8080)` |
| `http_response` | preview | `res := http_response(200, "ok")` |
| `json_response` | preview | `res := json_response({"ok": true})` |
| `jwt_encode` | preview | `tok := jwt_encode({"sub":"user"}, "secret")` |
| `jwt_decode` | preview | `payload := jwt_decode(tok, "secret")` |
| `oauth2_auth_url` | preview | `url := oauth2_auth_url(cfg)` |
| `oauth2_get_token` | preview | `tok := oauth2_get_token(cfg)` |

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
| `db_pool` | experimental | `pool := db_pool(cfg)` |
| `zip_create` | preview | `z := zip_create("out.zip")` |
| `zip_add_file` | preview | `zip_add_file(z, "a.txt")` |
| `unzip` | preview | `unzip("in.zip", "out")` |
| `sha256` | stable | `h := sha256("hello")` |
| `md5` | stable | `h := md5("hello")` |
| `hash_password` | preview | `h := hash_password("secret")` |
| `verify_password` | preview | `ok := verify_password("secret", h)` |
| `aes_encrypt` | experimental | `c := aes_encrypt("msg", key)` |
| `aes_decrypt` | experimental | `p := aes_decrypt(c, key)` |
| `rsa_generate_keypair` | experimental | `kp := rsa_generate_keypair()` |
| `rsa_sign` | experimental | `sig := rsa_sign(data, key)` |
| `load_image` | preview | `img := load_image("photo.png")` |
| `gif_to_webp` | preview | `out := gif_to_webp("in.gif", "out.webp")` |

## Dispatch and Coverage Guarantees

This reference is validated by tests to stay aligned with runtime dispatch:

- `tests/interpreter_tests.rs::test_builtin_names_include_release_hardening_contract_entries`
- `tests/interpreter_tests.rs::test_builtin_names_do_not_contain_duplicates`
- `tests/stdlib_reference_contract.rs::stdlib_reference_documents_runtime_builtins`

When adding/removing/renaming builtins, update all of:

- `src/interpreter/mod.rs` builtin registration/dispatch
- this document
- the reference contract tests
