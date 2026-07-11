# Language friction report

## 1. Dynamic records lack a canonical validation boundary

Evidence: 19 Agents SDK modules copy normalization helpers, while Dispatch,
Eval, AI Chat, and Kennel also use `dict_get_or`. Current `get_default` handles
one lookup, but it cannot express “required string at `request.tools[2].name`” or
preserve whether a field was absent versus explicitly null. The root cause is a
data-model/diagnostic boundary, not missing `dict_get_or` aliases.

Recommendation: prototype a package-level schema accessor that returns
`Result::Ok(value)`, `Result::Err({path, code, expected, actual})`, and an
explicit missing/null policy. Feed the result into optional typing before
considering syntax. See HLP-003 and HLP-017.

## 2. Effects are present but not discoverable as a unified model

Core has `Result`/`Option`, `try`/`except`, capability-gated I/O, structured
process results, and typed AI errors, but ecosystem modules often wrap failures
into ad hoc `{ok,error}` dictionaries. This is partly a documentation and
interoperability problem. A future design should specify when a helper returns a
tagged value versus throwing, and how `?`/propagation interacts with capability
errors. Do not add a second result family.

## 3. VM/interpreter parity raises the cost of small native additions

Kujo is VM-first with an interpreter fallback. Every core native helper needs
registration, capability classification, documentation, and parity/security
coverage. That makes package-first solutions preferable for CLI policy,
redaction profiles, retries, and project discovery.

## 4. Filesystem security needs runtime semantics, not string utilities

Local prefix checks do not agree on absolute paths, `..`, symlinks, missing
parents, Windows drive syntax, or case sensitivity. A correct HLP-002 primitive
must be implemented with canonicalization and explicit “target may not exist”
semantics in Rust, then exposed narrowly.

## 5. Core CLI API is incomplete at the policy layer

`arg_parser()` constructs an empty `ArgParser`; application code still scans
arrays and defines flag/value behavior independently. The stable part is token
recognition. Help rendering, subcommands, environment defaults, and exit codes
belong in an official package or application layer.

## 6. Existing capability discoverability is a real friction source

Repeated wrappers around collection, environment, time, path, process, JSON,
and secret functions indicate that examples and cross-links are insufficient.
The baseline should be added to an agent-facing cookbook with “current API vs
common wrong wrapper” examples before aliases are considered.
