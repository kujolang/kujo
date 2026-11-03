# Brainfuck post-warmup samples

Frozen accepted macOS binary SHA-256
17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325.
Two separate processes, VM and JIT, completed 120 calls to the official warmup
program and its existing checksum assertion, emitted `profile-ready`, then ran
1000 additional calls. `/usr/bin/sample` captured five seconds at five-millisecond
intervals after the marker. Both samplers exited zero; both owned processes were
then deliberately terminated. These are partial profiling runs, not completed
benchmark/checksum runs or performance comparisons. No build overlapped sampling.

The VM worker had 848 sampling observations; the JIT worker 856. Top-of-stack
memmove observations were 114 and 151 respectively. The collapsed report includes
many allocator/free leaves (VM tiny_malloc_should_clear 28, free_tiny 26; JIT 41
and 51). VM __findenv_locked had 66 leaves; JIT 5. Two waiting wrapper threads
account for 1696/1712 ulock observations; do not use all-thread totals as a CPU
percentage. Sampling is statistical, on a loaded laptop, and the chosen program
is the warmup program, not the timed-out full production program.

The executable is stripped (`profile.release.strip = symbols`), so Kujo frames
are offsets rather than named helpers. These observations prioritize symbolized
copy/allocation and environment-lookup attribution; they do not prove which
helper, copying semantic, or debug flag causes the cost. Source inspection finds
repeated env reads in VM/debug paths, but caching them blindly could alter runtime
environment changes. No runtime fix is justified solely by this profile.

Raw sample trees, collector, generated source, outputs and identity receipt are
preserved in bf-post-warmup-sample-2026-11-03.json.gz. Local originals are in
/tmp/kujo-bf-sample-review. The next useful diagnostic is a symbol-preserving
optimized build and a sampled full-production phase, with JIT region coverage
recorded separately. Local disk (~605 MiB free) is insufficient for that build;
use CI/build-host capacity rather than deleting unrelated files.
