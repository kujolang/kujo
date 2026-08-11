# Kujo Ecosystem Installation

The supported onboarding path is the repository's [`install.sh`](../install.sh). It installs Kujo and the local-first tooling needed to turn an ordinary agent request into scoped work, focused context, implementation, verification, and a durable handoff.

## One-command install

After the public Kujo release is tagged:

```bash
curl -fsSL https://raw.githubusercontent.com/kujolang/kujo/main/install.sh | bash -s -- --ref v1.0.1
export PATH="$HOME/.local/bin:$PATH"
kujo --version
```

Until release artifacts exist, use a source build explicitly:

```bash
curl -fsSL https://raw.githubusercontent.com/kujolang/kujo/main/install.sh | bash -s -- --source
```

The installer uses user-owned directories by default and does not require `sudo`:

| Location | Purpose |
| --- | --- |
| `~/.kujo/sources/` | Source snapshots for the selected tools |
| `~/.kujo/install.json` | Selected ref, profiles, and install paths |
| `~/.local/bin/` | Kujo and stable command shims |

Use `--prefix` and `--bin-dir` to change these locations. Use a release tag or commit ref for reproducible onboarding; `main` is the development default.

If the GitHub repositories are still private during a pre-launch test, provide a short-lived GitHub token through the environment rather than putting it in a URL:

```bash
export KUJO_GITHUB_TOKEN='your-token'
bash install.sh --source --ref main
```

## Profiles

The default is `core` plus `operating`. It is intentionally small enough for every Kujo developer or coding agent while still providing the complete local proof loop.

| Profile | Contents | Why it belongs there |
| --- | --- | --- |
| `core` | Kujo, Kennel, Spec, Eval, Scout, Scent, PackWrite, RunLedger, CaseFile, PatchBrief, ChangeBucket, Muzzle | Runtime, dependency foundation, task contracts, context, execution packs, verification, evidence, and quiet bounded runs |
| `ai` | AI SDK, Agents SDK, Dispatch, Watchdog, MCP, RAG, Relay | Provider-gated AI, agent orchestration, telemetry, gateways, retrieval, and resumable missions |
| `quality` | Concord, ShipCheck, Fence, Redact, Lens, Tribunal, Workcell, Howl | Cross-artifact drift, release checks, architecture boundaries, privacy, browser proof, review, Docker-backed execution, and asset rendering |
| `showcases` | SSG, CMS, CRUD API, AI Chat, Intake, Site Kit | Copyable applications and design/publishing surfaces; not required to write Kujo tools |
| `operating` | Kujo Skills, Kujo Agents, Kujo Workflows | Agent-readable instructions, role contracts, and runnable end-to-end workflow kits |

Install one additional profile with `--group`:

```bash
bash install.sh --group ai
bash install.sh --group quality --group showcases
```

Install everything with:

```bash
bash install.sh --all
```

`--all` installs source snapshots for all profiles. Optional Node dependencies for AI Chat, Intake, Site Kit, and the CRUD API frontend are installed only when requested:

```bash
bash install.sh --all --with-deps
```

This keeps the default path free of Node, browser, Docker, provider credentials, and network-service setup. Those are environment-specific prerequisites, not safe universal defaults.

## Core command map

The core shims are installed into the selected bin directory:

| Command | Repository | Role |
| --- | --- | --- |
| `kujo` | Kujo | Language runtime and CLI |
| `kennel` | Kennel | Package and lockfile workflows |
| `spec` | Spec | Structured task contracts |
| `eval` | Eval | Deterministic acceptance checks |
| `scout` | Scout | Repository intelligence and onboarding packs |
| `scent` | Scent | Bounded, redacted task context |
| `packwrite` | PackWrite | Agent execution packs |
| `runledger` | RunLedger | Run receipts, usage, verdicts, and comparisons |
| `casefile` | CaseFile | Reproducible failure evidence |
| `patchbrief` | PatchBrief | Diff summaries and implementation handoffs |
| `changebucket` | ChangeBucket | Change footprint and blast-radius budgets |
| `muzzle` | Muzzle | Quiet workflow execution with complete logs |

Optional profiles add stable shims for their CLI-bearing repositories. Library and content repositories are still installed under `~/.kujo/sources/` and are intentionally not made into fake commands.

## Agent handoff

Give an agent this file together with the installed root path. The normal sequence is:

```text
1. Read the repository's AGENTS.md and the relevant installed Kujo skill.
2. Use Spec for the task contract and Scout/Scent for bounded context.
3. Use PackWrite or a workflow kit for repeatable execution.
4. Use Eval and the repository's tests for verification.
5. Record the run with RunLedger and capture failures with CaseFile.
6. Use PatchBrief and ChangeBucket for reviewable handoff and scope control.
```

The installer provides tools and source; it does not grant an agent authority to commit, push, deploy, access credentials, or run untrusted code. Review each repository's security and readiness documentation before using optional tools in a sensitive or production environment.

## Verification

After installation:

```bash
export PATH="$HOME/.local/bin:$PATH"
kujo --version
kujo doctor --json
spec --help
scout --help
eval --help
runledger --help
```

For a pinned release, verify the downloaded Kujo archive checksum. For a source install, the installer runs `cargo build --release --locked`; the supported Rust version and platform matrix remain documented in [`INSTALLATION.md`](../INSTALLATION.md).

## Release boundary

Kujo's tagged binaries, checksum files, and published-artifact smokes are the canonical ecosystem onboarding path. Treat `main` as development state; use the immutable `v1.0.1` tag or a later supported patch tag for reproducible release installs.
