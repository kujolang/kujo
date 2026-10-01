# Stable toolchain atomic compatibility

CI run 36868620908 rejects three existing Atomic::fetch_update calls because
new stable Rust deprecates that spelling. Retain the operations and memory
orderings to preserve the declared Rust 1.89 MSRV. Scope deprecated allowances
to the three containing functions instead of suppressing warnings globally or
switching to an API unavailable on the minimum compiler.

The installation identity contract passed locally (one test). Latest-stable
Clippy and MSRV CI must pass before merging. Cargo registry publication remains
deferred; this is compiler compatibility for the existing runtime build.
