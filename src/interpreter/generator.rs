//! Owned statement continuations for tree-walking generators.
use super::{Environment, Interpreter, LeakyFunctionBody, RuntimeCapabilityPolicy, Value};
use crate::ast::{Expr, Stmt};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
enum Frame {
    Block { body: Arc<Vec<Stmt>>, pc: usize },
    ScopeEnd,
    Loop { condition: Option<Expr>, body: Arc<Vec<Stmt>> },
    For { var: String, source: Value, index: usize, body: Arc<Vec<Stmt>> },
    Handler { name: String, body: Arc<Vec<Stmt>> },
}

struct Continuation {
    env: Environment,
    frames: Vec<Frame>,
}

pub struct InterpreterGeneratorState {
    continuation: Option<Continuation>,
    failure: Option<Value>,
    pub(crate) exhausted: bool,
    running: bool,
    policy: RuntimeCapabilityPolicy,
    owner: Arc<()>,
}

impl Interpreter {
    pub(super) fn create_generator(
        &self,
        params: &[String],
        body: &LeakyFunctionBody,
        captured: &Option<Arc<Mutex<Environment>>>,
        args: &[Value],
    ) -> Value {
        let mut env = captured.as_ref().map_or_else(
            || self.env.generator_environment(false),
            |env| env.lock().unwrap_or_else(|p| p.into_inner()).generator_environment(true),
        );
        env.push_scope();
        if let Some(name) = &body.lexical_name {
            env.define(
                name.clone(),
                Value::GeneratorDef(params.to_vec(), body.clone(), captured.clone()),
            );
        }
        for (name, value) in params.iter().zip(args) {
            env.define(name.clone(), value.clone());
        }
        let continuation = Continuation {
            env,
            frames: vec![Frame::Block {
                body: Arc::new(super::generator_lowering::lower(&body.get())),
                pc: 0,
            }],
        };
        Value::Generator {
            params: params.to_vec(),
            state: Arc::new(Mutex::new(InterpreterGeneratorState {
                continuation: Some(continuation),
                failure: None,
                exhausted: false,
                running: false,
                policy: self.capability_policy.clone(),
                owner: self.env.global_owner(),
            })),
        }
    }

    pub(super) fn resume_generator(&mut self, generator: &Value) -> Value {
        let Value::Generator { state, .. } = generator else {
            return Value::Error("generator_next() can only be called on generators".to_owned());
        };
        let (mut continuation, policy) = {
            let Ok(mut state) = state.try_lock() else {
                return Value::Error("Generator is already being resumed".to_owned());
            };
            if state.running {
                return Value::Error("Generator is already being resumed".to_owned());
            }
            if let Some(error) = &state.failure {
                return error.clone();
            }
            if state.exhausted {
                return Value::Option { is_some: false, value: Box::new(Value::Null) };
            }
            if !Arc::ptr_eq(&state.owner, &self.env.global_owner()) {
                return Value::Error(
                    "Generator belongs to a different runtime environment".to_owned(),
                );
            }
            let Some(continuation) = state.continuation.take() else {
                return Value::Error("Generator continuation is missing".to_owned());
            };
            state.running = true;
            (continuation, state.policy.intersection(&self.capability_policy))
        };
        let mut caller_env = std::mem::replace(&mut self.env, continuation.env);
        self.env.swap_global_scope(&mut caller_env);
        let caller_return = self.return_value.take();
        let caller_control = std::mem::replace(&mut self.control_flow, super::ControlFlow::None);
        let caller_policy = std::mem::replace(&mut self.capability_policy, policy);
        let result = self
            .with_function_context("<generator>", |interpreter| {
                interpreter.run_generator_frames(&mut continuation.frames)
            })
            .unwrap_or_else(Err);
        self.env.swap_global_scope(&mut caller_env);
        continuation.env = std::mem::replace(&mut self.env, caller_env);
        self.return_value = caller_return;
        self.control_flow = caller_control;
        self.capability_policy = caller_policy;
        let mut state = state.lock().unwrap_or_else(|p| p.into_inner());
        state.running = false;
        match result {
            Ok(Some(value)) => {
                state.continuation = Some(continuation);
                Value::Option { is_some: true, value: Box::new(value) }
            }
            Ok(None) => {
                state.exhausted = true;
                Value::Option { is_some: false, value: Box::new(Value::Null) }
            }
            Err(error) => {
                state.exhausted = true;
                state.failure = Some(error.clone());
                error
            }
        }
    }

