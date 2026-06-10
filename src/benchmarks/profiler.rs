// Performance profiling infrastructure for Kujo
//
// Provides CPU and memory profiling capabilities to identify performance bottlenecks.
// Integrates with system profiling tools like perf and instruments.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Profiling configuration
#[derive(Debug, Clone)]
pub struct ProfileConfig {
    /// Enable CPU profiling
    pub cpu_profiling: bool,
    /// Enable memory profiling
    pub memory_profiling: bool,
    /// Sample interval for CPU profiling (microseconds)
    /// TODO: Implement sampling-based profiling (currently event-based)
    #[allow(dead_code)]
    pub sample_interval_us: u64,
    /// Enable JIT compilation statistics
    pub jit_stats: bool,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            cpu_profiling: true,
            memory_profiling: true,
            sample_interval_us: 1000, // 1ms
            jit_stats: true,
        }
    }
}

/// CPU profiling data
#[derive(Debug, Clone)]
pub struct CPUProfile {
    /// Function name -> execution time
    pub function_times: HashMap<String, Duration>,
    /// Total execution time
    pub total_time: Duration,
    /// Number of samples taken
    pub sample_count: usize,
}

impl CPUProfile {
    pub fn new() -> Self {
        Self { function_times: HashMap::new(), total_time: Duration::ZERO, sample_count: 0 }
    }

    /// Record execution time for a function
    /// TODO: Integrate into VM execution loop for automatic tracking
    #[allow(dead_code)]
    pub fn record_function(&mut self, name: String, duration: Duration) {
        *self.function_times.entry(name).or_insert(Duration::ZERO) += duration;
        self.total_time += duration;
        self.sample_count += 1;
    }

    /// Get top N hot functions by execution time
    pub fn top_functions(&self, n: usize) -> Vec<(String, Duration, f64)> {
        let mut functions: Vec<_> = self
            .function_times
            .iter()
            .map(|(name, &duration)| {
                let percentage = if self.total_time.as_nanos() > 0 {
                    (duration.as_nanos() as f64 / self.total_time.as_nanos() as f64) * 100.0
                } else {
                    0.0
                };
                (name.clone(), duration, percentage)
            })
            .collect();

        functions.sort_by(|a, b| b.1.cmp(&a.1));
        functions.into_iter().take(n).collect()
    }
}

/// Memory profiling data
#[derive(Debug, Clone)]
pub struct MemoryProfile {
    /// Peak memory usage in bytes
    pub peak_memory: usize,
    /// Current memory usage in bytes
    pub current_memory: usize,
    /// Total allocations
    pub total_allocations: usize,
    /// Total deallocations
    pub total_deallocations: usize,
    /// Allocation hotspots (location -> count)
    pub allocation_hotspots: HashMap<String, usize>,
}

impl MemoryProfile {
    pub fn new() -> Self {
        Self {
            peak_memory: 0,
            current_memory: 0,
            total_allocations: 0,
            total_deallocations: 0,
            allocation_hotspots: HashMap::new(),
        }
    }

    /// Record an allocation
    /// TODO: Integrate into Value allocation for automatic tracking
    #[allow(dead_code)]
    pub fn record_allocation(&mut self, size: usize, location: String) {
        self.current_memory += size;
        self.peak_memory = self.peak_memory.max(self.current_memory);
        self.total_allocations += 1;
        *self.allocation_hotspots.entry(location).or_insert(0) += 1;
    }

    /// Record a deallocation
    /// TODO: Integrate into Value drop for automatic tracking
    #[allow(dead_code)]
    pub fn record_deallocation(&mut self, size: usize) {
        self.current_memory = self.current_memory.saturating_sub(size);
        self.total_deallocations += 1;
    }

