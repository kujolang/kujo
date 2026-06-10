# Kujo Field Notes — Release Hardening Crypto Strict-Arity Contracts

**Date:** 2026-02-16
**Session:** 10:15 local
**Branch/Commit:** main / 9ad6c74
**Scope:** Expanded v0.10.0 P1 release-hardening compatibility contracts for crypto/hash builtins by enforcing strict arity (no silent extra-argument acceptance), extending dispatcher-level contract coverage, and synchronizing roadmap/changelog/readme.

---

## What I Changed
- Hardened runtime crypto builtin arity in `src/interpreter/native_functions/crypto.rs` for:
  - `sha256`, `md5`, `md5_file`
  - `hash_password`, `verify_password`
  - `aes_encrypt`, `aes_decrypt`, `aes_encrypt_bytes`, `aes_decrypt_bytes`
  - `rsa_generate_keypair`, `rsa_encrypt`, `rsa_decrypt`, `rsa_sign`, `rsa_verify`
- Preserved existing error-text contracts while adding explicit `len()` guards to reject extra arguments deterministically.
- Expanded native crypto module contract tests in `src/interpreter/native_functions/crypto.rs`:
  - `test_crypto_argument_validation_contracts` now includes strict extra-argument checks.
- Expanded dispatcher-level release-hardening crypto contracts in `src/interpreter/native_functions/mod.rs`:
  - `test_release_hardening_crypto_module_dispatch_argument_contracts` now includes strict extra-argument checks.
- Updated docs to reflect this hardening follow-through:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Positional argument matching (`arg_values.first()`, `arg_values.get(1)`) without explicit arity checks silently accepts extra arguments.
  - **Symptom:** APIs appear argument-validated but still tolerate unexpected trailing args.
  - **Root cause:** Pattern matching verifies required positions only; it does not enforce exact count.
  - **Fix:** Add explicit arity guards (`arg_values.len() != N`) before type matching.
  - **Prevention:** For release-hardening slices, always pair type-shape checks with strict arity checks for public builtins.

## Things I Learned
- Keeping the pre-existing error message text while tightening arity lets us harden behavior without broad test/doc churn.
- Dispatcher-level contract tests in `native_functions/mod.rs` are the best place to guard API stability against drift, while module-level tests ensure local native behavior stays strict.

## Debug Notes (Only if applicable)
- **Failing test / error:** None (no failures after implementation edits)
- **Repro steps:** N/A
- **Breakpoints / logs used:** N/A
- **Final diagnosis:** Strict arity gap only (no deeper runtime bug)

## Follow-ups / TODO (For Future Agents)
- [ ] Apply the same strict-arity hardening pattern to remaining public builtin groups that still rely on positional matching without `len()` guards.
- [ ] Keep dispatcher-level tests in `src/interpreter/native_functions/mod.rs` and module-local tests synchronized when tightening argument contracts.

## Assumptions I Almost Made
- I initially assumed existing argument-shape tests already implied strict arity, but many handlers only validated required positions and tolerated trailing arguments.

## Validation Snapshot
- Focused tests:
  - `cargo test test_crypto_argument_validation_contracts -- --nocapture`
  - `cargo test test_release_hardening_crypto_module_dispatch_argument_contracts -- --nocapture`
- Full suite:
  - `cargo test` (green)

## Commits
- `9ad6c74` — `:ok_hand: IMPROVE: enforce strict crypto builtin arity contracts`
- *(docs commit pending in this session at note time)*

## Links / References
- Files touched:
  - `src/interpreter/native_functions/crypto.rs`
  - `src/interpreter/native_functions/mod.rs`
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`
- Related docs:
  - `notes/README.md`
  - `notes/GOTCHAS.md`
