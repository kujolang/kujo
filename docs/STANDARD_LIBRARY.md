# Standard Library Inventory

Status: stable v1.0.0 inventory
Last updated: 2026-09-02

This inventory is the canonical support table for runtime-native functions registered by `Interpreter::get_builtin_names()` in `src/interpreter/mod.rs`.

Arity key:

- `exact N`: strict arity, exactly N arguments
- `A..=B`: inclusive range arity
- `variadic (N+)`: at least N arguments
- `handler-defined`: arity/type checks are currently enforced in the native handler implementation instead of centralized `native_callable_arity` metadata

Capability key:

- `none`: no capability gate
- other values map to `NativeCapability::as_str()` and require explicit allow flags in restricted mode

JSON conversion contract (`parse_json` / `to_json` / `to_json_pretty`):

- `parse_json` enforces a maximum input size of `8,388,608` bytes, aligned with file I/O, and a maximum nesting depth of `64`.
- Invalid JSON returns a `Value::Error` message including parse-location details from `serde_json`.
- `to_json` and `to_json_pretty` reject non-finite floats (`NaN`, `+/-inf`) with a `Value::Error` instead of silently coercing values.
- `to_json` and `to_json_pretty` accept JSON-compatible scalar/container values, including `secret` values which serialize as `"***"`; runtime structs, functions, bytes, and other unsupported values return `Value::Error`.
- Dictionary-like values are serialized with deterministic key ordering (lexicographic for string-key dictionaries, ascending for integer-key dictionaries, and declaration/index order for fixed and dense dictionaries).
- `to_json_pretty` uses the same ordering and conversion rules as `to_json`, adding only human-readable whitespace. `parse_json` accepts any JSON root value, caps input at `8,388,608` bytes and nesting at `64`, and includes parser-location details in invalid-input errors.

Bounded XML parsing contract (`parse_xml_bounded`):

- `parse_xml_bounded(xml, options)` parses an in-memory UTF-8 XML 1.0 document into a deterministic namespace-aware tree without filesystem, network, process, clock, or random access.
- The returned dictionary has `ok`, `code`, `schema`, `node_count`, `attribute_count`, `text_bytes`, `tree_bytes`, and `root`. Every element and non-namespace attribute exposes `name`, `qualified_name`, and `namespace`; elements also expose direct `text`, `attributes`, and `children` arrays in document order.
- `options` is required and accepts only `max_input_bytes`, `max_depth`, `max_nodes`, `max_attributes`, `max_text_bytes`, and `max_tree_bytes`. Each positive integer may lower, but never raise, the absolute ceilings of 8 MiB input, depth 64, 100,000 elements, 200,000 attributes (including namespace declarations), 8 MiB decoded text, and 16 MiB copied tree-string data. The tree-string ceiling prevents namespace/name amplification from turning a bounded input into unbounded owned output.
- XML declarations must be the first document event, identify XML 1.0 and, when present, UTF-8, and use only `yes` or `no` for `standalone`. Unknown prefixes, duplicate expanded attribute names, malformed or multiple roots, text outside the root, and limit violations fail deterministically. DTDs and non-predefined entity references are denied; XML character references and the five predefined entities are bounded and decoded; comments and processing instructions are discarded; bounded CDATA is preserved as direct text.

JSON Schema subset contract (`json_schema_validate`):

- `json_schema_validate(value, schema)` returns `{"valid": bool, "errors": [...]}` and never performs network, filesystem, clock, random, or process I/O.
- Supported validation keywords are `type`, `required`, `properties`, `additionalProperties`, `items`, `enum`, `const`, `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `minLength`, `maxLength`, `pattern`, `minItems`, `maxItems`, `anyOf`, `oneOf`, `allOf`, and local `$ref`.
- `$ref` supports local JSON pointers such as `#`, `#/$defs/name`, and `#/definitions/name`; remote references are rejected.
- Error entries are dictionaries with `path`, `message`, and `keyword`. Paths are JSON-pointer-like instance paths such as `/items/0/name`; the root path is an empty string.
- Unsupported schema keywords, malformed schemas, invalid regex patterns, remote or cyclic `$ref`, excessive schema recursion, excessive validation nodes, patterns larger than `1,024` bytes, and arrays larger than `100,000` items return `Value::Error`.
- Draft 2020-12 identification and annotation keywords `$schema`, `$id`, `$comment`, `title`, `description`, `default`, `examples`, `format`, `deprecated`, `readOnly`, and `writeOnly` are accepted as no-op metadata. `format` follows the draft's annotation-default behavior; it is not an assertion vocabulary.

Process result contract (`spawn_process` / `execute_status`):

- Both return a `ProcessResult` runtime struct accessed with dot fields:
  `exitcode`, `stdout`, `stderr`, `success`, `timed_out`, `cancelled`,
  `stdout_truncated`, and `stderr_truncated`.
- `spawn_process` uses an explicit argv array and does not invoke a shell. Its
  options are `timeout_ms`, `max_output_bytes`, `inherit_env`, `env_allow`,
  `env_deny`, `env`, `stdin`, `stream_channel`, `stream_stdout_path`,
  `stream_stderr_path`, `redact_values`, `cancel_file`, and optional `cwd`.
  The timeout
  defaults to 30,000 ms; output is capped at 1 MiB per stream by default and
  16 MiB maximum per stream. Stream sinks are bounded and redact exact byte
  sequences incrementally across chunks; a full stream channel applies
  backpressure. `cancel_file` is a portable cancellation hook, while `cwd`
  selects the child process working directory. SIGINT
  and SIGTERM cancel the current process execution and set `cancelled`.
  On Unix, each spawned command runs in its own process group so timeout and
  cancellation terminate descendant processes that inherited its output pipes.
- `execute_status` uses a shell command string and is separately gated as
  `shell-exec`; `spawn_process` is gated as `process-exec`. `execute` returns
  only stdout on a successful, non-truncated exit and otherwise returns a
  runtime error.
- Captured output is decoded lossily as UTF-8. A timeout or cancellation forces
  `success` to `false`; truncation flags must be checked before treating output
  as complete.
- `stdin` accepts a string or bytes value up to the runtime's 8 MiB file-I/O
  ceiling. The runtime stages it in a private, auto-deleting temporary file,
  reopens that file read-only, and unlinks it before spawning the child. This
  prevents writable-stdin disk growth and pipe-writer lifecycle deadlocks.
- `ProcessResult` is a runtime struct, not a JSON value. To serialize a receipt,
  copy selected fields into a dictionary and pass that dictionary to `to_json`.

Vector math contract (`vec_dot` / `vec_norm` / `vec_normalize` / `vec_cosine` / `vec_top_k`):

- Inputs are Kujo arrays of finite numbers; integers are promoted to floats.
- `vec_dot(a, b)`, `vec_cosine(a, b)`, and `vec_top_k(query, matrix, k)` require equal dimensions.
- `vec_cosine` returns `0.0` for zero vectors and clamps finite cosine scores to `[-1.0, 1.0]`.
- `vec_normalize` returns a zero-filled vector for a zero vector.
- `vec_top_k` scores rows by cosine similarity, returns dictionaries `{index, score}`, sorts by descending score with stable ascending-index tie-breaks, and returns all rows when `k` exceeds row count.
- Vectors are capped at `100,000` dimensions; matrices are capped at `100,000` rows and `5,000,000` cells. Non-finite inputs or non-finite results return `Value::Error`.
- `vec_top_k` uses Rayon parallel iteration for large matrices; vector helpers have no capability gate and do not store or index vectors.

Token estimation contract (`ai_count_tokens` / `ai_fit_context`):

