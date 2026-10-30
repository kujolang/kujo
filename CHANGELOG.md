# Changelog

## Unreleased — JSON decimal precision fix

- Enable correctly rounded serde_json float parsing. Decimal/exponent values at the safe-integer boundary now preserve their nearest IEEE-754 value instead of losing one unit. Integer token types, overflow rejection and existing JSON bounds are preserved. Added builtin and VM/interpreter regression coverage.


This file records user-visible changes to Kujo, following
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Promote eligible captured-array scalar journals into bounded batch storage for
  mixed or larger scalar updates, preserving compact one/two-entry updates,
  snapshots, owner replacement and ordinary publication boundaries.

- Avoid redundant hash lookups when oversized captured binding names retain their
  original iteration order. Keep exact membership validation and ordinary lookup
  fallback for reordered or host-edited scopes, including same-size replacements.

- Batch eligible compound indexed arithmetic in captured dictionaries while
  suspending caller optimization permissions inside operator callbacks; preserve
  snapshots, selector order and error publication.

- Admit primitive captured-scalar dependencies to eligible dictionary write
  batches, rechecking their types in the entered capture on every invocation.

- Retain eligible counted collection batches in sparse captured-array storage,
  validating bounds and publication boundaries without growing the write table.

- Keep two eligible array or dictionary payload replacements in bounded captured
  array storage, preserving snapshots and ordinary fallback at observation,
  mixed-payload and additional-index boundaries.

- Reuse capture-map storage after singleton read projections without growing the
  table for deleted buckets; retain the surviving owned key and value.

- Retain eligible collection-valued array replacements in bounded private storage,
  preserving snapshots, index evaluation, aliases and fallback mutation boundaries.
  Boxed payloads require a tail replacement with no later publication boundary,
  avoiding allocation overhead when ordinary mutation is still required.

- Preserve collection-write admission while constructing struct literals; field
  callbacks, owner reads and errors retain their ordinary evaluation boundaries.
  Transfer owned nested payloads through publication without copying struct names.

- Retain eligible nested collection replacements in existing assignment-path storage,
  preserving snapshots, aliases and fallback publication boundaries.

- Retain eligible collection-valued dictionary replacements in compact private entries,
  preserving snapshots and scalar/collection transitions without copying the owner map.

- Retain eligible initial scalar writes to nested dictionaries without detaching
  the captured owner, preserving observable snapshots and publication boundaries.
  Existing nested-key replacements avoid unnecessary full-table growth.

- Defer eligible scalar insertion into captured root dictionaries without copying
  the entire owner on each call, preserving snapshots, callbacks, errors and key aliases.
- Reuse validated oversized captured binding metadata with copy-on-write kind snapshots,
  preserving host membership edits and immutable binding rules.
- Resolve nested writes from borrowed VM opcode metadata and bounded VM/JIT path storage,
  preserving deep-path fallback, snapshots and assignment evaluation order.

- Pack oversized captured-local binding names into private compact tables while
  preserving immutable bindings, live owner visibility, snapshots and idle-pool limits.

- Retain larger scalar row-update batches in private overflow storage for
  admitted explicit assignments and bounded counted loops, preserving snapshots,
  aliases, callback visibility and partial writes before errors.

- Avoid repeated wide-leaf copies in captured scalar updates through unique
  deeper collection paths. Reuse assignment-path storage while preserving
  snapshots, shared-alias fallback, callbacks, errors and index/root evaluation
  order.

- Extend proven synchronous nested-dictionary scalar batches beyond four keys
  using sparse overflow storage. Borrow proof inputs and size initial storage
  without cloning unrelated values; preserve snapshots, error ordering, exact
  scalar values and fallback for uncertain bodies.

- Extend captured nested-dictionary updates to four scalar keys, including new
  keys after an existing-key write. Preserve snapshots, exact scalar values,
  callbacks, error ordering and fallback without increasing assignment storage.

- Avoid repeated copies when captured helpers update one scalar key in a nested
  dictionary under dictionary, struct or array owners. Preserve snapshots, numeric
  and string key behavior, callbacks, errors and replacement-root assignment order.

- Keep two existing canonical numeric dictionary keys inline during captured
  scalar updates, including compound assignments. Avoid repeated dictionary
  copies while preserving snapshots, callbacks, errors and fallback behavior.

- Avoid full captured-dictionary copies for proven synchronous helpers updating
  multiple keys. Preserve snapshots, public-owner publication, operator callbacks,
  errors and root/index evaluation order. Active batches also retain string and
  byte values and new keys; uncertain or escaping bodies keep ordinary fallback.

- Extend bounded captured-row updates to array-owned rows, preserving positive
  and negative selector aliases, snapshots, shared rows and larger-batch fallback.
  Invalid root-as-index reads avoid speculative snapshots while retaining their
  ordinary errors and publication order.

- Extend deferred captured-dictionary writes to floats, Booleans, null and mixed
  scalar types, avoiding repeated full-dictionary copies for one existing key.
  Preserve snapshots, exact floating-point bits, callbacks, error ordering and
  fallback for other keys or collection values without growing capture storage.

- Retain up to four distinct scalar writes to one captured struct/dictionary
  array row without copying the row on each helper call. Preserve snapshots,
  publication ordering and fallback for larger batches. Checked negative-index
  reads reuse pending values; invalid root-as-index expressions avoid speculative
  copies while retaining ordinary error behavior.

- Avoid copying dictionary keys while validating and traversing nested assignment
  paths. Existing-key writes reuse stored keys; insertion, snapshots, canonical
  integer keys and assignment error ordering remain unchanged.

- Avoid copying dictionary keys or formatting unused missing-key diagnostics on
  successful interpreter lookups, including imported helpers used by VM/JIT.
  Preserve canonical numeric keys, present null values and error spelling.

- Defer repeated integer writes to an existing captured-global dictionary key
  until publication in eligible synchronous helpers. Preserve public pre-return
  values, snapshots, recursion, callbacks and error ordering. Numeric-key admission
  uses canonical decimal formatting without an extra heap allocation on fallback.

- Defer bounded sparse scalar writes to captured global arrays until publication,
  avoiding repeated full-array copies in eligible synchronous helpers. Preserve
  public pre-return values, snapshots, binding permissions and error ordering;
  materialize ordinary arrays at unsupported operations and observation boundaries.

- Reserve fresh private captured-local maps from their actual key count, avoiding
  repeated table growth for oversized captures. Preserve live values, independent
  snapshots, immutable bindings and bounded idle storage.

- Reuse suspended private globals during synchronous global-only reentry through
  foreign module callbacks. Preserve public-owner publication, independent
  snapshots and completed writes before errors without enlarging call frames.

- Avoid cloning captured environments for synchronous helpers proven to use only
  parameters and local values. Keep lexical setup for callbacks, captured names,
  overloaded operators and unsupported syntax; preserve collection snapshots.

- Make channel-ordered writes to captured locals visible to active aliases in the
  interpreter, including field writes and writes before caught errors. Preserve
  independent closure and generator snapshots, public captured-scope map access,
  binding permissions and linear captured-array append allocation growth.

- Avoid unused module search-path and working-directory discovery when creating
  callback VMs that borrow the caller's module loader. Preserve ordinary VM
  initialization, import caches, capability checks and callback error recovery.

- Refresh captured ordinary and async task globals from each submission while
  retaining captured local state. Preserve separate module and caller namespaces,
  callback writes and caught errors without publishing task-global changes back
  to the parent or later task invocations.

- Merge only written captured bindings when ordinary interpreter task callbacks
  complete, preserving unrelated writes from task aliases. Keep recursive calls,
  callbacks and independent collection snapshots intact.

- Keep ordinary global-helper parameters out of async captured-write journals,
  including nested helpers and caught helper errors. Restore the caller's capture
  scope afterward so subsequent captured writes remain tracked.

- Preserve current captured state during recursive calls, callbacks, global-helper
  and generator suspension, including reentry through a separate VM bridge.
  Separately created closures and saved collection snapshots remain independent.

- Align VM/JIT direct indexed writes with the interpreter: evaluate the index
  before reading the target binding, so an index expression that replaces the
  container updates that replacement. Preserve snapshots, expression side effects
  on errors and compound assignment's existing separate read/write evaluations.

- Share immutable native-default metadata in imported module environments instead
  of copying unchanged native bindings for every helper call. User overrides,
  aliases, immutable declarations and snapshots remain independent; native JIT
  loops retain the correct global owner when replacing a shared default.

- Cache ordinary VM parameter slots and legacy named aliases per immutable code
  object; bytecode edits invalidate the cache and callable snapshots stay isolated.