    /// Get top N allocation hotspots
    pub fn top_hotspots(&self, n: usize) -> Vec<(String, usize)> {
        let mut hotspots: Vec<_> =
            self.allocation_hotspots.iter().map(|(loc, &count)| (loc.clone(), count)).collect();
        hotspots.sort_by(|a, b| b.1.cmp(&a.1));
        hotspots.into_iter().take(n).collect()
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            peak_mb: self.peak_memory as f64 / (1024.0 * 1024.0),
            current_mb: self.current_memory as f64 / (1024.0 * 1024.0),
            total_allocations: self.total_allocations,
            total_deallocations: self.total_deallocations,
            leaked_objects: self.total_allocations.saturating_sub(self.total_deallocations),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub peak_mb: f64,
    pub current_mb: f64,
    pub total_allocations: usize,
    pub total_deallocations: usize,
    pub leaked_objects: usize,
}

/// JIT compilation statistics
#[derive(Debug, Clone)]
pub struct JITStats {
    /// Number of functions compiled
    pub functions_compiled: usize,
    /// Number of recompilations (due to guard failures)
    pub recompilations: usize,
    /// Total compilation time
    pub total_compile_time: Duration,
    /// JIT cache hits
    pub cache_hits: usize,
    /// JIT cache misses
    pub cache_misses: usize,
    /// Guard successes
    pub guard_successes: usize,
    /// Guard failures (triggers deoptimization)
    pub guard_failures: usize,
}

impl JITStats {
    pub fn new() -> Self {
        Self {
            functions_compiled: 0,
            recompilations: 0,
            total_compile_time: Duration::ZERO,
            cache_hits: 0,
            cache_misses: 0,
            guard_successes: 0,
            guard_failures: 0,
        }
    }

    /// Get hit rate percentage
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            (self.cache_hits as f64 / total as f64) * 100.0
        }
    }

    /// Get guard success rate percentage
    pub fn guard_success_rate(&self) -> f64 {
        let total = self.guard_successes + self.guard_failures;
        if total == 0 {
            0.0
        } else {
            (self.guard_successes as f64 / total as f64) * 100.0
        }
    }
}

/// Complete profile data
#[derive(Debug, Clone)]
pub struct ProfileData {
    pub cpu: CPUProfile,
    pub memory: MemoryProfile,
    pub jit: JITStats,
    pub config: ProfileConfig,
}

impl ProfileData {
    pub fn new(config: ProfileConfig) -> Self {
        Self { cpu: CPUProfile::new(), memory: MemoryProfile::new(), jit: JITStats::new(), config }
    }
}

/// Profiler for collecting performance data
pub struct Profiler {
    data: ProfileData,
    start_time: Option<Instant>,
}

impl Profiler {
    pub fn new(config: ProfileConfig) -> Self {
        Self { data: ProfileData::new(config), start_time: None }
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn stop(&mut self) -> Duration {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            self.data.cpu.total_time = elapsed;
            elapsed
        } else {
            Duration::ZERO
        }
    }

    /// Record function execution time
    /// TODO: Integrate into VM function call/return for automatic profiling
    #[allow(dead_code)]
    pub fn record_function(&mut self, name: String, duration: Duration) {
        if self.data.config.cpu_profiling {
            self.data.cpu.record_function(name, duration);
        }
    }

    /// Record memory allocation
    /// TODO: Integrate into Value::new() for automatic tracking
    #[allow(dead_code)]
    pub fn record_allocation(&mut self, size: usize, location: String) {
        if self.data.config.memory_profiling {
            self.data.memory.record_allocation(size, location);
        }
    }

    /// Record memory deallocation
    /// TODO: Integrate into Value::drop() for automatic tracking
    #[allow(dead_code)]
    pub fn record_deallocation(&mut self, size: usize) {
        if self.data.config.memory_profiling {
            self.data.memory.record_deallocation(size);
        }
    }

    /// Record JIT compilation event
    /// TODO: Call from JitCompiler::compile()
    #[allow(dead_code)]
    pub fn record_jit_compile(&mut self, duration: Duration) {
        if self.data.config.jit_stats {
            self.data.jit.functions_compiled += 1;
            self.data.jit.total_compile_time += duration;
        }
    }

