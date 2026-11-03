# JSON port state audit — partial

The existing JSON kernels pass **360 full-state observations** across VM,
interpreter and JIT. These are crafted valid-schema controls, not the registered
random inputs, production runs or a competitive timing comparison. No port or
runtime source changed.

Eight coordinate counts are covered: 1, 2, 3, 9, 10, 11, 32 and 128. The middle
three include the registered test-size counts. Inputs contain positive/negative
dyadic coordinates, quoted/Unicode/newline/backslash/tab names, and additional
valid option entries. Every case runs three consecutive kernel iterations.

| Kernel | Observations | Checked state |
| --- | ---: | --- |
| Json::Generate | 72 | Complete emitted JSON text is parsed independently with Ruby JSON and compared with the full input document; accumulated result checked. |
| Json::ParseDom | 144 | Complete parsed document, every coordinate component, accumulated checksum and input text preservation. |
| Json::ParseMapping | 144 | Complete parsed document plus every mapped struct's x/y/z values, accumulated checksum and input text preservation. |

Parsing controls start with results 0 and 4,294,967,294 to exercise checksum
wrapping. Generation starts at zero; generation-counter overflow is not tested.
The original parser kernel and a diagnostic clone both run on every parsing
case and must return the same checksum. The clone only exports its final parsed
document and coordinate collection after the existing checksum calculation.
This makes internal state observable without replacing the original kernel.

An independent verifier checks exact source/binary identities, all case IDs,
complete documents and coordinate matrices, raw-output hashes and checksums.
It computes averages from exact rational values of the dyadic inputs before the
documented seven-decimal checksum formatting. This avoids relying only on
agreement among Kujo's three execution modes.

## Source-level work difference

The pinned Rust mapping implementation deserializes into `Coordinates` containing
`Vec<CoordinateData>` with x/y/z fields. Goose's mapping reader reads those three
fields and explicitly skips other fields. Kujo first calls `parse_json` to build
the complete dynamic document, then traverses its coordinates to allocate a
second collection of `Coordinate` structs before computing averages.

Thus mapping work is not structurally identical: Kujo retains name/options/info
in the intermediate document and performs an additional mapping pass. The
controls confirm correct results for the tested valid inputs; they do not
quantify the extra allocations or elapsed-time cost. Any remedy needs a general
parsing mechanism with explicit schema/error behavior, rather than silently
omitting benchmark work or introducing a workload-specific shortcut.

## Boundaries and evidence

- Random generation/preparation and cross-language rounding remain unaudited by
  these controls. Equal coordinate counts do not mean identical registered inputs.
- JSON values are compared; identical serialization bytes or key order are not
  claimed. Malformed input, missing/wrong-type fields, empty coordinate parsing,
  numeric extremes, and production-size behavior remain open.
- Six Rust/Goose source files match pinned upstream
  `3c9f84ddaeec834c0631747cb789e8cfd602f993`. Those comparator implementations were
  inspected, not executed by this audit.
- Kujo uses the accepted loop-fix CLI with SHA-256
  `b7d95533dc4f3c6007a1596352dce4a8dabd6af1250e509547da7514dde296a4`.
  The separate memory candidate is not used. Native-path participation and speed
  are not inferred from the JIT flag. Runs overlap the no-JIT build and are
  correctness-only.
- The initial fixture exceeded the 8,192-character string-literal limit. The
  corrected collector assembles the same input from bounded literals; no input
  size was reduced. The initial setup failure remains archived.

`probes/audit-json-state.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT` reproduces the
controls. `json-state-audit-2026-11-03.json.gz` retains the collector, generated
fixture, inputs, full raw outputs, source identities, pinned comparator sources
and independent verification. All three structural-equivalence rows remain open.
