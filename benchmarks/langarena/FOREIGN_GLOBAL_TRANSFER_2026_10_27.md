# Foreign callback global transfer — 2026-10-27

Runtime commit `0cd6ed6` removes repeated global-map materialization during
synchronous global-only reentry through another module. The suspended private
globals move into the recursive activation and back on return. Public owner maps
retain real published values throughout callbacks; collection snapshots, caller
locals, namespace isolation and completed writes before errors remain intact.

The return destination is derived from the remaining activation stack. This
keeps capture modes compact and avoids increasing ordinary call-frame storage.
Tasks and captures containing lexical local scopes retain their existing paths.
Pool limits and public collection representations are unchanged.

## Measured allocation result

The [machine evidence](foreign-global-transfer-2026-10-27.json) records matching
release artifacts, source identities, paired controls and checksum results.
At depth four with 2,048 unrelated module bindings, VM and JIT agree:

| Foreign callback operation | Baseline requests / bytes per outer call | Candidate requests / bytes per outer call |
| --- | ---: | ---: |
| Read | 6,596 / 2,063,204 | 275 / 19,975 |
| Scalar write | 6,673 / 2,063,920 | 352 / 20,691 |

These deltas subtract 16-call from 32-call runs against identical fixture paths.
They measure allocation traffic, not elapsed speed, peak memory or first-call
setup. Candidate repeated-call traffic is flat across 0/128/2,048 unrelated
bindings at depths one and four. Direct and same-module helper controls retain
their previous allocation counts. All 36 cold/oversized comparisons and all
eight identical-path append controls match baseline requests and bytes exactly.

## Verification

- 1,146 library tests passed; eight existing ignored tests remain ignored.
- 301 relevant integration tests passed; one manual profiling test remains
  ignored. Explicit interpreter ordering checks also passed.
- All 150 test-size checksums passed: 50 each in VM, interpreter and JIT.
- Direct, shared-helper and foreign-callback routes each passed 72 candidate and
  72 baseline allocation rows, 108 semantic controls and 36 paired comparisons.
- Cold/oversized controls passed 72 paired allocation rows and 54 semantic cases.
- Module-owner controls passed 128 allocation and 192 semantic cases. Imported,
  captured-read, oversized-read and module-write boundary gates passed as well.
- Nine host-owner contracts and three foreign-reentry language contracts cover
  public publication, snapshots, same-named namespaces, errors and native `map`
  callbacks. Additional three-module release cases passed in all three runtimes.
- Formatting, whitespace checks and all 820 frozen source identities passed.

The new `probes/foreign-capture-allocations.rb` gate runs the existing callback
characterization and requires flat depth-one and depth-four setup. It accepts
this candidate and rejects the old baseline after the baseline's semantic
controls pass. Pass matching probe, matching binary and a new output directory;
an optional fourth baseline probe enables paired non-increasing allocation checks.

## Rejected draft

The first draft stored a suspended-frame index in the capture-mode enum. Its
callback allocation improvement passed, but larger frame storage added 32 bytes
to shallow cold controls and 96 bytes at depth four. All 36 cold comparisons
caught this regression. That draft was rejected before any runtime commit.
The final unit variant derives the destination on return and passes the same
gate with zero request or byte growth. The rejected measurements are retained in
the machine evidence.

A missing companion module initially interrupted the draft's append harness.
The fixture was restored, the original failure retained, and the gate rerun with
the identity-checked probe. That setup error was separate from the real cold
allocation regression; neither was treated as a passing result.

## Remaining work

Oversized retained locals, general captured-prefix materialization, publication
map traversal and mutable module collection-width copying remain open. This
change does not establish constant CPU cost or remove initial module setup.
The [retained-local baseline](LOCAL_CAPTURE_ISOLATION_2026_10_27.md) identifies the
next measured oversized-capture cost. Later compiled-execution work, unchanged
production-size runs, platform/release gates and clean-server Goose/Rust speed
comparisons remain pending. Loaded-laptop timings are diagnostic only.
