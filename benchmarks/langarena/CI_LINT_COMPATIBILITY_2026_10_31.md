# CI lint compatibility follow-up

The loop candidate's CI run [41261332562](https://github.com/kujolang/kujo/actions/runs/41261332562)
failed Clippy on two baseline locations unchanged from accepted runtime `fd7b764`:

- `src/bytecode.rs`: newer Rust deprecates `AtomicU64::fetch_update` in favor of
  `try_update`. The [Rust API documentation](https://doc.rust-lang.org/std/sync/atomic/struct.Atomic.html)
  identifies the replacement as available from Rust 1.95, above Kujo's 1.89 MSRV.
  Keep the equivalent old spelling with a deprecation allowance scoped to that
  expression. Do not raise the MSRV or replace the atomic algorithm merely to
  accommodate a renamed API.
- `DictionaryManyPublication::contains_key` was only used by one unit assertion.
  Remove the unused production helper and check the missing key through `get`,
  the actual journal-aware read path. The existing ownership/snapshot test remains.

This branch is separate from the frozen loop candidate and accepted benchmark
binary. It makes no performance claim and does not change release gate settings.
Formatting and diff checks are local; MSRV, Clippy, and regression results must be
reviewed from this branch's CI before integration. No validation pass is implied
by committing the candidate.
