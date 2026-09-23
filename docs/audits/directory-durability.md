# Directory durability barrier — ReaderSignal follow-up

Starting repository: `kujolang/kujo`, branch `main`,
`df2858643e3c9ca4f7e730344c15fae13da2a1cc`. The user explicitly requested the
upstream support required by ReaderSignal's outstanding power-loss durability
contract. The prior working tree was clean.

Added POSIX preview `sync_directory_beneath(root, relative_directory)`: open
managed components without following symlinks, retain the final directory handle,
and call its `sync_all`. Dot selects the explicitly trusted root. It creates,
publishes and removes nothing. Errors distinguish rejected paths from an
unconfirmed durability barrier. A preceding publication remains published even
when this independent barrier fails. The caller owns publication receipts,
transaction ordering, journaling and retry policy. Existing atomic-write APIs
and return formats remain unchanged.

Registered the builtin with VM/interpreter discovery, native enumeration, arity,
static types and `filesystem-write` capability gating. No new dependency.
Unsupported platforms fail explicitly. Root/ancestor trust is unchanged.
The standard-library reference and changelog document the contract.

Unit tests verify root/nested directory barriers, non-directory/path/symlink
rejection and injected sync failure without lost existing content. Integration
tests verify capability denial and successful VM/interpreter execution.
ReaderSignal tests exercise transaction interruption at eight publication/sync/
cleanup boundaries. SIGKILL is a process-crash experiment, not a hardware
power-cut experiment; the guarantee is conditional on OS/filesystem/hardware
honoring successful sync. Network filesystems or failing hardware are not newly
certified. Existing vendor tiny_http warnings were preserved.

## Verification receipt

All commands from Kujo. Complete output is in `evidence/directory-durability/`.

| Command | Result |
|---|---|
| `cargo fmt --check` | Passed after formatting touched Rust code |
| `cargo test --lib directory_sync_confines_paths_and_reports_failure` | 1 passed |
| `cargo test --lib beneath_tests` | 22 passed |
| `cargo test --test native_api_security_boundaries directory_sync_capability_and_runtime_parity` | 1 passed |
| `cargo test --test native_api_security_boundaries` | 90 passed |
| `cargo test --lib` | 912 passed, 7 existing ignored |
| `cargo test --test readme_contracts --test cli_contracts --test cli_json_contracts --test diagnostics_golden` | 52 passed |
| `git diff --check` | Passed |

Release build and downstream gate results are recorded in ReaderSignal's
`docs/audits/durability-and-backups.md`. No throughput improvement is claimed;
barriers intentionally add I/O to establish ordering.