- Reserve byte-array conversion output once while preserving ordered type/range
  validation and source/output value snapshots.

- Store only immutable binding-permission exceptions in interpreter environments.
  Imported/captured calls no longer copy redundant mutable/native permission names;
  immutable snapshots, redefinition and helper ownership retain their behavior.

- Avoid parameter-name allocations and redundant argument snapshots during ordinary
  VM calls; retain legacy receiver adaptation and existing arity diagnostics.
  Reuse bounded, cleared local-slot buffers after VM returns and exception unwinds.
- Reuse global array and string-key dictionary storage for indexed writes when
  the loaded value still matches the destination. Real snapshots, reentrant index
  evaluation, failed writes and depth validation retain their existing behavior.

- Retain operation-owned VM hot state for synchronous sort and regex callbacks
  that call helpers, create closures or import modules. Module-loader ownership,
  current globals and temporary collection lifetimes are restored per call;
  async and generator callbacks retain the existing fallback behavior.

- Key native function caches and call-site guards by immutable bytecode identity
  instead of function names. Distinct same-name functions retain their own code
  and metadata after native compilation; bytecode mutation invalidates identity.

- Preserve the caller's module loader across VM/JIT callback execution. Repeated
  callback imports now reuse unchanged module exports, reload changed sources,
  and restore cache/cycle state after errors without changing capability policy.

- Reuse a private VM within synchronous `sort_by` and `regex_replace_with`
  operations for fixed leaf bytecode callbacks. Closures retain snapshot cell
  semantics; callbacks with calls, imports, async or other unsupported state
  continue through the existing fresh-VM bridge.

- Avoid standalone-script JIT compilation for internal callback and task wrappers
  whose arguments are already on the VM stack. Callee JIT, cancellation,
  capability checks, and callback value semantics remain enabled.

- Drop discarded native roots in place rather than returning full `Value`
  payloads only to destroy them. Ownership transfers, frame order, stale-handle
  checks, promotion and copy-on-write snapshots retain their existing behavior.

- Add optional LangArena comparison resource measurements (peak RSS and Linux
  host counters). The comparison-only schema-v2 manifest references individual
  sample JSON and raw output files instead of an inline `results` array, avoiding
  cumulative write limits. Truncated output is rejected as benchmark evidence.

- Poll task cancellation in the inline VM callback dispatcher, preventing
  cancelled higher-order callbacks from retaining language-task capacity.

- Propagate explicit JIT opt-in through language tasks, detached tasks and
  interpreter-to-bytecode callback bridges. Task cancellation tokens reach the
  native safepoints before compilation; interpreter and VM modes stay opt-out.

- Add native function-entry and backward-edge cancellation safepoints. Loop
  checks are emitted only by compilers created for cancellable execution;
  ordinary JIT loops retain their existing instruction paths. Native
  contexts borrow the enclosing task token, preserve earlier errors and unwind
  native frames on cancellation.

- Pass explicit JIT opt-in to isolated synchronous `parallel_map` bytecode
  workers. VM/interpreter modes remain opt-out. Nested synchronous parallel
  workers inherit the enclosing task cancellation token before native compilation.

- Unwind exited `try` handlers before `break`, `continue` and `return`, while
  retaining handlers around loops that remain active after those exits.

- Discard VM exception handlers when their function returns from inside `try`,
  preserving the caller's active handlers. Later errors now retain their intended
  diagnostics instead of jumping into a returned function's catch block.

- Transfer native callback arguments in order with one exact-sized allocation,
  preserving owned values, evaluation order and malformed-stack error behavior.

- Count calls from native code toward JIT helper promotion, including the first
  call to loop helpers. Hot helpers reached only through compiled callers can
  now compile under the existing eligibility and fallback rules.

- Compile guarded pop and dictionary removal rebinding through native root,
  field and indexed paths, preserving retained snapshots, shadowed calls,
  replacement effects and readonly errors.


- Compile generic indexed replacements and local indexed loop operations through
  checked, tagged native paths, preserving evaluation order and value snapshots.
  Enforce the existing VM nesting limit when sparse integer dictionaries convert
  to general values during indexed replacement.


- Compile guarded array and byte appends through field/index paths in native
  functions and loops, preserving snapshots, RHS effects and shadowed calls.

- Compile ordinary field replacement in experimental native functions and loops,
  preserving snapshots and nesting checks. Route invalid VM field assignments
  through exception handlers while retaining failed assignment targets.

- Release private native rebinding snapshots under caller exception handlers
  while retaining same-invocation recovery state and failed assignment targets.

- Reuse nested-value validation across native helper calls while preserving
  snapshot isolation and restoring validation state after assignment errors.

- Reuse unique parent and leaf nodes during checked pop and dictionary removal
  without dropping retained value snapshots or weakening nesting checks.

- Avoid depth-cache-induced Arc node relocation during checked array and byte
  appends while preserving copy-on-write for retained snapshots.

- Compile checked nested binding writes in experimental native functions and
  loops, preserving snapshots, evaluation order, readonly errors and nesting limits.

- Preserve global bindings and readonly errors when experimental native code
  assigns newly constructed collections; owned values use checked named assignment.

- Refresh VM-owned native context pointers after callbacks while preserving
  external contexts and shared local/global aliases.

- Compile normalized array/byte iteration in experimental native functions and
  loops, preserving exhaustion, snapshots and lazy generator callbacks. Analyze
  reachable control-flow edges so early returns do not corrupt loop stack depths.

- Compile guarded array/byte append rebinding and conditional local snapshot release
  in the experimental JIT, preserving builtin shadowing, RHS effects, value
  semantics and fallback behavior.

- Compile guarded string rebinding in native functions and loops while preserving
  snapshots, RHS evaluation, binding mutability and ordinary addition fallback.

- Preserve channel and ArgParser receivers when a native loop resumes in the VM
  to execute a method call.

- Restore builds without `runtime-jit` by keeping the disabled context compatible
  with shared VM loop setup.

- Compile field reads in experimental native functions and loops using the VM's
  field and method resolver. Preserve typed values, receiver markers, errors and
  function-frame stack cleanup.

- Preserve receiver markers when native callers invoke fallback functions using
  channel or ArgParser methods. VM and callback field resolution now share member
  precedence, method lookup and error behavior.

- Compile struct construction in experimental native functions and loops,
  retaining constructor metadata and preserving field order, value semantics
  and nesting limits.

- Persist owned native-loop locals back to physical VM slots on normal exits
  and caught errors, preserving updated array/dictionary values and snapshots.

- Compile ordinary array literals in experimental native functions and loops
  using typed operands, checked nesting limits and bounded ownership roots.
  Array spread bytecode continues through the existing fallback.

- Compile general stack index reads in the experimental JIT with typed operands,
  checked indexing errors and bounded owned-value root storage.

- Route VM field-access errors through exception handlers. Missing struct fields
  and reads from non-struct values now preserve failed assignment targets and
  prior effects when caught, including calls from native code.

- Allow owned arguments into eligible experimental native functions. Transfer
  named parameters into frame-local roots, preserve scalar fast paths, and drop
  temporary argument aliases before execution. Missing parameters exit before
  body effects; callbacks, recursion and snapshots retain value semantics.

- Route cold bytecode callbacks outside the inline dispatcher's supported subset
  through the full VM bridge before executing their bodies. Native callers can
  invoke array-building helpers without unsupported-opcode failures or replaying
  earlier effects.

- Compile string and byte constants through retained immutable native literals.
  Loads preserve value types and copy-on-write isolation, share backing buffers,
  and release temporary roots instead of falling back solely for these constants.

- Preserve interpreter errors from unary and binary struct operator methods.
  Failed operator assignments keep their previous target value, stop subsequent
  effects and remain catchable, matching VM behavior.

- Dispatch struct `op_add` methods for constant character/string appends in the
  VM and native functions. Preserve snapshots and retag native bindings when an
  operator returns an immediate value. Locked native loops resume in the VM
  before the operator callback.

- Resume native loops in the VM before calls or struct operator callbacks can
  reacquire the globals mutex. Preserve the exact instruction, operand stack and
  local state so prior effects are not replayed and exceptions remain catchable.

- Guard fused character-append loops and retain the original bytecode loop for
  unsupported types or bindings. Skipped loops preserve their counters; fractional
  counters, readonly bindings and catchable errors retain ordinary loop behavior.
  Fast appends run in bounded batches so dispatch cancellation checks remain active.
  Ordinary constant appends to `let`/`const` strings also use checked assignment.

