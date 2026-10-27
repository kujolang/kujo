# Oversized capture lookup storage — 2026-10-27

Accepted runtime `4f5087b` eliminates width-growing recursive reentry copies but
still rebuilds oversized outer activation maps. The private local maps contain
`Value::Null` placeholders; actual values stay in the public captured owner.
`Value` occupies 136 bytes on this measured macOS x64 build, so placeholder buckets
account for much of the remaining allocation traffic.

The accepted [runtime evidence](SPLIT_OWNER_TRANSFER_2026_10_27.md) already shows
outer-call writes requesting 41 allocations / 1,654 bytes through 4,096 locals,
8,240 / 2,721,367 at 8,192, and 16,432 / 5,447,495 at 16,384. These measurements
were reused rather than repeating the full runtime comparison.

## Fresh lookup-table component measurements

| Names | Placeholder map bytes | Key set bytes | Key/kind map bytes | Requests for each representation |
|---:|---:|---:|---:|---:|
| 0 | 0 | 0 | 0 | 0 |
| 128 | 42,274 | 7,458 | 9,506 | 129 |
| 2,048 | 678,842 | 121,786 | 154,554 | 2,049 |
| 4,096 | 1,358,778 | 244,666 | 310,202 | 4,097 |
| 8,192 | 2,718,650 | 490,426 | 621,498 | 8,193 |
| 12,288 | 2,761,898 | 533,674 | 664,746 | 12,289 |
| 16,384 | 5,444,778 | 988,330 | 1,250,474 | 16,385 |

This isolates fresh table/key storage. Setup and owner construction are excluded.
It is not a runtime optimization, elapsed-speed result or peak-memory measurement.
The same number of owned names is copied in every representation; that remaining
cost is not solved by reducing bucket payload.

An initial probe used actual `HashMap<String, Value>` null placeholders. An
independent three-way probe uses `ManuallyDrop<Value::Null>` (identical layout,
no owned payload to leak) and the actual runtime `BindingKind` type. Both probes
produce identical map/set request and byte counts at every width. Key-set membership,
null-placeholder and later owner-definition/removal assertions pass.
[Machine evidence](capture-key-layout-2026-10-27.json) preserves sources, build
arguments, library/probe hashes and all measurements. The committed storage probe
is [capture-key-layout.rs](probes/capture-key-layout.rs). Use the matching archived
release library and dependency flags with `rustc -O -C lto=thin`; omitting Rust LTO
failed to link the release bitcode with the installed Apple LLVM. That failed
build is not runtime evidence.

## Representation constraint and next implementation

The naive enum containing either the current live capture or an `Arc` key view
is 16 bytes, but its enclosing `Option` grows from 16 to 24 bytes on this build.
That would enlarge environments beyond the target workload. The
[layout probe](probes/capture-key-enum-layout.rs) records this rejected design;
it is not an integrated runtime regression.

The [candidate design](CAPTURE_KEY_VIEW_DESIGN_2026_10_27.md) considers reusing
existing private binding-kind storage for complete key/kind metadata. It must
preserve lexical membership snapshots, public owner maps, immutable guards,
mutations, nested captures, host callbacks, task journals and cleanup. Dense
mutation must not rebuild a second placeholder-value map and erase the saving.
Ordinary paths and enclosing environment layout need protection.

No runtime source changed. All-runtime semantic, matching release allocation,
regression and 150 checksum gates remain required before accepting an integration.
The broader performance checklist remains open; no Kostya contact occurred.
