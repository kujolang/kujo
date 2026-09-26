use kujo::compiler::Compiler;
use kujo::interpreter::{Interpreter, Value};
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use kujo::vm::VM;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[test]
fn full_channel_send_does_not_hold_the_receiver_lock() {
    for vm_runtime in [false, true] {
        let mut interpreter = Interpreter::new();
        let value = interpreter.call_native_function_impl("channel", &[]);
        let Value::Channel(channel) = &value else { panic!("expected channel") };
        let channel = channel.clone();
        {
            let guard = channel.lock().unwrap();
            while guard.0.try_send(Value::Int(1)).is_ok() {}
        }
        interpreter.env.define("channel_under_test".into(), value);
        let (completed_tx, completed_rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut parser = Parser::new(tokenize("channel_under_test.send(42)").unwrap());
            let parsed = parser.parse_with_diagnostics();
            assert!(parsed.diagnostics.is_empty());
            let result = if vm_runtime {
                let mut vm = VM::new();
                vm.set_globals(Arc::new(Mutex::new(interpreter.env)));
                vm.execute(Compiler::new().compile(&parsed.stmts).unwrap()).map(|_| ())
            } else {
                interpreter.eval_stmts(&parsed.stmts);
                match interpreter.return_value {
                    None => Ok(()),
                    error => Err(format!("{error:?}")),
                }
            };
            completed_tx.send(result).unwrap();
        });
        let before_receive = completed_rx.recv_timeout(Duration::from_millis(100));
        assert!(
            matches!(before_receive, Err(std::sync::mpsc::RecvTimeoutError::Timeout)),
            "a full channel must apply backpressure (vm={vm_runtime}): {before_receive:?}"
        );
        // Repeatedly drain with try_lock, so a regression fails within a bounded
        // deadline rather than hanging the whole test process on the channel lock.
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(result) = completed_rx.try_recv() {
                result.unwrap();
                break;
            }
            assert!(Instant::now() < deadline, "sender retained the shared channel lock");
            if let Ok(guard) = channel.try_lock() {
                let _ = guard.1.try_recv();
            }
            std::thread::yield_now();
        }
        worker.join().unwrap();
    }
}