- Preserve typed dictionary reads/writes and checked global assignments in
  experimental native loops. Guard dense integer access with explicit data/length
  views, refresh views after storage changes, and propagate errors without
  replaying prior effects or bypassing readonly bindings.

- Avoid cloning the entire value vector when replacing an existing fixed-key
  dictionary entry through VM/native indexed writes. Existing snapshots retain
  their own vectors, and nested collections retain normal copy-on-write behavior.

- Preserve native indexed writes in callee bindings using the VM's existing
  collection update logic. Carry local mutability metadata into compiled code,
  reject writes through `let`/`const`, preserve copy-on-write snapshots, and
  propagate invalid-index/byte errors before subsequent effects.

- Resolve indexed reads in experimental native whole functions against their
  callee bindings, using tagged operands and the VM's indexing rules. Preserve
  collection/string/byte result types, negative and fractional read indices,
  alias isolation and catchable errors without touching caller slots.

- Preserve native constant string/character appends in the actual callee binding,
  retain embedded string literals, and write owned loop locals back with their
  type tags. Append type errors are catchable and native helper errors stop later
  effects. Aliased strings retain copy-on-write isolation.

- Preserve owned operand semantics in experimental native arithmetic, ordering
  and unary operations through the reference VM, including string concatenation,
  struct operator methods and catchable errors. Immediate numeric paths remain
  native; non-immediate constants and wider owned arguments remain gated.

- Compile nonempty dictionary literals through tagged owned operands in the
  experimental native runtime, retaining fixed-key allocations for generated-code
  lifetime. Preserve VM nesting checks and make invalid dynamic literal-key
  errors catchable in VM/native execution without replaying earlier effects.

- Preserve last-value-wins semantics for repeated constant dictionary keys in the
  VM while evaluating every value expression in source order. Unique constant
  keys retain their fixed-dictionary representation.

- Release discarded native owned operands and overwritten SSA locals promptly,
  preserving separate roots for duplicates and local reads. Release marshaled
  callback arguments and consumed equality/logical operands without changing value
  semantics. Remaining legacy native collection paths are still experimental.

- Compare tagged native owned operands by ordinary value equality instead of
  handle identity, including equal strings, nested collections and mixed types.

- Preserve owned named values and callables in experimental native execution,
  including collection callback arguments, returns and checked named assignments.
  General owned equality and dead-operand release remain unfinished.

- Apply ordinary value truthiness to tagged owned results in experimental native
  conditions and logical operations, including empty callback-returned strings.
  Native global owned-value loads remain unfinished.

- Preserve owned callback results across the experimental native VM-stack boundary
  instead of replacing them with integer zero. Full native owned-value support
  and dead-operand lifetime integration remain unfinished.

- Connect lazy ownership scopes to experimental native invocation and recursive
  function exits. Owned-value bytecode support remains incomplete.

- Avoid allocating an unused empty bytecode chunk on each VM function call while
  retaining shared caller code for normal returns, exceptions and generators.

- Add bounded runtime measurements for actual native entries and JIT call
  dispatches containing owned arguments. Preserve older v1 measurement reports;
  the LangArena isolated runner can collect and hash per-case reports explicitly.

- Compile hot calls with float, Boolean and null arguments using tagged native
  parameters, including callbacks, loops, larger argument lists and direct
  recursion. Owned-value arguments retain the guarded VM path. Enforce the VM
  depth limit across native recursion/callbacks and preserve catchable call-entry
  errors without leaking depth slots after recovery.

- Preserve float and null immediate values through experimental native constants,
  globals, assignments, callbacks, returns and scalar loop locals. Compile float
  arithmetic/comparisons with checked zero divisors, signed-zero truthiness and
  the existing NaN/infinity/equality semantics. Implicit returns now produce null.

- Preserve Boolean/integer kinds on experimental native named loads and
  whole-function assignments. Explicit global bytecodes bypass same-named
  parameters and use checked global stores, including readonly enforcement.
  Supply stable cached name metadata to direct native calls and callbacks.

- Compile experimental native short-circuit branches with paired operand values
  and type tags at every join, including unconditional jumps and fallthroughs.
  Preserve scalar return metadata in function, loop and script entrypoints.

- Restore recursion depth after cold bytecode callbacks return, preventing repeated
  sequential callbacks from incorrectly exhausting the VM recursion limit.

- Enforce scalar operand types in experimental native arithmetic, ordering and
  unary operations. Keep Boolean/integer equality distinct and logical bytecode
  results Boolean. Route VM ordering and logical-negation errors through catch
  handlers, matching arithmetic error handling.

- Preserve Boolean scalar types across experimental JIT function returns,
  callback arguments/results and direct recursion. Track local scalar type tags
  through SSA branch merges instead of reconstructing predicates as integers.

- Keep assignment values on the stack in the JIT callback bytecode path, matching
  the main VM and preventing stack underflow when compiled callers invoke cold
  functions that update named, local or global bindings.

- Preserve checked integer arithmetic in experimental JIT execution, including
  overflow, zero divisors, signed division/remainder boundaries and negation.
  Native guards raise catchable runtime errors instead of wrapping or trapping.

- Fix named integer assignments in experimental whole-function JIT execution:
  update existing globals, enforce readonly bindings, and stop on assignment
  failures. Initialize direct native parameter values and propagate recursive
  failures before later side effects.

- Preserve callback failures across experimental JIT calls, stop compiled
  execution at the failed call, and route the original error to VM handlers.

- Guard experimental integer-only JIT function entry points against non-integer
  arguments, including cached calls whose argument types change after warmup.
  Preserve VM behavior for those calls while retaining compiled integer code.

- Route scalar VM arithmetic errors through `try`/`except`, including nested
  calls and checked integer overflow, while preserving uncaught diagnostics.

- Avoid depth-cache-induced `Arc` allocations on nested assignment paths while
  preserving copy-on-write alias isolation and nesting-limit validation.

- Serialize JSON from borrowed runtime values instead of constructing a second
  owned JSON tree. Preserve exact compact/pretty output, validation order,
  nesting limits and secret redaction. Correct the documented key ordering to
  match existing lexicographic JSON output, including integer and fixed keys.

- Reject unsupported constants in experimental JIT regions instead of executing
  integer-zero placeholders. Float literals such as a divisor of `2.0` retain
  their VM semantics rather than causing a native division trap after warmup.
- Resolve lazily registered builtins from experimental JIT code after checking
  local and global bindings, preserving user shadowing and normal VM invocation.

## [1.9.0] - 2026-10-22

See [release notes](docs/RELEASE_NOTES_1_9_0.md) for release scope and compatibility.

### Added, fixed, and performance

- Preserve numeric accumulator JIT compilation when applying the speculative
  string-append optimization; ordinary addition still handles later type changes.

- Defer global-environment snapshots in bytecode callbacks until interpreter
  fallback needs them, avoiding repeated copies of unrelated globals in regex
  replacement and comparator callbacks.

- Cache legacy VM parameter-binding metadata per code allocation, avoiding a
  function-body scan on every call while preserving hand-built chunk support.

- Share compiled bytecode across callable aliases and execution snapshots instead
  of copying function bodies on lookup and calls. Compiler and generator edits
  detach storage, preserving independent code and capture semantics.

- Cache stable string character counts without retaining payloads and accelerate
  ASCII indexing and substring extraction while preserving Unicode semantics.

- Reuse string capacity for dynamic self-appends in both runtimes, preserving
  retained snapshots, immutable bindings, and right-hand-side reassignment order.

- Infer numeric remainder (`%`) results so valid remainder-based string indices
  no longer produce spurious type-check warnings; defer unknown index types to
  runtime validation consistently with gradual typing.

- Move suspended VM caller bytecode into call frames instead of cloning its
  instructions and constants on every helper or recursive call.

- Match the interpreter's finite floating-point array/string/byte read indices in the
  VM; non-finite indices remain errors. Reject floating-point simple array writes
  and non-string/non-integer dictionary writes in the interpreter consistently
  with VM and nested writes.

- Avoid cloning VM globals for argument-only arithmetic, string, byte, JSON,
  and collection builtins; callback APIs still refresh their global dependencies.

- Support snapshot-safe iteration over bytes in VM/interpreter and generator loops.

- Restore the interpreter generator delegation limit independently of ordinary function recursion, matching VM depth and recovery behavior.

- Reuse unaliased byte-buffer capacity for consuming `concat` assignments,
  including struct fields and indexed paths, while preserving retained snapshots.

- Add `regex_count` for scanning without matched-string allocation and retain a
  bounded working set of 32 regex patterns to avoid small-cache compilation churn.

