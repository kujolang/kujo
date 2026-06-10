# Kujo Doctor Report Schema

Schema version: `0.1.0`

## Top-level fields

```json
{
  "schema_version": "0.1.0",
  "tool": "kujo-doctor",
  "command": "doctor",
  "profile": "generic",
  "status": "pass|warn|fail",
  "summary": {},
  "checks": [],
  "recommended_next_actions": []
}
```

Additional compatibility fields are included for workflow-pack integration (`pack`, `namespace`, optional `cwd`).

## Check fields

- `id` (string): stable identifier (`env.node`, `repo.working_tree`, etc.)
- `label` (string): human label
- `status` (enum): `pass|warn|fail|skip|info`
- `severity` (enum): `info|low|medium|high|critical`
- `category` (optional string): `environment|repository|dependencies|project|build|runtime`
- `reason` (optional string): machine-readable reason code
- `observed` / `expected` (optional string)
- `observed_major` / `minimum_major` (optional integer)
- `message` (optional string)
- `suggested_fix` (optional string)

## Reason values

Current reason values include:

- `missing`
- `version_too_old`
- `version_unparseable`
- `dependency_missing`
- `dirty_worktree`
- `command_noisy`
- `not_git_repo`
- `not_applicable`
- `config_missing`

## Recommended-next-actions behavior

`recommended_next_actions` is generated from structured check metadata (`id`, `reason`, `status`, `suggested_fix`) and deduplicated.

No action generation relies on parsing English message text.

## Versioning expectations

- Backward-compatible schema evolution should preserve existing keys/semantics.
- Breaking changes should increment `schema_version` and include migration notes.
