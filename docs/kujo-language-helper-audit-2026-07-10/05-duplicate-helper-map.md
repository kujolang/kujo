# Duplicate-helper map

```text
filesystem writes
  packwrite.write_text ─┐
  redact.write_text     ├─ delete/overwrite wrappers
  workcell.write_text   │
  tribunal.write_text ──┘
  tribunal.write_text + temp + rename ── atomic-like fork
  casefile.safe_write / mcp.safe_write ── guarded result envelopes
  RAG write_replace_file ── delete/overwrite fork
  => HLP-001: separate atomicity, overwrite, root guard, and result shape

root confinement
  CaseFile.path_within_root
  MCP.is_within_boundary
  Dispatch.path_is_within_root
  SSG.path_is_within_root
  RAG.path_within_root
  Workcell.path_within
  => HLP-002: lexical checks are not symlink-aware equivalently

dynamic dictionaries
  Agents SDK 19 dict_get_or + 18 normalize_dict + 14 normalize_array copies
  Eval/common.dict_get_or
  Dispatch/core/utils.dict_get_or
  AI Chat/Dispatch bridge dict_get_or
  Kennel/utils dict_get_or
  => HLP-003: normalize/validate/diagnose as one boundary contract

CLI parsing
  PatchBrief/Muzzle/Lens/ShipCheck/Kennel parse_subcommand + flag_value
  ChangeBucket/RunLedger/PackWrite/Tribunal/Fence parse_args variants
  => HLP-004: common token scanner, package-owned policy/specification

redaction
  Lens URL/header/DOM/network redaction
  Watchdog telemetry redaction
  CaseFile command/log/argv redaction
  Muzzle/Scent/AI SDK/Eval/Tribunal text or payload redaction
  => HLP-005: profiles in redaction package; secret wrapper in core

wrappers around existing core
  iso_now/today_utc -> now_utc + format_date
  slug -> slugify (sometimes intentionally different policy)
  pad_right -> pad_right
  parse_bool_env -> env_bool
  proc_* -> ProcessResult fields
  print_json/json_string -> to_json/to_json_pretty
  => documentation and naming, not new builtins
```

The map deliberately does not merge helpers solely by name. `safe_write` can
mean overwrite protection, path policy, size limits, or atomic replacement;
those are distinct contracts.