- Share struct field storage with copy-on-write, removing recursive subtree
  copies during reads while preserving mutation and closure snapshots.

- Route VM indexed-read, invalid indexed-write and immutable indexed-write errors
  through active exception handlers, preserving uncaught failures and unchanged
  values after rejected writes.

- Extend consuming pop/removal rebinding to fields and indexed collection paths,
  preserving parent snapshots and falling back for effectful index expressions.

- Reuse uniquely owned dictionary storage for adjacent removal/rebinding while
  preserving removed values, retained snapshots and ordinary callable fallbacks.

- Pass imported helper arguments, VM callback results and awaited collections
  without recursively normalizing their graphs, preserving shared storage and
  copy-on-write isolation.

- Support VM dictionary representations directly in interpreter lookup, iteration,
  spread, destructuring, optional access and truthiness, retaining canonical keys
  and snapshot isolation.

- Route VM array index-write failures through active exception handlers, including
  helper failures, so callers can recover without replacing their original value.

- Reuse uniquely owned collections across local helper rebinding and dead parameter
  aliases in both runtimes, preserving retained snapshots and catch recovery.

- Support up to 256 interpreter calls, including imported recursion from VM scripts,
  with guarded host-stack growth; retain the separate 32-entry callback bridge limit.

- Share persistent LIFO stack storage and reuse uniquely owned arrays in adjacent
  pop/rebind sequences, retaining aliases, popped values and empty-pop behavior.

- Reuse array storage for field and indexed-path append assignments, including
  dynamic indices, while preserving callback ordering and snapshot semantics.

- Avoid repeated VM row copies and depth scans through benchmark-state structs
  and dictionaries while preserving alias isolation and atomic failed writes.

- Reuse unaliased array storage for append rebinding after evaluating the original
  call arguments; retain ordinary calls when shadowing or callback effects prevent reuse.

- Use persistent shared queues with cached reversal and nesting metadata, avoiding
  full-queue copies and validation scans during functional queue updates.

- Validate nested assignment paths before writing, eliminating redundant row
  snapshots while retaining copy isolation and failed-write atomicity.

- Avoid exponential VM nesting-check work on shared collection graphs while
  retaining deeper-path validation and per-operation freshness.

- Add strict `utf8_encode`/`utf8_decode` and checked byte indexed assignment in
  both runtimes, including nested paths and copy-on-write alias isolation.

- Share byte-buffer storage on value clones, avoiding full-buffer copies during
  reads while preserving exact binary content and host API ownership.

- Add stable, fallible `sort_by(items, comparator)` without changing `sort`;
  callback failures stop sorting and the input remains unchanged.

- Add `regex_replace_with(text, pattern, callback)` with ordered capture groups,
  literal replacements, bounded output, and ordinary callback capability checks.

- Add bounded `parse_csv_rows(text)` for headerless positional string records,
  strict quoting, and consistent field counts; preserve existing `parse_csv`.

- Add locale-independent `format_fixed(value, precision)` and explicit signed
  64-bit `wrapping_add`, `wrapping_sub`, and `wrapping_mul`; ordinary arithmetic
  remains checked.
- Preserve floating-point signed zero when deduplicating bytecode constants.

- Fix VM struct field assignment write-back for global, local, and captured
  bindings, preserving immutable-binding checks and value-copy isolation.
- Support binding-rooted nested array, dictionary, and struct assignments in
  both runtimes, with checked write-back and unchanged roots on failed writes.
- Stop interpreter assignment when evaluating its right-hand side fails.

## [1.8.2] - 2026-10-19

See [release notes](docs/RELEASE_NOTES_1_8_2.md) for compatibility and distribution scope.

- Replace the retired bincode dependency in PDF hyphenation with a maintained,
  legacy-format-compatible codec; preserve bundled dictionary contents and
  remove the release audit exception.

### Runtime boundary hardening

- Upgrade ZIP to 9.0.5 to resolve RUSTSEC-2026-0367/0368/0369 while retaining
  Kujo-language archive behavior and filename decoding. Rust embedders using
  `Value::ZipArchive.writer` must migrate their direct ZIP dependency to version 9;
  see [the Rust migration guide](docs/ZIP_9_RUST_MIGRATION.md).
- Replace the unmaintained YAML parser dependencies with the YAML organization's
  maintained compatible fork. Keep YAML values, tags and serialization behavior
  covered by compatibility tests.
- Preserve native credential stores and existing Secret Service entries while
  replacing the unmaintained Linux zbus dependency chain with the maintained
  libdbus backend; add old/new stored-credential interoperability coverage.
- Check existing ZIP extraction ancestors before creating directories; reject
  outside aliases without outside directory creation while preserving in-root aliases.
- Do not cache initialized module exports against source metadata that changed
  during initialization. Deduplicate registered import roots while preserving
  search precedence and explicit lockfile-discovery refresh.
- Remove unused vcpkg OpenSSL builds from current-source Windows upgrade and
  release jobs; Cargo continues to build the configured vendored OpenSSL.
- Preserve caller capabilities during module initialization, test-run bodies and
  hooks, and parallel bytecode mapper execution. Invalidate evaluated module
  exports when the loader's capability policy changes.
- Enforce AI endpoint allowlists on every redirect while preserving structured
  `endpoint_denied` errors, private-network controls and redirect hop limits.
- Reject static-file aliases resolving to private names, including custom 404
  pages; ordinary public-file aliases remain supported.
- Bound schema diagnostics to 10,000 errors and 4 MiB of cumulative diagnostic
  string bytes per call, including discarded combinator branches. Exceeding a
  limit returns an operational error rather than a truncated validation result.
- Update the locked rustls dependency to 0.23.46.
- Avoid a redundant clone when returning an owned module's exported values.
- Grant the live agent provider bridge explicit randomness for AI SDK client
  identifiers; keep deterministic fixture permissions unchanged. Stop its mock
  server explicitly so a failed client cannot hang integration verification.
- Refresh the generated TODO source location and document `http_stream_collect`
  in the standard-library reference inventory.

### HTTP stream consumption

- Add one-shot `http_stream_collect` and `http_request` collection mode with UTF-8-safe incremental callbacks, bounded reads, cancellation, capability checks and reentrant ownership protection. Existing buffered requests and server forwarding remain compatible.

### Native layout security

- Encode all JSON control characters and HTML script delimiters in native
  layout JSON-LD, and prevent later template passes from reinterpreting data
  placeholders. Metadata remains JSON-round-trippable and ordinary rendered
  output is unchanged.

### Installation documentation

- Pin stable install examples to Kujo 1.8.1 using the installer’s existing
  version override, avoiding a stale website deployment’s 1.8.0 default.
- Add the human first-hour walkthrough and separate agent rehearsal evidence;
  adopter validation remains open pending the human session and a reproduced
  published-runtime Kennel network blocker.

### Language design evidence

- Complete the bounded `yield from` design decision with an explicit 1.9 deferral.
  Document supported for/yield delegation, add lifecycle/parity contracts, and
  preserve a mixed-call depth/error reproduction. No new syntax is introduced.

### Payload and output efficiency

- Stream canonical AI request serialization into SHA-256 with a bounded buffer;
  preserve request hashes, cassette keys, validation and secret redaction.
- Share immutable tool-call strings between raw and normalized response fields
  instead of copying their contents.
- Add opt-in `KUJO_RELEASE_GATE_LOG_DIR` command receipts with complete private
  per-run logs and unchanged failure exit codes. Default gate output is unchanged.
- Add deterministic context/output budgets and hash compatibility regressions.
- Redact configured credentials before shortening AI error previews, preventing
  partial credentials from surviving the preview boundary. Saturate oversized
  `Retry-After` seconds at the maximum integer millisecond value.

### Performance

- Reuse the validated character count when estimating ASCII token weights.
  Unicode weighting, character limits, errors and context-fitting policy remain
  unchanged. The release performance campaign records paired measurements and
  platform-specific resource evidence; no provider-token accuracy claim changes.
- Extend the release benchmark corpus and add a pinned Linux/macOS/Windows
  comparison workflow with correctness checks, raw evidence and noise controls.
  Remove disposable runner executables and Windows symbols from successful evidence.

### Fixed

- Preserve sparse integer dictionary keys in global/local assignments and fused
  fill loops without allocating dense storage up to the key. Keep explicit null
  entries readable and included in equality. Preserve distinct numeric-looking
  string keys such as `"00"` and `"+0"`.
- Accept all dictionary representations in native length, membership, clearing,
  merging, JSON/TOML/YAML serialization, and schema instance validation. Share
  merge/update handling; mixed representations no longer silently lose entries.
  Deduplicate equal dictionary values regardless of their internal storage.
