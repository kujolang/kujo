# Loop admission candidate: break-exit regression

Candidate `6d96c151e8b9f3d814043eebe5768ae5c61cde97` is **not accepted**.
Its frozen product binary (SHA-256
`dab54d3293041899e4605938ee4f3b00e5cfcd361a0e01d49ed010741c0d4396`)
passes 149 of 150 unchanged test-size benchmark combinations: VM 50/50,
interpreter 50/50, JIT 49/50. `Template::Parse` exits 4 with stack underflow;
it does not time out. Its count 10, iterations 2 and checksum 917360148
remain unchanged. Passing library and targeted tests did not catch this defect.

## Reproducer and cause

```kujo
func scan(text) {
    mut i := 0
    while i < len(text) {
        if i == 150 { break }
        i += 1
    }
    return i
}
assert_equal(scan(repeat("x", 300)), 150)
```

With whole-function JIT disabled, the candidate JIT reports
`Stack underflow in return`. VM and interpreter pass. The accepted baseline
passes all three modes; its JIT falls back for this loop. Removing the break
and expecting 300 passes all six baseline/candidate runtime combinations.
The full template workload also fails with whole-function JIT disabled.

The candidate trace shows normal condition exit at PC 23 (`Pop`), break exit
at PC 24 (`LoadLocal(1)`), and back edge at PC 22. Native compilation includes
PC 24, then resumes VM execution at PC 25 without publishing the loaded return
operand. The first post-loop instruction must not be consumed this way.
Different loop exit targets require their own operand-stack restoration and
VM continuation, including the normal condition's pending cleanup.

## Evidence and next gate

The companion compressed JSON bundle retains the complete three benchmark
reports, stopped matrix receipt, full-workload diagnostics and minimal
break/no-break controls with SHA-256 digests. Local source evidence lives under
`.audit-evidence/loop-condition-function-candidate-2026-10-31/`.

A repair is being verified separately on `codex/loop-break-exit-20261031`.
No repair is accepted by this report. Require targeted VM/interpreter/JIT
regressions, the full unchanged 150-checksum matrix and paired measurements
before accepting the loop optimization. No competitive speed claim follows.
Accepted runtime remains `fd7b7646e8adf99d1a798302ef9a18978e4c330d`.

The bounded sections 3/4 review is already complete; this loop admission work
belongs to section 5. General capture/collection residuals remain documented
in `SECTIONS_3_4_REVIEW_2026_10_31.md`.
