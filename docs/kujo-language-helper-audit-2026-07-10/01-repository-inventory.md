# Repository inventory

## Method and weighting

The workspace was enumerated by `.git` directories, then source files were
counted with `rg --files`. Broad helper searches excluded generated output,
vendored assets, `node_modules`, `target`, fixtures, and tests for the primary
duplication pass. Evidence weights were: **5** mature first-party production
code; **4** usable tool with contracts/tests; **3** prototype or showcase; **2**
benchmark/workflow fixture; **1** generated or copied evidence. Counts below
are approximate source-file counts, not LOC.

## Core, libraries, and developer tools

| Repository | Purpose/class | Approx. source | Weight |
|---|---|---:|---:|
| kujo | language, compiler, VM, runtime, CLI | 413 `.kujo`, 171 Rust | 5 |
| kennel | package/project manager | 38 `.kujo` | 5 |
| agents-sdk | agent runtime primitives | 53 `.kujo` | 4 |
| ai-sdk | provider SDK/retries/streaming | 16 `.kujo` | 4 |
| mcp | MCP server framework | 25 `.kujo` | 4 |
| rag | ingestion/retrieval/query kit | 117 `.kujo` | 4 |
| watchdog | telemetry proxy/dashboard | 10 `.kujo`, 37 JS | 4 |
| scout | codebase intelligence | 8 `.kujo`, 19 Python | 4 |
| eval | deterministic evaluation | 15 `.kujo` | 4 |
| spec | task contracts/schema | 5 `.kujo`, 3 Python | 4 |
| dispatch | workflow orchestration | 33 `.kujo` | 4 |
| fence | architecture-boundary linter | 31 `.kujo`, 2 Python | 4 |
| casefile | failure evidence bundles | 11 `.kujo` | 4 |
| runledger | run receipts | 8 `.kujo` | 4 |
| changebucket | change footprint/budgets | 9 `.kujo` | 4 |
| patchbrief | diff briefs | 7 `.kujo` | 4 |
| shipcheck | release readiness | 4 `.kujo` | 4 |
| concord | artifact drift checks | 12 `.kujo` | 4 |
| muzzle | quiet workflow runner | 7 `.kujo` | 3 |
| scent | context packs | 1 `.kujo` | 3 |
| packwrite | agent pack compiler | 12 `.kujo` | 3 |
| howl | showcase renderer | 13 `.kujo` | 4 |
| redact | redaction utility | 11 `.kujo` | 4 |
| tribunal | integration/audit tool | 19 `.kujo` | 3 |
| relay | workflow adapter | 10 `.kujo` | 3 |
| workcell | workflow/CLI tool | 13 `.kujo` | 3 |
| cinch | Rust desktop/application surface | 45 Rust | 3 |
| intake | JavaScript intake app | 52 JS | 3 |
| site-kit | design/component system | 1 JS plus assets | 2 |

## Applications, examples, and workflow collections

| Repository | Purpose/class | Approx. source | Weight |
|---|---|---:|---:|
| ai-chat | multi-provider chat app | 1 `.kujo`, 9 JS | 3 |
| cms | CMS showcase | 18 `.kujo` | 3 |
| crud-api | CRUD API showcase | 11 `.kujo` | 3 |
| ssg | static-site generator | 1 `.kujo`, 1 Python | 3 |
| kujo-workflows | workflow packs/demos | 1 `.kujo`, 8 JS | 2 |
| kujo-agents | Python/agent examples | 4 Python | 2 |
| kujo-skills | skill documentation | docs only | 2 |
| kujo-hyperframes | visual/static asset repo | 2 JS | 1 |
| benchmarks-capsule | benchmark runner/fixtures | 17 `.kujo`, mixed | 2 |
| kujo-benchmarks | benchmark capsules/fixtures | mixed | 1 |
| frontier-skills | evaluation/skill material | docs/scripts | 1 |

## Coverage boundary

The named prompt list also mentions Doctor, Security, SITREP, DocsGen, and
BZBY. Doctor and Security are Kujo-core namespaces, not separate git repos.
No SITREP or BZBY repository was present. DocsGen behavior was considered where
it appeared in Kujo core and workflow tooling. The older
`ecosystem-pattern-audit-2026-06-11` directory was used as historical context,
not counted as a production repository or independent implementation.

## Evidence classification rules

- **Independent**: semantically similar code authored in a distinct mature
  tool, counted separately.
- **Copy/template**: near-identical helper or starter scaffold; counted as one
  cluster and called out separately.
- **Wrapper**: delegates to an existing Kujo builtin; treated as discoverability
  evidence, not missing functionality.
- **Fork**: same purpose with materially different policy or edge cases; kept
  separate until semantics stabilize.
- **Test/generated**: retained only to verify behavior contracts, not ecosystem
  spread.
