# Kujo 1.9.0 release candidate

Version 1.9.0 prepares an additive runtime release. The published stable runtime
remains 1.8.2 until signed native artifacts, npm packages, and their installation
checks pass. Crates.io publication is not included in this release cycle.

## Language and library additions

- Checked nested array, dictionary, and struct assignment with atomic failed writes.
- Strict UTF-8 conversion and checked byte writes, including snapshot-safe byte iteration.
- Stable fallible comparator sorting, regex replacement callbacks and match counting.
- Headerless CSV records, fixed decimal formatting, and explicit wrapping arithmetic.

Existing sort, CSV, and checked arithmetic APIs remain available. Callback errors
propagate, callback capabilities remain enforced, and retained values keep their
snapshot semantics.

## Compatibility and performance

VM/interpreter index behavior is aligned: finite floating-point read indices follow
the documented conversion, while invalid simple writes and dictionary keys fail.
Imported interpreter recursion is bounded at 256 calls; the callback bridge keeps
its separate limit. See the changelog for the individual compatibility fixes.

Storage reuse, shared bytecode, callback metadata and string caches reduce copying
without changing retained aliases or failed-write atomicity. Numeric accumulators
retain JIT compilation alongside the speculative string-append optimization.

LangArena fixtures and recorded measurements are evidence for their exact workloads,
not a universal throughput or production-readiness guarantee. Existing Wave C beta,
Wave D alpha and unpublished participant SDK boundaries remain unchanged.

## Distribution and evidence

The intended native targets are Linux x64/arm64, macOS x64/arm64 and Windows x64,
with npm platform packages wrapping those same verified executables. Node packaging
requires the patched Node versions declared in each manifest. No install-time
binary download hook is introduced.

The pre-version source ff7e8b6fb197ba8dca8f8dc9e272a29792834d66 passed the full hosted
release gate in run 39820688222. Candidate-source and published-artifact evidence
must be recorded separately before this candidate is promoted as stable.
