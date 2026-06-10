# V1H-SEC-004 — html_response threat-model docs

Date: 2026-05-23
Item: V1H-SEC-004

## Summary
Documented explicit XSS threat-model boundaries for `html_response(...)` and added actionable safe usage guidance so operators and app authors can avoid propagating untrusted HTML without escaping.

## Documentation updates
- `docs/NATIVE_API_SECURITY_POSTURE.md`
  - Added `4.2.1 HTML Response Threat Model (html_response)` section.
  - Clarified that Kujo does not sanitize/escape response bodies.
  - Added defensive guidance and a script-level `escape_html` helper example.
- `README.md`
  - Added static-server behavior callout: `html_response(...)` emits raw output and requires caller-side escaping for untrusted content.

## Validation
- `cargo test --test security_posture_docs_contract` ✅ (2 passed)
- `cargo test --test readme_contracts` ✅ (1 passed)
- `cargo test --test native_api_security_boundaries` ✅ (48 passed)
- `cargo test --test runtime_security` ✅ (9 passed)

## Notes
- No runtime behavior changes were introduced in this loop.
