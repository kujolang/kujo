# Kujo ecosystem golden path

This is the canonical fixture-first integration path for a local release
demonstration. It composes existing examples and gates; it does not claim live
provider, hosted CI, multi-tenant, or enterprise certification.

## Evidence flow

```text
Kujo runtime
  -> AI SDK offline fixture
  -> Agents SDK no-network agent
  -> Dispatch offline workflow
  -> Workcell controlled execution + receipt
  -> Watchdog fixture/telemetry boundary
  -> Eval deterministic checks + checksums
  -> ShipCheck release gate
  -> RunLedger / CaseFile evidence handoff
```

Evidence labels are explicit: `fixture` is committed/offline, `local-real`
uses a local runtime or container, `platform-ci` requires a hosted runner, and
`external-live` requires credentials or operator infrastructure.

## Deterministic local procedure

Run from the repository collection with a pinned Kujo binary:

```bash
export KUJO_BIN="$PWD/kujo/target/release/kujo"
export PATH="$(dirname "$KUJO_BIN"):$PATH"
export GOLDEN_ROOT="$(mktemp -d /tmp/kujo-golden-path.XXXXXX)"
```

1. AI SDK fixture (`fixture`): run `ai-sdk/examples/main.kujo` without a
   provider key and retain its normalized response envelope.
2. Agents SDK fixture (`fixture`): run the existing
   `agents-sdk/examples/hello_agent.kujo` or
   `agents-sdk/examples/traced_agent.kujo` through
   `agents-sdk/examples/examples_smoke_runner.kujo` and retain the trace.
3. Dispatch fixture (`fixture`): follow
   `dispatch/examples/quickstart-walkthrough.md` with
   `DISPATCH_OFFLINE_FIXTURE=true`, `--yes`, `--non-interactive`, and an output
   directory below `$GOLDEN_ROOT`.
4. Controlled execution (`local-real` when Docker is available): run the
   existing `workcell/examples/hello/workcell.json` or
   `workcell/examples/verification/workcell.json` with `network.mode: none`.
   Preserve `receipt.json`, `manifest.json`, logs, and `workcell verify` output.
   If the engine is unavailable, record a blocked receipt; do not relabel the
   fixture result as container evidence.
5. Telemetry boundary (`fixture` or `local-real`): use Watchdog's committed
   fixture/test path or a local proxy configured with a fixture provider. Never
   add live credentials to this workflow.
6. Eval evidence (`fixture`): run an existing release-gate suite from
   `eval/examples/` into an isolated `$GOLDEN_ROOT/eval` directory with JSON
   output and artifact checksums, then run `verify-manifest`.
7. Release gate (`local-real`): run ShipCheck against the candidate repository
   with `gate --format json`. Treat warnings as review items and require zero
   errors.
8. Evidence handoff (`local-real`): record the run in RunLedger and package
   failure evidence with CaseFile. Store only paths, hashes, statuses, and
   redacted summaries in the handoff.

## Acceptance contract

The path is complete only when the evidence bundle contains:

- the exact Kujo runtime identity;
- AI SDK and agent response envelopes with no provider credential;
- a Dispatch run, trace, and deterministic status;
- Workcell receipt/manifest/verification output or an explicit blocked receipt;
- Watchdog telemetry or an explicit fixture-only marker;
- Eval JSON report and verified artifact manifest;
- ShipCheck JSON gate with zero errors;
- RunLedger status and CaseFile path/hash references;
- a summary that distinguishes fixture, local-real, platform-CI, and
  external-live evidence.

## Current gaps

- No existing single command currently composes all stages.
- Workcell Docker/Podman proof depends on the target engine and host.
- Watchdog live-provider proof depends on credentials and operator approval.
- Hosted CI and release provenance remain platform-owned evidence.
- RunLedger and CaseFile have local contracts, but their handoff is not yet a
  first-class adapter in the Dispatch/Workcell path.

The smallest next implementation is a repository-local orchestration wrapper
that calls these existing entrypoints, writes only under an isolated output
root, and fails closed when any stage is missing or mislabeled.
