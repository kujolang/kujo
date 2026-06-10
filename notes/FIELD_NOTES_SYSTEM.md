# Kujo AI Agent Field Notes & Gotchas System

This document defines a **mandatory post-work note-taking system** for AI agents working on the Kujo programming language. The goal is to capture *hands-on, experience-based knowledge* so future agents build faster, avoid mistakes, and develop correct mental models of the codebase.

This is not documentation.
This is **operational memory**.

---

## 🎯 Purpose

After implementing features, fixing bugs, refactoring, or debugging Kujo, the AI agent must distill what it learned into structured Markdown notes.

These notes should capture:

* surprising behavior
* incorrect assumptions
* fragile areas of the code
* ordering constraints
* edge cases
* rules the code implicitly relies on

Future agents should be able to read these notes and *immediately* avoid repeated mistakes.

---

## 📁 Folder Structure

```
./notes/
├── YYYY-MM-DD_HH-mm_short-kebab-summary.md
├── YYYY-MM-DD_HH-mm_another-session.md
├── GOTCHAS.md
└── README.md (optional index)
```

* One notes file per work session
* Notes are append-only (never overwrite or rename old files)

---

## 🧾 Session Notes File Rules

### Filename format (required)

```
YYYY-MM-DD_HH-mm_short-kebab-summary.md
```

Examples:

* `2026-01-25_10-14_parser-gotchas.md`
* `2026-01-25_18-02_vm-scope-fix.md`

If `./notes/` does not exist, create it.

---

## 🧠 When Notes Must Be Written

Create or update a session notes file after:

* adding a feature
* fixing a bug
* debugging a test failure
* changing parser / lexer / AST / evaluator / runtime behavior
* modifying CLI behavior
* discovering surprising or non-obvious behavior
* correcting an incorrect assumption

**One work session = one notes file**

---

## 🧱 Required Session Notes Template

Each session notes file **must use this exact structure**:

```md
# Kujo Field Notes — <short human title>

**Date:** <YYYY-MM-DD>
**Session:** <HH:mm local>
**Branch/Commit:** <branch name> / <commit hash (if known)>
**Scope:** <1–2 sentences describing what you worked on>

---

## What I Changed
- <bullet list of concrete changes>
- <include file paths when helpful>

## Gotchas (Read This Next Time)
- **Gotcha:** <what surprised you>
  - **Symptom:** <what you observed>
  - **Root cause:** <why it happened>
  - **Fix:** <what resolved it>
  - **Prevention:** <how to avoid it next time>

(repeat for each gotcha)

## Things I Learned
- <mental model updates>
- <rules of thumb>
- <implicit invariants or ordering constraints>

## Debug Notes (Only if applicable)
- **Failing test / error:** <exact error output>
- **Repro steps:** <how to reproduce>
- **Breakpoints / logs used:** <where you looked>
- **Final diagnosis:** <what it actually was>

## Follow-ups / TODO (For Future Agents)
- [ ] <specific next step>
- [ ] <tech debt introduced or deferred>

## Links / References
- Files touched:
  - `<path>`
  - `<path>`
- Related docs:
  - `README.md`
  - `ROADMAP.md`
  - <other docs>
```

---

## ✍️ Writing Style Rules (Critical)

* Write like you are leaving a note to your **future self** with partial context
* Be concrete, not theoretical
* Include file paths, function names, enums, structs, and commands
* If something *will surprise someone*, call it out explicitly
* If something relies on an ordering constraint, write it as a **rule**
* If unsure what to write, write *less*, but make it *specific*

### 🚨 Mandatory Capture Rule: "Justified Behavior"

If the agent has to **justify** why something is OK, expected, intentional, or safe, it **must be documented**.

This includes moments where the agent says or implies:

* "This warning is expected"
* "This is intentional"
* "This is safe because…"
* "We can ignore this"
* "It only happens in X, not Y"
* "This is a known limitation"
* "We’ll clean this up later"

These statements compress non-obvious reasoning and **will not be obvious from the code alone**.

When this happens, add an entry under **Gotchas** or **Things I Learned**.

### Example (Required Documentation Pattern)

```md
- **Gotcha:** Compiler warning about `Spread` being unused as an `Expr`
  - **Symptom:** Warning emitted during compile about `Spread` enum variant
  - **Root cause:** `Spread` is intentionally NOT a standalone `Expr`
  - **Fix:** None — this is expected behavior
  - **Prevention:** Do not refactor `Spread` into `Expr` without redesigning
    array/dict element handling. `Spread` is only valid within
    `ArrayElement` and `DictElement`.
```

### Optional (High Value): Assumptions I Almost Made

If applicable, add:

```md
## Assumptions I Almost Made
- I initially assumed `Spread` should be an `Expr`, but that breaks
  contextual evaluation rules
```

🚫 Avoid:

* generic explanations
* textbook definitions
* restating obvious code behavior

---

## 🧠 Curated GOTCHAS.md (Deduplicated)

In addition to session notes, maintain a **curated, long-lived gotchas file**:

```
./notes/GOTCHAS.md
```

### Purpose

* High-signal summary of the most important pitfalls in Kujo
* Deduplicated across sessions
* Clean, readable, and short
* Suitable for onboarding new agents

Session notes are raw.
`GOTCHAS.md` is refined.

---

## 📘 GOTCHAS.md Structure

```md
# Kujo — Known Gotchas & Sharp Edges

This document contains the most important non-obvious pitfalls in the Kujo codebase.

If you are new to the project, read this first.

---

## Parser & Syntax

### Expression precedence is NOT inferred
- **Problem:** <what breaks>
- **Rule:** <explicit rule the parser expects>
- **Why:** <design rationale>

## Runtime / Evaluator

### Variable scope is resolved at <stage>
- **Problem:** <symptom>
- **Rule:** <how scope resolution actually works>
- **Implication:** <what not to assume>

## CLI & Tooling

### Tests must be run with <specific command>
- **Problem:** <failure mode>
- **Rule:** <correct workflow>

---

## Mental Model Summary

- Kujo favors <design philosophy>
- The parser assumes <key assumption>
- The runtime guarantees <invariant>
- Do NOT assume <common incorrect assumption>
```

### Rules for Updating GOTCHAS.md

* Only add **confirmed, repeated, or high-impact** issues
* Merge duplicates instead of appending
* Prefer rules over stories
* Reference session notes where the discovery came from

Example:

```md
(Discovered during: 2026-01-25_10-14_parser-gotchas.md)
```

---

## 📑 Optional: notes/README.md Index

If the notes folder grows large, maintain an index:

```md
# Kujo Field Notes Index

- 2026-01-25_10-14_parser-gotchas.md — Parser edge cases & failure modes
- 2026-01-25_18-02_vm-scope-fix.md — Runtime scoping corrections
```

Only add entries for **high-signal sessions**.

---

## ✅ Definition of Done (Recommended)

A task is **not complete** unless:

* code compiles
* tests pass
* **session notes are written**
* GOTCHAS.md is updated *if applicable*

This system turns AI experience into durable project knowledge.