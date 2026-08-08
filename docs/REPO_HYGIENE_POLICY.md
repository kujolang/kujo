# Repository Hygiene Policy

Status: Active
Last updated: 2026-06-09

## Purpose

Keep the repository root production-facing and predictable by ensuring only intentional project assets are tracked at top level.

For cross-repository Kujo tooling artifacts, use the inventory in `docs/KUJO_TOOL_ARTIFACT_IGNORE_INVENTORY.md` and the reusable ignore block in `config/kujo-tool-artifacts.gitignore`.

## Root Surface Contract

Tracked files at repository root must be limited to canonical project metadata and operator entry docs.

Current allowed tracked root files:

- `.editorconfig`
- `.gitattributes`
- `.gitignore`
- `AGENTS.md`
- `CHANGELOG.md`
- `CONTRIBUTING.md`
- `Cargo.lock`
- `Cargo.toml`
- `INSTALLATION.md`
- `LICENSE`
- `README.md`
- `ROADMAP.md`
- `install.sh`
- `rustfmt.toml`

## Non-root Placement Rules

Generated, local, and transient artifacts must not be tracked at root and should live under purpose-specific directories:

- Build/test outputs: `target/`, `tmp/`, `test_dir/`, `var/`
- Generated docs/artifacts: `docs/generated/`
- Local databases/backups/scratch files: untracked and ignored via `.gitignore`, and kept under `tmp/` or `var/` rather than the repository root
- Runtime/source implementation: `src/`, `tests/`, `examples/`, `scripts/`, `docs/`, `notes/`
- Disallowed root clutter patterns: root-level `blocked.txt`, `.DS_Store`, `*.db`, `*.sqlite*`, `*.zip`, `*.tar*`, `*.bak`, `*.backup`, `*.orig`, and extracted scratch directory names such as `scratch*`, `backup*`, `extract*`, and `unzipped*`, even when ignored

## Retention And Cleanup

- Keep local-only scratch artifacts untracked.
- If a temporary file is required for debugging, place it under `tmp/` or `var/` and do not commit it.
- Prefer nested temp workspaces under `tmp/` or `var/` rather than cluttering the repository root with archives, database files, or extracted bundles.
- Remove stale local backup/database artifacts before release-candidate verification runs.

## Enforcement

- Fast guard script: `bash scripts/repo_hygiene_audit.sh`
- Targeted root audit: `bash scripts/repo_hygiene_audit.sh --root <repo-root>`
- Contract test: `cargo test --test repo_hygiene_contract`
- Checklist reference: `ER-P0-005` in `docs/V1_0_ENTERPRISE_READINESS_ENHANCEMENT_CHECKLIST.md`
