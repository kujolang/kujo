# Kujo native platform packages

These manifests are release templates. The corresponding release runner copies
its tested `kujo` or `kujo.exe` binary into `bin/` before `npm pack`. A source
checkout intentionally contains no placeholder executable or generated
`metadata.json`.

Every packed platform tarball includes deterministic provenance metadata with
the Kujo runtime version, the full source commit, the package target, and the
SHA-256 digest of the bundled executable.
