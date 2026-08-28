# Proposed implementation roadmap

This sequence is proposed for a later implementation mission; none of it was executed here.

## Stage A — Instrument

Add observe-only component accounting at the AI SDK/Agents SDK boundary. Capture hashes, sources, load reasons, cacheability, exact usage, estimates, retries, and redaction classification.

## Stage B — Remove proven duplication

Use ledger evidence to remove only identical or semantically redundant payloads. Preserve artifact references and tests.

## Stage C — Progressive disclosure

Introduce versioned skill/context manifests and deterministic fetch operations. Fail closed when required context is unavailable.

## Stage D — Typed handoffs and state

Add reference-based handoffs and compact resume state with source-hash validation. Preserve full artifacts outside the model-visible envelope.

## Stage E — Tool and repository retrieval

Add scoped tool catalogs, schema expansion on demand, structural repository indexes, and freshness checks.

## Stage F — CI ratchet and dispatch verification

Freeze normalized payload/component baselines, add approved-growth workflow, and prove live role/tool/skill dispatch with receipts.

## Stage G — Language/runtime changes

Only after the measured task corpus shows repeated Kujo boilerplate is a material cost and an additive stdlib/generator change preserves readability, diagnostics, safety, and compatibility.

Rollback for every stage is a feature flag or adapter-level fallback to the existing payload/state path, with old artifacts remaining readable.
