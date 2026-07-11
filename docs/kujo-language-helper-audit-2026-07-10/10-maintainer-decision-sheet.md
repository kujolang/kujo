# Maintainer decision sheet

| ID | Evidence minimum | Proposed owner | Decision |
|---|---|---|---|
| HLP-001 | 4+ independent write patterns; atomicity tests | Kujo runtime/stdlib | ☑ implemented |
| HLP-002 | CaseFile/MCP/Dispatch/SSG/RAG path cases; symlink matrix | Kujo runtime/stdlib | ☐ approve ☐ prototype ☐ revise ☐ defer |
| HLP-003 | Agents SDK copy cluster plus one external consumer; schema spike | language + data package | ☐ approve design ☐ prototype ☐ docs only ☐ defer |
| HLP-004 | 5 CLI implementations; strict parser compatibility tests | first-party CLI package | ☑ implemented spike |
| HLP-005 | redaction package contract + Lens/CaseFile/Watchdog migrations | `redact` package | ☐ approve ☐ prototype ☐ package instead ☐ reject |
| HLP-006 | nested/platform directory tests | filesystem package | ☐ approve ☐ prototype ☐ docs only ☐ defer |
| HLP-007 | current builtin examples | Kujo docs | ☐ docs only |
| HLP-008 | bounded read performance/security matrix | runtime/package | ☐ prototype ☐ defer |
| HLP-009 | ignore/symlink/order contract from 3 tools | package | ☐ prototype ☐ defer |
| HLP-010 | stable `{ok,value,error}` file envelope | package | ☐ package instead ☐ defer |
| HLP-011 | existing env APIs | Kujo docs | ☐ docs only |
| HLP-012 | 3 consumers with same root/config semantics | tooling package | ☐ package instead ☐ defer |
| HLP-013 | ProcessResult docs/examples | Kujo docs | ☐ docs only |
| HLP-014 | AI SDK + Dispatch policy convergence | official package | ☐ prototype ☐ defer |
| HLP-015 | current deterministic JSON contract | Kujo docs | ☐ docs only |
| HLP-016 | test-support package demand | test package | ☐ defer |
| HLP-017 | typed error/result design note | language/package design | ☐ design ☐ defer |
| HLP-018–022 | stronger independent evidence and stable semantics | package/local/none | ☐ defer ☐ reject |

**Completed in this pass:** HLP-001 implementation, HLP-004 package spike, and
documentation closures for HLP-007, HLP-011, HLP-013, and HLP-015.

**Next review boundary:** HLP-002 path semantics and HLP-003 structured-data
semantics require new cross-platform/design evidence before implementation.
