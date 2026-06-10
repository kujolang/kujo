# Kujo Field Notes — v0.12 Linter Initial CLI Slice

**Date:** 2026-04-29
**Session:** 22:57 local
**Branch/Commit:** main / a781f0f
**Scope:** Implemented the next highest-priority incomplete v0.12.0 track (Linter) with initial planned-rule coverage, safe autofix support, tests, and release evidence updates.

---

## What I Changed
- Added src/linter.rs with initial lint rules for:
  - unused variables
  - unreachable code after control-flow terminators
  - obvious annotation/literal type mismatches
  - missing error-handling patterns for selected fallible calls
- Added safe autofix support for selected rules:
  - unused-variable underscore-prefix rewrite on declaration lines
- Added CLI command in src/main.rs:
  - kujo lint <file> [--fix] [--json]
  - prints rule/severity/location/message in plain mode
  - emits structured issue payloads in JSON mode
- Added module declarations in both crate roots:
  - src/main.rs (mod linter;)
  - src/lib.rs (pub mod linter;)
- Added focused tests in src/linter.rs for:
  - unused variable detection + safe fix
  - unreachable code detection
  - obvious type mismatch detection
  - missing error-handling pattern detection

## Gotchas (Read This Next Time)
- **Gotcha:** Regex literals with embedded quotes are easy to mis-specify in Rust string syntax.
  - **Symptom:** Compilation failed with tokenization errors in linter regex declarations.
  - **Root cause:** Incorrect escaping pattern in regex string literals.
  - **Fix:** Switched to correctly escaped standard string literals and explicit compile-expect messages.
  - **Prevention:** Prefer explicit escaped standard strings when regex needs embedded quote characters and backslashes.

## Things I Learned
- A practical first linter slice can cover roadmap intent without full type-checker coupling by using conservative syntactic and token-based checks.
- Safe autofix should be narrow and deterministic in early versions; broad edits are better deferred until stronger semantic confidence exists.
- JSON output is critical to make linter results consumable for editor tooling and CI post-processing.

## Debug Notes (Only if applicable)
- **Failing test / error:** Initial build failed due malformed regex string literals in type-mismatch checks.
- **Repro steps:** cargo test linter -- --nocapture and cargo build.
- **Breakpoints / logs used:** rustc parse error output in src/linter.rs.
- **Final diagnosis:** incorrect backslash/quote escaping in regex string declarations.

## Follow-ups / TODO (For Future Agents)
- [ ] Add scope-aware unused-variable analysis for nested declarations and destructuring patterns.
- [ ] Improve unreachable-code detection with parser/AST block precision.
- [ ] Expand safe autofix coverage with rule-specific confidence gates.

## Links / References
- Files touched:
  - src/linter.rs
  - src/main.rs
  - src/lib.rs
  - ROADMAP.md
  - CHANGELOG.md
  - README.md
- Related docs:
  - notes/FIELD_NOTES_SYSTEM.md
  - notes/GOTCHAS.md
  - notes/2026-04-29_22-53_formatter-initial-cli.md
