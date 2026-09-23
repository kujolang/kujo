# Kujo language helper and standard-library opportunity audit

Audit date: 2026-07-10

This package records an evidence-led review of the Kujo workspace. It separates
missing primitives from wrappers around existing APIs, copied/template code,
domain policy, and language-design questions.

The original narrative reports and decision sheet are not present in this
repository. The retained [candidate ledger](helper-candidates.json) is the
reviewable source artifact for this historical audit.

Coverage: 39 git repositories under `/Users/robertdevore/2026/Kujolang/kujo-repos`.
Generated, vendored, node_modules, target, output, benchmark fixtures, and test
only copies were excluded from independent-implementation counts unless they
were needed to verify a contract.
