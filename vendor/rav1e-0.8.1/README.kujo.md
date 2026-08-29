# Kujo-maintained rav1e patch

This directory contains the source of `rav1e 0.8.1` as published on crates.io.
Kujo patches that package locally for one narrow maintenance change: the
unmaintained `paste 1.x` proc-macro dependency is replaced by the maintained,
API-compatible `pastey 0.2.3` successor under the same local dependency name.
No upstream `rav1e` implementation code is changed.

Provenance:

- Upstream package: `rav1e 0.8.1`
- Upstream repository: <https://github.com/xiph/rav1e/>
- Crates.io source archive SHA-256:
  `43b6dd56e85d9483277cde964fd1bdb0428de4fec5ebba7540995639a21cb32b`
- Local dependency change: `paste = "1.0"` to `paste = { package = "pastey",
  version = "0.2.3" }`

Remove this patch when a released `rav1e` version selected by `ravif` no longer
depends on `paste`. Until then, `cargo audit --deny warnings` and the release
dependency contract prevent the unmaintained package from returning.
