---
agent: "agent"
description: "Execute active-cycle roadmap work (v0.12.0+) and v1.0.0 readiness tasks with evidence, tests, docs, commits, and push workflow."
---

Read `.github/AGENT_INSTRUCTIONS.md` and follow all rules.

Before starting, read `./notes/README.md`, `./notes/GOTCHAS.md`, and any linked notes relevant to the selected roadmap item.

Review `ROADMAP.md` and determine the active release cycle and highest-priority incomplete item:
- Prefer the highest-priority incomplete item under the active release checklist/section.
- If no explicit checklist exists, pick the highest-priority incomplete roadmap item for the active cycle.
- Treat `v0.12.0` as active unless roadmap text explicitly says otherwise.

Important:
- If the top item is benchmark or release evidence, do not invent a new feature. Run required commands, capture outputs, classify evidence quality, and update release docs.
- If the top item is a stability or implementation task, implement it completely with tests.
- Do not mark any roadmap item complete unless required evidence or verification is actually collected.
- Treat benchmark warning output as release-relevant.
- If benchmark results are collected on a loaded machine, label them as local smoke evidence, not final release-gate evidence, unless roadmap policy explicitly allows exception handling.
- If a release exception is used, document rationale, deterministic checks that passed, and exact follow-up evidence still required.

Requirements:
1. Create a todo list covering investigation, implementation, testing, documentation, commit, and push steps.
2. Inspect current git status before editing; never overwrite unrelated work.
3. Complete the selected roadmap item end-to-end.
4. Add or update comprehensive tests whenever behavior changes.
5. Update only relevant docs when appropriate: `CHANGELOG.md`, `ROADMAP.md`, `README.md`, and notes files for evidence, behavior, release status, or plan changes.
6. Commit after each major completed step, not all at once.
7. Use clear, descriptive commit messages that follow `.github/AGENT_INSTRUCTIONS.md`.
8. Run required verification commands (build/test/benchmark as applicable) and report results clearly.
9. Push completed commits to origin unless explicitly told not to.
10. At the end, summarize:
- what changed,
- what was verified (with commands and pass/fail),
- what remains for the active release cycle and for `v1.0.0` readiness.
