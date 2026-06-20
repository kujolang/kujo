# v1.0 Stale Critical Notes Cleanup Evidence

Date: 2026-06-19

## Summary

Closed `V1RR-P1-001` for the named stale critical-note surfaces. The
historical notes remain available for provenance, but each now has a current
status header that prevents fixed or superseded failures from being read as
active v1.0 release blockers.

## Files Updated

- `docs/IMAGE_CONVERSION_AGENT_HANDOFF.md`
- `notes/bug_dict_index_assignment_hangs.md`
- `notes/MUTATION_OPERATOR_BUG.md`
- `docs/V1_0_RELEASE_READINESS_GAP_CHECKLIST.md`

## Current Evidence

- Image conversion: `tests/image_conversion_integration.rs` covers PNG -> WebP,
  JPEG -> WebP, and WebP -> PNG round trips in interpreter and VM mode, plus
  missing input, unsupported output extension, and invalid argument failures.
- Dict index assignment: `tests/vm_interpreter_parity_surfaces.rs` covers
  successful local, nested, and captured map updates in interpreter and VM mode.
- Assignment and mutation syntax: `docs/LANGUAGE_SPEC.md` documents `:=`, `=`,
  and compound assignment operators, with `let`/`mut`/`const` mutability rules;
  `tests/language_spec_contracts.rs` verifies the mutable and immutable paths.

## Live Reproduction Checks

Both historical mutation repros now complete instead of hanging:

- VM: `mut i := 0; while i < 5 { i = i + 1 }; print(i)` printed `5`.
- Interpreter: `mut i := 0; while i < 5 { i = i + 1 }; print(i)` printed `5`.
- VM: `mut d := {"x": 0}; d["x"] = 5; print(d["x"])` printed `5`.
- Interpreter: `mut d := {"x": 0}; d["x"] = 5; print(d["x"])` printed `5`.

## Validation

See `notes/release_evidence/2026-06-19_p1-001/status.tsv` for command output
paths and exit codes.