- Accept integer keys consistently in dictionary get/get_default/has_key/remove
  helpers; use precise string-or-integer type-checker signatures.

### Tests

- Add a native representation matrix and shared VM/interpreter behavior tests
  to the existing parity CI job, including the build without JIT. Cover all six
  dictionary representations and their 36 merge/update pairings, nested data,
  bytes, secrets, numeric boundaries, AI messages/context, mutation and errors.
  Explicitly rebuild the required-feature CLI before no-JIT checks so a stale
  default-feature executable cannot supply misleading parity evidence.

## [1.8.1] - 2026-10-10

### Fixed

- Accept VM dictionary rows in CSV export and stabilize column order. Reject
  duplicate headers, extra row columns, and compound cells that previously lost
  data silently; missing cells, escaping, and secret redaction are preserved.
- Validate JSON Schema structure independently of instance shape, including
  unvisited child schemas and malformed empty constraint arrays. Preserve empty
  `enum` semantics; bound structural traversal and per-call regex reuse, and use
  direct property lookup for additional-property checks.
- Bound large TOML parse diagnostics while preserving parser byte locations and
  reusing the Unicode-safe numeric-error preview helper. Short-input diagnostics
  remain unchanged.

- Return a runtime error for integer `sum` overflow; preserve distinct compound,
  byte, and secret values in `unique`; invert specialized integer dictionaries;
  and compare mixed integer/float sort keys without rounding large integers.
- Use scaled vector norms for finite values whose squares overflow or underflow,
  and reuse the query norm across `vec_top_k` rows.
- Compare JSON Schema `const`/`enum` numbers exactly, including nested values,
  within validation budgets; support local JSON pointers into schema arrays.
- Preserve AI history `name`, `tool_calls`, and `tool_call_id` fields. Tool loops
  retain assistant calls, correlate replies by ID, and reject malformed calls.
  Accept VM-specialized dictionary literals throughout messages and multimodal
  content, matching interpreter behavior without copying dictionaries to validate them.
  Plain-text request hashes remain unchanged; corrected tool histories require
  cassettes matching their corrected request bodies.
- Bound numeric conversion error input previews to 128 Unicode characters with
  the original byte count; retain existing diagnostics for short inputs.
- Avoid normalizing discarded/count-only token messages and cloning token-helper
  options. Token estimates, pruning policy, limits, and result fields are unchanged.

- Admit database pool waiters in FIFO order and notify them when capacity returns,
  replacing polling that allowed repeated acquisitions to starve older callers.
  Keep connection setup and health checks outside the admission queue; timed-out
  requests and failed connections cannot strand subsequent waiters.

- Update Hickory DNS crates to 0.26.3 for upstream DNSSEC, retry-loop and
  CNAME security fixes, including the 0.26.2 regression corrections.

- Own Windows native-command descendants with per-command jobs before execution;
  terminate the job on timeout/cancellation and close it on return. Incompatible
  job admission fails closed; background descendants cannot outlive the call.
- Run `gif_to_webp` through the shared 30-second deadline and 1 MiB-per-stream
  capture limits, preserving successful results and missing-tool diagnostics.
  Timeout, cancellation and truncated failure diagnostics are explicit. Fix
  lossless conversion by using gif2webp's default instead of an unsupported flag.

- Enforce private-network policy on actual connections and redirect hops, including
  IPv4-mapped IPv6; strict HTTP clients disable ambient proxies.
- Require environment-read for `os_environ`, source-read permission for compound
  file operations, and process-exec for `gif_to_webp`. Restricted callers must
  grant these effects explicitly; trusted defaults remain unchanged.
- Bound ZIP decoding and publish validated entries atomically; preserve existing
  contents on invalid entries. Publish AI cassettes through unique private staging.
- Return catchable shared-integer overflow errors without modifying shared state.
  Keep subprocess deadlines active until inherited output pipes finish draining.
- Fit AI context in one ordered pass without changing token estimates, retained
  message order, validation, or result keys.

- Account for queued connections when allocating HTTP connection workers, so
  concurrent keep-alive clients cannot strand a new request behind occupied
  workers until another connection closes. Includes a bounded reservation-race
  regression; no language or telemetry schema changes.

## [1.8.0] - 2026-10-03

### Added

- Support generator methods declared with `func*` inside structs in both the VM
  and interpreter, including explicit `self`, legacy direct-field bindings,
  receiver snapshots, shared alias progress and cached terminal failures.
- Share one immutable analyzed-program model between advisory CLI checking and
  the LSP, exposing inferred variable types, callable signatures, struct fields,
  methods and imported namespace members to hover and completion.
- Add bounded, content-hashed imported-module analysis caching plus explicit
  full-file and repeated-request latency gates.
- Extend advisory optional inference across nested destructuring, known struct
  fields, async completion values, callable aliases and analyzed module exports,
  with positive, negative, dynamic-fallback and VM/interpreter parity regressions.
- Expand the reproducible runtime-measurement workload corpus for collections,
  structs, retained closures, nested generators, reusable promises, channels and
  bounded task admission/cancellation, plus offline AI-native request hashing.
- Add a process-inclusive startup, project-check and LSP-diagnostics comparison
  harness for static-analysis performance regressions.

### Changed

- Analyze open LSP documents once per content revision and reuse that result for
  diagnostics, hover and completion; `didChange` invalidates exactly once and
  `didClose` releases the document snapshot.
- Publish optional checker findings as non-blocking LSP warnings while preserving
  lexer/parser errors and gradual fallback for genuinely dynamic values.
- Cache parsed imported-module ASTs across function, value and struct export
  analysis, avoiding repeated lexer/parser work while preserving dynamic import
  fallback.
- Add concise advisory diagnostics for known missing modules, impossible
  destructuring, struct annotation mismatches, missing fields and non-callable
  values. VM execution remains dynamically typed.
- Restore `cargo run -- ...` to the primary `kujo` binary now that the package
  also contains the lean `kujo-run` binary, keeping documented release commands
  and the canonical release gate executable.
- Refresh open LSP documents that transitively depend on an edited imported
  module, using unsaved editor buffers during analysis and returning to disk
  state when the buffer closes.
- Publish and post-publication verify a deterministic source archive alongside
  the supported native release archives; keep all six npm packages aligned with
  the runtime version and free of lifecycle download scripts.

### Fixed

- Align interpreter name shadowing with the VM so lexical declarations may
  shadow preloaded native function names while duplicate source declarations
  remain errors.

### Performance

- Avoid cloning a generator's complete bytecode instruction and constant vectors
  on every resume. The continuation already owns the active chunk; measured
  nested-generator alias work improved by 6.7% at the paired median, with the
  focused longitudinal workload showing a 20.0% mean reduction under noisy host
  conditions.

## [1.7.0] - 2026-10-01

Native archives and npm packages are published for all five supported targets.
Crates.io publication remains blocked by a separately tracked dependency patch.

### Added

- Add opt-in Windows `run --kill-children-on-exit` lifetime ownership for externally supervised runtimes; unsupported platforms or job admission failures reject before script execution. This bounds subprocess lifetime; it does not grant capabilities or sandbox untrusted code.

- Add pure `path_is_absolute` native syntax validation for portable process bindings; no filesystem capability or access required.

- First-party `kujo mcp` command group with `kujo mcp make [REPO] [OPTIONS]`,
  delegating to the canonical Kujo MCP package. The command resolves the
  package from `KUJO_MCP_PATH`, walk-up `kennel.toml` identity, the analyzed
  repository's Kennel lockfile, or the ecosystem install root
  (`$KUJO_INSTALL_ROOT/sources/mcp`, default `~/.kujo/sources/mcp`), and runs
  the generator VM-first with inherited output. Default repository is the
  current directory; relative and absolute repository paths are validated
  before delegation. The `mcp` top-level command name is recognized as a
  first-party runtime command, and the ecosystem installer's core profile now
  provisions the MCP package source locally.

- Document ecosystem-owned compatible preservation binding, bounded parent
  finalization and the minimum Wave F composition/provenance crosswalk.

## [1.6.0] - 2026-09-28

Native archives and runtime npm packages cover Linux x64/arm64, macOS x64/arm64
and Windows x64. Source/tag: `44af277848173664f72ca85f2a1b3b98d634ecdd`.
See [publication evidence](docs/KUJO_1_6_RELEASE.md).

### Runtime and tooling

