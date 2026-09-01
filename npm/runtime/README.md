# `@kujolang/kujo-runtime`

This package installs the `kujo` command from a platform-specific optional
dependency. It performs no network access and runs no npm lifecycle scripts.

The package also exports `resolveKujoBinary()` and `getKujoRuntimeInfo()` for
tools that need to locate the native executable without starting it. Runtime
information includes the package/runtime versions, binary path, and bundled
source. TypeScript declarations are included. Typed resolution failures include
stable `code`, `platform`, and `arch` fields.

Supported targets are Linux x64/arm64, macOS x64/arm64, and Windows x64.
Installing with optional dependencies disabled omits the runtime binary;
the launcher reports that condition with remediation guidance.
