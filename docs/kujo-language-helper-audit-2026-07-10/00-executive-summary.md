# Executive summary

## Result

The audit analyzed 39 git repositories and 443 non-generated Kujo source
files, 147 Rust files, and selected JavaScript/Python tooling. It
identified 22 credible candidates. The strongest evidence is concentrated in
filesystem safety, path confinement, CLI token parsing, dynamic structured-data
normalization, and redaction. The core already has 355 documented native
functions, including collections, JSON/TOML/YAML/CSV, typed environment access,
structured process results, path helpers, deterministic JSON, `secret`, and
`Result`/`Option` constructs. That baseline rules out a large “missing utility
library” response.

Recommended disposition:

- Language syntax/semantics: 0 immediate additions; 1 deeper design topic
  (typed structured-data boundaries and diagnostic paths).
- Core runtime/standard library: 2 near-term additions/prototypes
  (`write_file_atomic`, a symlink-aware path-boundary primitive); 2 bounded
  filesystem primitives for prototype (bounded reads and recursive walk).
- Official first-party packages: 4 (CLI parsing, redaction profiles, JSON-file
  envelopes, retry/backoff policy).
- Documentation/discoverability: 6 (existing collection, environment, path,
  process, time, serialization, and secret APIs).
- Shared internal tooling: 3 (project-root/config discovery, test workspaces,
  report formatting).
- Defer/reject: 4 (generic `group_by`, shell quoting, broad HTTP policy,
  pluralization/domain text helpers).

## Five highest-priority candidates

1. **HLP-001 — atomic bounded file write**: current `write_file` supports an
   overwrite flag but does not provide atomic replacement. Ecosystem code uses
   delete-then-write or temp-file-plus-rename in PackWrite, Tribunal, CaseFile,
   MCP, Workcell, Scent, and RAG scripts.
2. **HLP-002 — canonical path-boundary check**: CaseFile, MCP, Dispatch, SSG,
   RAG, and Workcell independently protect roots. This is security-sensitive
   runtime support, not a string-prefix helper.
3. **HLP-003 — typed structured-data boundary API**: 19 Agents SDK modules copy
   `dict_get_or`, `normalize_string`, `normalize_dict`, and related coercion
   helpers. Existing `get_default` is not enough because callers also need type
   checks, path-aware errors, and a stable missing/null contract.
4. **HLP-004 — first-party CLI argument package**: five mature tools implement
   variants of `parse_subcommand`, `has_flag`, `flag_value`, or `parse_args`,
   while core `arg_parser()` currently returns an empty `ArgParser` shell.
5. **HLP-005 — redaction profiles outside core**: Lens, Watchdog, CaseFile,
   Muzzle, Scent, AI SDK, Eval, and Tribunal redact sensitive data differently.
   Centralize policy in the existing `redact` package, but do not freeze
   domain-specific secret rules into the language runtime.

## Most important deeper language finding

The recurring `dict_get_or`/`normalize_*` cluster is a symptom of dynamic data
crossing typed conceptual boundaries, not evidence that Kujo needs dozens of
new convenience functions. The next design step should be a small, explicit
schema/record-access contract that can return a value, a missing/null state, or
path-aware validation errors and that composes with existing `Result`/`Option`
and optional typing. Do not add syntax before that contract is prototyped.

## Most important agent-safety finding

Agent-authored code needs canonical safe defaults for writes and path checks.
`write_file(path, content, true)` is explicit about overwrite but still permits
partial replacement semantics. A runtime-owned atomic write plus a symlink-aware
root check would remove several unsafe implementation choices from generated
code.

## Most important documentation finding

Many wrappers duplicate existing functions: `slug` around `slugify`, `pad_right`
around `pad_right`, `iso_now` around `now_utc`/`format_date`, `parse_bool_env`
around `env_bool`, and process field accessors around `ProcessResult`. The
standard-library inventory is strong but not discoverable enough from examples.

## Main over-abstraction risk

The ecosystem has many copied helpers inside one repository or one template
family. Promoting every repeated name would create a broad, unstable “Kujo
utility” surface. The report therefore treats identical code in Agents SDK as
one intra-repository centralization signal, not as 19 independent ecosystem
votes.

## Completion record

HLP-001 is implemented as the bounded core write_file_atomic API with
VM/interpreter contract coverage. HLP-004 is implemented as the first-party
modules/cli.kujo parser spike with VM/interpreter coverage. HLP-007, HLP-011,
HLP-013, and HLP-015 are documented in HELPER_DISCOVERY_COOKBOOK.md.

The remaining candidates were re-reviewed against the original evidence:
HLP-002, HLP-003, HLP-005, HLP-006, HLP-008, HLP-009, HLP-010, HLP-012,
HLP-014, HLP-016, and HLP-017 remain prototypes, package-boundary work, or
design work; HLP-018 is deferred; HLP-019 through HLP-022 are rejected or
rejected for core. No additional candidate met the evidence threshold for
implementation in this pass.