- `ai_count_tokens(text_or_messages, options?)` returns a deterministic estimate, not exact provider tokenization.
- `options.model` selects a small heuristic family by model prefix: `gpt*`, `text-embedding*`, or default. All current families estimate one token per four weighted characters; non-ASCII characters count as two weighted characters. Chat-message estimates also include documented role/content overhead.
- Message arrays contain dictionaries with non-empty string `role` and string `content`; optional string `name` is counted when present.
- `ai_fit_context(messages, max_tokens, options?)` drops the oldest non-system messages until the estimated count fits. It never drops system messages and preserves the last user message. If the minimum preserved context is still over budget, it returns it with `fits: false`.
- Text inputs are capped at `2,000,000` characters and message arrays at `100,000` messages. These helpers have no capability gate and perform no I/O.

AI message builder contract (`ai_text` / `ai_image_url` / `ai_message`):

- `ai_text(content)` builds a text content block.
- `ai_image_url(url, detail?)` builds an image URL content block with optional provider detail.
- `ai_message(role, content_or_blocks)` builds a chat message from a string or content block array. These helpers are pure, capability-free, and produce shapes accepted by the AI HTTP helpers.

Secret redaction contract (`secret` / `reveal` / `is_secret`):

- `secret(value)` wraps a string in a redacted runtime value. Printing, debug formatting, JSON/TOML/YAML/CSV serialization, errors, and AI cassette request metadata render secrets as `***` or `Secret(***)`.
- Secrets compare by their inner string value and clone like ordinary runtime values, but `reveal(secret_value)` is the only documented builtin that unwraps plaintext.
- `options.api_key` for AI helpers accepts either a plain string or a secret; cassettes and error body excerpts redact configured keys and sensitive authorization headers.

