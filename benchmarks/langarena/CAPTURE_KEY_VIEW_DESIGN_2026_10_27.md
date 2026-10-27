# Oversized capture lookup storage: historical design, now integrated

Runtime `82bdef6` implements the existing-kind-map design described below.
[Verified results](COMPACT_CAPTURE_VIEWS_2026_10_27.md) include the first candidate's
host-removal regression, its correction, dense writes, release integration tests
and all 150 test-size checksums. The remaining text preserves the pre-integration
design and baseline; it is not the current acceptance record.

Current accepted runtime4f5087b still builds local HashMap<String, Value> maps
containing only Null placeholders. Values themselves remain in the public owner.
Current split evidence's depth-zero delta is41requests/1654bytes through4096locals,
8240/2721367 at8192,12336/2764615 at12288,16432/5447495 at16384. No need to rerun
these accepted measurements simply to re-establish this same cost.

The completed fresh-storage probe separates string/key allocation from bucket
payload: Value is136bytes, align8. At16384keys, placeholder map5444778bytes vs
HashSet988330; both16385requests. The request count/key copying remains. This is
component evidence, not runtime speed or production acceptance. A second probe
measures HashMap<String, BindingKind>, using ManuallyDrop<Null> only to preserve
exact layout without payload destruction; that source has no owned Value payload.

Rejected initial representation: Option<enum{Existing(Weak owner,usize limit),
KeyView(Arc metadata)}> grows16to24bytes although the enum itself stays16. Do not
inflate every Environment or activation frame for an oversized-case optimization.

Promising implementation direction: reuse the existing binding_kinds maps for
complete key-plus-kind metadata in definitely oversized private local views,
leaving public owners unchanged. Small/pooled views retain current storage.
No raising cache bounds or retaining oversized idle maps. Admission should follow
a proven pool/storage bound, not a benchmark-specific identifier or width list.

Required work before integration:
- Preserve private membership snapshots independently of owner additions/removals;
  looking directly into the owner for every name would change lexical membership.
- Centralize local membership across placeholder maps and key metadata. Apply to
  get, duplicate checks, pending-global shadowing, mutation guards and runtime-leaf
  scope resolution. Globals/parameters remain ordinary maps.
- Generic set/assign/mutate/try_mutate must operate on owner-backed values without
  lazily inserting a full Value-map entry for every mutated key. Otherwise dense
  mutation recreates the old map plus metadata and regresses allocation.
- Preserve the unusual host removal/expired-owner fallback, immutable-kind snapshot,
  callback lock release, rejected mutation behavior and write journals.
- detached_snapshot/materialized_scopes must enumerate metadata keys and retain
  values/kinds; capture_for_callable/visible_bindings already materialize live views.
  Preserve clone/recursive prefix/restore behavior when dense and compact views mix.
- Review direct scope enumeration in Interpreter cleanup, declaration fast paths,
  ownership release and Rust embedding. Public captured Environment maps must retain
  actual readable/writable values. No unsafe borrowed String keys or weak caches
  that ignore direct host map edits.
- Keep ordinary hot-path lookup overhead bounded. A new optional metadata variant
  is not acceptable without a verified unchanged enclosing Environment layout.
  Any private packed flag needs an explicit Vec-length bound proof and layout tests;
  do not introduce a language-visible scope limit or encode names specially.

Verification: immutable/mutable captures, membership additions/removals, native-name
shadowing, nested snapshots, direct/foreign/split recursion, host callbacks, errors,
arrays/dictionaries, tasks and dense all-key writes in all runtimes. Preserve all
accepted allocation controls including small/warm/cold/append routes. Measure
candidate outer-activation requests/bytes on matched release artifacts at the
existing seven widths; reject total/per-call regressions. All150testsizechecksums
and broader release tests remain required for runtime integration. Key-copying and
full performance checklist remain open even if table bytes improve.
