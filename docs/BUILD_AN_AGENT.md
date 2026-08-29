# Build an Agent

Kujo Agent Projects make an agent's intelligence and execution contract a normal Git repository.

```bash
kujo agent new my-agent --profile basic
cd my-agent
kujo doctor agent
kujo agent inspect
kujo agent run "Say hello"
kujo agent eval
```

The root `agent.project.json` uses the versioned `kujo-agent-project/v1` contract. Commands discover it from project subdirectories without crossing the containing Git boundary. It references the Kujo Agents package, Agents SDK entrypoint, AI SDK model preference, local skills, tools, knowledge, policies, workflows, evals, Workcell, and optional integrations. All referenced paths are validated inside the repository.

| Profile | Use it for |
| --- | --- |
| `basic` | Simple fixture-first agents |
| `tools` | Agent tools and MCP connectivity |
| `knowledge` | Local knowledge and Kujo RAG |
| `workflow` | Dispatch workflows |
| `hardened` | Least-privilege capabilities and Workcell |
| `observable` | Watchdog telemetry and RunLedger evidence |
| `full` | The compatible composed local stack, including Relay adapter metadata |

The base scaffold is offline and makes no network request. `--install` is an explicit dependency-validation boundary. Ecosystem dependencies are recorded as immutable Git revisions in `kennel.toml`; the project does not depend on sibling source checkouts.

Provider and model choices live in `config/model.json`. Fixture mode requires no secret. For a live provider, change that file and supply the named credential through the environment. Secret values never belong in the repository, inspect output, or Doctor output.

Project-local Agent Skills use `SKILL.md`. Tools retain inspectable names, schemas, risk, and approval metadata. MCP is separate from WebMCP: publishing a public website is optional and belongs to Kujo SSG, whose existing `llms.txt` and WebMCP build surfaces can index this guide.

Kujo runtime capabilities authorize effects; they are not a sandbox. The hardened profile also declares a Workcell container boundary with a read-only root, bounded CPU, memory, PIDs, time, writable artifacts, and explicit network policy. Containers do not protect against a compromised host kernel or container daemon.

`kujo agent inspect --json`, `run --json`, `eval --json`, and `new --json` emit versioned machine-readable payloads. `kujo doctor agent --json` reuses the canonical Doctor report. There is deliberately no `kujo agent doctor` command.
