# Nested dictionary boundary regression baseline

Accepted runtime `b1bf786` passes 108 semantic checks across VM, interpreter and
JIT, with 72 VM/JIT allocation observations. This supplements the existing
[nested payload isolation](NESTED_DICTIONARY_PAYLOAD_BASELINE_2026_10_30.md).

The 36 fixtures combine dictionary, struct-field and deeper dictionary owners
with insertion, dense updates and repeated overwrites, widths 8/128 and 1/8
calls. Each call retains a fresh snapshot. Insertion adds two collection-valued
keys; dense updates replace every entry with an array; repeated updates overwrite
two keys 64 times using array and dictionary payloads. Assertions check previous
snapshots, final values, lengths, untouched entries and the original leaf.

Allocation counters cover the entire fixture, including verification helpers
and snapshot checks. They are regression controls, **not isolated mutation
costs**, and do not establish elapsed-time speed or a new performance diagnosis.
In particular, width also changes verification work in these fixtures.

[Raw results and reproducibility sources](nested-dictionary-boundaries-2026-10-31.json)
include exact fixtures, runner/verifier sources, runtime/probe hashes, arguments,
stdout and counters. Independent verification checks exact matrix coverage,
runtime modes, source identities, successful outputs and counter consistency.

The pending root-dictionary candidate is not measured here. Refresh these exact
controls on its accepted artifact before implementing the nested follow-up.
The nested implementation must also prove actual journal promotion, bounded
capacity and value ownership through implementation tests. This baseline does
not close checklist sections 3 or 4.