- Fix VM loop/early-return optimizer corruption: retain iterator exhaustion edges,
  relocate instruction targets after every shrinking pass, and preserve alternate
  entries. Valid programs retain interpreter semantics; stack checks remain intact.
- Add opt-in `kujo run --measurements PATH` numeric VM measurements and the additive
  `kujo.runtime-measurements/v1` summary. This is not complete heap accounting or
  provider billing; short-run synced-export overhead remains documented.

### Experimental ecosystem companions (not stabilized by this runtime release)

- Dispatch adds producer-neutral failure control, durable review checkpoints,
  immutable policy/configuration revisions and fresh-controller review/replay.
  Dispatch remains the only replay/admission authority.
- Watchdog observes runtime activity and RunLedger correlates evidence; neither
  becomes workflow authority.
- Wave C beta effect assurance covers SQLite, Workcell Git CAS and Ability within
  the opt-in local single-effect required/deny domain. Alpha remains supported.
- Wave D alpha generic correlation spans six participant forms across two effect
  families, independent TypeScript/Python codecs, experimental SDK APIs and private
  package distribution. Participant packages remain alpha and unpublished.
- Source-blind agent adopter rehearsal passed. Human adopter usability remains
  post-release validation. No exactly-once, universal rollback, general machine-loss
  recovery, multi-effect assurance or remote authenticated participant trust is claimed.

### Changed

- Async function calls start eagerly under bounded task admission and return a
  reusable completion promise. Body errors are reported by await. Detached spawn
  now executes in the VM and rejects unsupported referenced captures explicitly.
- `spawn_task` executes its callable; repeated waits share completion and
  cancellation requests cooperative exit without promising effect rollback.

- VM closures resolve captures at their definition site and access captured
  cells by index, preserving v1 per-closure snapshots and alias identity.
  Returned and nested closures retain owned captures after scope/frame exit;
  closure-bearing functions use the VM when the experimental JIT cannot handle them.

### Added

- Versioned, producer-neutral workflow-control JSON Schemas for evidence,
  execution and evaluation results, policy decisions, preservation outcomes,
  intervention requests/decisions, re-execution descriptors and control events.
  Contracts distinguish evaluator errors from failed judgments and carry rule,
  metric, idempotency-enforcement evidence, and explicit re-execution mode/attempt
  fields.

- POSIX `sync_directory_beneath(root, relative_directory)` preview durability barrier, capability-gated and descriptor-confined below a trusted root. Sync failures do not imply preceding publications were rolled back.

### Fixed

- Uncaught interpreter errors retain their call frames after unwinding; caught errors do not leak stale frames into later diagnostics.

- Interpreter argument-parser method calls and VM struct relational operators now reach their existing runtime implementations.
- Static checking recognizes lexical callable bindings, runtime builtin aliases, dynamic reassignment, and the two-argument seek-read contract without spurious warnings. Unknown collection values stay gradual instead of inheriting another element’s concrete type. Explicit annotation errors remain diagnostic.
- Runtime fixture checks support exact interpreter diagnostic snapshots alongside VM expectations.

- `kujo test` honors explicit inventory-skip headers, keeping provider-dependent
  probes out of ordinary fixture runs and avoiding snapshots of missing configuration.
- Interpreter namespace imports support imported callbacks and async callback globals.
- Hover reuses one tokenization across definition and reference lookup.
- Docgen HTTP test fixtures remain alive until their owner shuts them down,
  removing idle fixture expiry during concurrent test startup.

- Channel send releases its shared lock while waiting for queue capacity. VM
  channel methods accept the correct arguments, and receive waits for a value.

- Returned named closures can recursively call themselves in the interpreter.
- Interpreter closures retain referenced bindings and callable dependencies,
  avoiding retention of unrelated generator instances in supported bodies.

- Concurrent promise waiters share completion; timing out one waiter no longer
  consumes the result needed by another waiter.

- Generators now retain shared progress across aliases and resume nested loops
  and exception handlers in both runtimes. VM generators preserve
  owned continuation state, cache terminal failures and restore callers. For loops
  consume generators lazily, so breaking stops at the requested item.

- Routed HTTP server shutdown uses a bounded loopback wakeup for wildcard
  listeners, avoiding a macOS connection timeout during teardown.

- Closures retain the active lexical binding when a later block reuses its name.
  Nested named functions use lexical locals instead of leaking into globals.
  Returned closures also retain scoped constants and function-local imports,
  including import-all exports and module namespaces through intermediate closures.

- Type-checker depth accounting now unwinds after function-call inference, so
  large sequential workflows do not exhaust the nested-expression limit.

- Declared and documented Rust 1.89 as the source-build MSRV, added an MSRV CI
  check, made the REPL banner use the crate version, and corrected upgrade
  guidance that previously implied `kujolang` was published on crates.io.

- MySQL transaction control uses the text protocol, as required by MySQL 8.4,
  while parameterized application queries continue using prepared statements.

- PostgreSQL TLS session initialization and cleanup now stay outside an active
  Tokio executor, preventing nested-runtime panics from async interpreter code.
- Registered the existing `db_last_insert_id` builtin with the type checker,
  eliminating an incorrect undefined-function warning.

- `db_close` now releases SQLite, PostgreSQL and MySQL resources immediately,
  invalidates retained aliases, and rolls back uncommitted work. Repeated close
  remains successful; operations on closed or returned pool leases fail explicitly.
  Pool returns preserve the native session but invalidate the old lease so stale
  aliases cannot affect a later borrower. See `docs/DATABASE_LIFECYCLE.md`.
- MySQL connections now use Kujo's persistent async executor instead of a reactor
  destroyed after each operation, fixing queries against a shut-down runtime.

## [1.5.0] - 2026-09-22
- Added `digest_file_beneath` for bounded streaming SHA-256 of rooted regular
  files up to 64 MiB without retaining the full artifact in memory.

- Added `read_stdin(max_bytes)` for bounded, exact UTF-8 process input through EOF.

- Added bounded, descriptor-relative `list_dir_beneath` pages with explicit scan ceilings, suffix-based cursors, and symlink/race coverage.

### Changed

- Added `copy_file_beneath`, a bounded streaming, no-follow copy with atomic no-replace publication and a SHA-256/byte receipt. Both filesystem read and write capabilities are required.
- Added `sha256_file_beneath` for bounded streaming SHA-256 through no-follow
  rooted file handles, returning the digest and actual byte count up to 4 GiB.
- Added capability-gated `list_dir_page(path, after, limit, suffix)` for sorted
  directory pages retaining at most limit+1 filenames. Each page still scans
  the directory; entry/UTF-8 errors are explicit and no cross-call snapshot is
  promised.
- Streaming AES encryption/decryption now publish with atomic no-replace
  semantics, so concurrent writers cannot overwrite a successful output.
  Additive receipt fields expose publication and temporary-file cleanup facts.

- Added `read_binary_prefix_beneath` for bounded regular-file prefix reads
  through a rooted handle, retaining in-root relative symlinks and rejecting
  concurrent symlink escapes.

- Added opt-in `kujo run --isolated-imports` and inherited
  `KUJO_ISOLATED_IMPORTS=1` for installed tools: imports use entry-file and
  explicit module roots without discovering the caller's modules or lockfile,
  while filesystem operations retain the caller's working directory. Isolated
  runs preserve exact script arguments; default import behavior is unchanged.
- Added bounded, in-process `pdf_render_html` and atomic
  `pdf_render_html_to_file` APIs for branded business documents without an
  external renderer.
- Added `secure_random_token(byte_length)`, an OS-CSPRNG-backed, base64url,
  redacted token primitive isolated from deterministic random seeding.
- Added `db_pool_postgres_tls`, a bounded verified-TLS PostgreSQL pool with
  warm-up, deadlines, health checks, connection expiry, session reset, clean
  shutdown, and operational counters.

- Added lowercase `header_values` arrays to routed HTTP server requests in both
  runtimes, preserving duplicate headers for explicit application validation.

- Expanded scheduled fuzz smoke coverage from the lexer, parser, and XML parser
  to every maintained bounded decoder target, including gzip and ZIP.
- Added a weekly RustSec audit that rejects vulnerable locked dependencies.
- Updated package documentation to distinguish Kujo 1.0's historical local-only
  boundary from the public Kennel registry available today.

### Fixed

- Interpreter top-level function calls no longer read or overwrite unrelated
  caller-local bindings. Direct, indirect, callback, and pipe calls preserve
  lexical boundaries while retaining global assignment and captured closures.



- VM routed HTTP dispatch now invokes closures created by imported modules,
  allowing modular route registrars to capture application context with the
  same behavior as interpreter mode.
