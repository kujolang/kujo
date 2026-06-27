# Native API Security Posture

Status: v1.0.0 baseline draft (active, not a release-ready claim)
Last updated: 2026-05-16

This document is an operator-focused security guide for Kujo's host-effect runtime APIs.

Kujo is not a sandbox. Running Kujo code is equivalent to running local code with the current process privileges unless you explicitly apply capability restrictions and external isolation controls.

Release readiness remains tracked in `ROADMAP.md`.

## 1. Threat Model

### Security objective

Provide deterministic runtime controls and failure contracts for host-effect APIs so operators can run trusted automation safely and run untrusted scripts with explicit least-privilege policy.

### In scope

- Runtime capability gating for host-effect APIs (`--untrusted`, `--allow-*`).
- Deterministic runtime-denied/misuse errors.
- Built-in guardrails for selected high-risk surfaces (filesystem/network/process/archive/static serving).

### Out of scope (non-goals)

- Full process/container sandboxing.
- Kernel-level isolation.
- Guaranteed data exfiltration prevention without host firewall/policy controls.
- Multi-tenant isolation in one Kujo process.

## 2. Trust Modes And Defaults

### Default behavior

`kujo run` and `kujo test-run` default to trusted mode when no capability flags are provided.

Trusted mode means all host-effect capabilities are enabled and scripts run with ambient user permissions.

### Restricted behavior

Use `--untrusted` to switch to deny-by-default mode.

In `--untrusted` mode, host-effect calls fail unless explicitly re-enabled via `--allow-*` flags.

### Explicit policy behavior

- `--allow-*` flags imply restricted baseline with only requested capabilities enabled.
- `--allow-all` force-enables all capabilities and should be treated as trusted mode.

## 3. Capability Flags

| Flag | Capability | Typical APIs Unlocked | Primary Risk |
| --- | --- | --- | --- |
| `--allow-fs-read` | Filesystem read | `read_file`, `read_lines`, `read_binary_file`, metadata/path reads | Data disclosure |
| `--allow-fs-write` | Filesystem write | `write_file`, `append_file`, `write_binary_file`, mkdir/write helpers | Data tampering |
| `--allow-fs-delete` | Filesystem delete | `delete_file`, delete-adjacent flows | Data loss |
| `--allow-process-exec` | Direct process execution | `spawn_process`, `pipe_commands` | Arbitrary command execution |
| `--allow-shell-exec` | Shell-string execution | `execute`, `execute_status` | Shell injection/command abuse |
| `--allow-env-read` | Environment read | `env`, `env_list`, related env readers | Secret leakage |
| `--allow-env-write` | Environment write | `env_set` and env mutation | Process/session tampering |
| `--allow-net-client` | Outbound network | `http_get/post/request`, TCP/UDP client operations | Data exfiltration/SSRF-style pivots |
| `--allow-ai` | AI provider egress | `ai_chat`, `ai_stream_chat`, `ai_embedding`, `ai_tool_loop` | Prompt/data exfiltration to model endpoints |
| `--allow-net-server` | Listener/network server | `http_server.listen`, server-side sockets | Local service exposure |
| `--allow-net` | Net client + server | Union of network-client/network-server surfaces | Combined network risk |
| `--allow-database` | Database access | `db_connect`, query/transaction helpers | Unauthorized data access |
| `--allow-clock` | Clock/time | `now`, timestamp helpers | Timing side-channel support |
| `--allow-random` | Randomness | `random`, random helpers | Nondeterministic workflows |
| `--allow-all` | All capabilities | All host-effect APIs | Full ambient-host risk |

Per-function capability metadata is maintained in `docs/STANDARD_LIBRARY.md` and contract-tested in `tests/stdlib_reference_contract.rs`.

## 4. High-Risk Native Surface Guidance

### 4.1 Process and Shell APIs

Relevant APIs: `execute`, `execute_status`, `spawn_process`, `pipe_commands`.

Policy boundaries:

- `spawn_process` and `pipe_commands` require `--allow-process-exec`.
- `execute` and `execute_status` require `--allow-shell-exec`.

Operational guidance:

- Prefer argv-array APIs (`spawn_process`, `pipe_commands`) over shell strings.
- Never pass untrusted input directly into shell command strings.
- Keep `inherit_env` disabled unless explicitly required.
- Use `timeout_ms`, `max_output_bytes`, and env allow/deny controls for bounded execution.

### 4.2 Network APIs

Relevant APIs: HTTP/TCP/UDP helpers.

Policy boundaries:

- Outbound operations require `--allow-net-client`.
- Listener/server operations require `--allow-net-server`.
- `--allow-net` enables both.

Built-in guardrails:

