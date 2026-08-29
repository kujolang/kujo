# Kujo-maintained pulp patch

This directory contains the source of `pulp 0.22.3` as published on crates.io.
Kujo patches that package locally for one narrow maintenance change: the
unmaintained `paste 1.x` proc-macro dependency is replaced by the maintained,
API-compatible `pastey 0.2.3` successor under the same local dependency name.
No upstream `pulp` implementation code is changed.

Provenance:

- Upstream package: `pulp 0.22.3`
- Upstream repository: <https://github.com/sarah-quinones/pulp/>
- Crates.io source archive SHA-256:
  `046aa45b989642ec2e4717c8e72d677b13edd831a4d3b6cf37d9a3e54912496a`
- Local dependency change: `paste = "1"` to
  `paste = { package = "pastey", version = "0.2.3" }`

Remove this patch when a released `pulp` version used by `exr` no longer
depends on `paste`. Until then, `cargo audit --deny warnings` and the release
dependency contract prevent the unmaintained package from returning.
