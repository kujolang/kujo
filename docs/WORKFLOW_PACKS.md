# Kujo Workflow Packs

Workflow packs are modular, project-specific command namespaces for the Kujo CLI. They let teams and third parties add custom CLI commands without modifying Kujo core.

## Quick Start

```bash
# Run a workflow pack command via environment variable discovery
KUJO_PACK_PATH=/path/to/some-pack kujo pack run <namespace> <command>

# Machine-readable JSON output
KUJO_PACK_PATH=/path/to/some-pack kujo pack run <namespace> <command> --json

# Or install as a project-local pack and run directly
mkdir -p .kujo/packs && cp -r /path/to/some-pack .kujo/packs/
kujo pack run <namespace> <command>
```

Alias form (`kujo <namespace> <command>`) remains available for non-reserved namespaces.

## What Are Workflow Packs?

A workflow pack is a directory containing:

- `kujo-pack.yaml` — manifest declaring the pack identity, namespace, and commands
- Command implementations — Kujo scripts (`.kujo` files) or native executables that produce structured output

Workflow packs let different teams create their own CLI namespaces without modifying Kujo:

```
kujo acme doctor            # Acme Corp: environment readiness check
kujo hosting health         # Hosting team: server health check
kujo docs drift check       # Docs team: documentation drift detection
kujo release readiness      # Release team: pre-release validation
```

## Built-in Packs

Kujo currently ships with **no built-in workflow packs**. The workflow pack system is designed so that teams can create and distribute external packs without modifying Kujo core.

## Walkthrough: An Example Pack

Here is a concrete example of a workflow pack for a fictional company, "Acme Corp." The pack provides a `doctor` command that checks whether a developer's local environment is ready for Acme's projects.

### Pack structure

```
acme-tools/
  kujo-pack.yaml
  commands/
    doctor.kujo
  README.md
```

### Manifest (`kujo-pack.yaml`)

```yaml
id: acme-tools
namespace: acme
name: Acme Corp Tools
version: 0.1.0
description: Workflow commands for Acme Corp development.

commands:
  - name: doctor
    summary: Check whether the local development environment is ready.
    entry: commands/doctor.kujo
    safe: true
    writes_files: false
    runs_processes: true
    requires_network: false
```

### Command implementation (`commands/doctor.kujo`)

The command is a Kujo script that inspects the local environment and emits a JSON `DoctorReport` to stdout. A minimal example:

```kujo
# acme-tools/commands/doctor.kujo
# Checks common dev tools and repo state.

func check_pass(id, label, observed) {
    return {
        "id": id, "label": label, "status": "pass",
        "severity": "info", "observed": observed
    }
}

func check_warn(id, label, message, fix) {
    result := {
        "id": id, "label": label, "status": "warn",
        "severity": "medium", "message": message
    }
    if (fix != null) { result["suggested_fix"] = fix }
    return result
}

func command_first_line(cmd) {
    out := execute(cmd + " --version")
    lines := split(trim(out), "\n")
    if (len(lines) > 0) { return trim(lines[0]) }
    return ""
}

func compute_summary(checks) {
    summary := {"pass": 0, "warn": 0, "fail": 0, "skip": 0, "info": 0}
    i := 0
    while (i < len(checks)) {
        status := checks[i]["status"]
        if (status == "pass") { summary["pass"] = summary["pass"] + 1 }
        else if (status == "warn") { summary["warn"] = summary["warn"] + 1 }
        else if (status == "fail") { summary["fail"] = summary["fail"] + 1 }
        else if (status == "skip") { summary["skip"] = summary["skip"] + 1 }
        else { summary["info"] = summary["info"] + 1 }
        i = i + 1
    }
    return summary
}

func main() {
    checks := []

    # Check Git
    git_ok := execute_status("git --version").success == true
    if (git_ok) {
        checks = push(checks, check_pass("env.git", "Git", command_first_line("git")))
    } else {
        checks = push(checks, check_warn("env.git", "Git", "Git not found.", "Install Git."))
    }

    # Compute summary and emit report.
    summary := compute_summary(checks)

    overall := "pass"
    if (summary["warn"] > 0) { overall = "warn" }

    report := {
        "pack": "acme-tools", "namespace": "acme", "command": "doctor",
        "status": overall, "summary": summary, "checks": checks
    }
    print(to_json(report))
}

main()
```

