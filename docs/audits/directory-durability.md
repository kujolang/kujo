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

## Linux handle correction

CI run 35912612913 exposed `EBADF` when syncing a nested directory on Linux.
The directory traversal capability can be an `O_PATH` handle on that platform;
macOS unit tests did not expose this distinction. The barrier now opens `.` for
reading relative to the retained directory capability, then syncs that readable
handle. It never reopens the original ambient path. The same root/nested sync
regression remains enabled and passed locally after the change; Linux CI run 35916711263 passed the minimal smoke and VM/interpreter
parity jobs, and ReaderSignal Linux run 35916802587 passed all 218 assertions. No assertion was weakened. Docker-based local
Linux verification was unavailable because the Docker daemon was not running.

## Generated evidence correction

The broader native CI run 35916711263 subsequently detected stale source-line
references in the unsafe and TODO inventories after builtin registration shifted
source locations. Regenerated the four Markdown/CSV files using the authoritative
`bash scripts/generate_v1_code_todo_triage.sh --strict` and
`bash scripts/generate_unsafe_inventory.sh --strict` scripts in a clean checkout
of d501c2c. Only line references and the unsafe inventory generation date changed;
no classifications or assertions were relaxed.

The corrected optimized build (`cargo build --release --locked`) completed.
ReaderSignal's same 218-assertion gate passed on macOS in 21.28 seconds, with
Linux independently verified by CI. This timing is a verification receipt,
not a performance comparison or power-cut simulation.

Generated artifact freshness: all three unchanged contract tests passed in the
isolated d501c2c checkout with the regenerated inventories (32.18 s). To avoid
rebuilding or testing concurrent unrelated edits in the shared checkout, the
existing contract source was compiled directly with `rustc --edition=2021 --test
tests/generated_artifact_freshness_contract.rs`, the existing compiled chrono
dependency, and `CARGO_MANIFEST_DIR` pointing to that isolated checkout; the
existing debug Kujo binary supplied the VM inventory runner. Receipt:
`evidence/directory-durability/generated-freshness.txt`. The same tests in the
shared checkout additionally detected source-line shifts in concurrent upgrade
tests; those unrelated changes were preserved, not included in this commit.
