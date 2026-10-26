//! Host-visible module owners keep real values and remain accessible during callbacks.
use kujo::{
    compiler::Compiler,
    interpreter::{Environment, Interpreter, Value},
    lexer::tokenize,
    module::ModuleLoader,
    parser::Parser,
    vm::VM,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn verify(runtime: &str) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("owner_state.kujo"),
        r#"
        mut state := [0]
        let entered := channel()
        let resume := channel()
        export func edit() {
            state[0] = 9
            entered.send(1)
            resume.receive()
            return state[0]
        }
        "#,
    )
    .unwrap();
    let mut loader = ModuleLoader::new();
    loader.add_search_path(directory.path());
    let module = loader.load_module("owner_state").unwrap();
    let callable = module.exports["edit"].clone();
    let Value::Function(_, _, Some(owner)) = &callable else {
        panic!("expected captured AST export");
    };
    let owner = owner.clone();
    let (entered, resume) = {
        let environment = owner.lock().unwrap();
        let Some(Value::Channel(entered)) = environment.get("entered") else {
            panic!("missing entry channel");
        };
        let Some(Value::Channel(resume)) = environment.get("resume") else {
            panic!("missing resume channel");
        };
        (entered, resume)
    };
    let inspected = owner.clone();
    let observer = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            match entered.lock().unwrap().1.try_recv() {
                Ok(_) => break,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(error) => return Err(format!("entry channel: {error}")),
            }
            if Instant::now() >= deadline {
                return Err("entry callback did not run".to_owned());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        // Never wait for the owner lock: a regression holding it across the
        // callback must fail this test without deadlocking the resume channel.
        let observed = match inspected.try_lock() {
            Ok(environment) => match environment.scopes[0].get("state") {
                Some(Value::Array(values)) => match values.first() {
                    Some(Value::Int(value)) => Ok(*value),
                    _ => Err("module owner array lost its integer value".to_owned()),
                },
                _ => Err("module owner exposed a marker instead of its array".to_owned()),
            },
            Err(_) => Err("module owner lock held across user callback".to_owned()),
        };
        let sender = resume.lock().unwrap().0.clone();
        loop {
            match sender.try_send(Value::Int(1)) {
                Ok(()) => break,
                Err(std::sync::mpsc::TrySendError::Full(_)) => {}
                Err(error) => return Err(format!("resume channel: {error}")),
            }
            if Instant::now() >= deadline {
                return Err("resume callback did not run".to_owned());
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        observed
    });
    let program =
        Parser::new(tokenize("let result := edit() assert_equal(result, 9)").unwrap()).parse();
    let execution = if runtime == "interpreter" {
        let mut interpreter = Interpreter::new();
        interpreter.env.define("edit".into(), callable);
        interpreter.eval_stmts(&program);
        match interpreter.return_value {
            None => Ok(()),
            other => Err(format!("interpreter: {other:?}")),
        }
    } else {
        let mut environment = Environment::new();
        for (name, value) in kujo::builtins::get_builtins() {
            environment.set(name, value);
        }
        environment.define("edit".into(), callable);
        let mut vm = VM::new();
        vm.set_globals(Arc::new(Mutex::new(environment)));
        vm.set_jit_enabled(runtime == "jit");
        let chunk = Compiler::new().compile(&program).unwrap();
        vm.execute_until_suspend(chunk)
            .and_then(|_| vm.run_scheduler_until_complete_with_timeout(Duration::from_secs(60)))
    };
    let observed = observer.join().unwrap();
    assert_eq!(observed, Ok(0), "{runtime}: pre-return public owner changed");
    execution.unwrap();
    let environment = owner.lock().unwrap();
    assert!(matches!(
        environment.scopes[0].get("state"),
        Some(Value::Array(values)) if matches!(values.first(), Some(Value::Int(9)))
    ));
}

#[test]
fn vm_module_owner_remains_readable_during_callback() {
    verify("vm");
}

#[test]
fn interpreter_module_owner_remains_readable_during_callback() {
    verify("interpreter");
}

#[test]
fn jit_module_owner_remains_readable_during_callback() {
    verify("jit");
}
