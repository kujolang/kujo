# Indirect retained-local recursion — 2026-10-27

The accepted runtime `ce923d1` still copies retained local capture maps when
recursion reenters through a foreign module. All 144 allocation observations and
216 semantic runs pass their fixture assertions. This characterizes a remaining
cost; it does not establish optimization acceptance.

[Machine evidence](indirect-local-isolation-2026-10-27.json) preserves executable,
probe and generator identities, generated modules, sources and raw results.
[Runner instructions](probes/INDIRECT_LOCAL_GATES.md) describe reproduction.

## Measurements

Requested allocation per added outer call at depth four, computed from the
16-call and 32-call runs. VM and JIT agree for every measured group.

| Extra retained locals | Operation | Requests | Requested bytes |
| ---: | --- | ---: | ---: |
| 0 | read | 365 | 26010 |
| 0 | write | 390 | 26381 |
| 128 | read | 749 | 150852 |
| 128 | write | 774 | 151223 |
| 2048 | read | 6509 | 2060556 |
| 2048 | write | 6534 | 2060927 |
| 8192 | read | 41345 | 13620640 |
| 8192 | write | 41370 | 13621011 |

The full matrix covers 0/128/2,048/8,192 locals, read/write operations,
depths zero/one/four and one/16/32 outer calls. Every extra local is read and
checked by a final call. The foreign module increments a separate counter;
asserting exactly depth × calls verifies that callback reentry actually occurs.
Aliases share captured state, and independently created closures stay separate.

Depth zero separates ordinary activation from recursive entry. These fixtures
differ from the direct-recursion fixtures, so cross-route constants alone are
not a measured runtime patch delta. Any fix must compare identical paths using
matching baseline/candidate artifacts. Elapsed timings on the busy laptop and
peak memory are outside this measurement.

## Next verification requirements

Reproduce reduced indirect-entry scaling with paired artifacts while preserving
all no-recursion controls, direct-prefix transfer, private caller parameters,
public-owner visibility, immutable kinds, escaped snapshots, caught errors and
foreign-module state. Test VM, interpreter and JIT. Keep task and indirect
fallback ownership intact and verify cold/append and all-50 checksum controls.
Ordinary oversized activation and the rest of the performance checklist remain
open. No runtime code changed for this characterization.