    fn generator_expression(&mut self, expr: &Expr) -> Result<Value, Value> {
        let value = self.eval_expr(expr);
        if Self::is_error_value(&value) {
            Err(value)
        } else {
            Ok(value)
        }
    }

    fn generator_block(&mut self, frames: &mut Vec<Frame>, body: Arc<Vec<Stmt>>, scoped: bool) {
        if scoped {
            self.env.push_scope();
            frames.push(Frame::ScopeEnd);
        }
        frames.push(Frame::Block { body, pc: 0 });
    }

    fn unwind_generator_loop(
        &mut self,
        frames: &mut Vec<Frame>,
        continuing: bool,
    ) -> Result<(), Value> {
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::ScopeEnd => self.env.pop_scope(),
                frame @ (Frame::Loop { .. } | Frame::For { .. }) => {
                    if continuing {
                        frames.push(frame);
                    }
                    return Ok(());
                }
                _ => {}
            }
        }
        Err(Value::Error("loop control can only be used inside a loop".to_owned()))
    }

    fn handle_generator_error(
        &mut self,
        frames: &mut Vec<Frame>,
        error: Value,
    ) -> Result<(), Value> {
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::ScopeEnd => self.env.pop_scope(),
                Frame::Handler { name, body } => {
                    self.return_value = None;
                    self.env.push_scope();
                    self.env.define(name, Self::generator_error_object(error));
                    frames.push(Frame::ScopeEnd);
                    frames.push(Frame::Block { body, pc: 0 });
                    return Ok(());
                }
                _ => {}
            }
        }
        Err(error)
    }

    fn generator_error_object(error: Value) -> Value {
        let (message, stack, line, cause) = match error {
            Value::Error(message) => (message, Vec::new(), None, None),
            Value::ErrorObject { message, stack, line, cause } => (message, stack, line, cause),
            other => return other,
        };
        let mut fields = std::collections::HashMap::new();
        fields.insert("message".to_owned(), Value::Str(Arc::new(message)));
        fields.insert(
            "stack".to_owned(),
            Value::Array(Arc::new(stack.into_iter().map(|s| Value::Str(Arc::new(s))).collect())),
        );
        fields.insert("line".to_owned(), Value::Int(line.unwrap_or(0) as i64));
        if let Some(cause) = cause {
            fields.insert("cause".to_owned(), *cause);
        }
        Value::Struct { name: "Error".to_owned(), fields }
    }

    fn run_generator_frames(&mut self, frames: &mut Vec<Frame>) -> Result<Option<Value>, Value> {
        while let Some(frame) = frames.pop() {
            if self.task_is_cancelled() {
                return Err(Value::Error("Task was cancelled".to_owned()));
            }
            let step = (|| -> Result<Option<Option<Value>>, Value> {
                match frame {
                    Frame::ScopeEnd => self.env.pop_scope(),
                    Frame::Handler { .. } => {}
                    Frame::Loop { condition, body } => {
                        if let Some(condition) = &condition {
                            if !self.generator_expression(condition)?.is_truthy() {
                                return Ok(None);
                            }
                        }
                        frames.push(Frame::Loop { condition, body: body.clone() });
                        self.generator_block(frames, body, true);
                    }
                    Frame::For { var, mut source, index, body } => {
                        let next = match &source {
                            Value::Generator { .. } => match self.generator_next(&mut source) {
                                Value::Option { is_some: true, value } => Some(*value),
                                Value::Option { is_some: false, .. } => None,
                                error => return Err(error),
                            },
                            Value::Array(values) => values.get(index).cloned(),
                            _ => return Err(Value::Error("Invalid generator iterable".to_owned())),
                        };
                        if let Some(value) = next {
                            frames.push(Frame::For {
                                var: var.clone(),
                                source,
                                index: index + 1,
                                body: body.clone(),
                            });
                            self.env.push_scope();
                            self.env.define(var, value);
                            frames.push(Frame::ScopeEnd);
                            frames.push(Frame::Block { body, pc: 0 });
                        }
                    }
                    Frame::Block { body, pc } => {
                        let Some(stmt) = body.get(pc).cloned() else {
                            return Ok(None);
                        };
                        frames.push(Frame::Block { body, pc: pc + 1 });
                        match stmt {
                            Stmt::ExprStmt(Expr::Yield(expr)) => {
                                let value = match expr {
                                    Some(expr) => self.generator_expression(&expr)?,
                                    None => Value::Null,
                                };
                                return Ok(Some(Some(value)));
                            }
                            Stmt::Return(expr) => {
                                if let Some(expr) = expr {
                                    self.generator_expression(&expr)?;
                                }
                                return Ok(Some(None));
                            }
                            Stmt::Block(body) => {
                                self.generator_block(frames, Arc::new(body), true);
                            }
                            Stmt::If { condition, then_branch, else_branch } => {
                                let body = if self.generator_expression(&condition)?.is_truthy() {
                                    then_branch
                                } else {
                                    else_branch.unwrap_or_default()
                                };
                                self.generator_block(frames, Arc::new(body), true);
                            }
                            Stmt::While { condition, body } => frames.push(Frame::Loop {
                                condition: Some(condition),
                                body: Arc::new(body),
                            }),
                            Stmt::Loop { condition, body } => {
                                frames.push(Frame::Loop { condition, body: Arc::new(body) })
                            }
                            Stmt::For { var, iterable, body } => {
                                let mut source = self.generator_expression(&iterable)?;
                                if !matches!(source, Value::Generator { .. }) {
                                    source = self
                                        .call_native_function_impl("__vm_for_iterable", &[source]);
                                    if Self::is_error_value(&source) {
                                        return Err(source);
                                    }
                                }
                                frames.push(Frame::For {
                                    var,
                                    source,
                                    index: 0,
                                    body: Arc::new(body),
                                });
                            }
                            Stmt::Break => self.unwind_generator_loop(frames, false)?,
                            Stmt::Continue => self.unwind_generator_loop(frames, true)?,
                            Stmt::TryExcept { try_block, except_var, except_block } => {
                                frames.push(Frame::Handler {
                                    name: except_var,
                                    body: Arc::new(except_block),
                                });
                                self.generator_block(frames, Arc::new(try_block), true);
                            }
                            Stmt::Match { value, cases, default } => {
                                let value = self.generator_expression(&value)?;
                                let selected = self.generator_match_body(&value, cases, default);
                                self.generator_block(frames, Arc::new(selected), false);
                            }
                            stmt => {
                                self.eval_stmt(&stmt);
                                if let Some(value) = self.return_value.take() {
                                    match value {
                                        Value::Return(_) => return Err(Value::Error(
                                            "yield in a generator must be a standalone statement"
                                                .to_owned(),
                                        )),
                                        error => return Err(error),
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(None)
            })();
            match step {
                Ok(Some(outcome)) => return Ok(outcome),
                Ok(None) => {}
                Err(error) => self.handle_generator_error(frames, error)?,
            }
        }
        Ok(None)
    }

    fn generator_match_body(
        &mut self,
        value: &Value,
        cases: Vec<(String, Vec<Stmt>)>,
        default: Option<Vec<Stmt>>,
    ) -> Vec<Stmt> {
        let (tag, full_tag, fields) = match value {
            Value::Result { is_ok, value } => {
                let tag = if *is_ok { "Ok" } else { "Err" };
                (
                    tag.to_owned(),
                    format!("Result::{tag}"),
                    vec![("$0".to_owned(), (**value).clone())],
                )
            }
            Value::Option { is_some, value } => {
                let tag = if *is_some { "Some" } else { "None" };
                let fields =
                    if *is_some { vec![("$0".to_owned(), (**value).clone())] } else { Vec::new() };
                (tag.to_owned(), format!("Option::{tag}"), fields)
            }
            Value::Tagged { tag, fields } => (
                tag.clone(),
                tag.clone(),
                fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            ),
            Value::Enum(tag) => (tag.clone(), tag.clone(), Vec::new()),
            Value::Str(tag) => ((**tag).clone(), (**tag).clone(), Vec::new()),
            Value::Float(n) => (n.to_string(), n.to_string(), Vec::new()),
            _ => return default.unwrap_or_default(),
        };
        for (pattern, body) in cases {
            let (candidate, binding) = match pattern.split_once('(') {
                Some((tag, binding)) => (tag.trim(), Some(binding.trim_end_matches(')'))),
                None => (pattern.trim(), None),
            };
            if candidate == tag || candidate == full_tag {
                if let Some(binding) = binding {
                    for index in 0..fields.len() {
                        if let Some((_, value)) =
                            fields.iter().find(|(key, _)| *key == format!("${index}"))
                        {
                            let name = if index == 0 {
                                binding.to_owned()
                            } else {
                                format!("{binding}_{index}")
                            };
                            self.env.define(name, value.clone());
                        }
                    }
                }
                return body;
            }
        }
        default.unwrap_or_default()
    }
}
