# ER-P2-002 — Developer onboarding quality pass

Date: 2026-05-25
Item: ER-P2-002

## Summary

Closed ER-P2-002 by adding a concise operator-facing onboarding cookbook and wiring it into the root documentation surface.

## Changes

- Added `docs/FIRST_TOOL_COOKBOOK.md`:
  - First practical tool walkthrough (`quality_gate.kujo`).
  - Deterministic CLI/exit-code conventions for automation.
  - VM-default execution guidance and extension patterns.
- Updated `README.md` Core Reference Links to include `docs/FIRST_TOOL_COOKBOOK.md`.
- Updated `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md` with completion and evidence bullets.

## Validation commands

- `cargo test --test docs_examples` (pass)
- `cargo test --test readme_contracts` (pass)

## Residual risk

- Cookbook examples are currently covered by parseability/docs contract suites; if runtime semantics for docs snippets become mandatory in future policy, add a dedicated run-mode docs smoke classification for cookbook scripts.
