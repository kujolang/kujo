# v0.13.0 Tree-sitter Kujo Baseline Evidence

Date: 2026-04-30
Track: v0.13.0 Cross-IDE Foundation
Checklist item: Tree-sitter Grammar For Universal Highlighting

## Implemented

Added grammar package scaffold:

- `tree-sitter-kujo/package.json`
- `tree-sitter-kujo/grammar.js`

Added corpus fixtures:

- `tree-sitter-kujo/test/corpus/core.txt`

Added query files:

- `tree-sitter-kujo/queries/highlights.scm`
- `tree-sitter-kujo/queries/injections.scm`

Added CI guard test:

- `tests/tree_sitter_kujo_assets.rs`

Added documentation:

- `docs/TREE_SITTER_KUJO.md`

## Verification

Command:

- `cargo test --test tree_sitter_kujo_assets`

Result:

- PASS

## Notes

- Baseline adapter guidance in `docs/EDITOR_ADAPTER_BASELINES.md` references Tree-sitter highlight-query consumption path for editor integrations.
