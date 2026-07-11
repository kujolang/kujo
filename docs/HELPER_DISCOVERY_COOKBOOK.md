# Helper discovery cookbook

This cookbook maps recurring wrapper patterns to the canonical Kujo APIs.
Use the native helper before introducing a local wrapper with the same
semantics.

## HLP-007: canonical text and time helpers

Use `slugify` for a stable URL-friendly label, `pad_left`/`pad_right` for
character-width alignment, `now_utc` for an ISO-8601 UTC string, and
`format_date` for formatting Unix seconds. `pad_start` and `pad_end` are the
existing spelling aliases for `pad_left` and `pad_right`.

    slug := slugify("Release Candidate 1")
    label := pad_right(slug, 24, " ")
    timestamp := now_utc()
    day := format_date(now_unix(), "YYYY-MM-DD")

The timestamp units matter: `now`/`now_unix` return Unix seconds,
`current_timestamp` returns Unix milliseconds, and `now_utc` returns a UTC
string such as `2026-07-11T12:34:56Z`. Pass `now_unix()` (not
`current_timestamp()`) to `format_date`.

`slugify` lowercases text, keeps Unicode alphanumeric characters, maps spaces
and underscores to hyphens, removes other punctuation, and trims leading or
trailing hyphens. Padding counts Unicode characters; a negative width is an
error, an empty pad string uses a space, and only the first character of a
multi-character pad string is used. `format_date` supports the replacement
tokens `YYYY`, `MM`, `DD`, `HH`, `mm`, and `ss`; an invalid timestamp is
returned as an error string. `parse_date` currently accepts only
`YYYY-MM-DD` and returns Unix seconds or a runtime error.

These helpers are already part of the standard-library contract. Do not add
local slug, padding, or timestamp wrappers unless the caller intentionally
needs a different domain policy.

## HLP-011: typed environment configuration

Build configuration at the boundary with the existing typed accessors. Use
env_required for mandatory values, env_or for string defaults, and env_int,
env_float, or env_bool when a value must have a specific type.

    host := env_or("KUJO_HOST", "127.0.0.1")
    port := env_int("KUJO_PORT")
    enabled := env_bool("KUJO_ENABLED")
    token := env_required("KUJO_TOKEN")

`env_int` returns an integer, `env_float` returns a float, and `env_bool`
accepts only `true`, `1`, `yes`, `on`, `false`, `0`, `no`, or `off`,
case-insensitively. The typed functions and `env_required` return a runtime
error object when the variable is missing or cannot be parsed. `env_or` only
uses its string default when the variable is absent; `env` itself returns an
empty string for an absent variable, so use `env_required` when absence must be
distinguished from an empty value. `env_required` considers an explicitly set
empty value present.

All reads require the `env-read` capability in restricted execution:
`kujo run --untrusted --allow-env-read app.kujo`. `env_set` and `env-write` are
separate; do not use a write just to make a configuration example pass in
production. Keep environment reads in one configuration function, then pass
the resulting values into the rest of the program; do not repeat parsing in
each command.

## HLP-013: process-result accessors

Use spawn_process with an argv array when a tool needs a structured result.
ProcessResult exposes exitcode, stdout, stderr, success, timed_out,
stdout_truncated, and stderr_truncated as dot fields.

    result := spawn_process(["git", "status", "--short"], {"max_output_bytes": 65536})
    if result.success && !result.timed_out && !result.stdout_truncated {
        print(result.stdout)
    }

`spawn_process` does not invoke a shell and requires a non-empty array of
string arguments. Its optional dictionary accepts `timeout_ms` (default
30,000), `max_output_bytes` (default 1 MiB per stream, maximum 16 MiB),
`inherit_env`, `env_allow`, `env_deny`, and `env`. `stdout` and `stderr` are
lossy UTF-8 strings; `exitcode` is the runtime's exact field spelling. A
timeout makes `timed_out` and `success` false. Check both truncation flags and
the timeout before treating captured output as complete.

Use `execute_status` when a compatibility surface truly requires one shell
command string; it returns the same `ProcessResult` shape but requires the
`shell-exec` capability and rejects empty commands, NUL bytes, and newlines.
`execute` is exception-style shell execution: it returns stdout only on a
successful, non-truncated exit and otherwise returns a runtime error. Prefer
`spawn_process` when arguments contain user or external data. These APIs
require `process-exec` or `shell-exec` respectively in restricted runs.

Do not add local `proc_*` getters. If a JSON receipt is needed, copy the
required fields into a dictionary first; `ProcessResult` is a runtime struct
and is not directly accepted by `to_json`.

## HLP-015: deterministic JSON output

Use to_json for compact machine-readable output and to_json_pretty for
human-readable artifacts. Both reject non-finite floats and serialize
dictionary keys deterministically; parse_json enforces the documented input
size and nesting limits.

    payload := {"status": "ok", "items": ["a", "b"]}
    machine_line := to_json(payload)
    report_text := to_json_pretty(payload)
    round_trip := parse_json(machine_line)

Emit machine_line directly when stdout is a protocol. Do not add a local
print_json helper that changes ordering, float handling, or error behavior.
