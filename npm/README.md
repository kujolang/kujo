# Kujo npm distribution

This directory contains the lifecycle-script-free npm packaging for the Kujo
runtime. The public `@kujolang/kujo-runtime` package contains only a small Node.js
launcher. Exact-version optional dependencies provide the native binary for
each supported platform.

No package has `preinstall`, `install`, or `postinstall` scripts. Installation
never downloads or executes code. The launcher resolves an allow-listed
platform package already selected by npm and executes its bundled binary
without a shell.

Run the package tests and a local publication rehearsal with:

```bash
npm test --prefix npm
npm run pack:dry-run --prefix npm
```

The neutral package also exports `resolveKujoBinary()` and
`getKujoRuntimeInfo()`. Platform package source directories deliberately do not contain binaries.
Release automation must stage a freshly built binary at the path declared in
the package manifest before packing it.
