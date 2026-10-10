# Kujo 1.8.1 maintenance release

Status: released on 2026-10-10. Native/source and npm published-install checks
passed across all five supported platforms. See the
[publication receipt](../release/kujo-1.8.1-publication.json).

This release contains the accumulated post-1.8.0 correctness, security,
reliability and efficiency fixes. It introduces no new language syntax.

## Correctness and compatibility

- CSV export accepts VM fixed dictionaries and orders first-row columns
  lexicographically. Duplicate input headers, later extra columns and unsupported
  compound cells fail explicitly instead of losing data. Missing/null cells,
  CSV escaping and secret redaction remain supported. Consumers must use column
  headers and explicitly project or serialize heterogeneous/compound rows.
- Collection fixes preserve distinct compound values in `unique`, invert supported
  integer dictionaries correctly, compare large mixed numeric sort keys exactly,
  and report integer `sum` overflow as a catchable runtime error.
- Scaled vector calculations preserve finite extreme/subnormal directions.
  JSON Schema numeric equality is exact, local references traverse arrays, and
  malformed schemas are checked independently of instance shape. Empty `enum`
  remains valid and matches nothing. Structural checks are resource-bounded.
- AI messages preserve names, assistant tool calls and tool-call IDs, including
  VM dictionary literals. Tool-loop replies retain their call linkage. Corrected
  named/tool histories require cassettes matching the corrected request bodies;
  plain-text history hash behavior is unchanged.
- Database pool admission is FIFO, with notification rather than polling.
  HTTP worker reservations account for queued connections.

## Security and process reliability

- Private-network restrictions apply to connected destinations and redirects,
  including IPv4-mapped IPv6. Strict HTTP clients disable ambient proxies.
- Compound operations enforce their required capabilities, including environment
  reads, source-file reads and GIF converter process execution. Restricted scripts
  must grant required effects explicitly; trusted defaults are unchanged.
- ZIP decoding is bounded and validated entries publish atomically. AI cassettes
  use private unique staging; shared-integer overflow leaves state unchanged.
- Subprocess deadlines include descendant-held output pipes. Windows commands own
  descendants through per-command jobs admitted before execution; failed job
  admission fails closed. GIF conversion has a 30-second deadline and 1 MiB per
  captured output stream, with explicit failure/truncation diagnostics.
- Hickory DNS dependencies advance to 0.26.3 for upstream corrections.

## Efficiency and diagnostics

- Context fitting avoids repeated traversal and avoids normalizing discarded or
  count-only messages; estimation, pruning and result contracts are preserved.
- Schema validation reuses at most 32 compiled patterns per call and uses direct
  property membership lookup. The cache never survives the call.
- Long invalid numeric strings use bounded Unicode-safe previews. TOML inputs
  above 512 bytes produce structured parser locations and bounded error messages
  without echoing the source document. Short diagnostics remain unchanged.

The three repository hardening audits contain reproduction tests and bounded
measurement claims. Debug timing probes are not production throughput promises.
The release preserves public builtin signatures, CLI options and exit classes,
JSON result shapes, configuration keys and environment variable names. Kujo
remains a local-code runtime, not a security sandbox.

## Distribution and next work

The published distribution covers five native platforms (Linux x64/arm64, macOS
x64/arm64, Windows x64), deterministic source/checksums and six npm packages.
Crates.io is optional and is not part of this publication.

Follow [roadmap items 2–6](../ROADMAP.md#2-native-api-representation-and-runtime-parity)
for 1.9. Wave C/D contracts and participant SDKs remain experimental; their
promotion is not implied by this maintenance release.
