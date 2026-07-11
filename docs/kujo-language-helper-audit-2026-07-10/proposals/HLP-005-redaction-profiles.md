# HLP-005 — redaction profiles outside core

## Problem and evidence

Lens redacts URLs, headers, DOM, network entries, and findings. Watchdog
redacts telemetry values and configured keys. CaseFile handles commands, logs,
argv, and embedded secrets. Muzzle, Scent, AI SDK, Eval, and Tribunal have
additional text/payload redaction. The existing `redact` repository is already
an appropriate first-party home. The semantics are materially different, so
name repetition is not proof of one universal regex.

## Root cause and ownership

The need is a shared policy vocabulary and audited profiles, not a new builtin.
Core `secret` is the right primitive for values that should never serialize in
plaintext. Explicit redaction profiles belong in the `redact` package, with
domain adapters for URLs, headers, logs, and structured dictionaries.

## Proposed API

Proposed package API:

```kujo
profile := redact.profile('telemetry-v1', {
    keys: ['authorization', 'api_key', 'token'],
    patterns: []
})
result := redact.apply(value, profile, {'audit': true})
```

Signature: `redact.apply(value: any, profile: RedactionProfile,
options?: dict) -> Result<dict, RedactionError>`. Success returns `value`,
`changed`, `count`, `profile`, and optional audit entries that never include
plaintext matches. Structured dictionaries should recurse with depth and node
limits; strings should use explicit patterns. URL/header adapters should know
about authorization syntax and percent-encoding instead of applying generic
replacement.

## Security requirements

Never return or log the original value in an error. Redact before persistence,
rendering, telemetry, or AI cassette metadata. Make profile version and match
count deterministic. Test secrets shorter than minimum lengths, overlapping
patterns, Unicode, multiline logs, JSON escaping, URL userinfo, bearer tokens,
private keys, already-redacted values, and custom patterns. Treat `--no-redact`
as an explicit high-risk policy, not a default.

## Alternatives considered

- Add a global `redact(value)` builtin: too opinionated and unsafe for domain
  formats; it could create false confidence.
- Use only `secret`: excellent for values held in memory, insufficient for raw
  logs and third-party payloads already materialized as strings.
- Merge all existing implementations: their threat models and audit metadata
  differ; use profiles and adapters instead.

## Migration and compatibility

Start with the existing `redact` package and publish profile versions. Migrate
Muzzle or CaseFile first, then Watchdog and Lens. Keep old adapters as shims
until output fixtures and incident-review samples match. Adding a profile is
additive; changing a profile’s output is a compatibility change and needs a
version bump.

## Agent benefit and performance

Agents get one visible, reviewable redaction call and a profile name instead of
inventing regexes. Structured recursion is O(n) in nodes plus pattern cost;
enforce maximum depth, input bytes, and regex size. Avoid implicit redaction of
ordinary strings because it can corrupt source, hashes, or user content.

## Recommendation

Centralize outside core in the first-party `redact` package. Confidence is
medium: cross-repository demand is clear, but profile semantics require a threat
model and migration fixtures before becoming a stable contract.
