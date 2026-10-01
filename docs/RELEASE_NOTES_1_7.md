# Kujo 1.7 release candidate

Status: signed `v1.7.0` tag pushed; native/npm publication is in progress.
The stable release and default installer remain 1.6.0 until published-artifact
verification passes. Tag creation alone does not establish publication.

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

The versioned candidate `f68750b368e6ebaf4dcaf588b4984651bb4d9670` passed its
[full gate and five native builds](https://github.com/kujolang/kujo/actions/runs/36817611559),
then [all five isolated installed-package tests](https://github.com/kujolang/kujo-openai/actions/runs/36822256533).
Each installation exercised three canonical tools and exact receipt lookups.
The signed release commit `813072040a1ac643312f5163fcfa4f26474c9095` has an
identical source tree. Its tagged release gate passed; native/npm publication
and published-install verification are still pending. OpenAI public local-plugin
distribution support is a separate host requirement.

## Publication boundary

The owner supplied `UNBLOCK_V1_RELEASE` on 2026-10-01. Signed tag `v1.7.0` is
verified locally and by GitHub. Follow [RELEASE_PROCESS.md](RELEASE_PROCESS.md)
and retain 1.6.0 stable references until the new release and published-artifact
checks succeed.

### Crates.io exception

`cargo publish --dry-run --locked` from the exact signed release commit exits
101: Cargo's normalized registry package drops the local `tiny_http` patch and
resolves upstream 0.12.0, which lacks `Server::http_with_read_timeout`. Both the
VM and interpreter fail with E0599. No crate was published. This is the existing
registry-packaging limitation, not a native/npm build failure. Release policy
section 8 permits the independently verified GitHub binary release with this
exception recorded. A future registry fix must preserve the HTTP read-deadline
security behavior; removing the patched calls is not an acceptable workaround.