- TCP connect timeout: `10000 ms`
- TCP/UDP read-write timeout: `30000 ms`
- HTTP client timeout: `30000 ms`
- Max network response/receive body: `8 MiB`
- Max `parallel_http(...)` batch size: `128` URLs.
- Synchronous and asynchronous HTTP helpers share the same destination policy, timeout, URL-scheme, and response-size limits.
- HTTP native URL validation:
  - only `http` and `https` schemes are accepted.
  - malformed URLs and missing hosts fail early with deterministic diagnostics before request execution.
- Outbound destination policy mode:
  - `--deny-private-net`: CLI-level strict outbound policy for `kujo run` / `kujo test-run`, independent of environment variables and valid in trusted or untrusted mode.
  - `KUJO_NET_DESTINATION_POLICY=allow_all` (default): preserves backward-compatible permissive destination behavior.
  - `KUJO_NET_DESTINATION_POLICY=deny_private`: blocks outbound HTTP/TCP/UDP client destinations that resolve to loopback/private/link-local/multicast/unspecified IP ranges.
  - `KUJO_ALLOW_PRIVATE_NETWORK_DESTINATIONS=1`: explicit trusted-local override when strict policy mode is enabled.
  - Invalid policy values fail deterministically with: `Invalid KUJO_NET_DESTINATION_POLICY`.
  - Strict-mode blocked destinations fail deterministically with: `blocked by outbound destination policy`.

Operational guidance:

- Restrict egress and ingress at OS/network policy layers.
- Bind server listeners to explicit interfaces and non-privileged ports.
- Treat large unvalidated payloads as hostile by default.

### 4.2.1 HTML Response Threat Model (`html_response`)

`html_response(...)` is a raw response-construction helper. It does **not** sanitize or escape attacker-controlled content.

`render_markdown(...)` and native SSG helper output escape source text before
embedding it into HTML and reject unsafe Markdown link/image target schemes.
Use `html_response(...)` only when raw HTML is intentional.

Threat model boundary:

- If untrusted input is interpolated into HTML without escaping, downstream browsers can execute injected markup/script.
- Kujo runtime/server controls do not rewrite response bodies for XSS safety.

Safer usage patterns:

- Prefer JSON responses (`http_response`) for untrusted data APIs.
- Keep user-controlled data in text nodes only, and escape at least `&`, `<`, `>`, `"`, and `'` before interpolation.
- For rich HTML pages, render from trusted templates and pass pre-sanitized content only.

Example defensive escaping helper (Kujo script-level):

```kujo
func escape_html(input) {
  let out := replace(input, "&", "&amp;")
  out := replace(out, "<", "&lt;")
  out := replace(out, ">", "&gt;")
  out := replace(out, "\"", "&quot;")
  out := replace(out, "'", "&#39;")
  return out
}
```

### 4.3 AI Provider APIs

Relevant APIs: `ai_chat`, `ai_stream_chat`, `ai_embedding`, `ai_tool_loop`.

Policy boundaries:

- AI helpers require `--allow-ai` in `--untrusted` mode.
- `--allow-net-client` does not unlock AI helpers.
- `KUJO_AI_ALLOWED_ENDPOINTS` optionally restricts AI helper endpoints by scheme, host, optional port, and optional path prefix. When unset, trusted-mode behavior remains backward compatible.
- Endpoint allowlist misses return deterministic `kind:"endpoint_denied"` failures; with `options.structured_errors: true`, the error is a structured AI error dictionary.
- AI helpers still honor the outbound destination policy for live requests. Strict replay cassettes remain hermetic and do not open sockets.

Operational guidance:

- Prefer `--allow-ai` over broad `--allow-net-client` when untrusted code only needs LLM calls.
- Set `KUJO_AI_ALLOWED_ENDPOINTS` for shared automation and agent runs that must call only approved provider endpoints.
- Review committed cassettes because they can contain model output even when credentials are redacted.

### 4.4 Filesystem and Archive APIs

Relevant APIs: read/write/delete/path/directory/archive helpers.

Policy boundaries:

- Filesystem read paths require `--allow-fs-read`.
- Filesystem write paths require `--allow-fs-write`.
- Filesystem delete paths require `--allow-fs-delete`.

Built-in file IO guardrails:

- Whole-file read operations capped at `8 MiB`.
- Write payload operations capped at `8 MiB`.
- `write_file` and `write_binary_file` require explicit `overwrite=true` for replacement.
- `delete_file` rejects directory paths.

`unzip` hardening:

- Rejects absolute paths, `..` traversal, drive-prefixed names, null-byte names, and symlink entries.
- Fails extraction on first unsafe entry.
- Enforces extraction limits: 1024 entries, 16 MiB per entry, 64 MiB total uncompressed bytes.

