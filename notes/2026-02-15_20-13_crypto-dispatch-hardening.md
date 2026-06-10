# Kujo Field Notes — Crypto Dispatch Hardening Closure

**Date:** 2026-02-15
**Session:** 20:13 local
**Branch/Commit:** main / 71fd5d2
**Scope:** Closed the next v0.10.0 P1 release-hardening modular dispatch gap by migrating declared crypto/hash APIs into `native_functions/crypto.rs`, adding contract tests, and updating release docs.

---

## What I Changed
- Implemented modular native dispatch handlers in `src/interpreter/native_functions/crypto.rs` for:
  - Hashing: `sha256`, `md5`, `md5_file`
  - Password APIs: `hash_password`, `verify_password`
  - AES APIs: `aes_encrypt`, `aes_decrypt`, `aes_encrypt_bytes`, `aes_decrypt_bytes`
  - RSA APIs: `rsa_generate_keypair`, `rsa_encrypt`, `rsa_decrypt`, `rsa_sign`, `rsa_verify`
- Preserved legacy-compatible error and validation behavior:
  - Arity/type validation errors via `Value::Error`
  - Runtime crypto failures via `Value::ErrorObject`
  - Existing AES base64 + nonce prefix conventions
- Added comprehensive crypto tests in `src/interpreter/native_functions/crypto.rs`:
  - deterministic hash vectors + file hash behavior
  - bcrypt hash/verify true+false behavior
  - AES string and bytes round-trips
  - RSA keygen/encrypt/decrypt/sign/verify behavior
  - argument-shape and key-size validation checks
- Updated release-hardening dispatcher contracts in `src/interpreter/native_functions/mod.rs`:
  - Added crypto APIs to critical non-fallback coverage list
  - Removed migrated crypto APIs from expected known legacy gaps
  - Added crypto argument-contract hardening test
- Updated release docs:
  - `CHANGELOG.md`
  - `ROADMAP.md`
  - `README.md`

## Gotchas (Read This Next Time)
- **Gotcha:** Hardcoded digest vectors are easy to get wrong unless verified externally.
  - **Symptom:** Crypto unit test failed even though implementation was correct.
  - **Root cause:** Expected SHA-256/MD5 strings for `"kujo"` were incorrect in the test case.
  - **Fix:** Recomputed vectors with shell tools (`printf kujo | shasum -a 256` and `printf kujo | md5`) and updated assertions.
  - **Prevention:** Always verify hash vectors with a trusted external tool before locking tests.

- **Gotcha:** Dispatch-gap migrations must update both critical coverage and known-gap ledger.
  - **Symptom:** Drift-guard expectations become stale when migrated APIs are only added in one list.
  - **Root cause:** `critical_builtin_names` and `expected_known_legacy_dispatch_gaps` serve different but coupled contracts.
  - **Fix:** Added crypto APIs to critical coverage and removed them from known legacy gaps in the same change.
  - **Prevention:** Treat these two list edits as an atomic migration rule.

## Things I Learned
- Crypto APIs are stable to migrate module-by-module when behavior-level tests are added first-class in the module itself.
- In this codebase, release-hardening confidence comes from both targeted API contract tests and exhaustive drift-guard ledger maintenance.
- RSA contract tests are practical at 2048-bit in native-function unit tests and provide strong end-to-end migration confidence.

## Debug Notes (Only if applicable)
- **Failing test / error:** `test_sha256_and_md5_hashes_match_known_values` failed due to mismatched expected vector strings.
- **Repro steps:** `cargo test native_functions::crypto::tests:: --quiet`
- **Breakpoints / logs used:** External digest verification via shell (`shasum`, `md5`).
- **Final diagnosis:** Test expectations were wrong; handler implementation was correct.

## Follow-ups / TODO (For Future Agents)
- [ ] Continue release-hardening by closing the next declared dispatch gap cluster (`network` and/or archive `zip_*` APIs).
- [ ] Keep exhaustive drift-guard known-gap list synchronized with each migration commit.

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
  - `.github/AGENT_INSTRUCTIONS.md`
