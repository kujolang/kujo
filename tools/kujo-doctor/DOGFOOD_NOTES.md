# Dogfood Notes

## Tool being built

Kujo Doctor

## What was easy

- Reusing workflow-pack `DoctorReport` and renderer pipeline for human/JSON output.
- Adding structured `reason` and `category` fields to check generation.
- Reusing centralized safe process execution wrappers.

## What was awkward

- Bridging reserved core command UX (`kujo doctor`) with workflow-pack registration mechanics.
- Representing default/generic profile behavior while preserving profile-contribution extensibility.
- Keeping compatibility fields (`pack`, `namespace`) while improving machine-facing schema metadata.

## Missing language/runtime/stdlib capabilities

- Native semantic version parsing helper (major/minor/patch extraction).
- Safer built-in helpers for warning/deprecation/fatal text classification.
- Standardized structured process result adapters for common tool version probing.

## Missing workflow-pack capabilities

- Built-in profile entry execution path parity (non-script profile hooks).
- Profile contribution trust metadata/signature model.
- First-class `doctor` family dispatcher at workflow layer (instead of CLI-side orchestration glue).

## Recommended core/stdlib improvements

- Add semver parser utility in stdlib/runtime helpers.
- Add structured process probe helper API (`probe_tool`, `extract_version`, `classify_noise`).
- Add explicit manifest schema validation/testing for contribution blocks.
- Add workflow-pack capability metadata for profile-level process allowlists.

## Priority

- High:
  - first-class doctor profile dispatcher in workflow system
  - semver parsing utility
- Medium:
  - structured tool-probe helper APIs
  - stronger profile trust metadata
- Low:
  - richer human renderer grouping controls for profile-extended reports
