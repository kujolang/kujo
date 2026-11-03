# Template full-state controls

Both template implementations match an independent model in 342 complete
state/checksum observations across the accepted VM, interpreter and JIT modes.
This is bounded Kujo state evidence, not executed Rust/Goose equivalence,
production validation or native CPU/time coverage.

## Coverage

For each of Template::Regex and Template::Parse:

- Seven generated counts: 0, 1, 2, 3, 8, 17 and 100. Compare the complete prepared
  text, variable dictionary, rendered text, accumulated result and final checksum.
- Twelve boundary inputs: empty text, ordinary/missing/empty variables, adjacent
  substitutions, single braces, unmatched opening braces, nested/extra braces,
  newline-containing placeholders, Unicode and literal prefix/suffix text.
- Three successive runs per state, including a near-u32-wrap initial accumulator
  in boundary cases. Every full state is compared, not just rendered length/hash.

There are 38 input states, 114 transitions per runtime and 342 total observations.
Three altered-render controls leave the checksum untouched and are rejected by
full-state equality. The initial fixture embedded all JSON in one string and hit
Kujo's documented 8,192-character literal limit before execution. That failure is
retained. The corrected fixture reads the unchanged case JSON file; no lexer,
language or benchmark implementation change was made.

## Source comparison and limits

Rust and Goose template sources were byte-checked against upstream commit
`3c9f84ddaeec834c0631747cb789e8cfd602f993` and retained. Their generated ASCII input
construction, name/city cycles, comment cadence, literal single braces and result
accumulation correspond to the modeled preparation. The sources were inspected,
not executed by this diagnostic.

All three implementations distinguish regex matching from manual parsing: the
regex's dot does not match newlines by default, while the manual parser can find
closing delimiters across newlines. Missing keys contribute an empty string.
The model preserves that distinction; it does not force both variants to produce
the same boundary output.

Representation/work differences remain. Rust's parser reserves output capacity;
Goose uses byte buffers and copies the rendered buffer into state. Kujo uses
string indexing, substring creation and concatenation through its runtime.
Kujo has cached string length/ASCII access and cached regex construction, so the
source does not justify claiming repeated full-string length scans or regex
compilation on every call. Allocation/call/work counts still require measurement.

For non-ASCII literal text, Rust's manual parser casts each UTF-8 byte to a char;
Kujo indexes Unicode characters and Goose preserves bytes. The source therefore
indicates a boundary difference outside generated ASCII workloads. These Kujo
Unicode controls are not asserted to match Rust. Execute comparator fixtures
before making a cross-language full-state claim. Repeated prepare calls, wider
limits, exact competitor outputs, production counts and resource/work comparison
remain open.

## Provenance

Accepted CLI SHA-256:
`17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`.
The worker-reuse candidate is not the executable used here. Port source fragments
are unchanged; binary, inputs, source and fixture hashes are checked before/after.
Raw output, input/expected states, source snapshots, comparator files, failed
fixture and scripts are in `template-state-2026-11-03.json.gz`.

Reproduce with `probes/audit-template-state.rb FROZEN_KUJO PINNED_UPSTREAM NEW_OUTPUT`
and `probes/verify-template-state.rb NEW_OUTPUT`. No speed score follows.
