# Implementation roadmap

## Horizon 1 — high-confidence, low-risk

1. Completed `write_file_atomic` for text and bytes with explicit overwrite,
   size limit, same-directory temporary files, sync, cleanup, capability
   classification, and VM/interpreter contract tests.
2. Publish a standard-library cookbook section mapping common wrappers to
   existing `slugify`, `pad_right`, `now_utc`, `env_bool`, `get_default`,
   `to_json_pretty`, and `ProcessResult` APIs.
3. Completed the first-party `modules/cli.kujo` package spike using the
   common token scanner shape from PatchBrief/Muzzle/Lens/ShipCheck, with
   structured missing-value and unknown-option errors. Migration remains a
   follow-up after a compatibility matrix is expanded.
4. Document process argv arrays and output-limit options as the canonical
   agent-safe path; keep shell execution visibly exceptional.

## Horizon 2 — valuable prototypes

1. Prototype `path_within(root, candidate, options)` in Rust with symlink and
   missing-target tests across macOS/Linux; explicitly defer Windows behavior
   until a path contract is written.
2. Prototype `read_bounded(path, max_bytes)` and `walk(root, options)` in a
   package or runtime branch, measuring allocations and symlink/ignore behavior.
3. Prototype `data.decode(value, schema, options)` as a package-level
   typed-access contract using existing JSON Schema and `Result`/`Option`.
4. Create redaction profiles in the existing `redact` repository with stable
   versioned output and audit metadata; migrate one consumer at a time.
5. Create a retry/backoff policy package with deadline, jitter, idempotency,
   and classification fields; do not add it to core until AI SDK and Dispatch
   converge.

## Horizon 3 — deeper design work

- Define structured data path errors and missing/null semantics that compose
  with optional typing and `Result`/`Option`.
- Define effect/error conventions for capability-denied, validation, process,
  and transport failures so packages stop inventing `{ok,error}` shapes.
- Decide whether `arg_parser()` should remain a low-level constructor or be
  replaced by a package-owned declarative parser contract.

## Horizon 4 — defer/reject

Keep shell quoting, broad HTTP policy, pluralization, generic group-by,
automatic redaction, test helpers, and convenience aliases outside the core.

## Migration sequence

For each approved item: (1) write contract tests, (2) implement the canonical
API, (3) migrate one mature consumer, (4) compare edge-case behavior, (5) add
docs/examples, (6) migrate remaining independent consumers, (7) deprecate only
after a release boundary, and (8) add a lint/search check to prevent unsafe
reintroduction. HLP-001 and HLP-004 are implemented. The remaining
candidates retain their dispositions until their contracts and ownership are
ready.