    /// Record JIT recompilation event
    /// TODO: Call from JitCompiler when recompiling due to guard failures
    #[allow(dead_code)]
    pub fn record_jit_recompile(&mut self) {
        if self.data.config.jit_stats {
            self.data.jit.recompilations += 1;
        }
    }

    /// Record JIT cache hit
    /// TODO: Call from VM when JIT cache lookup succeeds
    #[allow(dead_code)]
    pub fn record_cache_hit(&mut self) {
        if self.data.config.jit_stats {
            self.data.jit.cache_hits += 1;
        }
    }

    /// Record JIT cache miss
    /// TODO: Call from VM when JIT cache lookup fails
    #[allow(dead_code)]
    pub fn record_cache_miss(&mut self) {
        if self.data.config.jit_stats {
            self.data.jit.cache_misses += 1;
        }
    }

    /// Record successful guard check
    /// TODO: Call from JIT-compiled code when type guards pass
    #[allow(dead_code)]
    pub fn record_guard_success(&mut self) {
        if self.data.config.jit_stats {
            self.data.jit.guard_successes += 1;
        }
    }

    /// Record failed guard check
    /// TODO: Call from JIT-compiled code when type guards fail
    #[allow(dead_code)]
    pub fn record_guard_failure(&mut self) {
        if self.data.config.jit_stats {
            self.data.jit.guard_failures += 1;
        }
    }

    /// Get profile data reference
    /// Used for incremental analysis during profiling
    #[allow(dead_code)]
    pub fn data(&self) -> &ProfileData {
        &self.data
    }

    pub fn into_data(self) -> ProfileData {
        self.data
    }
}

/// Generate a flamegraph-compatible stack trace format
pub fn generate_flamegraph_data(profile: &CPUProfile) -> String {
    let mut lines = Vec::new();

    for (func, duration) in &profile.function_times {
        // Format: function_name count
        // where count is in microseconds
        let micros = duration.as_micros();
        lines.push(format!("{} {}", func, micros));
    }

    lines.join("\n")
}

/// Render a deterministic, no-color profiling report for snapshots/tests.
pub fn render_profile_report_text(profile: &ProfileData) -> String {
    let mut out = String::new();
    out.push_str("\n=== Performance Profile Report ===\n");

    if profile.config.cpu_profiling {
        out.push_str("\nCPU Profile:\n");
        out.push_str(&format!("  Total Time: {:.3}s\n", profile.cpu.total_time.as_secs_f64()));
        out.push_str(&format!("  Samples: {}\n", profile.cpu.sample_count));

        let top_funcs = profile.cpu.top_functions(10);
        if !top_funcs.is_empty() {
            out.push_str("\n  Top Hot Functions:\n");
            for (i, (name, duration, percentage)) in top_funcs.iter().enumerate() {
                out.push_str(&format!(
                    "    {}. {:<30} {:>8.3}s ({:>5.1}%)\n",
                    i + 1,
                    name.chars().take(30).collect::<String>(),
                    duration.as_secs_f64(),
                    percentage
                ));
            }
        }
    }

    if profile.config.memory_profiling {
        out.push_str("\nMemory Profile:\n");
        let stats = profile.memory.stats();
        out.push_str(&format!("  Peak Memory: {:.2} MB\n", stats.peak_mb));
        out.push_str(&format!("  Current Memory: {:.2} MB\n", stats.current_mb));
        out.push_str(&format!("  Total Allocations: {}\n", stats.total_allocations));
        out.push_str(&format!("  Total Deallocations: {}\n", stats.total_deallocations));

        if stats.leaked_objects > 0 {
            out.push_str(&format!("  WARN Leaked Objects: {}\n", stats.leaked_objects));
        }

        let hotspots = profile.memory.top_hotspots(5);
        if !hotspots.is_empty() {
            out.push_str("\n  Top Allocation Hotspots:\n");
            for (i, (location, count)) in hotspots.iter().enumerate() {
                out.push_str(&format!("    {}. {:<40} {:>6} allocs\n", i + 1, location, count));
            }
        }
    }

    if profile.config.jit_stats {
        out.push_str("\nJIT Statistics:\n");
        out.push_str(&format!("  Functions Compiled: {}\n", profile.jit.functions_compiled));
        out.push_str(&format!("  Recompilations: {}\n", profile.jit.recompilations));
        out.push_str(&format!(
            "  Total Compile Time: {:.3}s\n",
            profile.jit.total_compile_time.as_secs_f64()
        ));
        out.push_str(&format!("  Cache Hit Rate: {:.1}%\n", profile.jit.hit_rate()));
        out.push_str(&format!("  Guard Success Rate: {:.1}%\n", profile.jit.guard_success_rate()));

        if profile.jit.guard_failures > 0 {
            let failure_rate = 100.0 - profile.jit.guard_success_rate();
            if failure_rate > 5.0 {
                out.push_str(&format!("  WARN High guard failure rate: {:.1}%\n", failure_rate));
                out.push_str("     Consider adjusting specialization thresholds\n");
            }
        }
    }

    out.push_str("\n===================================\n");
    out
}

