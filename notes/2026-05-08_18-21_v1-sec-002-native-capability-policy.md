# Kujo Field Notes — V1-SEC-002 Native Capability Policy

**Date:** 2026-05-08
**Session:** 18:21 local
**Branch/Commit:** main / 1fa4a22
**Scope:** Implemented roadmap item V1-SEC-002 by adding runtime capability policy controls for host-effect native APIs, enforcing policy in interpreter/VM paths, and adding integration tests plus docs/roadmap updates.

---

## What I Changed
- Added centralized capability model in `src/interpreter/capabilities.rs`:
  - `NativeCapability` enum (`filesystem-read`, `filesystem-write`, `filesystem-delete`, `process-exec`, `shell-exec`, `env-read`, `env-write`, `network-client`, `network-server`, `database`, `clock`, `random`)
  - `RuntimeCapabilityPolicy` with `trusted()` and `restricted()` modes
  - `capability_for_native_function(...)` mapping for host-effect native APIs
- Extended `Interpreter` (`src/interpreter/mod.rs`) with capability policy storage, setters/getters, and `require_capability(...)` checks.
- Enforced policy in native dispatch (`src/interpreter/native_functions/mod.rs`) before host-effect handlers execute.
- Added bypass hardening:
  - `spawn { ... }` child interpreter inherits parent capability policy.
  - async function execution interpreter inherits parent capability policy.
  - `http_server.listen()` method path enforces `network-server` capability.
  - `Image.save(...)` method path enforces `filesystem-write` capability.
- Updated VM integration (`src/vm.rs`):
  - `VM::set_capability_policy(...)`
  - capability checks for VM-only method-call paths (`__http_server_method_listen`, `__image_method_save`)
  - propagated policy into temporary VM used by HTTP handler wrappers.
- Added CLI controls in `src/main.rs` for `kujo run` and `kujo test-run`:
  - `--untrusted`
  - granular `--allow-*` flags
  - `--allow-net` convenience (client + server)
  - `--allow-all` trusted override
  - policy resolution: default trusted when no capability flags are set; restricted when `--untrusted` or explicit allow flags are used.
- Added/extended integration coverage in `tests/native_api_security_boundaries.rs`:
  - capability-denied errors across filesystem/process/shell/env/network/database/clock/random
  - success path for enabled capability
  - allow-only-requested granularity checks
  - VM/interpreter enforcement parity check
  - spawned-interpreter inheritance no-bypass check
- Updated docs and roadmap:
  - `README.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
  - `CHANGELOG.md`
  - `ROADMAP.md` (marked V1-SEC-002 complete)

## Gotchas (Read This Next Time)
- **Gotcha:** Method-call surfaces can bypass native dispatcher capability checks if they do host effects directly.
  - **Symptom:** `server.listen()` and `image.save()` could execute host-effect behavior without passing through central native-function capability mapping.
  - **Root cause:** These are method-call paths (`Expr::Call`/`Expr::MethodCall` + VM synthetic method markers) and not plain `call_native_function(...)` dispatch entries.
  - **Fix:** Added explicit capability checks at method-call boundaries in both interpreter and VM paths.
  - **Prevention:** When adding capability checks, always audit method-call helpers and VM synthetic method marker paths (`__*_method_*`) in addition to native dispatcher mapping.

- **Gotcha:** Spawned/async interpreters can silently reintroduce trusted defaults unless policy is explicitly propagated.
  - **Symptom:** Child execution contexts risk bypassing restricted mode and writing files/networking even when parent run is untrusted.
  - **Root cause:** `Interpreter::new()` defaults to trusted policy; child interpreters were created with fresh defaults.
  - **Fix:** Clone and pass parent policy into spawned thread interpreters and async-function interpreters.
  - **Prevention:** Treat child interpreter construction as a security boundary and always propagate runtime policy context.

## Things I Learned
- Capability enforcement needs to cover three surfaces: native dispatcher, interpreter method-call special cases, and VM synthetic method-call handlers.
- In this codebase, child runtime construction (`spawn`, async-function execution, temporary VMs) is a common policy-bypass vector unless policy state is propagated explicitly.
- CLI ergonomics can preserve compatibility (trusted default) while still providing a practical deny-by-default untrusted mode.

## Debug Notes (Only if applicable)
- **Failing test / error:** Initial capability tests failed with `unexpected argument '--untrusted'` until CLI flags were added to `run`/`test-run`.
- **Repro steps:** `cargo test --test native_api_security_boundaries`
- **Breakpoints / logs used:** Iterative failure-driven test runs; checked VM/interpreter method-call dispatch and child-interpreter constructors.
- **Final diagnosis:** Missing capability-aware CLI surface and non-dispatch method-call/child-context bypasses.

## Follow-ups / TODO (For Future Agents)
- [ ] Consider centralizing method-level capability metadata (for synthetic VM methods) into the same capability map used by native dispatcher.
- [ ] Evaluate whether default trusted mode should remain the long-term default for `kujo run` at 1.0, or whether a stricter default should be introduced behind a release-policy decision.

## Links / References
- Files touched:
  - `src/interpreter/capabilities.rs`
  - `src/interpreter/mod.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `src/vm.rs`
  - `src/main.rs`
  - `tests/native_api_security_boundaries.rs`
  - `README.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
  - `CHANGELOG.md`
  - `ROADMAP.md`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - `docs/NATIVE_API_SECURITY_POSTURE.md`
