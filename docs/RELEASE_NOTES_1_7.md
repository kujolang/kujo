# Kujo 1.7 release

Status: published native and npm release for Linux x64/arm64, macOS x64/arm64
and Windows x64. Signed source: `813072040a1ac643312f5163fcfa4f26474c9095`.
Crates.io is a separate, currently blocked distribution channel.

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

The tagged [publication workflow](https://github.com/kujolang/kujo/actions/runs/36850604536)
passed the full release gate and published native/npm artifacts. Both
[native installation smoke](https://github.com/kujolang/kujo/actions/runs/36857820990)
and [npm installation smoke](https://github.com/kujolang/kujo/actions/runs/36866507729)
passed on all five platforms. The initial Windows native smoke hit GitHub rate
limiting; its retry passed without changing artifacts. OpenAI public local-plugin
distribution support remains a separate host requirement.

## Publication boundary

The owner supplied `UNBLOCK_V1_RELEASE` before publication. The signed tag is
verified by GitHub. Stable installer references now select the tested 1.7.0 release.

`cargo publish --dry-run --locked` from the signed source fails because Cargo
normalization drops the local tiny_http patch, removing the required
`http_with_read_timeout` API. No crate was published. Release policy permits the
independently tested native release with this exception. A registry-compatible
fork is being prepared separately; HTTP deadlines must remain enforced.