### Running the pack

```bash
# Via environment variable
KUJO_PACK_PATH=/path/to/acme-tools kujo acme doctor

# Via project-local install
mkdir -p .kujo/packs
cp -r /path/to/acme-tools .kujo/packs/
kujo acme doctor
```

### Example human output

```
ACME Doctor

Environment
  PASS  Git: git version 2.42.0
  WARN  Node: Node not found.
        Suggested fix: Install Node.js.

Summary
  1 passed, 1 warnings, 0 failed, 0 skipped
```

### JSON output

```bash
kujo acme doctor --json
```

```json
{
  "pack": "acme-tools",
  "namespace": "acme",
  "command": "doctor",
  "status": "warn",
  "summary": {
    "pass": 1,
    "warn": 1,
    "fail": 0,
    "skip": 0,
    "info": 0
  },
  "checks": [
    {
      "id": "env.git",
      "label": "Git",
      "status": "pass",
      "severity": "info",
      "observed": "git version 2.42.0"
    },
    {
      "id": "env.node",
      "label": "Node",
      "status": "warn",
      "severity": "medium",
      "message": "Node not found.",
      "suggested_fix": "Install Node.js."
    }
  ]
}
```

## Check Result Schema

Every check in a workflow command result uses this shape:

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique check identifier (e.g., `env.git`) |
| `label` | string | Human-readable label |
| `status` | string | One of: `pass`, `warn`, `fail`, `skip`, `info` |
| `severity` | string | One of: `info`, `low`, `medium`, `high`, `critical` |
| `observed` | string? | What was observed (version string, path, etc.) |
| `expected` | string? | What was expected (version range, etc.) |
| `message` | string? | Human-readable message (shown for warn/fail) |
| `suggested_fix` | string? | Actionable suggestion to resolve the issue |

### Statuses

| Status | Meaning |
|--------|---------|
| `pass` | Check passed successfully |
| `warn` | Non-critical issue detected |
| `fail` | Critical issue detected |
| `skip` | Check not applicable or could not run |
| `info` | Informational only |

### Overall Command Status

| Overall | Condition |
|---------|-----------|
| `pass` | No warnings or failures |
| `warn` | Warnings present but no failures |
| `fail` | One or more failures |

### Exit Codes

| Exit Code | Meaning |
|-----------|---------|
| 0 | Pass or warn |
| 1 | Fail or error |

## How Kujo Discovers Packs

Discovery order (first match wins for built-in namespace protection):

1. **Built-in packs** — Compiled into the Kujo binary (highest priority, cannot be overridden)
2. **Project-local packs** — `./.kujo/packs/<pack-name>/kujo-pack.yaml`
3. **User-local packs** — `~/.kujo/packs/<pack-name>/kujo-pack.yaml`
4. **Env-path packs** — `KUJO_PACK_PATH=/path/to/pack-a:/path/to/pack-b`

## Creating a Custom Workflow Pack

### 1. Create the pack directory

```
my-pack/
  kujo-pack.yaml
  commands/
    doctor.kujo       # Kujo script implementing the command
```

### 2. Write the manifest (`kujo-pack.yaml`)

```yaml
id: my-team-tools
namespace: myteam
name: My Team Tools
version: 0.1.0
description: Custom workflow commands for my team.

commands:
  - name: doctor
    summary: Check whether the local environment is ready.
    entry: commands/doctor.kujo
    safe: true
    writes_files: false
    runs_processes: true
    requires_network: false

  - name: "release check"
    summary: Validate release readiness.
    entry: commands/release-check.kujo
    safe: true
    writes_files: false
    runs_processes: true
    requires_network: false
```

### 3. Manifest Fields

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique pack identifier (e.g., `acme-tools`) |
| `namespace` | Yes | CLI namespace, must be `[a-z][a-z0-9-]*` |
| `name` | Yes | Human-readable pack name |
| `version` | Yes | SemVer version string |
| `description` | No | Pack description |
| `commands` | Yes | List of command definitions (at least one) |

