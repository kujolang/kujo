//! Bounded language-task admission on the existing Tokio blocking lane.
use super::{AsyncRuntime, Environment, Interpreter, RuntimeCapabilityPolicy, Value};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub const MAX_LANGUAGE_TASKS: usize = 16;
static ACTIVE_TASKS: AtomicUsize = AtomicUsize::new(0);
struct Admission;
impl Drop for Admission {
    fn drop(&mut self) {
        ACTIVE_TASKS.fetch_sub(1, Ordering::AcqRel);
    }
}

pub struct TaskState {
    pub(crate) completion: Value,
    pub(crate) cancelled: Arc<AtomicBool>,
    sender: Mutex<Option<oneshot::Sender<Result<Value, String>>>>,
    abort: Mutex<Option<tokio::task::AbortHandle>>,
}
impl TaskState {
    fn complete(&self, result: Result<Value, String>) {
        if let Some(sender) = self.sender.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = sender.send(result);
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
    pub(crate) fn cancel(&self) -> bool {
        let mut sender = self.sender.lock().unwrap_or_else(|p| p.into_inner());
        let Some(sender) = sender.take() else {
            return false;
        };
        self.cancelled.store(true, Ordering::Release);
        let _ =
            sender.send(Err("Task was cancelled; external effects may have occurred".to_owned()));
        if let Some(abort) = self.abort.lock().unwrap_or_else(|p| p.into_inner()).take() {
            abort.abort();
        }
        true
    }
}

pub(crate) fn submit(
    function: Value,
    args: Vec<Value>,
    environment: Environment,
    policy: RuntimeCapabilityPolicy,
    output: Option<Arc<Mutex<Vec<u8>>>>,
) -> Result<Arc<TaskState>, String> {
    let params = match &function {
        Value::Function(params, ..) | Value::AsyncFunction(params, ..) => params,
        Value::BytecodeFunction { chunk, .. } if !chunk.is_generator => &chunk.params,
        _ => return Err("Task requires an ordinary or async function".to_owned()),
    };
    let name = match &function {
        Value::Function(_, body, _) | Value::AsyncFunction(_, body, _) => {
            body.lexical_name.as_deref()
        }
        Value::BytecodeFunction { chunk, .. } => chunk.name.as_deref(),
        _ => None,
    }
    .unwrap_or("task function");
    super::CallableArity::exact(name, params.clone()).validate(args.len())?;
    ACTIVE_TASKS
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
            (active < MAX_LANGUAGE_TASKS).then_some(active + 1)
        })
        .map_err(|_| format!("Language task limit of {MAX_LANGUAGE_TASKS} reached"))?;
    let admission = Admission;
    let (sender, receiver) = oneshot::channel();
    let state = Arc::new(TaskState {
        completion: Value::Promise {
            receiver: Arc::new(Mutex::new(receiver.into())),
            is_polled: Arc::new(Mutex::new(false)),
            cached_result: Arc::new(Mutex::new(None)),
            task_handle: None,
        },
        cancelled: Arc::new(AtomicBool::new(false)),
        sender: Mutex::new(Some(sender)),
        abort: Mutex::new(None),
    });
    let worker_state = state.clone();
    let worker = AsyncRuntime::runtime().spawn_blocking(move || {
        let _admission = admission;
        if worker_state.is_cancelled() {
            return;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if matches!(function, Value::BytecodeFunction { .. }) {
                crate::vm::VM::execute_language_task(
                    function,
                    args,
                    environment,
                    policy,
                    output,
                    worker_state.cancelled.clone(),
                )
            } else {
                let mut interpreter = Interpreter::with_environment(policy, environment);
                // Imported AST functions retain module lexical bindings, while
                // bytecode callbacks still resolve the submitting VM's globals.
                interpreter.vm_globals = Some(Arc::new(Mutex::new(interpreter.env.clone())));
                interpreter.output = output;
                interpreter.task_cancellation = Some(worker_state.cancelled.clone());
                let value = interpreter.execute_task_body(&function, &args);
                match value {
                    Value::Error(message) | Value::ErrorObject { message, .. } => Err(message),
                    value => Ok(value),
                }
            }
        }))
        .unwrap_or_else(|_| Err("Language task panicked".to_owned()));
        let result = result.and_then(|value| match value {
            Value::Error(message) | Value::ErrorObject { message, .. } => Err(message),
            value => Ok(value),
        });
        worker_state.complete(result);
    });
    *state.abort.lock().unwrap_or_else(|p| p.into_inner()) = Some(worker.abort_handle());
    // Completion travels through the promise. Dropping this join handle detaches
    // only the executor handle, without consuming the task result.
    drop(worker);
    Ok(state)
}

impl Interpreter {
    pub(crate) fn task_is_cancelled(&self) -> bool {
        self.task_cancellation.as_ref().is_some_and(|flag| flag.load(Ordering::Acquire))
    }

    pub(crate) fn submit_language_task(
        &mut self,
        function: Value,
        args: Vec<Value>,
    ) -> Result<Arc<TaskState>, String> {
        let environment = match &self.vm_globals {
            Some(globals) => {
                globals.lock().map_err(|_| "Task globals lock poisoned")?.global_snapshot()
            }
            None => self.env.global_snapshot(),
        };
        submit(function, args, environment, self.capability_policy.clone(), self.output.clone())
    }

