# repo_gate.kujo - Web Project Audit CLI

A Kujo-native CLI tool that audits a web project before CI accepts a change.

## Overview

`repo_gate.kujo` validates project structure, enforces security policies, and produces deterministic JSON reports for CI integration.

## Installation

The tool is located at `examples/repo_gate/repo_gate.kujo`. No additional installation is required beyond the Kujo runtime. Run the commands below from the repository's `examples/` directory so the sibling module import resolves without environment configuration.

## Usage

### Basic Command

```bash
../target/debug/kujo run repo_gate/repo_gate.kujo -- --root ./project --policy repo_gate/gate-policy.json --json gate-report.json
```

### Output JSON to stdout

```bash
../target/debug/kujo run repo_gate/repo_gate.kujo -- --root ./project --policy repo_gate/gate-policy.json --stdout-json
```

### Command-line Options

| Option | Short | Required | Description |
|--------|-------|----------|-------------|
| `--root` | `-r` | Yes | Root directory to audit |
| `--policy` | `-p` | Yes | Path to policy JSON file |
| `--json` | `-j` | No | Output JSON report path |
| `--stdout-json` | `-s` | No | Output JSON to stdout |

## Policy Format

The policy file is a JSON document with the following structure:

```json
{
  "required": ["README.md", "package.json"],
  "forbidden_fragments": [".env", "id_rsa", "node_modules"],
  "max_file_bytes": 1048576,
  "hash_extensions": [".js", ".css", ".kujo"]
}
```

### Policy Fields

- **required**: Array of file paths that must exist in the project root
- **forbidden_fragments**: Array of path fragments that trigger failure if found
- **max_file_bytes**: Maximum allowed file size in bytes (default: 1MB)
- **hash_extensions**: Array of file extensions that should be SHA256 hashed

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Pass - all checks passed |
| 1 | Completed gate failure - missing required, forbidden path, or oversized file found |
| 2 | Usage/policy errors - invalid arguments, missing policy, parse errors |
| 4 | Incomplete runtime scans - permission/read errors during scan |

## Output Format

### JSON Report Schema

```json
{
  "schema_version": "1.0.0",
  "root": "./project",
  "policy_path": "gate-policy.json",
  "gate_passed": true,
  "incomplete_scan": false,
  "counts": {
    "files_scanned": 10,
    "findings": 0,
    "hashes": 5,
    "skipped": 0,
    "errors": 0
  },
  "findings": [],
  "hashes": [
    {"path": "src/main.js", "sha256": "abc123..."},
    {"path": "src/app.kujo", "sha256": "def456..."}
  ],
  "skipped_paths": [],
  "errors": []
}
```

### Human-readable Output (stderr)

```
=== Repo Gate Audit Complete ===
Root: ./project
Files scanned: 10
Findings: 0
Gate passed: true
```

## Security Features

1. **Path Normalization**: Rejects paths with `..` escape attempts
2. **Absolute Path Rejection**: Only relative paths within root are processed
3. **No Symlink Following**: Directory symlinks are not followed
4. **Deterministic Output**: Findings are sorted for reproducible reports

## Kujo-specific Implementation Notes

### Mutability and Array Helpers

Kujo arrays are immutable by default. When using helpers like `push`, `append`, or `sort`, you must reassign the result:

```kujo
# Correct - reassign result
mut arr := []
arr = push(arr, "item")
arr = sort(arr)

# Incorrect - does not modify in place
mut arr := []
push(arr, "item")  # Result is discarded
```

This is because Kujo's array helpers return new arrays rather than mutating the original. The `mut` keyword allows reassignment of the variable binding, but the array value itself is immutable.

### Filesystem Helpers

The `fs_helpers.kujo` module exports:

- `normalize_path(root, relative_path)` - Safe path construction
- `walk_files(root, current_relative, accumulator)` - Recursive file listing
- `get_file_size(path)` - File size in bytes
- `hash_file_sha256(path)` - SHA256 hash of file content
- `has_forbidden_fragment(path, fragments)` - Security check
- `has_extension(path, extensions)` - Extension matching
- `sort_strings(arr)` - Deterministic sorting
- `array_contains(arr, value)` - Membership check

## Running Tests

```bash
cd examples
../target/debug/kujo run repo_gate/repo_gate_test.kujo
```

## VM-first Command (Trusted Host)

```bash
../target/debug/kujo run repo_gate/repo_gate.kujo -- \
  --root ./project \
  --policy gate-policy.json \
  --json gate-report.json
```

## Capability-minimal Untrusted Command

When running in untrusted mode with minimal filesystem permissions:

```bash
../target/debug/kujo run repo_gate/repo_gate.kujo -- \
  --allow-fs-read ./project \
  --allow-fs-read gate-policy.json \
  --allow-fs-write gate-report.json \
  --root ./project \
  --policy gate-policy.json \
  --json gate-report.json
```

## Example Fixtures

The `fixtures/` directory contains test projects:

- `fixtures/passing_project/` - Valid project that passes all checks
- `fixtures/failing_project/` - Project with forbidden path fragment

## License

Part of the Kujo ecosystem. See main repository license.
