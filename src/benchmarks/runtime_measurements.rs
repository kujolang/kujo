//! Fixed-cardinality production collector owned by the existing profiler.
//! One process session; no labels, payloads, event buffering, or exporter thread.
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

macro_rules! metrics {
    ($($name:ident => $key:literal),+ $(,)?) => {
        #[derive(Clone, Copy)]
        #[allow(dead_code)] // Fixed contract also exists in builds without JIT.
        pub(crate) enum Metric { $($name,)+ Count }
        const NAMES: &[&str] = &[$($key,)+];
    };
}

metrics! {
    VmEntries => "vm_entries",
    VmWallNs => "vm_inclusive_wall_ns",
    Calls => "vm_call_opcodes",
    Returns => "vm_return_opcodes",
    NativeCalls => "vm_native_call_opcodes",
    SchedulerRounds => "scheduler_rounds",
    Closures => "vm_closures_created",
    CaptureCells => "vm_capture_cells_created",
    CaptureShallowBytes => "vm_capture_value_shallow_bytes",
    Generators => "vm_generator_states_created",
    GeneratorDrops => "vm_generator_state_drops",
    GeneratorShallowBytes => "vm_generator_state_shallow_bytes",
    GeneratorResumes => "vm_generator_resume_attempts",
    TasksAdmitted => "tasks_admitted",
    TasksRejected => "tasks_admission_rejected",
    TasksStarted => "tasks_started",
    TasksExited => "task_bodies_exited",
    TasksCompleted => "task_completions_published",
    TasksCancelled => "task_cancellations_published",
    TaskQueueNs => "task_queue_wall_ns",
    Detached => "detached_tasks_observed",
    PromisePolls => "promise_polls",
    PromisePending => "promise_pending_polls",
    PromiseReady => "promise_ready_polls",
    JitCompileEntries => "jit_compile_entries",
    JitCompileWallNs => "jit_compile_inclusive_wall_ns",
    JitCacheHits => "jit_cache_lookup_hits",
    JitCacheMisses => "jit_cache_lookup_misses",
    JitGuardPass => "jit_type_guard_passes",
    JitGuardFail => "jit_type_guard_failures",
}

struct Collector {
    started: Instant,
    cpu_started: Option<f64>,
    counters: [AtomicU64; Metric::Count as usize],
}
static COLLECTOR: OnceLock<Collector> = OnceLock::new();

/// Start once per process. Reuse would mix detached tasks across run identities.
pub fn start() -> Result<(), &'static str> {
    COLLECTOR
        .set(Collector {
            started: Instant::now(),
            cpu_started: crate::interpreter::native_functions::web_data::process_usage_snapshot()
                .ok()
                .and_then(|value| value.0),
            counters: std::array::from_fn(|_| AtomicU64::new(0)),
        })
        .map_err(|_| "runtime measurements already started in this process")
}

// Keep fetch_update for the Rust 1.89 MSRV; newer Rust renames it to try_update.
#[allow(deprecated)]
#[inline]
pub(crate) fn add(metric: Metric, amount: u64) {
    if let Some(collector) = COLLECTOR.get() {
        let _ = collector.counters[metric as usize].fetch_update(
            Ordering::Relaxed,
            Ordering::Relaxed,
            |value| Some(value.saturating_add(amount)),
        );
    }
}

pub(crate) fn clock() -> Option<Instant> {
    COLLECTOR.get().map(|_| Instant::now())
}

pub(crate) fn elapsed(metric: Metric, started: Option<Instant>) {
    if let Some(started) = started {
        add(metric, started.elapsed().as_nanos().min(u64::MAX as u128) as u64);
    }
}

/// Inclusive boundary time, also closed on language errors and Rust unwinding.
pub(crate) struct Span(Metric, Option<Instant>);
impl Span {
    pub(crate) fn new(metric: Metric) -> Self {
        Self(metric, clock())
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        elapsed(self.0, self.1);
    }
}

#[cfg(feature = "runtime-jit")]
pub(crate) fn guard_result(value: i64) -> i64 {
    add(if value == 1 { Metric::JitGuardPass } else { Metric::JitGuardFail }, 1);
    value
}

/// Snapshot independently sampled counters. Workers may still be running.
/// A report is evidence, never permission to continue or replay execution.
pub fn snapshot(outcome: &str) -> serde_json::Value {
    let Some(collector) = COLLECTOR.get() else {
        return serde_json::Value::Null;
    };
    let (cpu_now, peak_rss_bytes) =
        crate::interpreter::native_functions::web_data::process_usage_snapshot()
            .unwrap_or((None, None));
    let cpu_seconds = collector.cpu_started.zip(cpu_now).map(|(start, end)| (end - start).max(0.0));
    let counters: BTreeMap<_, _> = NAMES
        .iter()
        .zip(&collector.counters)
        .map(|(name, value)| (*name, value.load(Ordering::Relaxed)))
        .collect();
    serde_json::json!({
        "schema": "kujo.runtime-measurements/v1",
        "scope": "process",
        "runtime": "vm",
        "runtime_version": env!("CARGO_PKG_VERSION"),
        "outcome": outcome,
        "wall_ns": collector.started.elapsed().as_nanos().min(u64::MAX as u128) as u64,
        "counters": counters,
        "cpu_seconds": cpu_seconds,
        "process_peak_rss_bytes": peak_rss_bytes,
        "snapshot_consistency": "independent_counters",
        "waited_for_detached": false,
        "unsupported": ["total_heap_bytes", "total_value_allocations", "total_value_drops", "retained_capture_graph_bytes", "provider_usage", "provider_cost"],
    })
}
