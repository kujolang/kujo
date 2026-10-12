# Item 6: first-hour adopter validation

Status: **human study deferred by owner decision, 2026-10-12; not a human pass**. This record is an agent rehearsal and
walkthrough preparation. No human usability evidence or time-to-first-program
measurement has been obtained. A participant new to the current workflow and
an operating system would be needed if the study resumes. The owner has no new
participant available and explicitly deferred the study so engineering can
continue. This removes the human-study gate, not automated release checks. Use the
[human walkthrough and evidence form](../FIRST_HOUR_WALKTHROUGH.md).

Repository: `kujolang/kujo`, main; starting commit
`776c3f0c9fd0b179f0bc7582025083096d5534fc`. Runtime under test: published npm
`@kujolang/kujo-runtime@1.8.1`, macOS x64, installed into an empty local prefix with
lifecycle scripts disabled. This isolates install files; it is **not** a fresh
OS account, VM or human clean-machine walkthrough. Existing host network and
system trust settings remain part of the environment.

## 2026-10-12 owner decision and published-path recheck

The owner explicitly deferred the human study because no new participant is
available. No action to recruit a participant is required now. Continue technical
readiness; retain the optional protocol and disclose the missing usability evidence.

The identical published executable (SHA-256 checked against the original receipt)
now passes the bounded HTTP probe, a fresh Kennel 1.1.1 installation in a new
user-owned directory, and `init`, `add ability`, `install`, `validate`. All exit 0.
See [the recheck receipt](evidence/first-hour-published-recheck-2026-10-12.json).
The original failure is no longer reproduced on this host; its cause remains
unknown and no code fix is claimed. The development binary had been removed
from `target/debug`, so a new development comparison was unavailable and is not
claimed. This recheck uses the actual published runtime throughout.

`cargo fmt --check` and `git diff --check` passed. No code or executable docs
examples changed; runtime tests were not rebuilt for this status-only change.
Original verification and failure observations below remain historical evidence.

## Original rehearsal evidence and fixes

| Check | Observation | Disposition |
| --- | --- | --- |
| Public website installer | Downloaded `https://kujolang.ai/install.sh` defaults to `v1.8.0`; dry-run confirms it | Local docs previously promised current 1.8.1 while inheriting that default. Fixed examples with the supported `KUJO_RELEASE_VERSION=v1.8.1` override; pinned dry-run confirms 1.8.1. Website deployment remains an external follow-up already named in `release/kujo-1.8.1-publication.json`. |
| Published npm installation | Empty-prefix install succeeds; executable reports `kujo 1.8.1` | Verified locally; no global install overwritten and no shell profile changed. |
| Default-VM quickstart | Check and run succeed; prints `Hello, Kujo!` | Agent evidence only. |
| Diagnose and repair | Undefined identifier gives exit 4 and actionable help; replacing it with a string succeeds | Agent evidence only; human comprehension remains unmeasured. |
| Restricted capabilities | File read denied with exit 4 under `--untrusted`; succeeds with only `--allow-fs-read` | Expected behavior, preserved. |
| Published-runtime Kennel install | Installer fetch fails; repeated standalone request and direct packaged-executable invocation also fail | Open blocker on this host. Root cause and other platforms unconfirmed. Do not call package onboarding successful. |
| Development control | Same HTTP request succeeds; separate Kennel 1.1.1 install and documented `init`, `add ability`, `install`, `validate` succeed | Diagnostic control only; never substituted for the failed published-runtime path. |
| Human first hour | No participant session | Owner-deferred 2026-10-12; no duration or success rate invented. |

The version pin affects only the runtime archive. Ecosystem source snapshots still
follow the installer's source selection; this is not an ecosystem-wide lockfile.
No installer or runtime source was changed and no sibling repository was edited.
The README/install docs contract now ties stable one-line examples to the Cargo
version so release updates must refresh them deliberately. The observer protocol
keeps observed failures, help received, participant timing and retests explicit.

## Network blocker reproduction

The credential-free [bounded probe](fixtures/first-hour-http-probe.kujo) makes the
same request as the public Kennel bootstrap and prints only status/error. Run it
with each executable explicitly, not whichever `kujo` happens to be on PATH:

```bash
/path/to/published/kujo run docs/audits/fixtures/first-hour-http-probe.kujo --interpreter
/path/to/development/kujo run docs/audits/fixtures/first-hour-http-probe.kujo --interpreter
```

On this host the published executable reports `HTTP request failed: error sending
request for url (https://kennel.kujolang.ai/install.kujo)` and the probe exits 1.
The development executable prints `200` and exits 0. The original shell installer
also exits 1. Direct invocation rules out the npm launcher as the sole trigger.
Curl can fetch the same resource. No proxy or certificate-path environment
overrides were present among `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, `NO_PROXY`,
`SSL_CERT_FILE` or `SSL_CERT_DIR`. These observations do **not** establish a TLS,
DNS, firewall, packaging or language-runtime root cause. Do not remove certificate
verification, add a fallback transport or suppress the error to make this pass.

Tracked in SignalBox capture `cap_bf4796c2-cf1c-42b6-9a61-79490cf4450c` and
signal `sig_16748c22-a277-4549-bc05-6f5ffd61a9ae`; exact and onboarding-concept
retrieval passed, with no matching duplicate found.

Follow-up should compare the immutable published artifact and environment against
a clean macOS x64 account/runner and inspect connection-error causes. Retest the
actual published installation and registry package flow after a validated fix.
Existing release smoke evidence remains historical; it does not override this
fresh failed onboarding attempt.

## Exact verification and limits

Detailed local logs: `.audit-evidence/first-hour/`. Portable observations and
artifact hashes: [`evidence/first-hour-preparation.json`](evidence/first-hour-preparation.json).

Executed on this host:

```bash
npm install --global --prefix "$PWD/.audit-evidence/first-hour/npm" --ignore-scripts --no-audit --no-fund @kujolang/kujo-runtime@1.8.1
bash .audit-evidence/first-hour/kujo-install.sh --dry-run --prefix "$PWD/.audit-evidence/first-hour/ecosystem" --bin-dir "$PWD/.audit-evidence/first-hour/bin"
KUJO_RELEASE_VERSION=v1.8.1 bash .audit-evidence/first-hour/kujo-install.sh --dry-run --prefix "$PWD/.audit-evidence/first-hour/ecosystem" --bin-dir "$PWD/.audit-evidence/first-hour/bin"
KUJO_BIN="$PWD/.audit-evidence/first-hour/npm/bin/kujo" sh .audit-evidence/first-hour/kennel-install.sh --home "$PWD/.audit-evidence/first-hour/kennel" --no-modify-path
KUJO_BIN="$PWD/target/debug/kujo" sh .audit-evidence/first-hour/kennel-install.sh --home "$PWD/.audit-evidence/first-hour/kennel-development-control" --no-modify-path
cargo test --test readme_contracts --test docs_examples --test release_process_docs_contract
cargo fmt --check
git diff --check
```

The affected documentation tests passed: 10 tests, zero failures. Formatting and
diff checks passed. The first Kennel command failed; the development control passed. Runtime command
arguments/exit codes and package-control outputs are recorded in the receipt.
Cargo checks used `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
CARGO_INCREMENTAL=0`. Runtime sources did not change, so the full runtime/release
matrix is not repeated for these docs and contract-test edits. Windows/Linux,
the full ecosystem install, pristine-OS installation and human comprehension are
not newly verified by this session.

## Remaining work and owners

- **Kujo runtime/release owner:** the published install/package recheck now passes.
  Preserve the earlier network failure for recurrence investigation and require
  final clean-install/platform matrices; root cause remains unknown.
- **Website owner (`kujolang/kujolang.ai`):** deploy the current installer already
  requested by the 1.8.1 publication receipt. Local pinned commands mitigate the
  runtime-version mismatch; they do not deploy the site.
- **Participant/observer (deferred):** if voluntarily resumed, run the human protocol, record actual timing and
  friction, then fix and retest confirmed blockers. No outreach was sent.

No public API, configuration, runtime environment variable, file format or
capability policy changed. Item 6 is deferred and must not be treated as a passed
usability study. Its absence does not block the 1.9 release decision.