    fn execute_task_body(&mut self, function: &Value, args: &[Value]) -> Value {
        let Value::AsyncFunction(params, body, captured) = function else {
            return self.call_user_function(function, args);
        };
        if let Some(captured) = captured {
            self.env = captured.lock().unwrap_or_else(|p| p.into_inner()).clone();
        }
        if captured.is_some() {
            self.env.track_capture_writes();
        }
        self.env.push_scope();
        if let Some(name) = &body.lexical_name {
            self.env.define(name.clone(), function.clone());
        }
        for (name, value) in params.iter().zip(args) {
            self.env.define(name.clone(), value.clone());
        }
        let outcome = self.with_function_context("<async function>", |interpreter| {
            interpreter.eval_stmts(&body.get())
        });
        let value = match outcome {
            Err(error) => error,
            Ok(()) => match self.return_value.take() {
                Some(Value::Return(value)) => *value,
                Some(error) => error,
                None => Value::Null,
            },
        };
        self.env.pop_scope();
        if let Some(captured) = captured {
            captured.lock().unwrap_or_else(|p| p.into_inner()).merge_capture_writes(&self.env);
        }
        value
    }
}

static DETACHED_FAILURES: AtomicUsize = AtomicUsize::new(0);
const MAX_DETACHED_REPORTS: usize = 16;
fn observe_detached(task: Arc<TaskState>) {
    AsyncRuntime::spawn_task(async move {
        let receiver = match &task.completion {
            Value::Promise { receiver, .. } => {
                receiver.lock().unwrap_or_else(|p| p.into_inner()).clone()
            }
            _ => return Value::Null,
        };
        let error = match receiver.await {
            Ok(Ok(_)) => return Value::Null,
            Ok(Err(error)) => error,
            Err(_) => "Detached task producer disappeared".to_owned(),
        };
        let report = DETACHED_FAILURES.fetch_add(1, Ordering::Relaxed);
        if report < MAX_DETACHED_REPORTS {
            let message: String =
                error.chars().take(512).map(|c| if c.is_control() { ' ' } else { c }).collect();
            eprintln!("Kujo detached task failed: {message}");
        } else if report == MAX_DETACHED_REPORTS {
            eprintln!(
                "Kujo detached task failure reporting limit reached; further failures suppressed"
            );
        }
        Value::Null
    });
}

fn transfer(name: &str, value: &Value) -> Result<Value, String> {
    super::SpawnCapturedValue::from_value(value).map(super::SpawnCapturedValue::into_value)
        .ok_or_else(|| format!("Spawn capture '{name}' is not transferable; channels, closures, promises, generators and host handles require an explicit sharing contract"))
}

impl Interpreter {
    pub(super) fn spawn_detached_body(&mut self, body: &[crate::ast::Stmt]) -> Result<(), String> {
        let visible = self.env.visible_bindings();
        let names = crate::compiler::Compiler::spawn_free_bindings(body, visible.keys().cloned())?;
        let mut environment =
            Interpreter::with_capability_policy(self.capability_policy.clone()).env;
        for name in names {
            let (value, kind) = &visible[&name];
            environment.define_with_kind(name.clone(), transfer(&name, value)?, *kind);
        }
        let function =
            Value::Function(Vec::new(), super::LeakyFunctionBody::new(body.to_vec()), None);
        observe_detached(submit(
            function,
            Vec::new(),
            environment,
            self.capability_policy.clone(),
            self.output.clone(),
        )?);
        Ok(())
    }
}

pub(crate) fn submit_detached_vm(
    function: Value,
    globals: &Environment,
    policy: RuntimeCapabilityPolicy,
    output: Option<Arc<Mutex<Vec<u8>>>>,
) -> Result<(), String> {
    use crate::bytecode::{Constant, OpCode};
    let Value::BytecodeFunction { chunk, captured, captured_binding_kinds } = function else {
        return Err("Spawn requires a bytecode callable".to_owned());
    };
    let mut transferred = std::collections::HashMap::new();
    for (name, cell) in captured {
        let value = cell.lock().map_err(|_| "Spawn capture lock poisoned")?;
        transferred.insert(name.clone(), Arc::new(Mutex::new(transfer(&name, &value)?)));
    }
    let visible = globals.visible_bindings();
    let mut environment = Interpreter::with_capability_policy(policy.clone()).env;
    let mut chunks = vec![&chunk];
    while let Some(chunk) = chunks.pop() {
        for instruction in &chunk.instructions {
            let name = match instruction {
                OpCode::LoadVar(name)
                | OpCode::LoadGlobal(name)
                | OpCode::StoreVar(name)
                | OpCode::StoreGlobal(name)
                | OpCode::EnsureMutableGlobalForMutation(name) => name,
                _ => continue,
            };
            if chunk.lexical_self.as_ref() == Some(name) {
                continue;
            }
            if let Some((value, kind)) = visible.get(name) {
                environment.define_with_kind(name.clone(), transfer(name, value)?, *kind);
            }
        }
        for constant in &chunk.constants {
            if let Constant::Function(child) = constant {
                chunks.push(child);
            }
        }
    }
    let function = Value::BytecodeFunction { chunk, captured: transferred, captured_binding_kinds };
    observe_detached(submit(function, Vec::new(), environment, policy, output)?);
    Ok(())
}
