# Consumed native call operands — 2026-10-24

Runtime `4e42bd2`, compared with `aa30071`. Generated ordinary calls and
direct-argument fallback calls now transfer their popped callable and argument
operands to the VM stack. `jit_stack_take_scalar` validates current-frame root
ownership before moving values. Borrowed, duplicate, peek and resume operations
retain `jit_stack_push_scalar`; the borrowed API still permits repeated reads.
No public Value representation or callable-resolution contract changes.

The consuming helper checks stack availability before taking a root, rejects
stale/invalid identities and caller-frame roots, and returns an error status
before dispatch when transfer fails. Existing generated error cleanup handles
partially transferred suffixes. Native calls preserve order, receiver layout,
recursion and snapshot isolation. Immediate scalar transfer preserves exact
float bits, including signed zero and a NaN payload.

## Structural measurements

Default-feature debug library on macOS x86_64. The isolated marshalling probe
excludes callable construction and root encoding and preallocates stack capacity:

| Calls | Borrowed requests / bytes | Consuming requests / bytes |
| ---: | ---: | ---: |
| 1 | 1 / 6 | 0 / 0 |
| 128 | 128 / 768 | 0 / 0 |
| 8,192 | 8,192 / 49,152 | 0 / 0 |

The unchanged QuickSort kernel verifies every sorted element, unique backing and
saved snapshots at 128, 1,024 and 8,192 elements in VM and JIT. At 8,192, JIT:

| Input | Requests before → after | Requested bytes before → after |
| --- | ---: | ---: |
| Unique | 14,638 → 7,322 | 89,624 → 45,728 |
| Shared | 14,634 → 7,318 | 1,201,944 → 1,158,048 |

Unique input retains its backing with zero array-sized copies; shared input
copies once and preserves its snapshot. All earlier allocation budgets and the
new `--expect-consuming-call-budget` (fewer than 8,000 requests in both 8,192
JIT cases) pass. Named callable loading still allocates; this change removes the
subsequent marshalling clone only. Counts are not elapsed time or peak RSS.

## Verification

- 1,097 library tests pass; 8 intentionally ignored.
- New unit tests verify pointer-preserving transfer, stale/invalid roots,
  logical and physical caller-frame rejection, missing-stack behavior, snapshot
  isolation and exact immediate bits with no ownership frames.
- 40 targeted integration tests pass across call/field/loop boundaries,
  recursion and ownership. The extended VM/interpreter/JIT regression checks
  nested array forwarding and 180 recoverable call errors; the strengthened
  native-promotion check passes for conversion, nested arrays and failing calls.
- Formatting, diff checks and no-default-feature compilation pass.

Latest-candidate release/checksum/timing/peak-memory gates remain pending. The
150-case release audit at `aa30071` does not automatically certify this change.
Production, platform and controlled competitor-comparison gates remain open.
