# Extending Kujo Doctor

## Goal

Doctor profiles let packages add contextual checks while preserving the generic baseline:

- `kujo doctor wordpress`
- `kujo doctor vercel`
- `kujo doctor acme`

## Profile contribution shape

Current workflow-pack manifest contribution block:

```yaml
contributes:
  doctor_profiles:
    - name: wordpress
      summary: WordPress-specific readiness checks.
      entry: commands/doctor-wordpress.kujo
```

## Generic + profile composition

Preferred behavior:

1. Run generic Kujo Doctor checks.
2. Run selected profile checks.
3. Merge checks and actions into one report.

Profile checks should extend generic checks, not replace them.

## What belongs where

Generic doctor should include:

- language/toolchain presence/version checks
- repository state checks
- common dependency/config signals

Profiles should include:

- framework/runtime-specific checks
- vendor tooling checks
- environment assumptions unique to that profile domain

## Collision safety

- Do not register top-level command aliases for reserved core names.
- Use doctor profile contributions instead of command-family ownership.
- Keep namespaces/package IDs non-spoofable and trust-scoped.

## Planned improvements

- richer profile trust/identity model
- registry-backed distribution/discovery
- explicit profile capability declarations
