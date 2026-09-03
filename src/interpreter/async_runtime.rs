// File: src/interpreter/async_runtime.rs
//
// Tokio async runtime wrapper for Kujo's async/await implementation.
// Provides a global, lazy-initialized tokio runtime for executing async tasks.
//
// This module wraps tokio's runtime to provide:
// - Task spawning (spawn_task)
// - Blocking execution of futures (block_on)
// - Async sleep (sleep)
// - Async timeout (timeout)
//
// The runtime is initialized once on first use and shared across the interpreter.

use once_cell::sync::Lazy;
use std::time::Duration;
use tokio::runtime::{Builder, Runtime, RuntimeFlavor};
use tokio::task::JoinHandle;

use crate::interpreter::Value;

/// Global tokio runtime instance, initialized lazily on first access
const KUJO_ASYNC_WORKER_STACK_BYTES: usize = 8 * 1024 * 1024;
const KUJO_ASYNC_WORKER_THREADS: usize = 1;

fn build_runtime() -> Runtime {
    Builder::new_multi_thread()
        .worker_threads(KUJO_ASYNC_WORKER_THREADS)
        .thread_stack_size(KUJO_ASYNC_WORKER_STACK_BYTES)
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime")
}

static RUNTIME: Lazy<Runtime> = Lazy::new(build_runtime);

/// Async runtime wrapper providing task execution capabilities
pub struct AsyncRuntime;

