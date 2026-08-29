# Kujo Agent Project Implementation Report

## Executive summary

Kujo now provides a repository-owned Agent Project experience across creation,
dependency installation, diagnosis, inspection, deterministic and live-model
execution, evaluation, workflow composition, bounded Workcell execution, and
run evidence. The repository owns intelligence configuration while existing
ecosystem components retain their established responsibilities.

## Final CLI

```text
kujo agent new <name> [--profile <profile>] [--dir <path>] [--provider <id>] [--model <id>] [--no-git] [--install] [--json]
kujo agent inspect [--json]
kujo agent run [prompt] [--file <path>] [--workcell] [--json]
kujo agent eval [--json]
kujo agent auth set <provider> [--from-stdin|--from-env] [--project] [--json]
kujo agent auth status <provider> [--project] [--json]
kujo agent auth remove <provider> [--project] [--json]
kujo agent auth set --name <CONNECTOR_CREDENTIAL_ENV> [--project]
kujo doctor agent [--json] [--deep]
```

The diagnostic command is deliberately `kujo doctor agent`; no competing
`kujo agent doctor` command exists.

## Doctor

Agent Doctor extends the existing Doctor framework and preserves its report and
JSON conventions. It checks project discovery and contracts, runtime entrypoints
and provider configuration, credential availability and safe storage, installed pinned dependencies, agent package paths,
enabled integrations, external tools, secret-like files, capability declarations,
and hardened Workcell configuration. `--deep` performs additional local checks
without silently initiating provider requests.

## Architecture audit and ownership

The pre-implementation audit found all required primitives already present in
the ecosystem. The Agent Project layer therefore composes rather than forks them:

| Owner | Responsibility used by Agent Project |
| --- | --- |
| Kujo | CLI façade, root discovery, schema/path validation, capability authorization |
| Kennel | immutable dependency sources and installation |
| AI SDK | provider presets, credentials, retries, and normalized live responses |
| Agents SDK | agent runner, package semantics, tools, retrieval and telemetry adapters |
| Kujo Agents / Skills | agent package and `SKILL.md` conventions |
| MCP | local tool/resource server behavior |
| RAG | offline indexing, retrieval, and citations |
| Dispatch | project workflow execution and resumable evidence |
| Relay | full-profile local mission adapter |
| Workcell | container-backed execution and receipts |
| Watchdog | optional live-provider telemetry target |
| RunLedger | observable-profile run receipts |
| Eval | evaluation execution and nonzero failure behavior |

## Agent Project contract

`agent.project.json` uses `kujo-agent-project/v1` and references a bundled
versioned JSON Schema. All declared paths are canonicalized inside the project
root. Root discovery works from subdirectories and stops at the containing Git
boundary. Generated dependency declarations use exact Git commits, and generated
projects do not contain sibling-checkout paths or credential values.

## Profiles

`basic` provides the minimal fixture runner. `tools` adds MCP and an Agents SDK
tool adapter. `knowledge` adds RAG ingestion/query and citation normalization.
`workflow` adds a runnable Dispatch workflow. `hardened` adds least-privilege
declarations and real Workcell execution. `observable` adds Watchdog adapter
configuration and RunLedger receipts. `full` composes compatible features and
adds Relay execution. Optional external services remain explicit in inspect and
Doctor output.

## Generated tree

The deterministic tree contains `agent.project.json`, the agent package,
project-owned skills/tools/knowledge/policies, provider and integration configs,
source entrypoints, Eval contract, workflow definitions, schema, Kennel manifest,
Kujo package files, `.env.example`, `.gitignore`, `AGENTS.md`, and project README.
The self-hosted `examples/owned-agent-project` knowledge profile is generated from
the same implementation and passes the standard lifecycle.

## Security

Scaffolding uses a sibling staging directory and atomic promotion, rejects unsafe
names, traversal, filesystem roots, symlink destinations, and non-empty targets,
and does not perform network access unless `--install` is explicit. Runtime paths
are revalidated before execution. Live provider and API-key connector credentials
are resolved from CI environment variables, an ignored owner-only project
`.env.local`, or the operating-system credential store. Interactive entry is
masked, automation can use stdin, credentials never appear in CLI arguments, and
child-process failures redact the resolved value. HTTPS is required except for an explicitly opted-in
loopback development endpoint. Secret-like repository files fail Doctor without
printing their values. Capability flags authorize effects; they are not described
as sandboxing. Workcell provides the separately documented container boundary.

## Offline proof

All seven profiles scaffold deterministically and pass Doctor, inspect, run, and
Eval using local fixture packages. The fixture model needs no credentials and no
provider network. MCP, RAG, Dispatch, Relay, and RunLedger paths execute their
own installed component code in applicable profiles.

## Live provider path

A deterministic loopback OpenAI-compatible server test proves the custom live
path end to end: AI SDK performs and normalizes the HTTP request, then Agents SDK
owns the agent run and returns the model text. Missing credentials, malformed
provider config, unsafe base URLs, and undeclared providers fail closed. Real
third-party availability and billing remain external operational concerns.

Credential contract tests exercise reusable and project-scoped storage, private
permissions, subdirectory discovery, non-disclosure, explicit config-only live
scaffolding, and automated stdin setup. Native OS credential storage is provided
by macOS Keychain, Windows Credential Manager, and Linux Secret Service.

## Tests

The implementation is covered by Rust CLI contract tests for every profile,
deterministic generation, root discovery, Doctor/inspect/run/Eval, live bridging,
portability, path escape, symlink, conflict, malformed contract/config, missing
dependency/file, secret non-disclosure, and JSON behavior. Repository validation
uses `cargo fmt`, Clippy with warnings denied, the full locked test suite, Bash
syntax checks, and diff checks.

## Portability proof

A generated project was copied away from the ecosystem checkout, its dependency
directory was recreated from the repository-owned pinned Kennel declaration, and
Doctor, inspect, run, and Eval succeeded. The automated portability test likewise
copies the project and installed package contents to an unrelated temporary root
before exercising the lifecycle.

## Repositories changed

- `kujolang/kujo`: Agent Project CLI, schema, tests, documentation, installer
  profile expansion, and self-hosted example.
- `kujolang/kujo-workflows`: executable `owned-agent-project` lifecycle kit and
  catalog entry.

Generated pins at implementation time: AI SDK `be9617a`, Agents SDK `d3904d3`,
Eval `955713f`, MCP `2ab8111`, RAG `28690e3`, Dispatch `662417c`, Relay `0480733`,
and Workcell `7bcdb7f`. Optional external tools are Watchdog `1af292b` and
RunLedger `12bbf2b`.

## Deferred work

Hosted deployment, public package/release publication, production Watchdog
service operation, and credentials-backed calls to each named
third-party model provider are intentionally outside this local first release.
