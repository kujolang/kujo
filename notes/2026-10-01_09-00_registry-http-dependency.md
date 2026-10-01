# Kujo Field Notes — Registry-safe HTTP dependency

**Date:** 2026-10-01
**Session:** 09:00 local
**Branch/Commit:** codex/crates-http-dependency / based on signed 813072040a1ac643312f5163fcfa4f26474c9095
**Scope:** Preserve patched HTTP behavior in normalized Cargo packages

---

## What I Changed

Gave the existing vendored HTTP patch the registry identity
`kujolang-tiny-http` 0.12.0-kujo.1, retaining the `tiny_http` library name,
upstream attribution and licenses. The root depends on that exact fork version
with a local path; Cargo normalization preserves the registry identity. Removed
only the tiny_http root patch override. HTTP implementation files are unchanged.

## Gotchas (Read This Next Time)

The dependency must be published before the normalized root package can resolve
it. This machine has no Cargo registry token; the owner has been asked to run
`cargo login` locally. Do not collect the token in chat. Do not overwrite or
retag 1.7.0: a runtime release containing this change needs a new signed version.

## Things I Learned

The full upstream suite initially hit a macOS cleanup error after the server
closed its socket. Its promptness helper now permits only NotConnected during
shutdown and still requires a nonempty response within the original deadline.
The new integration tests prove incomplete headers receive 408 before dispatch,
while complete requests retain peer identity and receive a successful response.

## Debug Notes (Only if applicable)

- Fork tests with ssl-openssl: 49 passed, 0 failed, including doctests.
- Fork normalized `cargo publish --dry-run --allow-dirty` with ssl-openssl:
  passed compilation and aborted upload as expected.
- Initial root `cargo check --locked` rejected the old lockfile as expected;
  offline resolution changed only the HTTP dependency identity.
- Root source compilation is a separate check; results must be verified before
  accepting the change. Registry-backed root packaging remains pending until
  dependency publication. No package publication is claimed by this note.

## Follow-ups / TODO (For Future Agents)

After local Cargo authentication, publish the reviewed dependency from a clean
commit, verify registry contents, rerun normalized root packaging, and prepare a
new signed runtime patch release. Preserve all HTTP deadline and body-bound
contracts. Native/npm 1.7.0 remains independently usable while this work proceeds.

## Links / References

- [Fork patch notes](../vendor/tiny_http-0.12.0/README.kujo.md)
- [Release policy](../docs/RELEASE_PROCESS.md)
- [1.7 registry exception](../docs/RELEASE_NOTES_1_7.md)