impl AsyncRuntime {
    /// Get reference to the global tokio runtime
    pub fn runtime() -> &'static Runtime {
        &RUNTIME
    }

    /// Spawn an async task that returns a Value
    ///
    /// The task runs on the tokio runtime thread pool and can be awaited
    /// from Kujo code using Promise/await syntax.
    ///
    /// # Arguments
    /// * `future` - The async computation to execute
    ///
    /// # Returns
    /// A JoinHandle that can be awaited to get the result
    pub fn spawn_task<F>(future: F) -> JoinHandle<Value>
    where
        F: std::future::Future<Output = Value> + Send + 'static,
    {
        Self::runtime().spawn(future)
    }

    /// Block the current thread until the future completes
    ///
    /// This is used by the `await` expression to synchronously wait for
    /// a promise to resolve. While this blocks the Kujo interpreter thread,
    /// the tokio runtime can still make progress on other tasks.
    ///
    /// # Arguments
    /// * `future` - The async computation to wait for
    ///
    /// # Returns
    /// The result of the future
    pub fn block_on<F>(future: F) -> F::Output
    where
        F: std::future::Future + Send,
        F::Output: Send,
    {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) if handle.runtime_flavor() == RuntimeFlavor::MultiThread => {
                // Kujo's CLI itself runs inside a Tokio multi-thread runtime. Calling
                // Runtime::block_on directly from that context panics with "Cannot
                // start a runtime from within a runtime". Move the synchronous
                // interpreter wait into Tokio's supported blocking lane while the
                // dedicated Kujo async runtime continues to drive the future.
                tokio::task::block_in_place(|| Self::runtime().block_on(future))
            }
            Ok(_) => {
                // block_in_place is unavailable on a current-thread runtime. A
                // scoped OS thread keeps the future borrowed only for this call and
                // avoids nesting either runtime on the caller thread.
                std::thread::scope(|scope| {
                    scope
                        .spawn(|| Self::runtime().block_on(future))
                        .join()
                        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
                })
            }
            Err(_) => Self::runtime().block_on(future),
        }
    }

    /// Create a future that completes after a duration
    ///
    /// Used by async_sleep() native function for non-blocking delays.
    ///
    /// # Arguments
    /// * `duration` - How long to sleep
    ///
    /// # Returns
    /// A future that completes after the duration
    pub async fn sleep(duration: Duration) {
        tokio::time::sleep(duration).await
    }

    /// Create a future that times out after a duration
    ///
    /// Used by async_timeout() native function to race a promise against
    /// a deadline.
    ///
    /// # Arguments
    /// * `future` - The async computation to timeout
    /// * `duration` - Maximum time to wait
    ///
    /// # Returns
    /// Ok(result) if completed in time, Err if timeout
    pub async fn timeout<F>(
        duration: Duration,
        future: F,
    ) -> Result<F::Output, tokio::time::error::Elapsed>
    where
        F: std::future::Future,
    {
        tokio::time::timeout(duration, future).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_runtime_initialization() {
        // Runtime should initialize successfully
        let _runtime = AsyncRuntime::runtime();
    }

    #[test]
    fn async_worker_stack_budget_is_bounded_and_nontrivial() {
        assert_eq!(KUJO_ASYNC_WORKER_STACK_BYTES, 8 * 1024 * 1024);
        assert_eq!(KUJO_ASYNC_WORKER_THREADS, 1);
        let runtime = build_runtime();
        let result = runtime.block_on(async {
            AsyncRuntime::spawn_task(async { Value::str("stack-ready".to_string()) })
                .await
                .expect("task should complete")
        });
        match result {
            Value::Str(value) => assert_eq!(value.as_str(), "stack-ready"),
            _ => panic!("unexpected async result type"),
        }
    }

    #[test]
    fn test_block_on_simple() {
        // block_on should execute future synchronously
        let result = AsyncRuntime::block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_block_on_inside_multithread_runtime_does_not_nest_runtime() {
        let outer = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("outer runtime should initialize");

        let result = outer.block_on(async { AsyncRuntime::block_on(async { 42 }) });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_block_on_inside_current_thread_runtime_uses_scoped_thread() {
        let outer = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("outer runtime should initialize");

        let result = outer.block_on(async { AsyncRuntime::block_on(async { 42 }) });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_sleep() {
        // Sleep should delay for at least the specified duration
        let start = Instant::now();
        AsyncRuntime::block_on(async {
            AsyncRuntime::sleep(Duration::from_millis(50)).await;
        });
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(50));
    }

    #[test]
    fn test_timeout_success() {
        // Timeout should return Ok if future completes in time
        let result = AsyncRuntime::block_on(async {
            AsyncRuntime::timeout(Duration::from_millis(100), async { 42 }).await
        });
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_timeout_expired() {
        // Timeout should return Err if future takes too long
        let result = AsyncRuntime::block_on(async {
            AsyncRuntime::timeout(Duration::from_millis(10), async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                42
            })
            .await
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_spawn_task() {
        // Spawned task should execute and return Value
        let handle = AsyncRuntime::spawn_task(async { Value::Int(42) });

        let result = AsyncRuntime::block_on(handle);
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Int(42) => {}
            _ => panic!("Expected Int(42)"),
        }
    }

    #[test]
    fn test_concurrent_tasks() {
        // Verify concurrency using relative timing to avoid brittle machine-dependent thresholds.
        let concurrent_elapsed = AsyncRuntime::block_on(async {
            let concurrent_start = Instant::now();

            let handle1 = AsyncRuntime::spawn_task(async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Value::Int(1)
            });

            let handle2 = AsyncRuntime::spawn_task(async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Value::Int(2)
            });

            let handle3 = AsyncRuntime::spawn_task(async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Value::Int(3)
            });

            let (result1, result2, result3) = tokio::join!(handle1, handle2, handle3);

            assert!(result1.is_ok());
            assert!(result2.is_ok());
            assert!(result3.is_ok());

            assert!(matches!(result1.unwrap(), Value::Int(1)));
            assert!(matches!(result2.unwrap(), Value::Int(2)));
            assert!(matches!(result3.unwrap(), Value::Int(3)));

            concurrent_start.elapsed()
        });

        let sequential_elapsed = AsyncRuntime::block_on(async {
            let sequential_start = Instant::now();

            tokio::time::sleep(Duration::from_millis(50)).await;
            tokio::time::sleep(Duration::from_millis(50)).await;
            tokio::time::sleep(Duration::from_millis(50)).await;

            sequential_start.elapsed()
        });

        let concurrent_ms = concurrent_elapsed.as_millis();
        let sequential_ms = sequential_elapsed.as_millis();

        // Concurrent execution should beat sequential execution with a small buffer for jitter.
        assert!(
            concurrent_ms + 10 < sequential_ms,
            "expected concurrent runtime to beat sequential runtime by at least 10ms (concurrent={}ms, sequential={}ms)",
            concurrent_ms,
            sequential_ms
        );
    }

    #[test]
    fn test_concurrent_tasks_under_timeout_budget() {
        // Keep a generous timeout budget to ensure tasks complete even on busy CI machines.
        let result = AsyncRuntime::block_on(async {
            AsyncRuntime::timeout(Duration::from_secs(2), async {
                let handle1 = AsyncRuntime::spawn_task(async {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Value::Int(10)
                });

                let handle2 = AsyncRuntime::spawn_task(async {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Value::Int(20)
                });

                let handle3 = AsyncRuntime::spawn_task(async {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Value::Int(30)
                });

                let (result1, result2, result3) = tokio::join!(handle1, handle2, handle3);

                vec![result1.unwrap(), result2.unwrap(), result3.unwrap()]
            })
            .await
        });

        assert!(result.is_ok(), "concurrent tasks exceeded timeout budget");

        let values = result.unwrap();
        assert_eq!(values.len(), 3);
        assert!(matches!(values[0], Value::Int(10)));
        assert!(matches!(values[1], Value::Int(20)));
        assert!(matches!(values[2], Value::Int(30)));
    }
}
