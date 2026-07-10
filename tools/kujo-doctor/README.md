# Kujo Doctor

Kujo Doctor is the first-party workflow extension for generic development-environment and repository readiness checks.

## Run

Current supported CLI surface:

- `kujo doctor`
- `kujo doctor --json`
- `kujo doctor --deep`
- `kujo doctor --list-profiles`
- `kujo doctor <profile>`
- `kujo doctor --profile <profile>`

Canonical workflow-pack path remains available:

- `kujo pack run doctor doctor`

## Output modes

- Human mode prints grouped checks, summary, and recommended next actions.
- JSON mode prints a stable machine-readable report contract (`schema_version: "0.1.0"`).

## What it checks

Generic baseline checks include:

- Tool availability/version signals (`git`, `node`, `npm`, `php`, `composer`, optional `wp`)
- Git repository detection, branch, and working-tree cleanliness
- Project dependency/config signals (`package.json`, `node_modules`, `composer.json`, `vendor`)
- npm script inventory and generic WordPress project signal detection

## What it does not do yet

- It does not run framework/vendor-specific profile checks by default.
- It does not implement remote registry-backed profile discovery yet.

## Profiles and extension model

`generic` is the default profile.

Future profile extensions are intended to support:

- `kujo doctor wordpress`
- `kujo doctor vercel`
- `kujo doctor astro`
- `kujo doctor acme`

See [docs/EXTENDING_DOCTOR.md](docs/EXTENDING_DOCTOR.md).

## Safety and process permissions

- Doctor does not write files.
- Doctor does not require network access.
- Doctor uses controlled process probes through Kujo's workflow process runner.

Release-style binary smoke commands and cross-platform handoff requirements are
in [docs/RELEASE_ARTIFACT_SMOKE.md](docs/RELEASE_ARTIFACT_SMOKE.md).

## Generic vs framework-specific doctor checks

Kujo Doctor's default profile focuses on generic, reusable readiness checks.
Framework-specific checks should ship as dedicated doctor profiles that extend generic output rather than replacing it.