| Function | Signature | Arity | Return Type | Errors | Capability | Example |
| --- | --- | --- | --- | --- | --- | --- |
| `print` | `print(...)` | variadic (0+) | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := print(...)` |
| `eprint` | `eprint(...)` | variadic (0+) | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := eprint(...)` |
| `println` | `println(...)` | variadic (0+) | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := println(...)` |
| `__vm_for_iterable` | `__vm_for_iterable(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := __vm_for_iterable(...)` |
| `abs` | `abs(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := abs(...)` |
| `sqrt` | `sqrt(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := sqrt(...)` |
| `pow` | `pow(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := pow(...)` |
| `floor` | `floor(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := floor(...)` |
| `ceil` | `ceil(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := ceil(...)` |
| `round` | `round(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := round(...)` |
| `min` | `min(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := min(...)` |
| `max` | `max(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := max(...)` |
| `sin` | `sin(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := sin(...)` |
| `cos` | `cos(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := cos(...)` |
| `tan` | `tan(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := tan(...)` |
| `log` | `log(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := log(...)` |
| `exp` | `exp(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := exp(...)` |
| `bit_and` | `bit_and(left, right)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_and(...)` |
| `bit_or` | `bit_or(left, right)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_or(...)` |
| `bit_xor` | `bit_xor(left, right)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_xor(...)` |
| `bit_not` | `bit_not(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_not(...)` |
| `bit_shl` | `bit_shl(left, right)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_shl(...)` |
| `bit_shr` | `bit_shr(left, right)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bit_shr(...)` |
| `vec_dot` | `vec_dot(a, b)` | exact 2 | float | Value::Error on invalid args/types, non-finite inputs/results, vector dimension mismatch, or resource-limit violations. | `none` | `score := vec_dot([1, 2], [3, 4])` |
| `vec_norm` | `vec_norm(a)` | exact 1 | float | Value::Error on invalid args/types, non-finite inputs/results, or resource-limit violations. | `none` | `length := vec_norm([3, 4])` |
| `vec_normalize` | `vec_normalize(a)` | exact 1 | array | Value::Error on invalid args/types, non-finite inputs/results, or resource-limit violations; zero vector returns zero-filled vector. | `none` | `unit := vec_normalize([3, 4])` |
| `vec_cosine` | `vec_cosine(a, b)` | exact 2 | float | Value::Error on invalid args/types, non-finite inputs/results, vector dimension mismatch, or resource-limit violations; zero vectors return `0.0`. | `none` | `score := vec_cosine([1, 0], [0, 1])` |
| `vec_top_k` | `vec_top_k(query, matrix, k)` | exact 3 | array | Value::Error on invalid args/types, non-finite inputs/results, matrix row dimension mismatch, negative `k`, or resource-limit violations. | `none` | `matches := vec_top_k([1, 0], [[1, 0], [0, 1]], 1)` |
| `ai_count_tokens` | `ai_count_tokens(text_or_messages, options?)` | 1..=2 | int | Value::Error on invalid args/types, invalid options, malformed messages, or resource-limit violations. | `none` | `tokens := ai_count_tokens("Hello Kujo", {"model":"gpt-4o"})` |
| `ai_fit_context` | `ai_fit_context(messages, max_tokens, options?)` | 2..=3 | dict | Value::Error on invalid args/types, invalid options, malformed messages, negative `max_tokens`, or resource-limit violations. | `none` | `fit := ai_fit_context(messages, 2000, {"model":"gpt-4o"})` |
| `ai_text` | `ai_text(content)` | exact 1 | dict | Value::Error on invalid args/types. | `none` | `block := ai_text("Describe this image")` |
| `ai_image_url` | `ai_image_url(url, detail?)` | 1..=2 | dict | Value::Error on invalid args/types or empty URL. | `none` | `block := ai_image_url("https://example.test/image.png", "low")` |
| `ai_message` | `ai_message(role, content_or_blocks)` | exact 2 | dict | Value::Error on invalid role, content type, or malformed content block arrays. | `none` | `message := ai_message("user", [ai_text("Hi")])` |
| `len` | `len(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := len(...)` |
| `substring` | `substring(value, start, end)` | exact 3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := substring(...)` |
| `substr` | `substr(value, start, end)` | exact 3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := substr(...)` |
| `to_upper` | `to_upper(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_upper(...)` |
| `upper` | `upper(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := upper(...)` |
| `to_lower` | `to_lower(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_lower(...)` |
| `lower` | `lower(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := lower(...)` |
| `capitalize` | `capitalize(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := capitalize(...)` |
| `escape_xml` | `escape_xml(...)` | handler-defined | dynamic (Value) | Value::Error on missing argument or invalid args/types/operation; capability-denied when gated. | `none` | `result := escape_xml(...)` |
| `render_markdown` | `render_markdown(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := render_markdown(...)` |
| `render_listing_card` | `render_listing_card(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := render_listing_card(...)` |
| `render_layout_native` | `render_layout_native(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := render_layout_native(...)` |
| `trim` | `trim(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := trim(...)` |
| `trim_start` | `trim_start(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := trim_start(...)` |
| `trim_end` | `trim_end(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := trim_end(...)` |
| `contains` | `contains(value, needle)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := contains(...)` |
| `replace_str` | `replace_str(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := replace_str(...)` |
| `replace` | `replace(value, from, to)` | exact 3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := replace(...)` |
| `split` | `split(value, delimiter)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := split(...)` |
| `join` | `join(values, separator)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := join(...)` |
| `ssg_render_pages` | `ssg_render_pages(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := ssg_render_pages(...)` |
| `ssg_build_output_paths` | `ssg_build_output_paths(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation or when the requested count exceeds the generated-sequence limit; capability-denied when gated. | `none` | `result := ssg_build_output_paths(...)` |
| `ssg_render_and_write_pages` | `ssg_render_and_write_pages(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := ssg_render_and_write_pages(...)` |
| `ssg_read_render_and_write_pages` | `ssg_read_render_and_write_pages(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := ssg_read_render_and_write_pages(...)` |
| `starts_with` | `starts_with(value, prefix)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := starts_with(...)` |
| `ends_with` | `ends_with(value, suffix)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := ends_with(...)` |
| `pad_left` | `pad_left(value: string, width: number, pad_char: string) -> string` | exact 3 | string | Value::Error on non-string arguments, negative/non-finite width, or generated-output limit. Empty `pad_char` uses a space; only its first character is used. | `none` | `result := pad_left("kujo", 6, "0")` |
| `pad_right` | `pad_right(value: string, width: number, pad_char: string) -> string` | exact 3 | string | Same contract as `pad_left`; padding counts Unicode characters. | `none` | `result := pad_right("kujo", 6, ".")` |
| `pad_start` | `pad_start(value: string, width: number, pad_char: string) -> string` | exact 3 | string | Alias of `pad_left`; same errors and Unicode-character width. | `none` | `result := pad_start("kujo", 6, "0")` |
| `pad_end` | `pad_end(value: string, width: number, pad_char: string) -> string` | exact 3 | string | Alias of `pad_right`; same errors and Unicode-character width. | `none` | `result := pad_end("kujo", 6, ".")` |
| `lines` | `lines(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := lines(...)` |
| `words` | `words(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := words(...)` |
| `str_reverse` | `str_reverse(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := str_reverse(...)` |
| `slugify` | `slugify(value: string) -> string` | handler-defined | string | Value::Error when the argument is not a string. Lowercases, keeps Unicode alphanumerics, maps spaces/underscores to hyphens, removes other punctuation, and trims edge hyphens. | `none` | `result := slugify("Release Candidate 1")` |
| `truncate` | `truncate(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := truncate(...)` |
| `to_camel_case` | `to_camel_case(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_camel_case(...)` |
| `to_snake_case` | `to_snake_case(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_snake_case(...)` |
| `to_kebab_case` | `to_kebab_case(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_kebab_case(...)` |
| `index_of` | `index_of(value, needle)` | exact 2 | dynamic (Value) | Searches strings, arrays, or raw bytes; returns the zero-based index or `-1`. Both operands must be bytes for byte search. Value::Error on invalid args/types/operation. | `none` | `result := index_of(...)` |
| `bytes_is_ascii` | `bytes_is_ascii(value)` | exact 1 | bytes | Returns true only when every octet is in the inclusive 0–127 range. | `none` | `safe := bytes_is_ascii(payload)` |
| `repeat` | `repeat(value, count)` | exact 2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := repeat(...)` |
| `char_at` | `char_at(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := char_at(...)` |
| `is_empty` | `is_empty(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_empty(...)` |
| `count_chars` | `count_chars(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := count_chars(...)` |
| `push` | `push(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := push(...)` |
| `append` | `append(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := append(...)` |
| `pop` | `pop(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := pop(...)` |
| `insert` | `insert(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := insert(...)` |
| `remove` | `remove(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := remove(...)` |
| `remove_at` | `remove_at(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := remove_at(...)` |
| `clear` | `clear(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := clear(...)` |
| `slice` | `slice(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := slice(...)` |
| `concat` | `concat(left, right)` | exact 2 | array or bytes | Concatenates two arrays or two raw-byte values of the same kind; mixed or invalid inputs return `Value::Error`. | `none` | `payload := concat(bytes([0]), bytes([255]))` |
| `map` | `map(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := map(...)` |
| `filter` | `filter(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := filter(...)` |
| `reduce` | `reduce(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := reduce(...)` |
| `find` | `find(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := find(...)` |
| `sort` | `sort(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := sort(...)` |
| `reverse` | `reverse(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := reverse(...)` |
| `unique` | `unique(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := unique(...)` |
| `sum` | `sum(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := sum(...)` |
| `any` | `any(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := any(...)` |
| `all` | `all(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := all(...)` |
| `chunk` | `chunk(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := chunk(...)` |
| `flatten` | `flatten(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := flatten(...)` |
| `zip` | `zip(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := zip(...)` |
| `enumerate` | `enumerate(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := enumerate(...)` |
| `take` | `take(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := take(...)` |
| `skip` | `skip(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := skip(...)` |
| `windows` | `windows(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := windows(...)` |
| `range` | `range(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := range(...)` |
| `format` | `format(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := format(...)` |
| `keys` | `keys(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := keys(...)` |
| `values` | `values(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := values(...)` |
| `items` | `items(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := items(...)` |
| `has_key` | `has_key(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := has_key(...)` |
| `get` | `get(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := get(...)` |
| `merge` | `merge(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := merge(...)` |
| `invert` | `invert(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := invert(...)` |
| `update` | `update(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := update(...)` |
| `get_default` | `get_default(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := get_default(...)` |
| `input` | `input(prompt?)` | 0..=1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := input(...)` |
| `parse_int` | `parse_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parse_int(...)` |
| `parse_float` | `parse_float(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parse_float(...)` |
| `to_int` | `to_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_int(...)` |
| `to_float` | `to_float(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_float(...)` |
| `to_string` | `to_string(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_string(...)` |
| `str` | `str(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := str(...)` |
| `secret` | `secret(value)` | exact 1 | secret | Value::Error when `value` is not a string. | `none` | `api_key := secret("sk-local")` |
| `reveal` | `reveal(secret)` | exact 1 | string | Value::Error when the argument is not a secret. | `none` | `plain := reveal(api_key)` |
| `to_bool` | `to_bool(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_bool(...)` |
| `bytes` | `bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := bytes(...)` |
| `dict` | `dict()` | exact 0 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := dict(...)` |
| `array` | `array(...)` | variadic (0+) | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := array(...)` |
| `error` | `error(message)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := error(...)` |
| `type` | `type(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := type(...)` |
| `type_of` | `type_of(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := type_of(...)` |
| `is_truthy` | `is_truthy(value)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_truthy(...)` |
| `is_int` | `is_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_int(...)` |
| `is_float` | `is_float(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_float(...)` |
| `is_string` | `is_string(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_string(...)` |
| `is_secret` | `is_secret(value)` | exact 1 | bool | Value::Error on invalid arity. | `none` | `is_key := is_secret(api_key)` |
| `is_bool` | `is_bool(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_bool(...)` |
| `is_array` | `is_array(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_array(...)` |
| `is_dict` | `is_dict(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_dict(...)` |
| `is_null` | `is_null(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_null(...)` |
| `is_function` | `is_function(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := is_function(...)` |
| `assert` | `assert(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := assert(...)` |
| `debug` | `debug(...)` | variadic (0+) | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := debug(...)` |
| `read_file` | `read_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := read_file(...)` |
| `read_file_lossy` | `read_file_lossy(path)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := read_file_lossy(...)` |
| `read_file_beneath` | `read_file_beneath(root, relative_path, max_bytes)` | exact 3 | string | Rejects invalid relative paths, symlink/reparse traversal, non-regular files, oversized reads, and invalid UTF-8; capability-denied when gated. | `filesystem-read` | `text := read_file_beneath("trusted", "docs/note.txt", 1048576)` |
| `read_binary_file_beneath` | `read_binary_file_beneath(root, relative_path, max_bytes)` | exact 3 | bytes | Rejects invalid relative paths, symlink/reparse traversal, non-regular files, and oversized reads; capability-denied when gated. | `filesystem-read` | `blob := read_binary_file_beneath("trusted", "docs/file.pdf", 5000000)` |
| `write_file` | `write_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := write_file(...)` |
| `write_file_atomic` | `write_file_atomic(path, content_or_bytes, overwrite?)` | 2..=3 | bool | Value::Error on invalid args/types/operation, size-limit violations, or atomic finalization failures; defaults to no-overwrite. | `filesystem-write` | `ok := write_file_atomic("state.json", payload, true)` |
| `append_file` | `append_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := append_file(...)` |
| `file_exists` | `file_exists(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := file_exists(...)` |
| `read_lines` | `read_lines(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := read_lines(...)` |
| `jsonl_query` | `jsonl_query(path, options)` | exact 2 | array | Streams JSONL with bounded `max_rows`, optional dotted-field equality filtering, and optional constant-memory nested joins. Join options `join_path`, `left_field`, and `right_field` must be supplied together. Lines are capped at 1 MiB and invalid records fail closed. | `filesystem-read` | `rows := jsonl_query("evidence.jsonl", {"filter_field": "provider", "filter_equals": "crux", "max_rows": 100})` |
| `list_dir` | `list_dir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := list_dir(...)` |
| `create_dir` | `create_dir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := create_dir(...)` |
| `file_size` | `file_size(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := file_size(...)` |
| `delete_file` | `delete_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-delete` | `result := delete_file(...)` |
| `rename_file` | `rename_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := rename_file(...)` |
| `publish_file_noreplace` | `publish_file_noreplace(source_path, destination_path)` | exact 2 | dictionary receipt | Atomically hard-links a regular single-link source at an absent same-filesystem destination, removes the source, syncs affected directories, and verifies retained identity. It never replaces an existing destination. A returned receipt always has `published=true`; callers must require `verified=true` and reconcile any false durability fact. | `filesystem-write`, `filesystem-delete` | `receipt := publish_file_noreplace("private-spool/body", "blobs/message.body")` |
| `copy_file` | `copy_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := copy_file(...)` |
| `read_binary_file` | `read_binary_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := read_binary_file(...)` |
| `write_binary_file` | `write_binary_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := write_binary_file(...)` |
| `io_read_bytes` | `io_read_bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := io_read_bytes(...)` |
| `io_write_bytes` | `io_write_bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := io_write_bytes(...)` |
| `io_append_bytes` | `io_append_bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := io_append_bytes(...)` |
| `io_read_at` | `io_read_at(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := io_read_at(...)` |
| `io_write_at` | `io_write_at(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := io_write_at(...)` |
| `io_seek_read` | `io_seek_read(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := io_seek_read(...)` |
| `io_file_metadata` | `io_file_metadata(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := io_file_metadata(...)` |
| `io_set_permissions` | `io_set_permissions(path, mode)` | exact 2 | dictionary | Fails closed off Unix; rejects special bits; returns the requested and handle-verified actual POSIX mode. | `filesystem-write` | `receipt := io_set_permissions("private.pem", 384)` |
| `io_write_private_file` | `io_write_private_file(path, content, mode)` | exact 3 | dictionary | Creates a same-directory temporary file with the restrictive mode at creation, verifies the same handle, syncs, and atomically publishes without overwrite. | `filesystem-write` | `receipt := io_write_private_file("private.pem", pem, 384)` |
| `io_private_spool_open` | `io_private_spool_open(path, max_bytes, mode)` | exact 3 | private spool | Opens a single-use same-directory private spool, creates its temporary file with the requested restrictive mode, rejects group/other-writable parent directories, and enforces a 1 GiB absolute maximum. The destination remains absent until finish. | `filesystem-write` | `spool := io_private_spool_open("message.eml", 26214400, 384)` |
| `io_private_spool_write` | `io_private_spool_write(spool, content)` | exact 2 | dictionary | Appends string or bytes through the retained file handle, rejects the write before crossing `max_bytes`, and returns written/remaining byte counts. | `filesystem-write` | `progress := io_private_spool_write(spool, chunk)` |
| `io_private_spool_write_file_range` | `io_private_spool_write_file_range(spool, path, offset, count, mode)` | exact 5 | dictionary | Streams an exact range from a non-symlink regular file through a fixed 64 KiB buffer into a private spool. Modes are `raw`, newline-normalizing `crlf`, MIME-safe `base64-crlf` with 76-column lines, and newline-normalizing `smtp-dot-stuff-crlf`, which also guarantees the DATA content ends in CRLF. Input is capped at 64 MiB per call and output remains subject to the spool's total bound. | `filesystem-write`, `filesystem-read` | `progress := io_private_spool_write_file_range(spool, "body.bin", 0, size, "base64-crlf")` |
| `io_private_spool_finish` | `io_private_spool_finish(spool)` | exact 1 | dictionary | Syncs and verifies the retained handle, publishes without overwriting an existing destination, and returns path, mode, byte count, incremental SHA-256, and explicit `published`, `temporary_removed`, `directory_synced`, and aggregate `verified` facts. Once `published` is true, post-publication cleanup/durability failure is reported in the receipt rather than as an ambiguous error. The handle is then closed. | `filesystem-write` | `receipt := io_private_spool_finish(spool)` |
| `io_private_spool_abort` | `io_private_spool_abort(spool)` | exact 1 | bool | Closes and removes an unpublished spool. Dropping the last unfinished handle also removes its temporary file during normal shutdown. | `filesystem-write` | `removed := io_private_spool_abort(spool)` |
| `io_truncate` | `io_truncate(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := io_truncate(...)` |
| `io_copy_range` | `io_copy_range(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := io_copy_range(...)` |
| `parse_json` | `parse_json(json_string)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation, oversized input (>8,388,608 bytes), excessive nesting (>64), invalid JSON parse, or capability-denied when gated. | `none` | `result := parse_json("{\"ok\":true}")` |
| `parse_xml_bounded` | `parse_xml_bounded(xml, options)` | exact 2 | dictionary | Value::Error on invalid args/types/options, malformed or unsupported XML, denied DTD/non-predefined entity content, namespace errors, duplicate expanded attributes, or resource-limit violations. | `none` | `result := parse_xml_bounded("<root/>", {})` |
| `to_json` | `to_json(value)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation, unsupported value conversion, non-finite float serialization, or capability-denied when gated. | `none` | `result := to_json({"ok": true})` |
| `to_json_pretty` | `to_json_pretty(value)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation, unsupported value conversion, non-finite float serialization, or capability-denied when gated. | `none` | `result := to_json_pretty({"ok": true})` |
| `json_schema_validate` | `json_schema_validate(value, schema)` | exact 2 | dict | Value::Error on invalid args/types, malformed or unsupported schema keywords, invalid regex patterns, unsupported `$ref`, or resource-limit violations. Integer bounds are compared without converting them through floating point. Otherwise returns `{valid, errors}` with JSON-pointer-like paths. | `none` | `result := json_schema_validate({"name":"Kujo"}, {"type":"object","required":["name"]})` |
| `parse_toml` | `parse_toml(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parse_toml(...)` |
| `to_toml` | `to_toml(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_toml(...)` |
| `parse_yaml` | `parse_yaml(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parse_yaml(...)` |
| `to_yaml` | `to_yaml(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_yaml(...)` |
| `parse_csv` | `parse_csv(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parse_csv(...)` |
| `to_csv` | `to_csv(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := to_csv(...)` |
| `encode_base64` | `encode_base64(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := encode_base64(...)` |
| `decode_base64` | `decode_base64(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := decode_base64(...)` |
| `decode_base64_utf8` | `decode_base64_utf8(text)` | exact 1 | string | Error on malformed base64 or invalid UTF-8. | `none` | `text := decode_base64_utf8(encoded)` |
| `encode_uri_component` | `encode_uri_component(...)` | handler-defined | string | Value::Error on invalid args/types. | `none` | `part := encode_uri_component("café & tea")` |
| `random` | `random(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := random(...)` |
| `random_int` | `random_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := random_int(...)` |
| `random_choice` | `random_choice(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := random_choice(...)` |
| `uuid_v4` | `uuid_v4(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := uuid_v4(...)` |
| `random_id` | `random_id(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := random_id(...)` |
| `set_random_seed` | `set_random_seed(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := set_random_seed(...)` |
| `clear_random_seed` | `clear_random_seed(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `random` | `result := clear_random_seed(...)` |
| `now` | `now() -> float` | handler-defined | float | Capability-denied when clock access is restricted. Unix seconds, fractional. | `clock` | `result := now()` |
| `now_utc` | `now_utc() -> string` | handler-defined | string | Capability-denied when clock access is restricted. ISO-8601 UTC in `YYYY-MM-DDTHH:mm:ssZ` form. | `clock` | `result := now_utc()` |
| `now_unix` | `now_unix() -> int` | handler-defined | int | Capability-denied when clock access is restricted. Unix seconds. | `clock` | `result := now_unix()` |
| `now_utc_seconds` | `now_utc_seconds() -> int` | handler-defined | int | Alias of `now_unix`; same capability and units. | `clock` | `result := now_utc_seconds()` |
| `current_timestamp` | `current_timestamp() -> int` | handler-defined | int | Capability-denied when clock access is restricted. Unix milliseconds. | `clock` | `result := current_timestamp()` |
| `time` | `time(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := time(...)` |
| `performance_now` | `performance_now(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := performance_now(...)` |
| `time_us` | `time_us(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := time_us(...)` |
| `time_ns` | `time_ns(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := time_ns(...)` |
| `format_duration` | `format_duration(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := format_duration(...)` |
| `elapsed` | `elapsed(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := elapsed(...)` |
| `format_date` | `format_date(timestamp: number, format: string) -> string` | handler-defined | string | Value::Error on wrong arity/types; invalid or out-of-range timestamps return an error string. Input is Unix seconds and fractional seconds are truncated. | `clock` | `result := format_date(now_unix(), "YYYY-MM-DD")` |
| `parse_date` | `parse_date(date: string, format: string) -> float` | handler-defined | float | Value::ErrorObject for invalid dates or unsupported formats; only `YYYY-MM-DD` is supported. Returns Unix seconds. | `clock` | `result := parse_date("1970-01-01", "YYYY-MM-DD")` |
| `env` | `env(name: string) -> string` | handler-defined | string | Missing variables return `""`; wrong argument shape returns `Value::Error`; restricted runs require `env-read`. | `env-read` | `result := env("HOME")` |
| `env_or` | `env_or(name: string, default: string) -> string` | handler-defined | string | Uses the default only when the variable is absent; both arguments must be strings; restricted runs require `env-read`. | `env-read` | `result := env_or("MODE", "dev")` |
| `env_int` | `env_int(name: string, default?: int) -> int` | handler-defined | int | Without a default, returns `Value::ErrorObject` when missing or not a valid integer; with a default, returns it for missing or invalid values. Restricted runs require `env-read`. | `env-read` | `result := env_int("PORT", 8080)` |
| `env_float` | `env_float(name: string, default?: number) -> float` | handler-defined | float | Without a default, returns `Value::ErrorObject` when missing or not a valid float; with a default, returns it for missing or invalid values. Restricted runs require `env-read`. | `env-read` | `result := env_float("TIMEOUT", 1.5)` |
| `env_bool` | `env_bool(name: string, default?: bool) -> bool` | handler-defined | bool | Without a default, returns `Value::ErrorObject` when missing or invalid; with a default, returns it for missing or invalid values. Boolean text accepts `true/1/yes/on` or `false/0/no/off` case-insensitively. Restricted runs require `env-read`. | `env-read` | `result := env_bool("ENABLED", false)` |
| `env_required` | `env_required(name: string) -> string` | handler-defined | string | `Value::ErrorObject` when absent; an explicitly set empty value is returned; restricted runs require `env-read`. | `env-read` | `result := env_required("TOKEN")` |
| `kv_set` | `kv_set(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := kv_set(...)` |
| `kv_get` | `kv_get(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := kv_get(...)` |
| `env_set` | `env_set(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `env-write` | `result := env_set(...)` |
| `env_list` | `env_list(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `env-read` | `result := env_list(...)` |
| `args` | `args(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := args(...)` |
| `arg_parser` | `arg_parser(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := arg_parser(...)` |
| `exit` | `exit(code?)` | 0..=1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := exit(...)` |
| `sleep` | `sleep(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := sleep(...)` |
| `execute` | `execute(command: string, options?: dict) -> string` | handler-defined | string | Runtime error on non-zero exit, timeout, output truncation, invalid command/options, or wrong types; rejects empty/newline/NUL shell text. | `shell-exec` | `result := execute("echo hi", {"timeout_ms": 1000})` |
| `execute_status` | `execute_status(command: string, options?: dict) -> ProcessResult` | handler-defined | `ProcessResult` | Runtime error on invalid command/options or spawn/wait/read failure; rejects empty/newline/NUL shell text. | `shell-exec` | `result := execute_status("echo hi")` |
| `os_getcwd` | `os_getcwd(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := os_getcwd(...)` |
| `os_chdir` | `os_chdir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := os_chdir(...)` |
| `os_rmdir` | `os_rmdir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-delete` | `result := os_rmdir(...)` |
| `os_environ` | `os_environ(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := os_environ(...)` |
| `join_path` | `join_path(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := join_path(...)` |
| `dirname` | `dirname(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := dirname(...)` |
| `basename` | `basename(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := basename(...)` |
| `path_exists` | `path_exists(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_exists(...)` |
| `path_join` | `path_join(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_join(...)` |
| `path_absolute` | `path_absolute(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_absolute(...)` |
| `path_is_dir` | `path_is_dir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_is_dir(...)` |
| `path_is_file` | `path_is_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_is_file(...)` |
| `path_is_symlink` | `path_is_symlink(...)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_is_symlink(...)` |
| `path_extension` | `path_extension(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := path_extension(...)` |
| `regex_match` | `regex_match(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := regex_match(...)` |
| `regex_find_all` | `regex_find_all(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := regex_find_all(...)` |
| `regex_replace` | `regex_replace(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := regex_replace(...)` |
| `regex_split` | `regex_split(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := regex_split(...)` |
| `http_get` | `http_get(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_get(...)` |
| `http_request` | `http_request(request)` or `http_request(url, options, on_stream_event?)` | 1..=3 | `Result<dictionary-or-HttpResponse, string>` | Buffered by default. With `options.response_stream=true`, returns a single-use incremental HTTP response suitable for returning from an `http_server` route. The optional callback receives bounded `headers`, `chunk`, `complete`, `error`, and `cancelled` events; returning `false` cancels later upstream reads. `max_response_bytes`, timeout, redirect, DNS-pinning, destination-policy, binary request body, and capability controls remain enforced. | `network-client` | `result := http_request(url, {"method":"POST", "response_stream":true}, on_event)` |
| `http_post` | `http_post(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_post(...)` |
| `http_put` | `http_put(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_put(...)` |
| `http_delete` | `http_delete(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_delete(...)` |
| `http_get_binary` | `http_get_binary(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_get_binary(...)` |
| `http_download_file` | `http_download_file(url, output_path, options)` | exact 3 | dictionary | Streams to a same-directory temporary file, enforces `max_bytes`, verifies a SHA-256 digest, and atomically publishes only complete responses. | `network-client`, `filesystem-write` | `receipt := http_download_file(url, "artifact.bin", {"max_bytes": 1073741824})` |
| `http_upload_file` | `http_upload_file(url, input_path, options)` | exact 3 | dictionary | Streams a file as a POST/PUT body without base64 expansion and bounds the response with `max_response_bytes`. | `network-client`, `filesystem-read` | `receipt := http_upload_file(url, "artifact.bin", {"method":"PUT"})` |
| `ai_request_hash` | `ai_request_hash(prompt_or_messages, options)` | exact 2 | string | Value::Error on invalid args/options contracts; hashes normalized endpoint/model/messages/body/relevant headers without network I/O. | `none` | `hash := ai_request_hash("Hi", {"endpoint":"https://example.ai/chat","model":"gpt"})` |
| `ai_chat` | `ai_chat(prompt_or_messages, options)` | exact 2 | dynamic (Value) | Value::Error on invalid args/options contracts; `Result(Err)` with deterministic transport/API/replay failures; success adds `usage`, `finish_reason`, `tool_calls`, and `provider` when available; `options.structured_errors` opts into typed error dictionaries. | `network-ai` | `result := ai_chat("Hi", {"endpoint":"https://example.ai/chat","model":"gpt"})` |
| `ai_stream_chat` | `ai_stream_chat(prompt_or_messages, options, on_chunk?)` | 2..=3 | dynamic (Value) | Value::Error on invalid args/options/callback contracts; `Result(Err)` with deterministic transport/API/replay failures; success adds `usage`, `finish_reason`, and `provider` when available; optional callbacks receive `(delta, raw_chunk)` and can return `false` to cancel later chunks; supports replay cassettes and structured errors. | `network-ai` | `result := ai_stream_chat("Hi", {"endpoint":"https://example.ai/chat","model":"gpt"}, on_chunk)` |
| `ai_embedding` | `ai_embedding(input, options)` | exact 2 | dynamic (Value) | Value::Error on invalid args/options contracts; `Result(Err)` if embedding response contract is missing `data[0].embedding`; success adds `usage`, `finish_reason`, and `provider` when available; supports replay cassettes and structured errors. | `network-ai` | `result := ai_embedding("query", {"endpoint":"https://example.ai/embed","model":"text-embed"})` |
| `ai_tool_loop` | `ai_tool_loop(prompt_or_messages, options)` | exact 2 | dynamic (Value) | Value::Error on invalid args/options contracts; `Result(Err)` for missing tool results or deterministic transport/API/replay failures; success adds `usage`, `finish_reason`, `tool_calls`, and `provider` when available; supports replay cassettes and structured errors. | `network-ai` | `result := ai_tool_loop("Plan this", {"endpoint":"https://example.ai/chat","model":"gpt","tool_results":{"lookup":"ok"}})` |
| `parallel_http` | `parallel_http(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := parallel_http(...)` |
| `jwt_encode` | `jwt_encode(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := jwt_encode(...)` |
| `jwt_decode` | `jwt_decode(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := jwt_decode(...)` |
| `jwt_verify` | `jwt_verify(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := jwt_verify(...)` |
| `oauth2_auth_url` | `oauth2_auth_url(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := oauth2_auth_url(...)` |
| `oauth2_get_token` | `oauth2_get_token(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := oauth2_get_token(...)` |
| `http_get_stream` | `http_get_stream(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := http_get_stream(...)` |
| `http_server` | `http_server(...)` | handler-defined | dynamic (Value) | Creates a routed server with an 8 MiB buffered-body bound and a socket read deadline. On Unix, `server.route_upload(method, path, spool_directory, max_body_bytes, authorize, handler)` requires `network-server`, `filesystem-write`, and `filesystem-delete`, plus a pre-existing non-symlink directory owned by the effective user with exact mode 0700. It preflights metadata before accepting bytes, then streams at most 64 MiB into a descriptor-relative mode-0600 file. The completion handler receives `body_artifact` (`schema_version`, `path`, `bytes`, `sha256`); it must atomically adopt the file before returning or the runtime removes it. Cleanup failure returns 500. Other platforms fail closed until equivalent private-file semantics exist. Request dictionaries include socket-derived peer fields. | `none` | `server := http_server(8080); server = server.route_upload("PUT", "/blob", "private/uploads", 67108864, authorize, accept)` |
| `http_listen` | `http_listen(...)` | handler-defined | dynamic (Value) | Creates a host-bound routed server with the same bounded-body, read-deadline, and peer-identity behavior as `http_server`; listening is capability-gated. | `network-server` | `result := http_listen(...)` |
| `http_response` | `http_response(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := http_response(...)` |
| `json_response` | `json_response(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := json_response(...)` |
| `html_response` | `html_response(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := html_response(...)` |
| `redirect_response` | `redirect_response(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := redirect_response(...)` |
| `set_header` | `set_header(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_header(...)` |
| `set_headers` | `set_headers(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_headers(...)` |
| `db_connect` | `db_connect(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_connect(...)` |
| `db_connect_readonly` | `db_connect_readonly(db_type: string, path: string) -> Database` | handler-defined | Database | Opens an existing checkpointed SQLite database as immutable with engine-enforced read-only flags and no WAL/SHM creation; rejects missing files, `:memory:`, non-SQLite types, non-empty WAL state, writes, invalid args and capability denial. | `database` | `db := db_connect_readonly("sqlite", "state.db")` |
| `db_execute` | `db_execute(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_execute(...)` |
| `db_query` | `db_query(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_query(...)` |
| `db_close` | `db_close(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_close(...)` |
| `db_pool` | `db_pool(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_pool(...)` |
| `db_pool_acquire` | `db_pool_acquire(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_pool_acquire(...)` |
| `db_pool_release` | `db_pool_release(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_pool_release(...)` |
| `db_pool_stats` | `db_pool_stats(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_pool_stats(...)` |
| `db_pool_close` | `db_pool_close(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_pool_close(...)` |
| `db_begin` | `db_begin(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_begin(...)` |
| `db_commit` | `db_commit(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_commit(...)` |
| `db_rollback` | `db_rollback(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_rollback(...)` |
| `db_last_insert_id` | `db_last_insert_id(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `database` | `result := db_last_insert_id(...)` |
| `Set` | `Set(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := Set(...)` |
| `set_add` | `set_add(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_add(...)` |
| `set_has` | `set_has(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_has(...)` |
| `set_remove` | `set_remove(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_remove(...)` |
| `set_union` | `set_union(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_union(...)` |
| `set_intersect` | `set_intersect(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_intersect(...)` |
| `set_difference` | `set_difference(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_difference(...)` |
| `set_to_array` | `set_to_array(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_to_array(...)` |
| `Queue` | `Queue(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := Queue(...)` |
| `queue_enqueue` | `queue_enqueue(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_enqueue(...)` |
| `queue_dequeue` | `queue_dequeue(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_dequeue(...)` |
| `queue_peek` | `queue_peek(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_peek(...)` |
| `queue_size` | `queue_size(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_size(...)` |
| `queue_is_empty` | `queue_is_empty(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_is_empty(...)` |
| `queue_to_array` | `queue_to_array(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := queue_to_array(...)` |
| `Stack` | `Stack(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := Stack(...)` |
| `stack_push` | `stack_push(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_push(...)` |
| `stack_pop` | `stack_pop(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_pop(...)` |
| `stack_peek` | `stack_peek(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_peek(...)` |
| `stack_size` | `stack_size(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_size(...)` |
| `stack_is_empty` | `stack_is_empty(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_is_empty(...)` |
| `stack_to_array` | `stack_to_array(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := stack_to_array(...)` |
| `channel` | `channel(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := channel(...)` |
| `shared_set` | `shared_set(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := shared_set(...)` |
| `shared_get` | `shared_get(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := shared_get(...)` |
| `shared_has` | `shared_has(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := shared_has(...)` |
| `shared_delete` | `shared_delete(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := shared_delete(...)` |
| `shared_add_int` | `shared_add_int(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := shared_add_int(...)` |
| `async_sleep` | `async_sleep(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := async_sleep(...)` |
| `async_timeout` | `async_timeout(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `clock` | `result := async_timeout(...)` |
| `async_http_get` | `async_http_get(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := async_http_get(...)` |
| `async_http_post` | `async_http_post(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := async_http_post(...)` |
| `async_read_file` | `async_read_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := async_read_file(...)` |
| `async_read_files` | `async_read_files(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := async_read_files(...)` |
| `async_write_file` | `async_write_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := async_write_file(...)` |
| `async_write_files` | `async_write_files(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := async_write_files(...)` |
| `spawn_task` | `spawn_task(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := spawn_task(...)` |
| `await_task` | `await_task(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := await_task(...)` |
| `cancel_task` | `cancel_task(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := cancel_task(...)` |
| `Promise.all` | `Promise.all(promises, concurrency?)` | 1..=2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := Promise.all(...)` |
| `promise_all` | `promise_all(promises, concurrency?)` | 1..=2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := promise_all(...)` |
| `await_all` | `await_all(promises, concurrency?)` | 1..=2 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := await_all(...)` |
| `parallel_map` | `parallel_map(items, mapper, concurrency?)` | 2..=3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := parallel_map(...)` |
| `promise_wait` | `promise_wait(promise)` | exact 1 | dynamic (Value) | Resolves to the promise value or Value::Error; capability-denied when gated. | `none` | `result := promise_wait(parallel_map(...))` |
| `par_map` | `par_map(items, mapper, concurrency?)` | 2..=3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := par_map(...)` |
| `par_each` | `par_each(items, mapper, concurrency?)` | 2..=3 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := par_each(...)` |
| `set_task_pool_size` | `set_task_pool_size(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := set_task_pool_size(...)` |
| `get_task_pool_size` | `get_task_pool_size(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := get_task_pool_size(...)` |
| `assert_equal` | `assert_equal(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := assert_equal(...)` |
| `assert_true` | `assert_true(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := assert_true(...)` |
| `assert_false` | `assert_false(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := assert_false(...)` |
| `assert_contains` | `assert_contains(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := assert_contains(...)` |
| `load_image` | `load_image(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := load_image(...)` |
| `gif_to_webp` | `gif_to_webp(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := gif_to_webp(...)` |
| `zip_create` | `zip_create(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := zip_create(...)` |
| `zip_add_file` | `zip_add_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := zip_add_file(...)` |
| `zip_add_dir` | `zip_add_dir(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := zip_add_dir(...)` |
| `zip_close` | `zip_close(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := zip_close(...)` |
| `unzip` | `unzip(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-write` | `result := unzip(...)` |
| `gzip_compress` | `gzip_compress(bytes_data)` | exact 1 | bytes | Compresses up to 64 MiB of raw bytes as a gzip stream using the default compression level. | `none` | `compressed := gzip_compress(payload)` |
| `gzip_decompress` | `gzip_decompress(bytes_data, max_output_bytes)` | exact 2 | bytes | Decompresses all gzip members in memory and fails before returning more than the caller-provided limit (maximum 64 MiB). | `none` | `plain := gzip_decompress(compressed, 4194304)` |
| `zip_single_file_read` | `zip_single_file_read(bytes_data, max_output_bytes)` | exact 2 | dictionary | Reads exactly one safe regular ZIP entry in memory, rejects traversal/symlink/multi-entry archives, and enforces the caller-provided output limit (maximum 64 MiB). | `none` | `entry := zip_single_file_read(archive, 4194304)` |
| `sha256` | `sha256(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := sha256(...)` |
| `hmac_sha256` | `hmac_sha256(secret, message)` | exact 2 | string | Returns lowercase hexadecimal HMAC-SHA256 for string or bytes secret/message; Value::Error on invalid args/types. | `none` | `signature := hmac_sha256(secret, payload)` |
| `hmac_sha256_verify` | `hmac_sha256_verify(secret, message, expected_hex)` | exact 3 | bool | Verifies a lowercase or uppercase hexadecimal HMAC-SHA256 using the HMAC implementation's constant-time tag comparison; malformed tags return `false`. | `none` | `valid := hmac_sha256_verify(secret, payload, signature)` |
| `sha256_file` | `sha256_file(path)` | exact 1 | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := sha256_file(...)` |
| `sha256_file_range` | `sha256_file_range(path, offset, count)` | exact 3 | string, int, int | Streams an exact file range through SHA-256 with bounded memory; rejects negative or beyond-EOF ranges. | `filesystem-read` | `digest := sha256_file_range(path, 0, size)` |
| `decode_file_range_info` | `decode_file_range_info(path, offset, count, encoding, max_output_bytes, prefix_bytes)` | exact 6 | string, int, int, string, int, int | Streams an exact file range through strict `identity`, `base64`, or `quoted-printable` decoding. Returns bounded metadata, a SHA-256 digest, and a caller-capped prefix (maximum 4096 bytes); decoded output is capped at 64 MiB. | `filesystem-read` | `info := decode_file_range_info(path, 0, size, "base64", 1048576, 64)` |
| `sha256_canonical_text_file_range` | `sha256_canonical_text_file_range(path, offset, count, mode)` | exact 4 | string, int, int, string | Streams and hashes an exact CRLF text range after deterministic `relaxed-crlf` or `simple-crlf` line canonicalization. Bare CR/LF, lines over 1 MiB, invalid ranges, and unsupported modes fail closed. | `filesystem-read` | `digest := sha256_canonical_text_file_range(path, body_offset, body_size, "relaxed-crlf")` |
| `md5` | `md5(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := md5(...)` |
| `md5_file` | `md5_file(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `filesystem-read` | `result := md5_file(...)` |
| `hash_password` | `hash_password(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := hash_password(...)` |
| `verify_password` | `verify_password(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := verify_password(...)` |
| `aes_encrypt` | `aes_encrypt(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := aes_encrypt(...)` |
| `aes_decrypt` | `aes_decrypt(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := aes_decrypt(...)` |
| `aes_encrypt_bytes` | `aes_encrypt_bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := aes_encrypt_bytes(...)` |
| `aes_encrypt_file_stream` | `aes_encrypt_file_stream(input_path, output_path, key, chunk_size)` | exact 4 | dictionary | Constant-memory framed AES-256-GCM with authenticated order, lengths, final frame, SHA-256 receipt, and atomic output publication. | `filesystem-read`, `filesystem-write` | `receipt := aes_encrypt_file_stream("in.bin", "out.aead", key, 1048576)` |
| `aes_decrypt_file_stream` | `aes_decrypt_file_stream(input_path, output_path, key)` | exact 3 | dictionary | Authenticates every frame before writing and removes temporary output on truncation, tampering, reordering, or format failure. | `filesystem-read`, `filesystem-write` | `receipt := aes_decrypt_file_stream("out.aead", "restored.bin", key)` |
| `aes_decrypt_bytes` | `aes_decrypt_bytes(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := aes_decrypt_bytes(...)` |
| `rsa_generate_keypair` | `rsa_generate_keypair(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := rsa_generate_keypair(...)` |
| `rsa_public_key_info` | `rsa_public_key_info(key)` | exact 1 | string or bytes | Parses an RSA public key from PEM text or DER bytes and returns normalized PEM, bit strength, input format, algorithm, and a SHA-256 SubjectPublicKeyInfo fingerprint. Non-RSA or malformed keys fail closed. | `none` | `info := rsa_public_key_info(decode_base64(record_value))` |
| `rsa_encrypt` | `rsa_encrypt(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := rsa_encrypt(...)` |
| `rsa_decrypt` | `rsa_decrypt(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := rsa_decrypt(...)` |
| `rsa_sign` | `rsa_sign(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := rsa_sign(...)` |
| `rsa_verify` | `rsa_verify(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := rsa_verify(...)` |
| `spawn_process` | `spawn_process(argv: array<string>, options?: dict) -> ProcessResult` | handler-defined | `ProcessResult` | Runtime error on empty/non-string argv, invalid options, or spawn/wait/read failure. Options include bounded output, stream channel/file sinks, cross-chunk secret redaction, and cancellation; defaults are 30s timeout, 1 MiB per-stream output, and 16 MiB maximum per stream. | `process-exec` | `result := spawn_process(["echo", "hi"], {"max_output_bytes": 4096})` |
| `pipe_commands` | `pipe_commands(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `process-exec` | `result := pipe_commands(...)` |
| `tcp_listen` | `tcp_listen(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-server` | `result := tcp_listen(...)` |
| `tcp_accept` | `tcp_accept(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-server` | `result := tcp_accept(...)` |
| `ip_cidr_contains` | `ip_cidr_contains(address, cidr)` | exact 2 | bool | Tests IPv4 or IPv6 CIDR membership with explicit bounded prefixes; mixed families return false and malformed inputs fail closed. | `none` | `allowed := ip_cidr_contains("192.0.2.4", "192.0.2.0/24")` |
| `ip_classify` | `ip_classify(address)` | exact 1 | dictionary | Deterministically classifies an IP literal as global, private, loopback, link-local, multicast, documentation, shared, benchmark, mapped, reserved, or unspecified and reports fail-closed public routability. | `none` | `result := ip_classify("192.0.2.1")` |
| `tcp_bind_probe` | `tcp_bind_probe(address)` | exact 1 | dictionary | Binds and immediately closes an ephemeral TCP listener to report local-address availability without aborting on an expected bind failure. | `network-server` | `result := tcp_bind_probe("127.0.0.1")` |
| `tcp_connect` | `tcp_connect(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := tcp_connect(...)` |
| `tcp_connect_bound` | `tcp_connect_bound(host, port, source_ip)` | handler-defined | TcpStream | Resolves deterministic same-family candidates, binds a validated unicast source IP before connecting, and applies bounded socket timeouts. | `network-client` | `stream := tcp_connect_bound("mx.example.net", 25, "192.0.2.10")` |
| `tcp_send` | `tcp_send(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := tcp_send(...)` |
| `tcp_send_file_range` | `tcp_send_file_range(stream, path, offset, count)` | exact 4 | int | Streams exactly the requested range from a non-symlink regular file through a fixed 64 KiB buffer, caps each call at 64 MiB, and fails closed on invalid ranges or partial I/O. | `network-client`, `filesystem-read` | `written := tcp_send_file_range(conn, "message.eml", 0, size)` |
| `tcp_receive` | `tcp_receive(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := tcp_receive(...)` |
| `tcp_close` | `tcp_close(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := tcp_close(...)` |
| `tcp_set_nonblocking` | `tcp_set_nonblocking(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := tcp_set_nonblocking(...)` |
| `tcp_info` | `tcp_info(stream)` | handler-defined | dictionary | Returns the stable `kujo.tcp.info.v1` envelope with socket-derived peer/local address, IP and port fields plus read/write timeouts; errors after close or TLS upgrade. | `none` | `info := tcp_info(conn)` |
| `tcp_set_timeouts` | `tcp_set_timeouts(stream, read_ms, write_ms)` | handler-defined | bool | Sets both blocking socket timeouts, each bounded to 1–600000 ms; the settings survive a TLS upgrade. | `none` | `tcp_set_timeouts(conn, 30000, 30000)` |
| `udp_bind` | `udp_bind(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-server` | `result := udp_bind(...)` |
| `udp_send_to` | `udp_send_to(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := udp_send_to(...)` |
| `udp_receive_from` | `udp_receive_from(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `network-client` | `result := udp_receive_from(...)` |
| `udp_close` | `udp_close(...)` | handler-defined | dynamic (Value) | Value::Error on invalid args/types/operation; capability-denied when gated. | `none` | `result := udp_close(...)` |
| `dns_lookup_a` | `dns_lookup_a(name, options?)` | handler-defined | dictionary | Bounded IPv4 address lookup with deterministic sorting, per-record TTL/DNSSEC proof, and `kujo.dns.lookup.v1` envelope. | `network-client` | `result := dns_lookup_a("mail.example.com")` |
| `dns_lookup_aaaa` | `dns_lookup_aaaa(name, options?)` | handler-defined | dictionary | Bounded IPv6 address lookup with deterministic sorting, per-record TTL/DNSSEC proof, and `kujo.dns.lookup.v1` envelope. | `network-client` | `result := dns_lookup_aaaa("mail.example.com")` |
| `dns_lookup_mx` | `dns_lookup_mx(name, options?)` | handler-defined | dictionary | Bounded MX lookup with deterministic sorting, per-record TTL/DNSSEC proof, and `kujo.dns.lookup.v1` envelope. | `network-client` | `result := dns_lookup_mx("example.com")` |
| `dns_lookup_txt` | `dns_lookup_txt(name, options?)` | handler-defined | dictionary | Bounded TXT lookup; split DNS character strings are joined in wire order and records are deterministically sorted. The envelope distinguishes `NXDOMAIN` from `NOERROR`/NODATA with `response_code` and `name_exists`. | `network-client` | `result := dns_lookup_txt("_dmarc.example.com")` |
| `dns_lookup_ptr` | `dns_lookup_ptr(ip, options?)` | handler-defined | dictionary | Bounded reverse lookup for a validated IPv4 or IPv6 address. | `network-client` | `result := dns_lookup_ptr("192.0.2.1")` |
| `dns_lookup_tlsa` | `dns_lookup_tlsa(name, options?)` | handler-defined | dictionary | Bounded TLSA lookup with numeric usage/selector/matching fields, lowercase association-data hex, and DNSSEC proof. | `network-client` | `result := dns_lookup_tlsa("_25._tcp.mail.example.com")` |
| `tls_connect` | `tls_connect(host, port, options?)` | handler-defined | TlsStream | Opens a TLS 1.2+ client connection with mandatory certificate-chain, hostname, and SNI verification. Options permit only `min_version` and bounded public `ca_pem`. | `network-client` | `conn := tls_connect("mail.example.com", 465)` |
| `tls_upgrade_client` | `tls_upgrade_client(stream, server_name, options?)` | handler-defined | TlsStream | Consumes a TcpStream and performs a verified client handshake for STARTTLS-style protocols; the original TcpStream becomes unusable even when the handshake fails. | `network-client` | `tls := tls_upgrade_client(conn, "mail.example.com")` |
| `tls_acceptor` | `tls_acceptor(cert_chain_path, private_key_path, options?)` | handler-defined | TlsAcceptor | Loads a PEM certificate chain and private key, rejects group/world-readable private keys on Unix, verifies the key pair, and enforces TLS 1.2+; no key material is emitted. | `network-server`, `filesystem-read` | `acceptor := tls_acceptor("cert.pem", "key.pem")` |
| `tls_upgrade_server` | `tls_upgrade_server(stream, acceptor)` | handler-defined | TlsStream | Consumes an accepted TcpStream and performs the server handshake; the original stream cannot be reused. | `network-server` | `tls := tls_upgrade_server(conn, acceptor)` |
| `tls_send` | `tls_send(stream, data)` | handler-defined | int | Writes and flushes string or bytes data over TLS using the underlying bounded socket timeout policy. | `network-client` | `written := tls_send(tls, "EHLO sender.example\r\n")` |
| `tls_send_file_range` | `tls_send_file_range(stream, path, offset, count)` | exact 4 | int | Streams an exact regular-file range over verified TLS with the same fixed-memory, symlink-denial, range, timeout, and 64 MiB per-call bounds as `tcp_send_file_range`. | `network-client`, `filesystem-read` | `written := tls_send_file_range(tls, "message.eml", 0, size)` |
| `tls_receive` | `tls_receive(stream, size)` | handler-defined | string or bytes | Reads at most the validated receive-size limit and returns UTF-8 text or raw bytes. | `network-client` | `reply := tls_receive(tls, 4096)` |
| `tls_close` | `tls_close(stream)` | handler-defined | bool | Performs TLS and TCP shutdown and idempotently consumes the live stream. | `none` | `tls_close(tls)` |
| `tls_info` | `tls_info(stream_or_acceptor)` | handler-defined | dictionary | Returns a `kujo.tls.info.v1` envelope with negotiated protocol/cipher and SHA-256 certificate fingerprints, never certificate private material. | `none` | `info := tls_info(tls)` |

## Coverage Contract

The integration contract test `tests/stdlib_reference_contract.rs` verifies:

- every runtime builtin is documented here exactly once
- documented capability values match runtime capability policy mapping
- documented arity labels match centralized arity metadata for builtins that use it

When adding/removing/renaming native builtins, update runtime registration and regenerate/update this table in the same change.
