# Struct literal publication boundaries

The next struct-literal optimization must preserve field evaluation order, callback
publication, retained owner snapshots and errors. The committed probe checks those
contracts for root and nested dictionary owners in VM, interpreter and JIT.

All six executions pass on accepted runtime `d853be4` and all six pass on the
uncommitted nested-replacement candidate (CLI SHA-256
`57e5e1727be30e49ece56d01a45d52f96feba0c43145d4db71f6390b61f9f662`).
The generated owner and main sources match between runs. Binary identities,
coverage, source hashes, successful exits and exact output were independently
checked. [Raw results](struct-literal-boundaries-2026-10-29.json) preserve each run.
This is semantic baseline evidence, not acceptance of either pending optimization.

The checks cover snapshots captured by fields before and after a mutating callback,
a failing field that must leave the preceding successful write intact, and literal
fields exposing the owner before subsequent replacement. They do not measure
allocation or elapsed speed; use the separate
[struct construction baseline](STRUCT_REPLACEMENT_BASELINE_2026_10_29.md) for costs.

Reproduce with a fresh output directory:

```sh
ruby benchmarks/langarena/probes/struct-literal-boundaries.rb /path/to/kujo /path/to/new-output
```

Keep this contract when admitting struct construction through the publication
boundary. Runtime source remains unchanged by this baseline commit.
