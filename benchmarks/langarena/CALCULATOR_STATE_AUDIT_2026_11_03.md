# Calculator state audit: checksum overflow reproduced

The unchanged canonical Calculator::Interpreter port fails in VM, interpreter
and JIT when an existing result of 4,294,967,290 is added to the evaluated signed
value 9,223,372,036,854,775,807. Its ordinary addition overflows before the outer
`u32` conversion. This is a port accumulator bug, not a runtime arithmetic bug:
checked ordinary addition is the language contract.

Pinned Rust uses `wrapping_add(result as u32)`; Goose adds the converted u32
result. The isolated candidate changes only the fixture's accumulator expression
to `u32(wrapping_add(state["result"], value))`. The canonical source remains
unchanged while the frozen production sequence is active. This report does not
claim that the port fix has been integrated.

With that isolated correction, 252 complete states pass: fourteen valid programs,
both calculator wrappers, three repeated iterations, and VM/interpreter/JIT.
The independent token-stream precedence parser and integer evaluator check the
complete AST, every expression value and variable map, unchanged input text and
wrapping wrapper result. Cases cover precedence/associativity, whitespace,
nested assignments and their order, unknown variables, zero division/remainder,
negative division/remainder, numeric literal wrapping, arithmetic overflow and
u32 accumulator wrapping. An empty variables snapshot remains unchanged while
evaluation results retain each intermediate map.

Three altered-variable-map controls preserve wrapper results but fail full-state
equality. Runtime binary SHA is
`17a26b324469d5d61eb2da5406bf335cf2906b328c851188244706566663c325`.
Rust/Goose calculator source files are byte-checked at
`3c9f84ddaeec834c0631747cb789e8cfd602f993`, not executed. The JIT fallback notice is
retained; execution mode alone does not establish native coverage.

Next: integrate the one-line correction separately from worker reuse, add a
regression across all runtimes, and rerun unchanged all150 checksum checks.
Random program preparation, malformed syntax/error policy, signed-minimum
division, work/allocation attribution, comparator execution and production
coverage remain open. No elapsed performance claim follows from this audit.
