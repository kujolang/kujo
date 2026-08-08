# Security Response

Status: stable v1.0.0 policy
Last updated: 2026-06-27

This page describes how Kujo maintainers should handle security reports for the language/runtime repository. It is a project policy document, not a warranty or a claim that Kujo is a sandbox.

## Reporting

Report suspected vulnerabilities privately to the project maintainers through the repository's private security advisory channel when available, or through the maintainer contact path listed by the project owner.

Do not include active credentials, private model prompts, proprietary datasets, or exploitable payloads beyond what is required to reproduce the issue.

## Scope

In scope:

- Capability bypasses in `--untrusted` mode.
- AI egress allowlist bypasses.
- Private-network destination-policy bypasses.
- Secret redaction failures.
- Unsafe archive/filesystem traversal.
- Panics or unbounded resource use reachable from ordinary Kujo source.
- Release artifact integrity problems.

Out of scope:

- Running trusted Kujo scripts with ambient host privileges.
- Data exfiltration by code intentionally granted the needed capabilities.
- Provider-side AI model behavior unrelated to Kujo request handling.
- Denial of service that requires already-unbounded host resources and no Kujo-specific trigger.

## Embargo And Triage

Maintainers should acknowledge new private reports promptly, reproduce the issue in a private branch or local fixture, and avoid public details until a fix and release plan are ready.

When the report affects a supported release, prepare a patch release and advisory. For unreleased development branches, document the fix in `CHANGELOG.md` and the relevant verification evidence before tag review.

## Supported Versions

The supported release line is `1.0.x`. Security fixes target `main` and the latest supported patch release. This policy does not promise long-term support beyond the current v1 line.

## Disclosure Checklist

Before public disclosure:

1. Add or update a regression test.
2. Verify the relevant security gate or `scripts/enterprise_verify.sh --full`.
3. Update `CHANGELOG.md`.
4. Update affected operator docs.
5. Publish the advisory or release note with mitigation guidance.
