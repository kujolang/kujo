# Build an Agent

Kujo Agent Projects make an agent's intelligence and execution contract a normal Git repository.

```bash
kujo agent new my-agent --profile basic
cd my-agent
kennel install
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

The base scaffold is offline and makes no network request. `--install` is an explicit dependency-installation boundary. Ecosystem dependencies are recorded as immutable Git revisions in `kennel.toml`; the project does not depend on sibling source checkouts.

Provider and model choices live in `config/model.json`. Fixture mode requires no secret. OpenAI, OpenRouter, DeepSeek, and custom OpenAI-compatible providers use AI SDK; the normalized response then enters Agents SDK execution. For example:

```bash
kujo agent new live-agent --provider openai --model gpt-5-mini
cd live-agent && kennel install
export OPENAI_API_KEY=...
kujo agent run "Say hello"
```

For a custom provider, set `provider`, `model`, `base_url`, and
`api_key_env` in `config/model.json`. HTTPS is required unless
`allow_insecure_localhost` explicitly enables a loopback development endpoint.
Secret values never belong in the repository, inspect output, or Doctor output.

Project-local Agent Skills use `SKILL.md`. Tools retain inspectable names, schemas, risk, and approval metadata. MCP is separate from WebMCP: publishing a public website is optional and belongs to Kujo SSG, whose existing `llms.txt` and WebMCP build surfaces can index this guide.

Kujo runtime capabilities authorize effects; they are not a sandbox. The hardened profile also declares a Workcell container boundary with a read-only root, bounded CPU, memory, PIDs, time, writable artifacts, and explicit network policy. Run it with `kujo agent run --workcell`. The generated fixture boundary has no network. Containers do not protect against a compromised host kernel or container daemon.

The executable lifecycle workflow lives in the `kujo-workflows` repository at
`owned-agent-project/scripts/run.sh`. This repository also self-hosts a generated
knowledge profile at `examples/owned-agent-project`.

`kujo agent inspect --json`, `run --json`, `eval --json`, and `new --json` emit versioned machine-readable payloads. `kujo doctor agent --json` reuses the canonical Doctor report. There is deliberately no `kujo agent doctor` command.
