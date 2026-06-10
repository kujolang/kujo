# V1U-OPEN-001 External Handoff: Kujo MCP Closure-Mutation Docs Drift

Date: 2026-05-20  
Checklist link: `docs/PRE_V1_MASTER_UNFINISHED_CHECKLIST.md` (`V1U-OPEN-001`)  
Source checklist item: `docs/PRE_V1_ACTION_CHECKLIST.md` (`PREV1-RUN-002`)

## Why This Is External

This repository does not contain editable `mcp` source docs.

Validation evidence:

- `rg -n "mcp|mcp.kujo" README.md docs notes -g '*.md'`
- `rg --files | rg "mcp|MCP|mcp|mcp.kujo"`

Result: only references and generated external outputs are present here; no source-doc edit target exists for `mcp` docs.

## Handoff Target

- External repository: `mcp` (source repo where `README`/`mcp.kujo` docs live)
- Suggested owner: Kujo MCP maintainers / docs owner
- Handoff ticket ID: `V1U-OPEN-001-HANDOFF-2026-05-20`

## Requested Doc Update

Update any stale wording that claims closure mutation behavior is still limited.

Proposed replacement guidance:

1. State that Kujo runtime now supports named nested closure capture and mutation behavior.
2. Replace legacy caveats that imply this remains unsupported.
3. Link parity evidence to Kujo note:
   - `notes/2026-05-12_23-23_NO-ROADMAP_named-nested-closure-capture-parity.md`

## Validation Steps (in `mcp` repo)

1. Update docs (`README`, `mcp.kujo`, or equivalent canonical page).
2. Confirm no stale closure-mutation limitation language remains.
3. Record commit hash / PR URL.
4. Back-link resulting PR/commit into Kujo notes for traceability.

## Completion Condition For Kujo Side

Once external PR/commit URL is available, append it to this note and to the matching checklist evidence line.
