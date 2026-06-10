# Command Extension Model

This document defines how Kujo core commands, workflow families, and external workflow packs coexist safely.

## Core command ownership

Core CLI commands are reserved and cannot be claimed by external packs or aliases.

Examples:

- `kujo run`
- `kujo check`
- `kujo pack`
- `kujo doctor` (reserved family)

## Reserved workflow families

Workflow families are top-level names owned by Kujo core and extended only through explicit contribution points.

Example family:

- `doctor`

## Profile contribution model

Workflow packs can contribute doctor profiles in `kujo-pack.yaml`:

```yaml
contributes:
  doctor_profiles:
    - name: wordpress
      entry: commands/doctor-wordpress.kujo
```

This contribution does not grant ownership of the `doctor` top-level command.

## Pack-local command model

Canonical pack-local execution path:

- `kujo pack run <namespace> <command>`

Example:

- `kujo pack run acme sprint-report`

## Optional alias model

Alias form remains supported for non-reserved namespaces:

- `kujo <namespace> <command>`

Alias routing rejects collisions with reserved top-level names and reserved namespaces.

## Collision rules

- External packs cannot claim reserved namespaces.
- External aliases cannot use reserved top-level names.
- External packs cannot spoof first-party reserved pack IDs.
- Duplicate namespace/command/profile registrations are rejected.

## Examples

- `kujo doctor`:
  - Reserved core family (future/first-party handling surface).
- `kujo doctor wordpress`:
  - Intended profile execution shape from contributed doctor profiles.
- `kujo pack run acme sprint-report`:
  - Safe canonical pack-local command execution path.
