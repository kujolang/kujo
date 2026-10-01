# Kujo Field Notes — Published 1.7 installation defaults

**Date:** 2026-10-01
**Session:** 09:10 local
**Branch/Commit:** codex/stable-v1.7-installation / based on 1413ed2
**Scope:** Synchronize defaults after verified native/npm publication

---

## What I Changed

Updated current installation instructions, setup action and installer defaults to
published 1.7.0. Added a separate publication record; candidate preparation
metadata remains historical. The identity test uses the publication record when
available, otherwise the candidate's previous stable release.

## Gotchas (Read This Next Time)

Tag existence alone is insufficient. Both native and npm published-install
matrices passed on all five targets before these changes. The initial Windows
native smoke hit GitHub API rate limiting; its retry passed without changing
artifacts. Crates.io remains independent and unpublished.

## Things I Learned

The adapter's five-target registry installation matrix also passed at 5f0ce50,
including real canonical tools and exact receipts. A separate registry-safe HTTP
fork is under review; it does not alter the signed 1.7.0 release.

## Debug Notes (Only if applicable)

- Release-state guard: passed with stable 1.7.0 references.
- Installer release-manifest contract: passed, including explicit overrides.
- Formatting: checked after the identity-test update.
- Initial local identity-test compilation exhausted disk space; generated build
  artifacts were cleaned with Cargo and the check restarted without debug info.
  Its final result must be checked separately; no failed check is counted as pass.

## Follow-ups / TODO (For Future Agents)

Finish the targeted identity test and synchronize the website installer. Verify
live installer bytes and a disposable installation after deployment. Keep the
crate publication exception explicit until normalized registry packaging passes.

## Links / References

- [Native smoke](https://github.com/kujolang/kujo/actions/runs/36857820990)
- [npm smoke](https://github.com/kujolang/kujo/actions/runs/36866507729)
- [Adapter registry acceptance](https://github.com/kujolang/kujo-openai/actions/runs/36866757041)
- [Publication record](../release/kujo-1.7.0-publication.json)
