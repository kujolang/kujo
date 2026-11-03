# Concurrent owner and repeated lineage resource check

The existing ignored Linux resource diagnostic now supports
`KUJO_OWNER_RESOURCE_CONCURRENT=1`. This mode creates eight successive pools,
each with production candidate quotas (two idle owners, 512 definitions, 4 MiB
emitted code and 8 MiB visible metadata per owner).

Each pool runs sixteen rounds of two native workers. Both workers hold exclusive
owners until the controller observes them ready; no owner can return early and
serve the other worker. The controller requires zero idle owners while both are
held, then releases the workers, joins them and requires exactly two idle owners.
Every native invocation checks its integer result. It records RSS and executable
mapping size while owners are held and idle, and after each pool is destroyed.
The common warmup runs before the baseline snapshot.

This is a mechanism-level lease test. It does not measure scheduler throughput,
exercise every async route, attribute all RSS to the pool, or measure total live
heap. The existing overlapping task integration test covers actual task-route
concurrency. Thread stacks and allocator retention can affect RSS here and must
be distinguished from executable mappings in review. Sampling is not OS peak RSS.

CI executes the mode in a separate process with a 180-second deadline, using the
same pinned test binary as the serial resource matrix. The verifier requires all
128 active/idle observations and eight drop observations, exact test membership,
successful summary, positive resource values and two-owner counts. Seven
synthetic controls cover successful parsing of Rust's inline test-output format
and rejection of duplicate rounds, missing drops, wrong owner counts, invalid
RSS, fabricated membership and failed tests.

Local parser controls and formatting do not establish runtime success. Execution
and raw resource review remain pending CI; no memory bound or leak conclusion is
accepted merely because the verifier accepts the record structure.
