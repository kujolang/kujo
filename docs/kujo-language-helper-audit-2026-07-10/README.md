# Kujo language helper and standard-library opportunity audit

Audit date: 2026-07-10

This package records an evidence-led review of the Kujo workspace. It separates
missing primitives from wrappers around existing APIs, copied/template code,
domain policy, and language-design questions.

Start with [00-executive-summary.md](00-executive-summary.md), then use the
[maintainer decision sheet](10-maintainer-decision-sheet.md) to disposition the
candidates. The [candidate verification record](11-candidate-verification.md)
records the current implementation/doc/example checks for the four documented
HLP candidates. The JSON file is intentionally small and stable enough for
later automation.

Coverage: 39 git repositories under `/Users/robertdevore/2026/Kujolang/kujo-repos`.
Generated, vendored, node_modules, target, output, benchmark fixtures, and test
only copies were excluded from independent-implementation counts unless they
were needed to verify a contract.
