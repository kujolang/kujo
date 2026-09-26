use kujo::interpreter::promise::PromiseReceiver;
use kujo::interpreter::AsyncRuntime;
use kujo::interpreter::{Interpreter, Value};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn pending() -> (tokio::sync::oneshot::Sender<Result<Value, String>>, Value) {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    (
        sender,
        Value::Promise {
            receiver: Arc::new(Mutex::new(receiver.into())),
            is_polled: Arc::new(Mutex::new(false)),
            cached_result: Arc::new(Mutex::new(None)),
            task_handle: None,
        },
    )
}

fn waiter(value: &Value) -> PromiseReceiver {
    match value {
        Value::Promise { receiver, .. } => receiver.lock().unwrap().clone(),
        other => panic!("expected promise, got {other:?}"),
    }
}

#[test]
fn pending_promise_wakes_every_waiter_and_keeps_completion() {
    AsyncRuntime::block_on(async {
        let (sender, promise) = pending();
        let mut first = Box::pin(waiter(&promise));
        let mut second = Box::pin(waiter(&promise));
        assert!(futures::poll!(&mut first).is_pending());
        assert!(futures::poll!(&mut second).is_pending());
        sender.send(Ok(Value::Int(42))).unwrap();
        let (first, second) =
            tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(first, second) })
                .await
                .expect("all promise waiters must be notified");
        assert!(matches!(first, Ok(Ok(Value::Int(42)))));
        assert!(matches!(second, Ok(Ok(Value::Int(42)))));
        assert!(matches!(waiter(&promise).await, Ok(Ok(Value::Int(42)))));
    });
}

#[test]
fn dropping_a_pending_waiter_preserves_the_producer_and_other_waiters() {
    AsyncRuntime::block_on(async {
        let (sender, promise) = pending();
        let mut abandoned = Box::pin(waiter(&promise));
        assert!(futures::poll!(&mut abandoned).is_pending());
        drop(abandoned);
        sender.send(Ok(Value::Int(9))).unwrap();
        assert!(matches!(waiter(&promise).await, Ok(Ok(Value::Int(9)))));
    });
}

#[test]
fn producer_drop_and_rejection_remain_repeatable() {
    AsyncRuntime::block_on(async {
        let (sender, promise) = pending();
        drop(sender);
        assert!(waiter(&promise).await.is_err());
        assert!(waiter(&promise).await.is_err());
        let (sender, promise) = pending();
        sender.send(Err("failed".into())).unwrap();
        for _ in 0..2 {
            assert!(matches!(waiter(&promise).await, Ok(Err(error)) if error == "failed"));
        }
    });
}

#[test]
fn cooperative_polling_and_async_waiting_observe_the_same_result() {
    AsyncRuntime::block_on(async {
        let (sender, promise) = pending();
        let mut cooperative = waiter(&promise);
        assert!(matches!(
            cooperative.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        let mut asynchronous = Box::pin(waiter(&promise));
        assert!(futures::poll!(&mut asynchronous).is_pending());
        sender.send(Ok(Value::Int(7))).unwrap();
        assert!(matches!(cooperative.try_recv(), Ok(Ok(Value::Int(7)))));
        assert!(matches!(asynchronous.await, Ok(Ok(Value::Int(7)))));
        assert!(matches!(cooperative.try_recv(), Ok(Ok(Value::Int(7)))));
    });
}

#[test]
fn timeout_does_not_consume_original_promise() {
    let mut interpreter = Interpreter::new();
    let (sender, promise) = pending();
    let timed =
        interpreter.call_native_function_impl("async_timeout", &[promise.clone(), Value::Int(1)]);
    let outcome = AsyncRuntime::block_on(async {
        tokio::time::timeout(Duration::from_secs(5), waiter(&timed)).await.unwrap()
    });
    assert!(matches!(outcome, Ok(Err(error)) if error.contains("Timeout")));
    sender.send(Ok(Value::Int(12))).unwrap();
    assert!(matches!(AsyncRuntime::block_on(waiter(&promise)), Ok(Ok(Value::Int(12)))));
}

#[test]
fn promise_all_accepts_duplicate_pending_promise_handles() {
    let mut interpreter = Interpreter::new();
    let (sender, promise) = pending();
    let joined = interpreter.call_native_function_impl(
        "promise_all",
        &[Value::Array(Arc::new(vec![promise.clone(), promise.clone()]))],
    );
    sender.send(Ok(Value::Int(5))).unwrap();
    let outcome = AsyncRuntime::block_on(async {
        tokio::time::timeout(Duration::from_secs(5), waiter(&joined)).await.unwrap()
    });
    assert!(
        matches!(outcome, Ok(Ok(Value::Array(values))) if matches!(values.as_slice(), [Value::Int(5), Value::Int(5)]))
    );
}
