# Kujo capability baseline

Source of truth: `kujo/docs/STANDARD_LIBRARY.md`, `kujo/docs/LANGUAGE_SPEC.md`,
`kujo/src/interpreter/native_functions/`, and `kujo/src/interpreter/mod.rs`.

## Already present

The current inventory documents 355 native functions. Relevant surfaces include:

- Strings: trim, case conversion, substring, replace, split/join, lines/words,
  prefix/suffix checks, padding, slugify, truncate, index/search, repeat,
  Unicode-aware helpers, and regex operations.
- Collections: map/filter/reduce/find/sort/unique/any/all/chunk/flatten,
  slice/concat/take/skip/windows/range/zip/enumerate, set/queue/stack, and
  `has_key`, `get`, `get_default`, `merge`, and `update`.
- Filesystem/path: read/write/append/delete/rename/copy, binary I/O, file
  metadata, directory listing/creation, `path_absolute`, `path_join`,
  `dirname`, `basename`, extension and file/dir/symlink checks, and bounded
  atomic text/bytes writes through `write_file_atomic`.
- Structured data: JSON/TOML/YAML/CSV parse and serialization, deterministic
  JSON key ordering, limits, JSON Schema validation with path-aware errors, and
  base64 encoding.
- Environment/CLI/process: `env`, `env_or`, `env_int`, `env_float`, `env_bool`,
  `env_required`, `args`, `arg_parser`, `execute_status`, `spawn_process`, and
  `pipe_commands` with timeout/output truncation fields. The first-party
  `modules/cli.kujo` package adds declarative token parsing without expanding
  core policy.
- Time/concurrency: now/UTC/unix timestamps, durations, date parse/format,
  async tasks, promises, cancellation, bounded pools, parallel map/each, and
  HTTP helpers.
- Security/AI: capability gates, private-network policy, `secret`/`reveal`,
  redacted serialization, hashing, password verification, AI replay/structured
  results, token budgeting, and schema-first message builders.
- Errors/types: dynamic `Value` errors, `Result::Ok/Err`, `Some/None`, `try` /
  `except`, `throw`, optional type annotations, `match`, and structured
  diagnostics.

## Actual gaps or friction

1. There is no documented, symlink-aware general path-boundary API for scripts;
   archive extraction has lower-level Rust path security, while applications
   reimplement lexical prefix checks.
2. Dynamic dictionaries have field access and defaults, but no standard
   typed/schema access helper that reports the failing data path and preserves a
   clear missing-versus-null distinction.
3. Bounded file reads and recursive walks with explicit limits/ignore policy
   are not one canonical script-facing primitive.

## Existing APIs frequently mistaken for missing helpers

The evidence ledger records wrappers around `slugify`, `pad_right`,
`format_date`/`now_utc`, `env_bool`, `to_json_pretty`, `html/escape_xml`,
`get_default`, and `ProcessResult` fields. These should receive examples and
cross-links before new aliases are added. Naming aliases would increase the
already large surface without removing semantic complexity.

## Compatibility posture

`LANGUAGE_SPEC.md` makes builtin names and argument contracts stable within a
minor line; additive fields are non-breaking but removing or changing field
types is breaking. New core APIs therefore need narrow signatures, explicit
limits, VM/interpreter parity tests, and capability classification.
