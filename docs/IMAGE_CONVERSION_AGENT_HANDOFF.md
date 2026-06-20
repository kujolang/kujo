# Image Conversion Agent Handoff

## Current Status - Superseded For v1.0 Release Readiness

As of 2026-06-19, this handoff is historical context, not an active
release blocker. The original `img.save(...)`/`img.resize(...)` dispatch
failure has been superseded by current implementation and tests:

- `tests/image_conversion_integration.rs` covers PNG -> WebP, JPEG -> WebP,
  and WebP -> PNG round trips in both interpreter and VM execution paths.
- The same test file covers missing input paths, unsupported output
  extensions, and invalid image method argument types.
- `notes/2026-04-29_17-02_image-method-dispatch-parity.md` records the
  implementation session that closed the VM method-dispatch parity gap.

Keep the rest of this file only as the implementation handoff that led to
the current behavior. Do not treat the "Verified Current Behavior" section
below as current release status.

## Background

This handoff captures the current state of image conversion support in Kujo and defines the implementation plan for enabling reliable format conversion (for example, JPEG/PNG to WebP).

A review of the repository found:

- Kujo has an image dependency (`image = "0.25"`) and image value types.
- Kujo exposes `load_image(...)` and has code paths that appear to support image methods like `save`, `resize`, `crop`, etc.
- Examples currently claim format conversion support, including WebP.

However, runtime behavior shows a mismatch between documented behavior and actual method dispatch in the active execution paths.

## Verified Current Behavior

### What works

- `load_image(...)` returns an image value type.

### What does not currently work reliably

- Method calls on image values (for example `img.save(...)`, `img.resize(...)`) fail in normal usage paths.

Observed runtime failures include:

- `Unknown method: save` (interpreter mode)
- `Unknown method: resize` (interpreter mode)
- `Cannot access field on non-struct` (VM mode for method syntax)

## Root Cause Summary

There is a dispatch split:

- Parser emits `Expr::MethodCall` for `obj.method(...)` syntax.
- Active `Expr::MethodCall` flow routes through a generic `call_method(...)` handler that currently focuses on iterator-like methods.
- A separate image-method handling block exists elsewhere, but it is not being used by the active `Expr::MethodCall` path.
- In VM mode, method calls are compiled through generic field access/call mechanics that do not handle image values as method-bearing objects.

Result: image methods are present in code but not consistently reachable through the normal execution pipeline.

## Goal

Enable reliable image method support, including format conversion via:

- `img := load_image("input.jpg")`
- `ok := img.save("output.webp")`

across both:

- `kujo run file.kujo` (VM path)
- `kujo run --interpreter file.kujo` (tree-walk interpreter path)

## Scope

### In scope

- Fix image method dispatch for active method-call execution path.
- Ensure parity between interpreter and VM behavior.
- Add real integration tests for format conversion.
- Align examples/docs with real executable behavior.

### Out of scope

- Building a brand new image API surface.
- Large refactors unrelated to method dispatch.
- Non-image method system redesign unless required for parity.

## Implementation Instructions

## 1) Unify image methods in the active method-call path

Implement image method handling directly in the active `call_method(...)` execution path used by `Expr::MethodCall`.

Required image methods:

- `resize(width, height [, mode])`
- `crop(x, y, width, height)`
- `rotate(degrees)`
- `flip(direction)`
- `save(path)`
- `to_grayscale()`
- `blur(sigma)`
- `adjust_brightness(factor)`
- `adjust_contrast(factor)`

Implementation notes:

- Reuse existing logic where possible to avoid behavior drift.
- Keep argument validation and error text consistent.
- Return values should match current conventions (`Value::Image` for transforms, `Value::Bool(true)` or typed error for save).

## 2) Remove duplicate/unreachable image method logic

After method handling is unified:

- Remove or refactor legacy/duplicate branches that are no longer used.
- Prefer one canonical implementation for image method behavior.

## 3) Add VM parity for image methods

In VM mode, `obj.method(...)` currently flows through generic field access semantics.

Implement one of:

- Dedicated image method opcodes/native dispatch in method-call compilation, or
- Extended field/call handling that recognizes image method dispatch similarly to existing special-case objects.

Requirement:

- Image method results and errors must match interpreter behavior closely.

## 4) Add integration tests that execute real conversions

Add tests that perform actual file-based conversions end-to-end.

Minimum required test cases:

- PNG -> WebP
- JPEG -> WebP
- WebP -> PNG

Each test should verify:

- Input file loads successfully.
- Save returns success.
- Output file is created.
- Output file is non-empty.
- Optional: output can be reloaded with `load_image(...)`.

Also include failure-path tests:

- Missing input path
- Unsupported output extension (if applicable)
- Invalid argument types

Important:

- Avoid tests that only print expected behavior.
- Use deterministic fixtures.

## 5) Update example and docs for truthfulness

Update `examples/image_processing.kujo` and related docs so claims match runtime reality.

If WebP is supported after implementation, keep and validate those examples.
If any format is not supported on all targets, document caveats explicitly.

## Acceptance Criteria

All of the following must be true:

1. `img.save("out.webp")` works in interpreter mode.
2. `img.save("out.webp")` works in VM mode.
3. Conversion tests pass in CI/local test command.
4. No stale duplicate image-method dispatch paths remain.
5. Docs/examples reflect actual supported behavior.

## Suggested Work Order

1. Implement interpreter `call_method(...)` image handling.
2. Run targeted interpreter tests/scripts.
3. Implement VM dispatch parity.
4. Add integration tests.
5. Update examples/docs.
6. Run full test suite.

## Validation Checklist

- Run interpreter smoke script using a real image fixture.
- Run VM smoke script using the same fixture.
- Run targeted image tests.
- Run full test command before final handoff.
- Confirm no regressions in non-image method calls.

## Handoff Notes for Next Agent

When reporting back, include:

- Exact files modified
- Why each change was needed
- Test commands executed
- Key output snippets proving conversion works
- Any unresolved caveats (platform codec support, feature flags, etc.)
