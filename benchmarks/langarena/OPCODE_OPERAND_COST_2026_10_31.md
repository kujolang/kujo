# Scalar opcode operand extraction

The next dispatch experiment is justified at the mechanism level: copying an
opaque `LoadLocal`/`LoadConst` opcode before extracting its numeric operand costs
more than borrowing the same operand in this local control. Runtime source is
unchanged. This is not acceptance of a VM optimization.

| Path | Minimum | Median | Maximum |
| --- | ---: | ---: | ---: |
| Owned enum, ns/read | 8.427 | 8.759 | 9.777 |
| Borrowed operand, ns/read | 2.290 | 2.668 | 3.344 |

The probe uses 256 alternating local/constant instructions, eight paired rounds
with alternating order, and two million reads per observation (32 million total).
All 16 checksums match an independently computed operand sum. The collector
checks exact row count, ordering, reads, checksums, finite timing values and
unchanged source/library/executable hashes. Raw output, identities and ranges
are retained in [the result](opcode-operands-2026-10-31.json).

Both paths link against the accepted `fd7b764` library, SHA-256
`ffce3ed37b9cd1935ac3898ce042c354024c5b64e21148a255636dabde8bbd2a`.
Build with Rust 2021, `-O -C lto=thin`, `--extern kujo=LIBRARY` and
`-L dependency=target/release/deps`; then run
`ruby benchmarks/langarena/probes/measure-opcode-operands.rb BINARY LIBRARY OUTPUT.json`.
The first link without Rust thin LTO failed because Apple's LLVM reader could
not read the Rust dependency bitcode. The corrected build completed successfully;
no runtime build was restarted or changed.

## Limits and next gate

The owned path deliberately puts the cloned enum through `black_box`, preventing
clone elision. This isolates materialization cost, not the optimizer's actual
decisions in the large VM dispatcher. The borrowed path also uses an opaque
input. Both use non-inlined helper calls. The difference cannot be multiplied by
an instruction count to predict a language speedup. Laptop load varies, and
there is no clean-host or competitor comparison here.

Current main dispatch still clones these opcodes before matching. A candidate
can extract their scalar operands in the existing borrowed-metadata dispatch,
preserving instruction-pointer advancement before errors, value cloning,
constant conversion, cancellation polling and stack behavior. The callback
dispatcher has a separate clone site and must be covered explicitly if changed.
Before accepting any candidate, verify error/ownership behavior, all three
runtimes, and matched whole-workload measurements against the frozen accepted
runtime. This probe does not justify changing collection value semantics or
claiming zero regressions. No runtime regression suite was rerun for this
diagnostic-only change; repository formatting and probe checks passed.