- Kept the source-bound TCP security regression server open until the client
  closes, removing a startup-speed race on slower debug builds.
- Restored the runtime mismatch inventory to 149/149 VM matches by adding
  charset snapshots, making integer membership checks explicitly Boolean, and
  excluding the credentialed PostgreSQL TLS probe from offline inventory runs.

## [1.4.0] - 2026-09-09

### Added

- Added `kujo run --scheduler-no-timeout` for long-lived services supervised by
  another process.
- Added `kujo run --isolated-imports` and inherited import isolation for installed
  tools. Isolated runs use the entry file and explicit module paths instead of the
  caller's project, while preserving exact script arguments.
- Added the bounded web and artifact primitives needed by native Kujo tools:
  HTML tokenization, URL handling, text decoding, streaming XML, JSON/JSONL file
  operations, digests, private staging, and atomic no-replace publication.
- Added POSIX file locks, ownership checks, atomic symlink updates, exact process
  replacement, and confined atomic file publication for native package installers
  and command launchers.
- Added `byte_length(value)` for the UTF-8 byte length of strings and raw bytes.
- Preserved repeated HTTP response headers in `http_request.header_values`.

### Fixed

- Streaming AEAD now creates private output files.
- `delete_file` can unlink symlinks, including dangling ones, without touching
  their targets. It still rejects real directories.
- String `contains` keeps its integer `1`/`0` result, while collection membership
  is inferred as Boolean. User and imported signatures still take precedence.
- Windows no-replace directory publication now preserves UTF-16 paths, rejects
  embedded NULs, and atomically refuses existing destinations.
- VM loop scopes now initialize fresh `let` and `const` values and unwind cleanly
  on control flow, returns, and exceptions.
- Async map workers now keep independent state and immutable capture metadata.
- Cross-runtime callbacks now preserve globals, captures, capabilities, errors,
  and bounded recursion without cloning unrelated bytecode state on each call.
- Regex helpers now reuse compiled expressions through a bounded cache.
- Explicit DNS pinning and deny-private HTTP clients now ignore ambient proxies,
  preventing a proxy from resolving the destination again.
- Binary file fixtures, non-JIT parity tests, and bounded web-data regressions now
  cover the shipped behavior instead of skipping or stopping early.

## [1.3.1] - 2026-09-06

### Fixed

- Prevent transient Linux `Text file busy` errors from aborting verified native
  runtime upgrades. Staged version checks retry only this condition, sharing
  one deadline across retries and process execution. Other execution errors,
  checksum verification, version matching and destination checks remain intact.

### Changed

- Exercise inherited writable executable descriptors and repeated parallel
  native upgrades in Linux CI. Verify clean, lifecycle-script-disabled npm
  installations on every supported native target.

## [1.3.0] - 2026-09-05

### Added

- Add native runtime-only `kujo upgrade [VERSION]`, exact stable release selection,
  read-only `--check`, documented `--json`, explicit downgrade opt-in, official
  SHA-256 verification, managed-install guidance, and retained recovery binaries.
  Upgrade archive support remains available without default language features.

- Add database-capability-gated `db_connect_postgres_tls` for PostgreSQL over
  mandatory TLS 1.2+ with peer and hostname verification, an explicit bounded
  public CA bundle, TCP-host enforcement, and secret-free deterministic
  connection failures.

- Add capability-free `decode_charset(bytes, charset, max_output_bytes)` with
  strict no-replacement decoding, explicit 64 MiB resource ceilings, exact
  MIME ISO-8859-1 semantics, registered legacy charset support, and
  VM/interpreter parity.

- Add filesystem-read-gated `decode_text_file_range_info` to compose strict
  identity/Base64/quoted-printable streaming with strict registered-charset
  conversion, independent decoded/UTF-8 hashes, fixed memory, explicit output
  and prefix ceilings, and VM/interpreter parity.

- Add `db_connect_readonly("sqlite", path)` for capability-gated, fail-closed
  immutable inspection of existing checkpointed SQLite databases without
  create, write, WAL or shared-memory side effects.

- Preserve doctor-profile arguments, bounded structured reports up to 1 MiB,
  and optional readiness/profile data through generic Doctor composition.

- Add routed, authorization-preflighted HTTP uploads with fixed-memory body
  streaming into auto-cleaned private files, caller-lowerable 64 MiB ceilings,
  SHA-256 receipts, read deadlines, and interpreter/VM parity. Upload routes
  require explicit network-server, filesystem-write, and filesystem-delete
  capabilities and never expose partial files to application handlers.

- Add `publish_file_noreplace` for atomic same-filesystem adoption without
  overwriting an existing destination. It requires filesystem-write and
  filesystem-delete capabilities and returns explicit publication, unlink,
  directory-durability, inode-identity, byte-count, and verification facts.

- Add bounded incremental response streaming to generic `http_request`, with
  transparent routed-server pass-through, response headers, chunk lifecycle
  callbacks, callback cancellation, downstream-disconnect reporting, and
  interpreter/VM server support without full-response buffering.

- Add bounded TCP/TLS file-range streaming, transformed file-range private
  spools, tracked SQLite immediate transactions, and expanded IP-literal facts.
- Add strict per-request destination policies, DNS pinning, redirect controls,
  and bounded timeouts for streaming HTTP file uploads and downloads.

### Fixed

- Keep synchronous PostgreSQL operations safe when called from async workers
  and preserve PostgreSQL parameter types.
- Run interpreter execution outside the CLI async runtime to avoid nested
  runtime failures for synchronous host APIs.
- Refresh generated TODO and unsafe inventories for the shipped source tree.


- Capture the lexically nearest same-named local in VM closures. This prevents
  a closure created in a later block from binding an inactive earlier block's
  slot, while preserving interpreter parity.

- Accept VM fixed-dictionary option literals in TLS client and server APIs,
  preserving the same allowlist, size limits and minimum-version validation as
  interpreter dictionaries.

- Make imported async Kujo functions callable through both VM call paths with
  interpreter-equivalent arity, captured environment and promise behavior.

- Bound the shared async runtime to one scheduler worker instead of inheriting
  host CPU count. Nonblocking tasks still interleave and explicitly bounded
  blocking/Rayon lanes retain parallelism without host-dependent stack growth.

## [1.2.3] - 2026-09-02

### Added

- Add capability-free `parse_xml_bounded` with namespace-aware deterministic
  trees, caller-lowerable absolute resource ceilings, VM/interpreter parity,
  a dedicated cargo-fuzz target, and fail-closed DTD, non-predefined entity,
  namespace, and malformed-document handling.

- Add a filesystem-read-gated, fixed-memory file-range decoder for strict
  identity, Base64 and quoted-printable transfer encodings. It returns a
  bounded inspection prefix, decoded length, SHA-256, UTF-8/ASCII/NUL facts
  and input-line evidence without returning the decoded body.

- Add stable `response_code` and `name_exists` fields to bounded DNS lookup
  envelopes so Kujo programs can distinguish `NXDOMAIN` from an existing
  name with no records of the requested type.

- Add capability-gated, bounded streaming canonical CRLF text-range hashing
  and normalized RSA public-key inspection so protocol implementations can
  verify large signed artifacts without unbounded buffering or exposing key
  parsing through protocol-specific runtime code.

- Add descriptor-relative text and binary reads that reject symlinks, reparse
  points, special files, traversal, replacement races, and size-limit drift
  beneath an explicitly authorized filesystem root.

- Add bounded string/byte stdin to structured process execution, staged in a
  private auto-deleting file and reopened read-only before process spawn.

### Fixed

- Point the cargo-fuzz harness at the renamed `kujolang` package while keeping
  the library import alias `kujo`, restoring all configured fuzz targets.

- Give both CLI and shared interpreter async runtimes a fixed 8 MiB worker
  stack so bounded protocol and policy functions can run under `promise_all`
  without overflowing Tokio's smaller default worker stack.
- Run the complete CLI lifecycle on a dedicated large-stack thread so Windows
  does not overflow its smaller default main-thread stack before VM or
  interpreter execution.

### Security

- Close post-validation filesystem replacement races for applications that
  read files selected beneath a trusted root.
- Let applications pass already-authorized bytes to structured argv processes
  without shell interpolation, unbounded pipe writers, writable inherited
  stdin handles, or hidden capability expansion.

## [1.2.2] - 2026-09-01

### Added

- Preserve raw bytes for outbound and server-side HTTP request and response
  bodies across interpreter and VM paths.
- Add binary-safe gzip compression for byte payloads.
- Add raw byte concatenation and search primitives.

### Fixed

