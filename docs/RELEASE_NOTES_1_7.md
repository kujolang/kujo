# Kujo 1.7 release candidate

Status: unpublished preparation. The stable release and default installer remain
1.6.0. This document does not authorize publication or establish a tested 1.7.0
artifact cohort.

## Runtime additions

- `path_is_absolute(path)` checks native platform path syntax without accessing
  the filesystem or requiring filesystem authority. This lets canonical Ability
  process bindings accept absolute Windows executable paths without inventing
  host-specific path rules.
- Windows `kujo run --kill-children-on-exit` gives an external supervisor a
  process-lifetime boundary. Job admission must succeed before script execution;
  unsupported platforms reject the option. It neither grants capabilities nor
  makes Kujo a sandbox. Ordinary execution without the flag remains unchanged.
- `kujo mcp make` delegates repository-specific MCP generation to canonical Kujo
  MCP source, resolving it through explicit configuration, repository identity,
  Kennel lockfiles or the ecosystem install root. The core install profile now
  includes that source. Registry policy remains owned by Kennel.

These additive features warrant a minor version. Existing source/runtime
contracts remain supported. The source cohort also includes the separately
maintained Kujo/Go benchmark harness; its measurements are not installation or
security guarantees.

## Adapter compatibility and evidence

Kujo OpenAI's Windows provider requires the lifetime flag and native absolute
path validation. Published runtime 1.6.0 does not provide these additions. A
successful local/source-built adapter test cannot make that older package
compatible.

The pre-version source cohort at
`9fbad956eecd37448d6dc2f3568105cd1da7b0b0` passed the
[full gate and five native artifact builds](https://github.com/kujolang/kujo/actions/runs/36811793883).
Those exact optimized packages also passed
[five-platform isolated adapter installation](https://github.com/kujolang/kujo-openai/actions/runs/36816195097),
with install scripts disabled, three canonical Ability tools, and exact receipt
lookups on each platform. These were unpublished rehearsal artifacts labeled
1.6.0; they must never replace the existing registry release.

The versioned 1.7.0 commit still needs its own gates, artifact provenance checks,
publication authorization, and published-install verification. OpenAI public
local-plugin distribution support is a separate host requirement.

## Publication boundary

Follow [RELEASE_PROCESS.md](RELEASE_PROCESS.md), including the explicit
`UNBLOCK_V1_RELEASE` directive. No tag, crate, npm package, GitHub release or
stable installer update is authorized by this preparation. Retain 1.6.0 stable
references until the new release and published-artifact checks succeed.
