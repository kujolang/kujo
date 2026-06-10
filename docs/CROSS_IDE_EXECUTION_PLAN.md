# Kujo Cross-IDE Execution Plan

This document defines the execution sequence for making Kujo first-class across editor ecosystems.

Principle: implement reusable language tooling first, then keep editor adapters thin.

## Outcomes

- One canonical Kujo language-server implementation that works across LSP-capable editors.
- Stable machine-readable CLI/LSP contracts that external tooling can trust.
- Universal syntax-highlighting path through a shared grammar.
- Reduced per-editor maintenance by avoiding duplicate language logic.

## Delivery Sequence

### Phase A (P0): Shared Contracts + Kujo LSP Server

1. Define and publish language/tooling contracts.

- Add a versioned language specification document.
- Define structured response contracts for diagnostics, symbols, edits, and errors.
- Add compatibility policy for contract evolution.

2. Promote current LSP CLI slices into a long-running `kujo lsp` server.

- Implement JSON-RPC transport and server lifecycle.
- Route diagnostics/completion/hover/definition/references/rename/code actions through handlers.
- Maintain deterministic behavior parity with CLI-based feature implementations.

3. Stabilize machine-readable CLI outputs.

- Ensure JSON output coverage for format/lint/docgen/LSP-facing commands.
- Add schema/snapshot tests and explicit exit-code contracts.

Definition of done:

- CI enforces contract fixtures for success and failure payloads.
- End-to-end protocol smoke tests pass in at least two editors.

### Phase B (P1): Grammar + Conformance

4. Deliver a Tree-sitter grammar.

- Create `tree-sitter-kujo` grammar and queries.
- Add corpus tests and CI validation.

5. Build protocol conformance suite.

- Add fixture-based LSP protocol tests for all core request types.
- Validate completion ordering and stable edit range behavior.
- Add regression tests for diagnostics and error payloads.

Definition of done:

- Grammar corpus tests pass in CI.
- LSP conformance tests gate releases for incompatible behavior.

### Phase C (P2): Thin Adapters + Ecosystem Docs

6. Publish adapter guidance and thin clients.

- Keep VS Code/Cursor adapters as launch/config layers only.
- Provide setup guidance for JetBrains (LSP plugin), Neovim, and other LSP clients.
- Standardize editor setup docs around one Kujo LSP configuration path.

Definition of done:

- Adapter implementations do not duplicate parser/type-analysis logic.
- New editor integrations reuse the same Kujo LSP contracts without language-core changes.

## Non-Goals

- Rewriting language intelligence per editor.
- Editor-specific feature forks that diverge from Kujo contracts.
- Unversioned ad-hoc output changes without compatibility handling.

## Release Tracking

- Source of truth for schedule/priorities: ROADMAP.md
- Release evidence: CHANGELOG.md
- This file: execution details and acceptance sequence
