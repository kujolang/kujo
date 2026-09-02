# Build an Agent

Kujo Agent Projects make an agent's intelligence and execution contract a normal Git repository.

## Test the complete installation

Install the stable Kujo v1.2.3 runtime and focused Agent Development Platform
in one command:

```bash
curl -fsSL https://kujolang.ai/install.sh | bash -s -- --group agent
export PATH="$HOME/.local/bin:$PATH"
```

The installer downloads the published release archive, verifies its checksum,
and installs the focused ecosystem tools. It must finish without errors, and
these commands must resolve:

```bash
kujo --version
kennel --help
kujo agent --help
```

First verify the offline path. It needs no API key and makes no model request:

```bash
mkdir -p "$HOME/kujo-agent-tests"
cd "$HOME/kujo-agent-tests"
kujo agent new fixture-agent --profile basic --install --no-git
cd fixture-agent
kujo doctor agent
kujo agent inspect
kujo agent run "Reply with exactly: install verified"
kujo agent eval
```

Doctor, inspect, run, and eval must all succeed. The basic profile does not
require Watchdog or RunLedger. Those dependencies are declared and installed
when `observable` or `full` is selected.

Then verify a real provider. This prompt is masked and saves the key in the
operating-system credential store, not in shell history or the project:

```bash
kujo agent auth set openai
kujo agent auth status openai
cd "$HOME/kujo-agent-tests"
kujo agent new live-agent --provider openai --install --no-git
cd live-agent
kujo doctor agent
kujo agent run "Reply with exactly: live agent verified"
```

The status command must report that the credential is available without
printing its value. The final command must return a real model response. Later
agents reuse the same stored key automatically. To remove the test credential,
run `kujo agent auth remove openai`.

For the observable path, repeat the scaffold with
`--profile observable --install`. Missing ecosystem dependencies are surfaced by
Agent Doctor with the exact install command; `--install` normally resolves them
during scaffolding.

## Create an Agent Project

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
# One-time, masked setup. The key is saved in the operating-system credential store.
kujo agent auth set openai

# Every later agent can reuse it without another export or copied secret.
# Auto-selection finds the configured provider and its starter model.
kujo agent new live-agent --install
cd live-agent
kujo agent run "Say hello"
```

When a live provider is selected and no credential exists, interactive
`kujo agent new` asks for it once with hidden input. Non-interactive automation
uses `--credential-stdin`; `--no-credential` is the explicit config-only escape
hatch. Environment variables remain supported for CI, but they are no longer the
primary local workflow.

Credentials resolve in this order: the current process environment, the
project's ignored `.env.local`, then the user's operating-system credential
store. Project overrides are created with `kujo agent auth set openai --project`
and written with owner-only permissions. `kujo agent auth status openai` reports
the source without printing the value, and `kujo agent auth remove openai`
revokes the saved user credential.

API-key connectors use the same secure path without adding secrets to connector
configuration or source control:

```bash
kujo agent auth set --name LINEAR_API_TOKEN
kujo agent auth status --name LINEAR_API_TOKEN
```

The connector configuration owns only the credential name. OAuth consent and
token refresh remain the responsibility of the connector implementation; do not
substitute a long-lived API key when the connector supports scoped OAuth.

For a custom provider, set `provider`, `model`, `base_url`, and
`api_key_env` in `config/model.json`. HTTPS is required unless
`allow_insecure_localhost` explicitly enables a loopback development endpoint.
Secret values never belong in the repository, command arguments, inspect output,
Doctor output, or JSON contracts.

Project-local Agent Skills use `SKILL.md`. Tools retain inspectable names, schemas, risk, and approval metadata. MCP is separate from WebMCP: publishing a public website is optional and belongs to Kujo SSG, whose existing `llms.txt` and WebMCP build surfaces can index this guide.

Kujo runtime capabilities authorize effects; they are not a sandbox. The hardened profile also declares a Workcell boundary with a read-only root, bounded CPU, memory, PIDs, time, writable artifacts, and explicit network policy. Run the generated v1 definition with `kujo agent run --workcell`. Containers do not protect against a compromised host kernel or container daemon.

Portable Workcell v2 definitions keep backend choice in host/operator configuration rather than the Agent Project's intelligence definition. Select one explicitly at invocation time:

```bash
kujo agent run --workcell \
  --workcell-profiles /etc/kujo/workcell-profiles.json \
  --workcell-profile ci \
  --workcell-manifest /opt/kujo/adapters/e2b/manifest.json \
  --workcell-backend e2b \
  "Review this change"
```

Kujo passes the portable definition, immutable Git source, profile, and adapter manifest to Workcell. It consumes Workcell's exact machine-readable `receipt_path`; it does not search for the newest run or call a provider API itself. The profile and manifest flags are rejected for v1 definitions, and all three required v2 selections must be present before execution.

The executable lifecycle workflow lives in the `kujo-workflows` repository at
`owned-agent-project/scripts/run.sh`. This repository also self-hosts a generated
knowledge profile at `examples/owned-agent-project`.

`kujo agent inspect --json`, `run --json`, `eval --json`, `new --json`, and
`auth ... --json` emit versioned machine-readable payloads. `kujo doctor agent
--json` reuses the canonical Doctor report. There is deliberately no `kujo agent
doctor` command.