/// Print profiling report
pub fn print_profile_report(profile: &ProfileData) {
    if std::env::var_os("NO_COLOR").is_some() {
        print!("{}", render_profile_report_text(profile));
        return;
    }

    use colored::Colorize;

    println!("\n{}", "=== Performance Profile Report ===".bold().cyan());

    // CPU Profile
    if profile.config.cpu_profiling {
        println!("\n{}", "CPU Profile:".bold().yellow());
        println!("  Total Time: {:.3}s", profile.cpu.total_time.as_secs_f64());
        println!("  Samples: {}", profile.cpu.sample_count);

        let top_funcs = profile.cpu.top_functions(10);
        if !top_funcs.is_empty() {
            println!("\n  {} Hot Functions:", "Top".bold());
            for (i, (name, duration, percentage)) in top_funcs.iter().enumerate() {
                println!(
                    "    {}. {:<30} {:>8.3}s ({:>5.1}%)",
                    i + 1,
                    name.chars().take(30).collect::<String>(),
                    duration.as_secs_f64(),
                    percentage
                );
            }
        }
    }

    // Memory Profile
    if profile.config.memory_profiling {
        println!("\n{}", "Memory Profile:".bold().yellow());
        let stats = profile.memory.stats();
        println!("  Peak Memory: {:.2} MB", stats.peak_mb);
        println!("  Current Memory: {:.2} MB", stats.current_mb);
        println!("  Total Allocations: {}", stats.total_allocations);
        println!("  Total Deallocations: {}", stats.total_deallocations);

        if stats.leaked_objects > 0 {
            println!("  {} Leaked Objects: {}", "⚠️".yellow(), stats.leaked_objects);
        }

        let hotspots = profile.memory.top_hotspots(5);
        if !hotspots.is_empty() {
            println!("\n  {} Allocation Hotspots:", "Top".bold());
            for (i, (location, count)) in hotspots.iter().enumerate() {
                println!("    {}. {:<40} {:>6} allocs", i + 1, location, count);
            }
        }
    }

    // JIT Statistics
    if profile.config.jit_stats {
        println!("\n{}", "JIT Statistics:".bold().yellow());
        println!("  Functions Compiled: {}", profile.jit.functions_compiled);
        println!("  Recompilations: {}", profile.jit.recompilations);
        println!("  Total Compile Time: {:.3}s", profile.jit.total_compile_time.as_secs_f64());
        println!("  Cache Hit Rate: {:.1}%", profile.jit.hit_rate());
        println!("  Guard Success Rate: {:.1}%", profile.jit.guard_success_rate());

        if profile.jit.guard_failures > 0 {
            let failure_rate = 100.0 - profile.jit.guard_success_rate();
            if failure_rate > 5.0 {
                println!("  {} High guard failure rate: {:.1}%", "⚠️".yellow(), failure_rate);
                println!("     Consider adjusting specialization thresholds");
            }
        }
    }

    println!("\n{}", "===================================".bold().cyan());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_profile_recording() {
        let mut profile = CPUProfile::new();

        profile.record_function("func_a".to_string(), Duration::from_millis(100));
        profile.record_function("func_b".to_string(), Duration::from_millis(200));
        profile.record_function("func_a".to_string(), Duration::from_millis(50));

        assert_eq!(profile.sample_count, 3);
        assert_eq!(profile.total_time, Duration::from_millis(350));

        let top = profile.top_functions(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, "func_b");
        assert_eq!(top[1].0, "func_a");
    }

    #[test]
    fn test_memory_profile_tracking() {
        let mut profile = MemoryProfile::new();

        profile.record_allocation(1024, "location_a".to_string());
        profile.record_allocation(2048, "location_b".to_string());
        profile.record_deallocation(1024);

        assert_eq!(profile.current_memory, 2048);
        assert_eq!(profile.peak_memory, 3072);
        assert_eq!(profile.total_allocations, 2);
        assert_eq!(profile.total_deallocations, 1);

        let stats = profile.stats();
        assert_eq!(stats.leaked_objects, 1);
    }

    #[test]
    fn test_jit_stats() {
        let mut stats = JITStats::new();

        stats.cache_hits = 90;
        stats.cache_misses = 10;
        assert_eq!(stats.hit_rate(), 90.0);

        stats.guard_successes = 950;
        stats.guard_failures = 50;
        assert_eq!(stats.guard_success_rate(), 95.0);
    }

    #[test]
    fn test_profiler_workflow() {
        let config = ProfileConfig::default();
        let mut profiler = Profiler::new(config);

        profiler.start();
        std::thread::sleep(Duration::from_millis(10));

        profiler.record_function("test_func".to_string(), Duration::from_millis(5));
        profiler.record_allocation(1024, "test_location".to_string());
        profiler.record_jit_compile(Duration::from_micros(100));
        profiler.record_cache_hit();
        profiler.record_guard_success();

        let elapsed = profiler.stop();
        assert!(elapsed >= Duration::from_millis(10));

        let data = profiler.data();
        assert_eq!(data.cpu.sample_count, 1);
        assert_eq!(data.memory.total_allocations, 1);
        assert_eq!(data.jit.functions_compiled, 1);
        assert_eq!(data.jit.cache_hits, 1);
    }

    #[test]
    fn render_profile_report_text_is_deterministic_and_no_color() {
        let mut profiler = Profiler::new(ProfileConfig::default());
        profiler.start();
        profiler.record_function("parse".to_string(), Duration::from_millis(7));
        profiler.record_allocation(2048, "parser.rs:10".to_string());
        profiler.record_jit_compile(Duration::from_millis(3));
        profiler.record_cache_hit();
        profiler.record_guard_success();
        profiler.stop();

        let output = render_profile_report_text(profiler.data());
        assert!(output.contains("=== Performance Profile Report ==="));
        assert!(output.contains("CPU Profile:"));
        assert!(output.contains("Memory Profile:"));
        assert!(output.contains("JIT Statistics:"));
        assert!(output.contains("parse"));
        assert!(!output.contains("\u{1b}["), "text render should not include ANSI escapes");
    }

    #[test]
    fn generate_flamegraph_data_is_stable_for_known_profile() {
        let mut profile = CPUProfile::new();
        profile.record_function("func_a".to_string(), Duration::from_micros(250));
        profile.record_function("func_b".to_string(), Duration::from_micros(500));

        let output = generate_flamegraph_data(&profile);
        assert!(output.contains("func_a 250"));
        assert!(output.contains("func_b 500"));
        assert_eq!(output.lines().count(), 2);
    }
}
