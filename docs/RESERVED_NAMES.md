# Reserved Names

Kujo now enforces a reserved-name policy so external workflow packs and future packages cannot claim critical CLI or ecosystem names.

## Why this exists

Without reservation rules, third-party extensions can collide with core commands and first-party ecosystem names, making the CLI ambiguous and unsafe to evolve.

## Source of truth

Reserved names live in:

- `config/reserved_names.toml`

The file is loaded and validated by:

- `src/reserved_names.rs`

## Reserved-name categories

- `core_commands`: top-level CLI command names owned by Kujo core
- `workflow_families`: reserved command families that can be extended via contribution points
- `first_party_tools`: names reserved for first-party Kujo tools
- `reserved_namespaces`: namespace tokens external packs cannot claim
- `reserved_package_names`: package names external packages cannot claim
- `reserved_profile_names`: profile names reserved for core/official behavior
- `blocked_aliases`: generic alias-like names blocked from top-level namespace routing
- `first_party_pack_ids`: exact pack IDs accepted only by trusted builtin registration

## What is enforced now

- External alias routing (`kujo <namespace> <command>`) rejects reserved top-level names and reserved namespaces before workflow-pack lookup.
- Workflow-pack manifest validation rejects reserved namespaces for third-party packs.
- Workflow-pack registration rejects spoofed first-party pack IDs.
- `kujo.toml` package parsing rejects reserved package names for third-party manifests.

## First-party exceptions

First-party exceptions are explicit and trust-scoped.

- Workflow packs: trust is tied to internal first-party/builtin registration flows, not a user-provided manifest flag.
- Package names: code paths can opt into `PackageTrust::FirstParty`; third-party parsing remains restricted.

There is no `official: true` escape hatch in external manifests.

## How to update reserved names

1. Edit `config/reserved_names.toml`.
2. Keep entries lowercase and unique within each category.
3. Keep every repository in `install.sh`'s ecosystem profile catalog represented
   in `first_party_tools`, even when it is a library or showcase without a CLI.
4. Add/adjust tests where behavior changes.
5. Update this doc if categories or semantics change.

## Guidance for external package authors

- Use a non-reserved namespace and run commands via:
  - `kujo pack run <namespace> <command>`
- Use contribution points (for example `contributes.doctor_profiles`) instead of claiming reserved core families directly.