- Keep Unix-only private-spool metadata checks out of Windows builds, restoring
  the complete native release and npm package matrix.
- Run `npm pack` through the Windows command shell so the native `npm.cmd`
  launcher works during platform-package assembly.

## [1.2.1] - 2026-09-01

### Added

- Add capability-gated bounded private-file spools (`io_private_spool_open`,
  `io_private_spool_write`, `io_private_spool_finish`, and
  `io_private_spool_abort`) with restrictive-at-creation permissions,
  single-use handles, incremental SHA-256 receipts, no-overwrite atomic
  publication, VM/interpreter parity, fail-closed cleanup, and explicit
  post-publication durability/temporary-cleanup facts.
- Add lifecycle-script-free npm runtime packaging with exact-version native
  platform packages, an allow-listed launcher, package contract tests, and
  trusted-publishing provenance.
- Extend `tcp_info` with socket-derived peer/local IP and port fields so raw
  protocol servers can enforce address-scoped admission policy without parsing
  ambiguous combined address strings.

### Fixed

- Honor declared HTTP request-body lengths in routed interpreter and VM
  servers, rejecting declared overflow before reading and responding as soon as
  a complete keep-alive body arrives instead of waiting for the read deadline.
- Reject tag-time binary and npm publication when the Git tag, Cargo version,
  and npm package versions do not match exactly.

### Security

- Close the routed HTTP request boundary so oversized declared bodies are
  rejected before buffering and completed requests cannot be held until the
  read deadline.

## [1.2.0] - 2026-09-01

### Added

- Add capability-free bounded in-memory gzip decompression and safe
  single-regular-entry ZIP reading for hostile compressed protocol inputs.
- Add a checksum-verified reusable GitHub Action for installing exact Kujo
  release versions on Linux, macOS, and Windows CI runners.
- Add deterministic IP-scope classification and structured TCP bind probes for
  network policy checks and server startup diagnostics.
- Add capability-gated, bounded MX, TXT, PTR, and TLSA DNS lookup primitives
  with deterministic result envelopes and DNSSEC proof status.
- Add verified TLS client sockets, server acceptors, consumptive STARTTLS-style
  TCP upgrades, bounded TLS I/O, certificate fingerprints, and TLS 1.2+
  fail-closed protocol policy.
- Add deterministic TCP peer/local-address inspection and bounded per-stream
  read/write timeout controls for protocol servers.
- Add strict `decode_base64_utf8(text)` decoding for text-based binary protocol
  fields without filesystem conversion or lossy UTF-8 behavior.
- Add capability-gated `tcp_connect_bound(host, port, source_ip)` so protocols
  that require deterministic egress-IP selection can bind a validated unicast
  local address before connecting.
- Add per-request `http_request` destination denial, DNS pinning, and redirect
  disabling so untrusted webhook-style callbacks can close DNS-rebinding and
  redirect-based SSRF paths without relying on process-global policy; callers
  can also lower the bounded response-body ceiling per request.

### Changed

- Bound routed HTTP header/body reads with a configurable socket deadline and
  expose direct socket peer address, IP, port, and transport fields with
  matching VM/interpreter behavior.
- Use `kujolang` as the Cargo/crates.io package name while preserving `kujo` as
  the installed CLI command and Rust library crate name.
- Align `parse_json`'s input ceiling with the 8 MiB file-I/O boundary so JSON
  written and read by Kujo remains parseable without an artificial 1 MiB gap.

### Fixed

- Replace the yanked `mysql_async 0.37.0` dependency with `0.37.1` so the
  release dependency audit remains warning-free.

## [1.1.0] - 2026-08-30

### Added

- Add the repository-owned `kujo agent` project lifecycle with deterministic
  profiles, Agent Doctor diagnostics, inspect/run/eval commands, pinned
  ecosystem composition, live AI SDK bridging, Workcell execution, and a
  checked-in self-hosted knowledge-agent example.
- Add first-class `kujo agent auth` credential management backed by macOS
  Keychain, Windows Credential Manager, or Linux Secret Service, with masked
  interactive entry, stdin/CI setup, private project overrides, connector-key
  support, credential readiness diagnostics, and secret redaction.
- Add stable `encode_uri_component(text)` RFC 3986 UTF-8 percent encoding and strict `decode_uri_component(text)` decoding for provider and web integrations.
- Add bounded, streaming `jsonl_query(path, options)` filtering and constant-memory join support for Kujo-native evidence workflows.
- Accept standard JSON Schema Draft 2020-12 identification and annotation keywords in `json_schema_validate`, including `$schema`, `$id`, `$comment`, `format`, `deprecated`, `readOnly`, and `writeOnly`.
- Add a deterministic Kujo-native repository policy gate example with stable
  JSON reports and passing/failing contract fixtures.

### Fixed

- Make Agent project integration fixtures portable to hosted CI by checking out
  every composed ecosystem repository at its exact scaffolded commit.
- Preflight JIT benchmark bytecode before execution so unsupported benchmark
  programs report a bounded result instead of triggering a Cranelift panic, and
  render unavailable runtime speedups as `N/A`.

- Eliminate the remaining Cargo audit maintenance warnings by upgrading
  Cranelift off `region 2`/`mach` and replacing the unmaintained `paste` macro
  used by image-codec dependencies with maintained `pastey` compatibility
  patches while preserving the current image/EXR feature set and parallelism;
  the release gate now denies warnings.
- Remove the `RUSTSEC-2023-0071` RSA timing advisory by moving public RSA
  operations to a current vendored OpenSSL implementation and using AWS-LC for
  the HS256-only JWT backend; the release gate no longer suppresses the advisory.
- Preserve exact integer comparisons in `json_schema_validate` beyond the IEEE-754 safe-integer range.
- Reject incomplete `jsonl_query` join configurations instead of silently returning an empty join.
- Bound `ssg_build_output_paths` before allocating generated path arrays.
- Prevent `spawn_process` stream redaction from exposing secrets split across incremental flush boundaries.
- Make LSP definition, reference, hover, rename, document-symbol, and inlay-hint handling recognize standalone `mut` bindings and lexical scope correctly.
- Preserve CRLF source text during LSP rename edits and accept lexer-supported Unicode identifier continuations.
- Use the LSP-required UTF-16 code-unit coordinate system for incoming positions and outgoing ranges, including semantic tokens.
- Reject LSP messages larger than 8 MiB before allocating their payload buffer.
- Update transitive database and error-handling dependencies to patched releases for `RUSTSEC-2026-0190` and `RUSTSEC-2026-0253`.
- Recognize fallible calls inside multiline `try` blocks in `kujo lint` instead of reporting false missing-error-handling warnings.
- Preserve outer-block indentation after formatting a nested closing brace.
- Ignore braces inside strings and comments when computing formatter indentation.
- Stop definition, reference, hover, and rename lookups from selecting an identifier when the cursor is immediately after it.
- Emit LSP reference ranges that span the complete identifier.
- Emit zero-width LSP edits for missing-delimiter insertion actions.
- Limit range-formatting edits to the client-requested range.
- Limit inlay hints to the client-requested range.
- Include trailing newline positions in full-document formatting edits.
- Consume cancelled request IDs so later requests may safely reuse them.
- Accept the case-insensitive `Content-Length` header names required by the LSP transport protocol.

## [1.0.2] - 2026-08-26

### Added

- Automatically discover locked Kennel-installed dependency roots from the nearest project `kennel.lock`, so normal package consumers no longer need manual `KUJO_MODULE_PATH` wiring.

### Changed

- Preserve `KUJO_MODULE_PATH` as an explicit override/extension point while keeping project-scoped, deterministic package resolution and path-containment protections.

### Security

- Update the locked `h2` dependency to `0.4.16` to address `RUSTSEC-2026-0258`.

## [1.0.1] - 2026-08-11

### Fixed

- Build Linux release binaries on Ubuntu 22.04 so the published artifact remains compatible with Ubuntu 22.04 hosts such as Cloudflare Pages.

## [1.0.0] - 2026-08-08

### Added

- Initial public release of Kujo.
- VM-first language runtime, CLI, LSP, package lockfile workflow, static server, capability controls, and deterministic AI runtime primitives.
- Prebuilt Linux x64, macOS x64/arm64, and Windows x64 binaries with SHA-256 checksums.
- User-local ecosystem installer with core, AI, quality, showcase, and operating profiles.

### Changed

- Finalized machine-readable CLI and runtime diagnostic contract version identifiers at `1.0.0`.

### Fixed

- Repaired all remaining syntax-drifted examples, removed the expected-fail example list, and added exhaustive per-file verification coverage.
