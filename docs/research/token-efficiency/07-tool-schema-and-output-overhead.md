# Tool schema and output overhead

## Evidence

The Agents SDK tool contract includes id, name, description, input schema, output schema, permissions, risk, timeout, and metadata (`agents-sdk/src/agents/tools/registry.kujo:215-238`). `list_tools` returns metadata views for all registered tools (`registry.kujo:1094-1118`). This is safe and inspectable, but a full registry can be unnecessarily expensive when a task only needs a subset.

Tool execution already validates input, applies approval, enforces timeouts, sanitizes output, and can emit artifacts (`registry.kujo:676-839`). Those fields and evidence must remain available even if the model-facing view is compact.

## Directions

- Register tools by capability/task scope; expose a compact catalog first and full schema on deterministic discovery.
- Prefer stable schema IDs and hashes in repeated turns, with explicit retrieval when a hash is unknown.
- Return summary + artifact reference by default; fetch full logs, diffs, and stack traces on error or request.
- Keep machine error codes and redacted evidence; do not hide approval/risk/permission fields.

Provider compatibility is an experiment: some providers require complete tool schemas in the request and do not support schema references. The adapter must expand references deterministically when required and record the expansion cost.
