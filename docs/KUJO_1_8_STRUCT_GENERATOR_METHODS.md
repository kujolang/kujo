# Kujo 1.8 Struct Generator Methods

Status: implemented on the 1.8 development line.

## User-visible contract

Structs may declare generator methods with the existing `func*` syntax:

```kujo
struct Counter {
    value: int,

    func* emit(self, count) {
        mut offset := 0
        while offset < count {
            yield self.value + offset
            offset += 1
        }
    }
}
```

Calling `Counter.emit` creates an ordinary lazy generator. The receiver is
snapshotted at invocation, so rebinding the variable that held the struct does
not change the continuation. Generator aliases share progress. `return`
completes without yielding a value, and a terminal error is cached for later
observers.

Legacy methods that omit `self` continue to see receiver fields as direct
bindings. Those fields are copied into the generator's owned lexical
environment at invocation. Resumer-local bindings cannot leak into that
environment.

## Compatibility

- Syntax: no new syntax; an existing rejected `func*` method form is now valid.
- Runtime behavior: programs using struct generator methods now execute instead
  of receiving the former unsupported-feature error.
- Ordinary struct methods and top-level generators are unchanged.
- VM and interpreter use the same external arity rule: explicit `self` is not a
  caller-supplied argument.
- Capability authority is still the intersection of generator creation and
  resumption policy. No cancellation, rollback or async-generator contract was
  added.

## Implementation and regression evidence

The compiler lowers a generator method to the existing bytecode generator
function representation. VM method dispatch supplies the receiver through the
existing method-call argument path. The interpreter records generator methods
in the struct definition and creates an owned continuation with either explicit
`self` or a snapshotted legacy field scope.

Focused coverage includes:

- explicit-`self` and legacy-field success paths in both engines;
- receiver snapshot behavior and aliases sharing progress;
- early `return` completion and terminal-error caching;
- caller restoration after terminal failure;
- precise generator-method arity failures;
- existing generator capability, lazy iteration and continuation-lifetime
  contracts, which are shared by method generators.

Async generators and `yield from` remain deferred. Struct generator methods do
not reopen generator ownership, task scheduling or the v1 closure snapshot
contract.
