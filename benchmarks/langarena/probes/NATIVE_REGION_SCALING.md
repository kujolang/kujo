# Native read-region scaling controls

`native-region-scaling.kujo CASE WIDTH READ_GROUPS verify|measure` isolates a
function performing eight repeated reads per loop iteration. Cases are
`integer-array`, `negative-array`, `float-array`, `bytes`, `integer-dictionary`
and `string-dictionary`. Dictionary names describe their source key types;
specific internal representations require separate inspection.

Setup, warmup and assertions are outside the timed kernel. Verification mode
does not read the clock. The measured kernel returns an asserted numeric sum;
the caller retains and checks a snapshot. This is a diagnostic, not an official
LangArena workload or competitor score. Compilation/native-region participation
must be established separately before attributing its timings to native code.

The accepted CLI (`3d6b3ea8…`) and first revised release CLI (`acf5569d…`) pass
all six cases in VM, interpreter and JIT at width 8 and 128 read groups:
**36/36 correctness controls**. These ran during an unrelated integration build;
no timing result was collected or claimed. Full hashes, exact invocations and
stdout/stderr are retained in
`../array-region-fallback-controls-2026-11-02.json.gz`.

Before accepting the array-view experiment, measure fallback paths as well as
the three target LangArena workloads. Use fixed binary/source identities,
sequential rotating pairs and widths 8/256/8192, with enough groups to separate
kernel work from one call's setup cost. Include both uninterrupted array reads
and the non-array fallback cases. Compare VM controls separately from JIT.
Keep build activity stopped and preserve all failed or incomplete outcomes.

This probe does not replace callback/mutation/error fixtures, allocation/root
counts, the compiled integration test, or all-50 checksum and production gates.