Operational guidance:

- Treat archive extraction as a high-risk write surface.
- Constrain writable roots for Kujo processes.
- Avoid running untrusted archive workflows in privileged directories.

### 4.5 Database APIs

Relevant APIs: connection/query/pool/transaction helpers.

Policy boundaries:

- Database access requires `--allow-database`.

Operational guidance:

- Use least-privileged DB accounts.
- Restrict DB network reachability to required endpoints.
- Parameterize query data wherever possible.

### 4.6 Crypto APIs

Relevant APIs: hash/password/AES/RSA helpers.

Policy boundaries:

- Crypto helpers are not capability-gated separately today; they execute within runtime trust context.

Operational guidance:

- Keep keys and secrets out of source files.
- Use external secret management and key rotation workflows.
- Treat crypto API errors as hard failures.

## 5. Static Server (`kujo serve`) Security Defaults

`kujo serve` is intended for local static preview/testing and should not be treated as a hardened internet-facing platform.

Key defaults and controls:

- Root-bound canonical path checks and traversal rejection.
- Single-pass percent-decoding with malformed encoding/null-byte rejection.
- Hidden/private path blocking (`.env`, `.git`, backup/swap-style names).
- Deterministic request limits (line/header/body sizes, header count, max connections).
- Safe MIME fallback (`application/octet-stream`) for unknown/extensionless paths.
- Baseline response headers (`X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`).
- Hardened-mode headers with `--hardened`.

Operator guidance:

- Use a reverse proxy, TLS termination, and network ACLs for shared environments.
- Treat `kujo serve` as preview infrastructure, not as a full production edge server.

## 6. Safe vs Unsafe Configuration Patterns

These examples intentionally use VM-default `kujo run` paths. Use `--interpreter` only for targeted compatibility/debug isolation when diagnosing a known runtime-path divergence.

### Safer local review of untrusted script with minimal permissions

```bash
kujo run --untrusted --allow-fs-read ./script.kujo
```

### Safer network client workflow with explicit egress-only capability

```bash
kujo run --untrusted --allow-net-client ./fetch.kujo
```

### Unsafe pattern: full capability escalation for untrusted input

```bash
kujo run --allow-all ./untrusted.kujo
```

### Unsafe pattern: enabling shell execution for interpolated user input

```bash
kujo run --untrusted --allow-shell-exec ./script_that_builds_shell_strings.kujo
```

## 7. Recommended External Sandboxing Controls

For high-risk or shared environments, apply host/container controls in addition to Kujo runtime capability policy:

- Run Kujo in containers or VM sandboxes with least privileges.
- Use read-only filesystems where possible; mount narrow writable directories.
- Drop Linux capabilities and apply seccomp/AppArmor/SELinux profiles.
- Apply strict outbound and inbound firewall rules.
- Use dedicated low-privilege service accounts.
- Isolate secrets from environment variables when possible.

## 8. VM/JIT Unsafe Boundary Policy

Kujo's JIT/VM internals still require `unsafe` for FFI pointer boundaries and generated function-pointer execution. For v1 hardening work, the policy is:

- concentrate function-pointer `unsafe` invocation behind shared wrapper helpers
- document pointer lifetime and ownership invariants at wrapper boundaries
- avoid ad hoc `unsafe` callsites in `src/vm.rs` execution paths
- require targeted regression tests whenever `unsafe` boundaries are moved

Current hardening status:

- VM JIT function-pointer invocation is centralized through `src/jit.rs` wrappers:
  - `invoke_compiled_fn(...)`
  - `invoke_compiled_fn_with_arg(...)`
- VM callsites no longer invoke compiled JIT pointers through scattered inline `unsafe` blocks.

## 9. Verification And Regression Expectations

Security boundary changes must include test updates and document updates in this file.

Primary regression suites:

```bash
cargo test --test native_api_security_boundaries
cargo test --test runtime_security
cargo test --test serve_command_integration
```

These suites cover capability denial/allow behavior, archive and path safety boundaries, request-boundary handling, and deterministic failure contracts.

Cross-platform module-escape coverage strategy:

- Unix builds run a real symlink-escape integration regression in `tests/runtime_security.rs` (`runtime_security_rejects_module_symlink_escape`).
- Non-Unix environments use deterministic module-name traversal hardening coverage via `runtime_security_module_loader_rejects_parent_traversal_import_name_cross_platform` (integration) plus module-loader unit contracts that reject unsafe import names before filesystem resolution.
- This split avoids flaky Windows symlink privilege assumptions while preserving deterministic escape-boundary coverage for release gates.
