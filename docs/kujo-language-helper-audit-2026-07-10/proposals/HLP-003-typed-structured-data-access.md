# HLP-003 — typed structured-data access

## Problem and evidence

Agents SDK contains 19 copies of `dict_get_or`, 19 `normalize_string`, 18
`normalize_dict`, 14 `normalize_array`, 13 integer normalizers, and 13 deep
cloners. Dispatch, Eval, AI Chat, and Kennel also use local lookup/default
helpers. These are not identical: some unwrap `_self`, some coerce strings,
some preserve arbitrary dictionaries, and some silently replace invalid values
with defaults.

## Root cause and ownership

The root cause is dynamic structured data at package boundaries. Adding more
one-off accessors would cement silent coercion. Start with a package-level
schema/data API, then use the result to inform optional typing and any future
language feature. No immediate syntax addition is justified.

## Proposed API

Proposed package API:

```kujo
decoded := data.decode(payload, {
    type: 'object',
    required: ['name'],
    properties: {'name': {'type': 'string'}}
}, {unknown: 'preserve', nulls: 'reject'})
```

Signature: `data.decode(value: any, schema: dict, options?: dict) ->
Result<any, DataError>`. `DataError` is a dictionary with `path`, `code`,
`expected`, `actual`, and `message`. Add `data.get(value, path, options?)` only
if the decode spike shows a real need; it should return a tagged missing/null
state rather than a magic fallback.

Do not silently stringify numbers, turn invalid arrays into empty arrays, or
collapse null and absent unless the caller opts in. Preserve unknown fields by
default for forward-compatible receipts; reject them only by schema option.

## Alternatives considered

- Add `dict_get_or`: already approximated by `get_default`; insufficient.
- Add `normalize_*` builtins: would standardize policy-free coercion and hide
  malformed AI/model output.
- Require static types immediately: incompatible with Kujo’s optional typing and
  dynamic JSON-heavy workflow style.
- Use existing `json_schema_validate` only: validation exists, but a decoder
  needs conversion, defaults, unknown-field policy, and a stable result shape.

## Safety, performance, and compatibility

Bound schema recursion, input size, node count, and error count using the same
limits as JSON Schema validation. Preserve value identity only where documented;
deep conversion may allocate. Keep schema errors deterministic and ordered by
data path. Package-only avoids core compatibility while the shape evolves.

## Migration

Create an Agents SDK shared internal module first and migrate the three most
copied modules. Then adapt one external consumer, preferably Dispatch or AI
SDK. Keep legacy normalizers during one release and compare malformed-input
behavior before deprecating. Migration is medium complexity because silent
fallback behavior may be relied upon.

## Tests

Cover required/missing/null, wrong scalar type, nested arrays/dictionaries,
unknown fields, defaults, Unicode, large inputs, invalid schema, deterministic
error ordering, `_self` compatibility, preservation of unknown fields, and VM /
interpreter parity if promoted to core. Add AI-output fixtures with malformed
tool arguments and truncated payloads.

## Recommendation

Design further and prototype as a package. Confidence is medium-high for the
problem and low-medium for a final API. The maintainable outcome may be one
schema-aware package plus documentation rather than a core builtin.
