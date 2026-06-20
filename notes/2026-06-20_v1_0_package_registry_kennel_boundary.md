# V1.0 Package Registry And Kennel Boundary

Date: 2026-06-20
Checklist item: V1RR-P2-002

## Decision

Kujo v1.0 includes local package and workflow-pack workflows only:

- `kujo init`
- `kujo package-add`
- `kujo package-install`
- `kujo package-install --frozen`
- local workflow-pack discovery and `kujo pack run`

Kujo v1.0 does not include a public Kennel registry, remote package resolution,
registry authentication, upload transport, or a first-party `kennel.toml` for
the language/runtime repository.

`kujo package-publish` remains a metadata preview command. The `--publish` flag
is reserved for future registry transport and now fails deterministically
instead of printing a misleading `published` line.

## Updated Surfaces

- `README.md`
- `docs/RELEASE_PROCESS.md`
- `docs/WORKFLOW_PACKS.md`
- `docs/KENNEL_NAMESPACE_PLAN.md`
- `docs/SHIPCHECK_RELEASE_EXCEPTIONS.md`
- `src/main.rs`
- `tests/package_module_workflow_integration.rs`
- `tests/package_registry_boundary_docs_contract.rs`

## Validation

Command logs and exit statuses are recorded in
`notes/release_evidence/2026-06-20_p2-002/status.tsv`.