### 4. Command Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Command name; use spaces for nested commands (e.g., `"card check"`) |
| `summary` | Yes | Short description shown in help |
| `entry` | Yes | Path to implementation relative to pack root (e.g., `commands/doctor.kujo`) |
| `safe` | No | Whether the command is safe to run without review (default: `false`) |
| `writes_files` | No | Whether the command writes to the filesystem |
| `runs_processes` | No | Whether the command executes external processes |
| `requires_network` | No | Whether the command makes network requests |

### 5. Implement the command

Commands are Kujo scripts (`.kujo` files) that output a JSON `DoctorReport` to stdout. The workflow system executes `.kujo` scripts via `kujo run --allow-all` so commands can use `execute()`, `file_exists()`, `path_is_dir()`, and other built-in functions.

The expected JSON shape is:

```json
{
  "pack": "<pack-id>",
  "namespace": "<namespace>",
  "command": "<command-name>",
  "status": "pass|warn|fail",
  "summary": { "pass": 0, "warn": 0, "fail": 0, "skip": 0, "info": 0 },
  "checks": [...]
}
```

### 6. Install the pack

**Project-local** (only for current project):

```bash
mkdir -p .kujo/packs/my-team
cp -r my-pack/* .kujo/packs/my-team/
kujo myteam doctor
```

**User-local** (available everywhere):

```bash
mkdir -p ~/.kujo/packs/my-team
cp -r my-pack/* ~/.kujo/packs/my-team/
kujo myteam doctor
```

**Custom path** (via environment variable):

```bash
KUJO_PACK_PATH=/path/to/my-pack kujo myteam doctor
```

## Namespace Collision Rules

- **Built-in packs** always win; external packs cannot override built-in namespaces or commands
- **Duplicate namespaces** between external packs produce a clear error
- **Duplicate command names** within the same manifest produce a validation error
- **Reserved namespaces** are rejected for third-party packs
- **Reserved top-level alias tokens** are rejected before alias-based workflow routing

## Creating Built-in Packs (For Kujo Contributors)

To add a new built-in workflow pack to Kujo:

1. Create a new module under `src/workflow_pack/builtins/<pack-name>.rs`
2. Implement your command handlers as functions matching the `CommandHandler` signature
3. Register the pack in `src/workflow_pack/builtins/mod.rs` in the `register_all()` function
4. Add the pack manifest and handler map

The builtins module contains an empty placeholder showing the registration pattern; built-in packs follow the same manifest schema as external packs but implement commands as Rust functions.

## Architecture

```
src/workflow_pack/
  mod.rs              — Module root, initialization, CLI routing
  types.rs            — CheckResult, CheckStatus, CommandContext, DoctorReport
  manifest.rs         — YAML manifest parsing and validation (kujo-pack.yaml)
  discovery.rs        — Multi-source pack discovery
  registry.rs         — Namespace/command registration and routing
  renderer.rs         — Human-readable and JSON output renderers
  process_runner.rs   — Safe external process execution
  builtins/
    mod.rs            — Built-in pack registrations (currently empty)
```

### Key Design Decisions

- **Manifest format**: YAML (`kujo-pack.yaml`) — human-friendly, already a project dependency via `serde_yaml`
- **CLI integration**: Uses clap's `allow_external_subcommands` to capture unknown namespaces without modifying the core command enum
- **Built-in packs**: Compiled Rust code for performance and zero external dependencies
- **External packs**: Discovered at runtime via manifest files; commands run as Kujo scripts or native executables
- **Security**: Manifest parsing is pure data validation; external command code is never auto-executed during discovery

## Current Limitations

- External pack commands must be Kujo scripts (`.kujo` files) or native executables
- No trust/permission prompts for commands that declare `runs_processes: true`
- No `kujo workflow list` command yet (planned)
- Nested command names (e.g., `card check`) are parsed but not yet routable since no packs use them yet
- External Kujo scripts run with `--allow-all` capabilities (future versions will use manifest metadata for fine-grained capability gating)

## Future Directions

- `kujo workflow list` — list all available namespaces and commands
- Permission gating based on manifest `safe`/`writes_files`/`runs_processes`/`requires_network` metadata
- Pack installation/management commands (`kujo pack init`, `kujo pack add`)
- More sophisticated check grouping and output formatting
- A `--deep` flag convention for commands to opt into heavier validation
