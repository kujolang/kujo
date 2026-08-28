# Kujo language token density

## Findings

The language itself is compact for tiny scripts: `examples/hello.kujo` is 50 bytes (~13 heuristic tokens), and a simple test is 111 bytes (~28). Agent SDK examples range from 279–441 estimated tokens. This makes language syntax a lower-confidence first target than context assembly.

The main accidental verbosity appears in ecosystem glue written in Kujo: large runner/dispatch/scout/scent files contain repeated normalization, cloning, validation, and envelope-building helpers. This improves explicitness and safety but increases the tokens an agent must read when modifying infrastructure.

Valuable verbosity includes capability boundaries, explicit result shapes, structured errors, schema validation, deterministic ordering, and test fixtures. Risky candidates include shorthand syntax, implicit imports, implicit error handling, magical context selection, or hiding permissions in defaults.

## Decision matrix

| Candidate | Token savings | Human readability | Agent readability | Safety | Compatibility | Recommendation |
|---|---|---|---|---|---|---|
| New syntax shorthand | unknown/low | lower | uncertain | risk | breaking/migration | Do not pursue without real task corpus |
| Standard-library envelope builders | medium in repeated glue | higher | higher | positive if explicit | additive | Experiment after measuring generated patterns |
| Generated canonical CLI/schema boilerplate | medium | higher | higher | positive | additive | Strong experiment |
| Implicit context or imports | potentially high | lower | lower | negative | risky | Do not change |
| Better symbol/docgen output | indirect/high | positive | positive | positive | additive | Go after instrumentation |

No language change is justified by current evidence. The likely language-level win is canonical generators and richer agent-readable indexes, not terser syntax.
