# Remaining dictionary scalar publication costs

Accepted runtime: `82bdef6021de5ebf7961be58e912e1309fd2d518`. This is a
baseline measurement, not a runtime fix or an elapsed-speed comparison.

The same existing dictionary key is assigned an integer, float, Boolean, null,
or a repeating mixture of those types. Widths are 2, 128 and 2,048 keys. Each
case runs 16 and 32 helper calls, with either an initial retained snapshot or a
fresh snapshot retained across every call. Original snapshots and untouched
keys are asserted in VM, interpreter and JIT.

| Value assigned | Added-call bytes, 128 keys | Added-call bytes, 2,048 keys |
| --- | ---: | ---: |
| Integer | 4,806 | 4,806 |
| Float | 44,081 | 631,601 |
| Boolean | 44,081 | 631,601 |
| Null | 44,085 | 631,605 |
| Mixed scalar types | 34,263.25 | 474,903.25 |

These are once-snapshot results. Added-call bytes are the allocation difference
between 32 and 16 calls divided by 16. VM and JIT deltas agree. The mixed case
averages a complete four-type cycle; its fractional result is expected.
Fresh-snapshot controls preserve their required copying and are included in the
raw evidence. No timing conclusion is drawn on the loaded laptop. Absolute
counts are specific to these fixtures; do not combine them with counts from a
different probe.

All 120 allocation measurements and 180 semantic checks completed. The
[raw evidence](dictionary-scalar-isolation-2026-10-27.json) records artifact
identities, exact fixture text and hashes, the full matrix and computed deltas.
Coverage, source hashes and delta arithmetic were independently verified.

The confirmed source restriction is in `Environment::try_defer_dictionary_write`:
it admits only `Value::Int`. `DictionaryPublication` retains one existing key
and an `i64`, so other scalar types materialize the dictionary. The integer
control demonstrates that a width-independent path already exists for eligible
ownership. Supporting other types must retain value semantics and the current
capture-frame size; simply enlarging the inline publication payload is not an
accepted solution. Different-key and deeper-path updates remain separate open
work.

Reproduce with the [runner](probes/dictionary-scalar-isolation.rb):

```sh
ruby benchmarks/langarena/probes/dictionary-scalar-isolation.rb \
  ACCEPTED_ALLOCATION_PROBE ACCEPTED_KUJO_BINARY NEW_OUTPUT_DIRECTORY
```

The allocation probe must match the runtime library and build configuration;
its recorded SHA identifies the probe used here. This characterization does not
close the broader performance checklist, production-size verification, platform
checks or clean-host Goose/Rust comparison.
