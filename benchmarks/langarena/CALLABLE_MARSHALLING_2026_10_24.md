# Callable marshalling allocation isolation — 2026-10-24

Measured against default-feature debug runtime `aa30071` on macOS x86_64.
`probes/callable_marshalling_allocations.rs` constructs a native `to_int` operand
outside the counted interval, then measures `jit_stack_push_scalar` alone.
The VM stack has spare capacity; root creation and callable-name construction
are excluded. A second uncounted read proves the operand remains available
before explicit release and function unwind.

| Calls | Allocation requests | Requested bytes |
| ---: | ---: | ---: |
| 1 | 1 | 6 |
| 128 | 128 | 768 |
| 8,192 | 8,192 | 49,152 |

The helper clones the owned `Value::NativeFunction(String)` into the VM stack.
The ordinary generated `OpCode::Call` path subsequently releases its callable
and argument operands. This identifies a candidate for explicit consuming
marshalling at call sites; it does **not** justify changing the borrowing helper
used by other operations. Named operand loading separately clones the name and
is not measured by this probe. Neither this probe nor the QuickSort histogram
proves the CPU-time share of either clone.

A consuming path must validate current-frame ownership, preserve argument order,
handle invalid/stale roots without taking caller roots, transfer values exactly
once, and retain borrowed behavior at Dup/peek/resume sites. Native function and
loop execution, recursion, aliases, callbacks, field receivers and error cleanup
need regression coverage before integrating it. No runtime change is included
in this diagnostic. Release/all-50 verification of `aa30071` remains separate.

Reproduce after building the default-feature library (substitute its rlib path):

```sh
rustc --edition=2021 benchmarks/langarena/probes/callable_marshalling_allocations.rs \
  --extern kujo=target/debug/deps/libkujo-f2954510d2a1ce69.rlib \
  -L dependency=target/debug/deps -o /tmp/kujo-callable-marshalling-probe
/tmp/kujo-callable-marshalling-probe
```

All output identity, repeated-borrow and unwind assertions pass at all three
sizes. Counts are structural allocation evidence, not elapsed-time or RSS data.
