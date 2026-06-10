# V1H-UNSAFE-004 — Optional Unsafe Safety Gate

Date: 2026-05-22  
Checklist item: `V1H-UNSAFE-004`

## Objective

Add a repeatable optional nightly safety gate for unsafe-boundary verification with documented failure modes.

## Implementation

1. Added `scripts/unsafe_safety_gate.sh`:
   - Base gate:
     - `bash scripts/generate_unsafe_inventory.sh`
     - `cargo test --test unsafe_inventory_contract`
     - `cargo test --test vm_interpreter_parity_surfaces`
   - Optional Miri probe with `--with-miri`:
     - `cargo +nightly miri test --test vm_interpreter_parity_surfaces vm_and_interpreter_resolve_defined_identifiers`
   - Failure-mode exits:
     - `2`: unsupported arguments
     - `3`: missing nightly/Miri prerequisites when `--with-miri` is requested
2. Added `tests/unsafe_safety_gate_contract.rs` with:
   - help output contract
   - dry-run command emission contract (including Miri command)
   - unknown argument failure-path contract

## Commands Run

```bash
cargo test --test unsafe_safety_gate_contract
bash scripts/unsafe_safety_gate.sh
```

## Results

- `unsafe_safety_gate_contract`: pass (`3 passed, 0 failed`)
- `unsafe_safety_gate.sh`:
  - inventory generation: success
  - `unsafe_inventory_contract`: pass (`2 passed`)
  - `vm_interpreter_parity_surfaces`: pass (`86 passed`)

## Blocker Notes Recorded This Loop

- `V1H-UNSAFE-002`: blocked due high-churn manual `SAFETY:` annotation volume without checker support (`51` unsafe markers vs `3` existing `SAFETY:` comments).
- `V1H-UNSAFE-003`: blocked pending deterministic gate/checker foundation for safe wrapper reduction verification.
