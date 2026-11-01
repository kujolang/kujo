# Performance-era unsafe contract review

Current checkpoint: JIT, VM and integration instrumentation boundary review is
complete. The strict JIT contract checker passes all 145 checked boundaries.
The inventory now records exactly 269 executable matches, including exactly 147
in JIT and explicit counts for VM and all ten integration files. The allocation
counter overflow fix has five isolated passing regressions. All three inventory
contract tests pass. The full release gate remains pending; schema/count checks
are not a universal memory-safety proof or runtime acceptance.
## Integration boundary review and explicit budget

Reviewed the ten integration files named in the inventory contract. The five
allocator wrappers forward original pointers/layouts to System, never dereference
allocation data, and keep accounting in const-initialized, non-dropping TLS Cells.
Counter updates now saturate. Native read/append fixtures retain names/literals
through encoding, keep root handles in paired activations and release snapshots.
Direct compiled-function/loop tests retain their JitCompiler and all context
storage throughout calls; scalar tags, name tables and local slots match their
constructed chunks. Callback tests use an independent stack and exclusive VM.
Malformed-call fixtures supply null or live initialized storage; nulls and counts
are rejected before dereference or handled with checked stack operations.
Missing call-site explanations in native_call_transfer were added.

The reviewed total is now exactly 269, with JIT exactly 147 (previously a ceiling
of 47), VM exactly 15, and exact counts for all ten new integration files. Existing
platform/process budgets remain unchanged. This updates the explicit baseline
after review, rather than removing the guard. Future count drift still fails.

Validation: rustc --test compiled the complete dependency-free
`tests/unsafe_inventory_contract.rs` with CARGO_MANIFEST_DIR set to this worktree;
all three tests pass, including deterministic generation and exact budgets.
Formatting, strict generation, the 145-boundary JIT checker and diff checks pass.
The full Cargo compatibility gate still must run on the committed revision.
