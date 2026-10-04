# Kujo 1.8 release candidate notes

Status: candidate preparation; not yet tagged or published. Kujo 1.7.0 remains
the current stable release.

Kujo 1.8 is a **Smarter. Faster. Tighter.** release. It expands useful gradual
analysis, makes editor results safer and more reusable, removes measured runtime
cost, and strengthens cross-engine confidence without adding migration-heavy
syntax or a new concurrency model.

## Language improvements — Smarter

- Optional inference follows nested collection destructuring, known module
  exports, struct fields, async completion values and callable aliases.
- Struct generator methods work in both engines with explicit `self`, legacy
  field bindings, snapshot behavior, alias progress and cached terminal errors.
- Dynamic or unknowable values still fall back gradually; Kujo does not become a
  mandatory static type system.

## Runtime performance — Faster

- Generator resume reuses the continuation-owned bytecode chunk rather than
  cloning its instruction and constant vectors at every yield.
- In the interleaved runtime campaign, the nested-generator alias workload moved
  from a 727.596 ms baseline median to 667.238 ms candidate median, with a -6.7%
  paired median. A separate 20-run process-inclusive workload measured 4.512 s
  to 3.609 s means (-20.0%), but had 19–25% relative standard deviation and is
  retained as supporting evidence rather than a general speed claim.
- Closure, task admission, promises, channels, scheduling and cancellation were
  measured. No additional optimization was retained where results stayed within
  host variance or increased complexity without evidence.

## Compiler and VM — Tighter

- Stable longitudinal workloads now cover data/CLI processing, multi-file
  imports, generators, async/concurrency and a mixed application in both engines.
- The interpreter now permits lexical declarations to shadow preloaded native
  names, matching VM resolution; duplicate source declarations remain errors.
- Existing optimizer relocation and loop/early-return regressions remain the
  safety boundary. No new control-flow transform was added without a demonstrated
  hotspot.

## Developer experience — Smarter and Tighter

- CLI and LSP requests share immutable analyzed-program facts for diagnostics,
  hover and completion.
- LSP analysis uses unsaved module buffers and refreshes open direct and
  transitive dependents after edits. Closing or deleting a referenced buffer
  invalidates dependents back to disk state instead of serving stale exports.
- Imported-module parsing remains content-hashed and bounded, with explicit cold,
  warm and invalidation regression coverage.
- Release automation now produces a deterministic source archive alongside the
  five native archives and verifies its checksum and embedded version contract
  after GitHub publication. All six npm runtime packages remain lifecycle-script
  free and version-aligned.

## Compatibility

- Syntax changes: none in this stabilization tranche.
- Migration requirements: none expected for valid Kujo 1.7 programs.
- Intentional behavior correction: interpreter bindings may now shadow native
  function names, as the VM already allowed.
- Runtime lifecycle, cancellation, capability, lexical snapshot and effect-order
  contracts are unchanged.
- Async generators, `yield from`, ownership cycles, atomic shared-state
  primitives, generics, macros, broad FFI and a major WASM target remain deferred.

Tagging, native/npm publication and public installer promotion are intentionally
not performed by candidate preparation.
