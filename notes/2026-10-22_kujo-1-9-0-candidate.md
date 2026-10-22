# Kujo 1.9.0 candidate evidence

The user authorized the full Kujo release cycle. This is an additive minor release:
new checked nested writes, bytes/UTF-8 APIs, comparator sorting, regex callbacks,
CSV records, fixed formatting and wrapping arithmetic accompany runtime fixes.

Pre-version source ff7e8b6fb197ba8dca8f8dc9e272a29792834d66 passed hosted run
39820688222: full release gate, VM/interpreter parity, minimal smoke, strict lint,
format and MSRV. A local Rust build was stopped under disk pressure; it is not
claimed as a local full-gate pass.

On Node 26.12.1, candidate metadata passed release-state consistency, roadmap-only
readiness, all 12 npm tests, six-package pack dry-run, installer release-manifest
contract and whitespace verification. Public stable strings and installer remain
at published 1.8.2 until actual native/npm publication and installation checks.

Candidate-source CI is still required before a signed tag. Native artifacts,
checksums, npm publication, all-platform install/upgrade verification and public
consumer promotion remain mandatory. Crates.io publication is outside this cycle.
Historical benchmark limits and experimental ecosystem boundaries are preserved.
