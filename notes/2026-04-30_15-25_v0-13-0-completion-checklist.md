# v0.13.0 Completion Checklist Evidence

Date: 2026-04-30

## Command/Test Evidence

Core verification commands run during release-cycle completion:

- `cargo test`
- `cargo test --test cli_json_contracts`
- `cargo test --test lsp_conformance_harness`
- `cargo test --test lsp_latency_guardrails`
- `cargo test --test editor_adapter_contracts`
- `cargo test --test tree_sitter_kujo_assets`
- `cargo build --release`
- `./target/release/kujo lsp --help`

## Completion Notes

- Language spec + compatibility policy published
- Official `kujo lsp` server entrypoint and lifecycle implemented
- LSP parity handlers implemented across required method set
- Machine-readable CLI/LSP contract tests and docs published
- Tree-sitter grammar baseline assets published
- Fixture-driven LSP conformance harness published
- Reliability/timeouts/cancellation/guardrails tests published
- Thin editor adapter baseline docs + smoke contract tests published
- Release artifact + install/upgrade docs published

## Deferred v1.0.0 Follow-Ups

- richer generic type system
- union types and enum method ergonomics
- macro/metaprogramming pipeline
- FFI and WASM target maturation
- expanded ecosystem docs and package registry hardening
