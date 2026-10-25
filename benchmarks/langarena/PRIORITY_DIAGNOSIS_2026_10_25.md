# Measurement foundation and priority diagnosis — 2026-10-25

Scope: checklist sections 1 and 2. No runtime implementation fixes in this round.
The checklist is `PERFORMANCE_CHECKLIST.md`; the reconciled machine ledger is
`performance-plan.json`. Full completion of sections 1 and 2 is **not claimed**.

## Measurement foundation

The surviving `target/release/kujo` matches the verified `ea0ea3e` candidate:
SHA256 `36ae7ad967dfafbb125701ec97153a66e669c6d86e757b000e41d27a7e858505`.
The port, test and production configurations match their saved comparison hashes.
Original baseline `f037705`, Goose `ea1f368c109c083a2161580ff1d74c70200b6cc8`,
and LangArena `3c9f84ddaeec834c0631747cb789e8cfd602f993` remain pinned in the
historical evidence and Linux bootstrap. Their old temporary executables have
been removed, so no fresh original-versus-candidate/competitor ranking is claimed.

The user confirmed that no dedicated Linux host is available. Local observations
are diagnostic only. Continuous telemetry during the first pass recorded CPU
speed limits between 22 and 64 percent. This is a reported system limit, not a
measurement of actual clock speed and not a formula for correcting elapsed time.
Builds were completed before their associated diagnostic runs.

The ordinary CLI produced checksum-valid VM/JIT observations for nine workloads,
with phase checkpoints, runtime counters and external RSS. Allocation counting
and CPU sampling use separate probe invocations. The probe links the current
release library and initializes the same constant globals as the CLI. Its binary
and execution entry differ from the CLI; their elapsed times are not comparable.

An initial harness omitted the CLI's `null` global. CacheSimulation/TextRaytracer
failed in that harness while ordinary CLI checks passed. Those failing probe
observations were rejected; the harness was corrected and the probe pass rerun.
An initial shell empty-array issue on macOS Bash was also corrected. Neither is
a runtime regression or a benchmark pass.

## Verified local observations

The strict audit passed 54 main observations (nine workloads × VM/JIT × three observation types) and all 820 checksum repetitions in ten sustained samples. These are diagnostic repetitions, not additional benchmark cases.

| Workload | VM allocation requests | JIT allocation requests | VM requested bytes | JIT requested bytes |
| --- | ---: | ---: | ---: | ---: |
| Sort::Quick | 23180164 | 932809 | 1925197820 | 111908887 |
| Sort::Merge | 50694818 | 11436362 | 5411959563 | 2938981477 |
| Etc::NeuralNet | 40495373 | 17805318 | 2494414398 | 1557934731 |
| Etc::CacheSimulation | 895181 | 715228 | 534330903 | 565788942 |
| Etc::GameOfLife | 237413 | 212523 | 14848514 | 67230946 |
| Brainfuck::Array | 310974 | 313328 | 31808378 | 95399753 |
| Brainfuck::Recursion | 325053 | 289057 | 33089785 | 76867544 |
| Etc::TextRaytracer | 28651657 | 8819947 | 1995794752 | 465657759 |
| Base64::Encode | 698037 | 454595 | 62228153 | 97112823 |

Counts cover the whole VM execution, including initialization, warmup, compilation and checksums; they do not isolate kernel allocations. Different JIT compilation work is included, so these totals alone do not establish a runtime speedup.

## Findings and next experiments

1. **VM dispatch and metadata copying:** QuickSort, MergeSort and NeuralNet
   samples expose dispatch, `OpCode::clone`, value movement and allocator work.
   `VM::execute_dispatch` clones an instruction before dispatch. Next isolate
   payload-bearing versus scalar instructions and measure allocation/scaling.
   A future optimization must preserve chunk lifetimes, jumps and reentrancy.
2. **Environment lookups in hot paths:** sorting samples include
   `__findenv_locked`; runtime code checks debug/JIT environment switches in call
   and dispatch paths. Next isolate disabled-debug lookup cost. Do not simply
   cache all environment variables forever: runtime environment-change behavior
   and policy propagation must remain correct.
3. **JIT sorting boundaries:** indexed reads, native-root encoding/detachment,
   argument/call bridging, hashing and value movement appear in samples. Next
   use existing indexed-read/call/root probes to isolate each mechanism and test
   unique versus shared collection owners before choosing a change.
4. **NeuralNet:** JIT field access, value movement and allocation are observed.
   The earlier apparent VM regression remains unconfirmed without a repeated
   original/candidate comparison. These profiles do not prove its cause.
5. **Small-workload JIT costs:** ordinary counters record native entries for all
   nine JIT cases, but nontrivial compilation time for the small cases. Native
   entries prove execution occurred, not that the whole kernel was native or
   that compilation paid off. Repeated short-workload probes provide longer
   sampling windows while preserving each individual checksum.

6. **CacheSimulation:** sustained VM and JIT samples both put `Value::nesting_depth` first in the top-of-stack summary, followed by hashing and table growth. Next isolate validation traversal from required dictionary work and compare unique/shared owners; preserve depth enforcement.
7. **GameOfLife and Brainfuck:** sustained VM samples expose instruction cloning and dispatch; JIT Life exposes field reads/root encoding, while both Brainfuck variants expose operand loads, roots and hashing. These are attribution targets, not evidence that dispatch or ownership checks can be removed.
8. **Base64 control:** sustained JIT samples include Cranelift legalization and VM/native dispatch. Repeated entry calls include setup and compilation, so this is not steady-state kernel coverage.

These observations justify targeted investigation; they are not estimates of
the speedup available from removing any one mechanism. Stack samples are short,
and inclusive compile counters are not percentages of wall time.

TextRaytracer also exposes VM allocation/dispatch and JIT field/root operations in its accepted samples; isolate vector/helper boundaries before selecting a change.

## Remaining gates

- Reconstruct and freeze the original/competitor binaries; run seven rotating
  comparisons on a quiet host with continuous resource telemetry.
- Confirm or reject the NeuralNet VM regression against the original baseline.
- Separate allocation traffic by setup/warmup/kernel phase and attribution site.
- Isolate required copy-on-write detachment versus accidental extra ownership.
- Establish scaling and small reproducers for the specific proposed fixes.
- Attribute native/fallback **time**, rather than converting event counts into
  an unsupported execution-coverage percentage.
- No production checksum, final competitive ranking or universal speedup follows
  from this diagnosis. Sections 3–5 remain for a separately authorized fix round.

## Evidence

See `priority-diagnosis-2026-10-25.json` for raw outputs, input/build identities,
allocation totals, CPU samples, runtime counters and host telemetry. Reproduction
and measurement boundaries are documented in `probes/PRIORITY_DIAGNOSTICS.md`.
All accepted observations require exact checksums and successful exits; failed
or interrupted observations must remain outside the accepted result set.
