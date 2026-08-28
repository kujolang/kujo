# Skills and reference loading

## Inventory

The installed skills pack contains 84 `SKILL.md` files and 348,787 bytes. The largest individual file is ~2,350 heuristic tokens. The index itself is a routing catalog and should be available to the router, not necessarily to every worker.

## Findings

1. Skill payloads are prose Markdown with repeated policy vocabulary. There is no common manifest declaring core instructions, optional references, triggers, token estimate, version, or dependencies.
2. Several skills point to repository docs and tests, but the contract for loading a reference at the exact step is convention-based rather than machine-enforced.
3. `agents/openai.yaml` files provide default prompts for some skills, but this is provider-facing metadata and does not establish provider-neutral lazy loading.
4. `kujo-skills/SKILLS_INDEX.md` already routes to the narrowest skill. This is the strongest existing basis for Layer 1 selection.

## Recommended shape

Each skill should eventually expose a versioned manifest containing `id`, `core`, `references`, `triggers`, `capabilities`, `security_always`, `estimated_bytes`, `hash`, and `tests`. Markdown remains the human source; generated metadata prevents a second independent source of truth.

Behavioral equivalence must be proven with skill trigger tests, reference-fetch tests, adversarial missing-reference tests, and paired task evaluations. No prose should be removed merely because it is repeated until coverage demonstrates the same decisions and guardrails.
