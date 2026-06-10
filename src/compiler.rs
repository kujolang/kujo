// File: src/compiler.rs
//
// Bytecode compiler for the Kujo programming language.
// Compiles AST nodes into bytecode instructions for the VM.

use crate::ast::{ArrayElement, DictElement, Expr, Pattern, Stmt};
use crate::bytecode::{BytecodeBindingKind, BytecodeChunk, Constant, OpCode};
use crate::errors::unsupported_struct_generator_method_message;
use crate::optimizer::Optimizer;
use std::collections::HashSet;
use std::sync::Arc;

/// Compiler state for generating bytecode from AST
#[allow(dead_code)] // Compiler not yet integrated into execution path
pub struct Compiler {
    /// Current bytecode chunk being compiled
    chunk: BytecodeChunk,

    /// Stack of loop start positions for break/continue
    loop_starts: Vec<usize>,

    /// Stack of loop end jump indices to patch for break statements
    loop_ends: Vec<Vec<usize>>,

    /// Current scope depth (0 = global)
    scope_depth: usize,

    /// Local variables in current scope
    locals: Vec<Local>,

    /// Next available local slot index
    next_local_slot: usize,

    /// Names of captured variables for this compiler
    upvalue_names: HashSet<String>,

    /// Variables that are read in this compiler scope
    used_locals: HashSet<String>,

    /// Stack of local counts for nested lexical scopes.
    scope_markers: Vec<usize>,

    /// Parent compiler (for nested functions/closures)
    parent: Option<Box<Compiler>>,

    /// Tracks whether the chunk uses logical short-circuit lowering patterns that
    /// current optimizer passes are not yet stack-shape aware for.
    has_logical_short_circuit: bool,

    /// Tracks whether the chunk uses exception-flow lowering patterns that current
    /// optimizer passes are not yet control-flow/stack-shape aware for.
    has_exception_flow: bool,

    /// Tracks whether the chunk uses method-call lowering patterns that current
    /// optimizer passes are not yet stack-shape aware for.
    has_method_call_flow: bool,

    /// Whether this compiler instance can use local slots (function/method/lambda bodies).
    /// The root script compiler keeps declarations in the runtime environment instead.
    uses_local_slots: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Helper struct for incomplete feature
struct Local {
    name: String,
    depth: usize,
    slot: usize,
    binding_kind: BytecodeBindingKind,
}

#[allow(dead_code)] // Compiler not yet integrated into execution path
impl Compiler {
    fn is_hoistable_top_level_function(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::FuncDef { .. } => true,
            Stmt::Export { stmt } => matches!(stmt.as_ref(), Stmt::FuncDef { .. }),
            _ => false,
        }
    }

    pub fn new() -> Self {
        Self {
            chunk: BytecodeChunk::new(),
            loop_starts: Vec::new(),
            loop_ends: Vec::new(),
            scope_depth: 0,
            locals: Vec::new(),
            next_local_slot: 0,
            upvalue_names: HashSet::new(),
            used_locals: HashSet::new(),
            scope_markers: Vec::new(),
            parent: None,
            has_logical_short_circuit: false,
            has_exception_flow: false,
            has_method_call_flow: false,
            uses_local_slots: false,
        }
    }

    /// Compile a list of statements into bytecode
    pub fn compile(&mut self, statements: &[Stmt]) -> Result<BytecodeChunk, String> {
        self.compile_with_optimization(statements, true)
    }

    /// Compile with optional optimization
    pub fn compile_with_optimization(
        &mut self,
        statements: &[Stmt],
        optimize: bool,
    ) -> Result<BytecodeChunk, String> {
        self.used_locals = Self::collect_used_variables(statements);

        // Hoist top-level function declarations so calls can appear earlier in the file.
        for stmt in statements {
            if Self::is_hoistable_top_level_function(stmt) {
                self.compile_stmt(stmt)?;
            }
        }

        for stmt in statements {
            if Self::is_hoistable_top_level_function(stmt) {
                continue;
            }
            self.compile_stmt(stmt)?;
        }

        // Ensure we return None at the end if no explicit return
        self.chunk.emit(OpCode::ReturnNone);

        // Record local slot count
        self.chunk.local_count = self.next_local_slot;

        // Apply optimizations if enabled
        let mut chunk = self.chunk.clone();
        let has_match_case_flow = chunk.instructions.iter().any(|instr| {
            matches!(instr, OpCode::MatchCasePattern(_) | OpCode::BeginCase | OpCode::EndCase)
        });

        if optimize
            && !has_match_case_flow
            && !self.has_logical_short_circuit
            && !self.has_exception_flow
            && !self.has_method_call_flow
        {
            let mut optimizer = Optimizer::new();
            optimizer.optimize(&mut chunk);

            // Log optimization stats in debug mode
            if cfg!(debug_assertions) {
                if optimizer.stats.constants_folded > 0
                    || optimizer.stats.dead_instructions_removed > 0
                    || optimizer.stats.peephole_optimizations > 0
                {
                    eprintln!("Compiler optimization: {} constants folded, {} dead instructions removed, {} peephole optimizations",
                        optimizer.stats.constants_folded,
                        optimizer.stats.dead_instructions_removed,
                        optimizer.stats.peephole_optimizations);
                }
            }
        }

        Ok(chunk)
    }

    fn add_local(&mut self, name: &str, depth: usize, binding_kind: BytecodeBindingKind) -> usize {
        let slot = self.next_local_slot;
        self.next_local_slot += 1;
        self.locals.push(Local { name: name.to_string(), depth, slot, binding_kind });

        if slot == self.chunk.local_names.len() {
            self.chunk.local_names.push(name.to_string());
        } else if slot < self.chunk.local_names.len() {
            self.chunk.local_names[slot] = name.to_string();
        }

        if slot == self.chunk.local_binding_kinds.len() {
            self.chunk.local_binding_kinds.push(binding_kind);
        } else if slot < self.chunk.local_binding_kinds.len() {
            self.chunk.local_binding_kinds[slot] = binding_kind;
        }

        slot
    }

    fn resolve_local_slot(&self, name: &str) -> Option<usize> {
        if !self.uses_local_slots {
            return None;
        }

        self.locals
            .iter()
            .rev()
            .find(|local| local.name == name && local.depth <= self.scope_depth)
            .map(|local| local.slot)
    }

    fn has_local_in_current_scope(&self, name: &str) -> bool {
        self.locals.iter().rev().any(|local| local.depth == self.scope_depth && local.name == name)
    }

    fn is_inside_loop(&self) -> bool {
        !self.loop_starts.is_empty()
    }

    fn declare_local(
        &mut self,
        name: &str,
        binding_kind: BytecodeBindingKind,
    ) -> Result<usize, String> {
        if self.has_local_in_current_scope(name) {
            return Err(format!("Duplicate declaration in the same scope: {}", name));
        }

        Ok(self.add_local(name, self.scope_depth, binding_kind))
    }

    fn enter_scope(&mut self) {
        self.scope_markers.push(self.locals.len());
        self.scope_depth += 1;
    }

    fn exit_scope(&mut self) {
        if let Some(marker) = self.scope_markers.pop() {
            self.locals.truncate(marker);
        }
        self.scope_depth = self.scope_depth.saturating_sub(1);
    }

    fn is_upvalue(&self, name: &str) -> bool {
        self.upvalue_names.contains(name)
    }

    /// Compile a single statement
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::ExprStmt(expr) => {
                self.compile_expr(expr)?;
                self.chunk.emit(OpCode::Pop); // Discard expression result
                Ok(())
            }

            Stmt::Let { pattern, value, mutable, .. } => {
                // Compile the value
                self.compile_expr(value)?;

                // Bind the pattern
                let binding_kind = if *mutable {
                    BytecodeBindingKind::Mutable
                } else {
                    BytecodeBindingKind::LetImmutable
                };
                self.compile_pattern_binding(pattern, binding_kind)?;

                // Pop the value from stack - StoreVar peeks, doesn't pop
                // This keeps the stack clean between statements
                self.chunk.emit(OpCode::Pop);

                Ok(())
            }

            Stmt::Assign { target, value } => {
                if let (Expr::Identifier(target_name), Expr::BinaryOp { left, op, right }) =
                    (target, value)
                {
                    if op == "+" {
                        if let (Expr::Identifier(left_name), Expr::String(_)) = (&**left, &**right)
                        {
                            if left_name == target_name {
                                if let Some(slot) = self.resolve_local_slot(target_name) {
                                    // Use dedicated in-place constant string append opcode
                                    if let Expr::String(s) = &**right {
                                        if let Some(ch) = {
                                            let mut chars = s.chars();
                                            match (chars.next(), chars.next()) {
                                                (Some(c), None) => Some(c),
                                                _ => None,
                                            }
                                        } {
                                            self.chunk
                                                .emit(OpCode::AppendConstCharInPlace(slot, ch));
                                        } else {
                                            self.chunk.emit(OpCode::AppendConstStringInPlace(
                                                slot,
                                                Arc::from(s.clone()),
                                            ));
                                        }
                                        return Ok(());
                                    }

                                    self.compile_expr(right)?;
                                    self.chunk.emit(OpCode::AddInPlace(slot));
                                    self.chunk.emit(OpCode::Pop);
                                    return Ok(());
                                }
                            }
                        }
                    }
                }

                // Compile the value
                self.compile_expr(value)?;

                // Compile the assignment target
                self.compile_assignment(target)?;

                // Pop the value from stack - StoreVar peeks, doesn't pop
                // This keeps the stack clean between statements
                self.chunk.emit(OpCode::Pop);

                Ok(())
            }

            Stmt::If { condition, then_branch, else_branch } => {
                // Compile condition
                self.compile_expr(condition)?;

                // Jump to else block if condition is false
                let else_jump = self.chunk.emit(OpCode::JumpIfFalse(0));
                self.chunk.emit(OpCode::Pop); // Pop condition

                // Compile then block
                self.chunk.emit(OpCode::PushScope);
                self.enter_scope();
                for stmt in then_branch {
                    self.compile_stmt(stmt)?;
                }
                self.exit_scope();
                self.chunk.emit(OpCode::PopScope);

                // Jump over else block
                let end_jump = self.chunk.emit(OpCode::Jump(0));

                // Patch else jump
                self.chunk.patch_jump(else_jump);
                self.chunk.emit(OpCode::Pop); // Pop condition

                // Compile else block if present
                if let Some(else_stmts) = else_branch {
                    self.chunk.emit(OpCode::PushScope);
                    self.enter_scope();
                    for stmt in else_stmts {
                        self.compile_stmt(stmt)?;
                    }
                    self.exit_scope();
                    self.chunk.emit(OpCode::PopScope);
                }

                // Patch end jump
                self.chunk.patch_jump(end_jump);

                Ok(())
            }

            Stmt::While { condition, body, .. } => {
                if let Some((target_slot, index_slot, limit_slot, append_char)) =
                    self.match_append_char_until_local_pattern(condition, body)
                {
                    self.chunk.emit(OpCode::AppendConstCharUntilLocalInPlace(
                        target_slot,
                        index_slot,
                        limit_slot,
                        append_char,
                    ));
                    return Ok(());
                }

                if let Some((map_slot, index_slot, limit_slot)) =
                    self.match_fill_int_map_with_double_until_local_pattern(condition, body)
                {
                    self.chunk.emit(OpCode::FillIntMapWithDoubleUntilLocalInPlace(
                        map_slot, index_slot, limit_slot,
                    ));
                    return Ok(());
                }

                if let Some((map_slot, sum_slot, index_slot, limit_slot)) =
                    self.match_sum_int_map_until_local_pattern(condition, body)
                {
                    self.chunk.emit(OpCode::SumIntMapUntilLocalInPlace(
                        map_slot, sum_slot, index_slot, limit_slot,
                    ));
                    return Ok(());
                }

                let loop_start = self.chunk.instructions.len();
                self.loop_starts.push(loop_start);
                self.loop_ends.push(Vec::new());

                // Compile condition
                self.compile_expr(condition)?;

                // Jump to end if condition is false
                let end_jump = self.chunk.emit(OpCode::JumpIfFalse(0));
                self.chunk.emit(OpCode::Pop); // Pop condition

                // Compile body
                self.enter_scope();
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }
                self.exit_scope();

                // Jump back to condition
                self.chunk.emit(OpCode::JumpBack(loop_start));

                // Patch end jump
                self.chunk.patch_jump(end_jump);
                self.chunk.emit(OpCode::Pop); // Pop condition

                // Patch all break statements
                if let Some(breaks) = self.loop_ends.pop() {
                    for break_jump in breaks {
                        self.chunk.patch_jump(break_jump);
                    }
                }
                self.loop_starts.pop();

                Ok(())
            }

            Stmt::For { var, iterable, body } => {
                // For now, compile as a while loop with an iterator
                // This is a simplified implementation
                self.enter_scope();

                let loop_var_slot = if self.uses_local_slots && !self.is_upvalue(var) {
                    Some(self.declare_local(var, BytecodeBindingKind::Mutable)?)
                } else {
                    None
                };

                let iter_var = format!("__iter_{}", self.scope_depth);
                let index_var = format!("__index_{}", self.scope_depth);
                let iter_slot = if self.uses_local_slots {
                    Some(self.declare_local(iter_var.as_str(), BytecodeBindingKind::Mutable)?)
                } else {
                    None
                };
                let index_slot = if self.uses_local_slots {
                    Some(self.declare_local(index_var.as_str(), BytecodeBindingKind::Mutable)?)
                } else {
                    None
                };

                // Evaluate the iterable
                self.compile_expr(iterable)?;
                // Normalize VM for-loop inputs so numeric/string/dict iterations align with
                // interpreter semantics without changing generic index operation rules.
                self.chunk.emit(OpCode::LoadGlobal("__vm_for_iterable".to_string()));
                self.chunk.emit(OpCode::Call(1));

                // Store in a temporary variable for iteration
                if let Some(slot) = iter_slot {
                    self.chunk.emit(OpCode::StoreLocal(slot));
                } else {
                    self.chunk.emit(OpCode::StoreVar(iter_var.clone()));
                }

                // Create index variable
                let zero_index = self.chunk.add_constant(Constant::Int(0));
                self.chunk.emit(OpCode::LoadConst(zero_index));
                if let Some(slot) = index_slot {
                    self.chunk.emit(OpCode::StoreLocal(slot));
                } else {
                    self.chunk.emit(OpCode::StoreVar(index_var.clone()));
                }

                let loop_start = self.chunk.instructions.len();
                self.loop_starts.push(loop_start);
                self.loop_ends.push(Vec::new());

                // Load iterator and index
                if let Some(slot) = iter_slot {
                    self.chunk.emit(OpCode::LoadLocal(slot));
                } else {
                    self.chunk.emit(OpCode::LoadVar(iter_var.clone()));
                }
                if let Some(slot) = index_slot {
                    self.chunk.emit(OpCode::LoadLocal(slot));
                } else {
                    self.chunk.emit(OpCode::LoadVar(index_var.clone()));
                }

                // Check if index < len(iterable) - using built-in len
                if let Some(slot) = iter_slot {
                    self.chunk.emit(OpCode::LoadLocal(slot));
                } else {
                    self.chunk.emit(OpCode::LoadVar(iter_var.clone()));
                }
                self.chunk.emit(OpCode::LoadGlobal("len".to_string()));
                self.chunk.emit(OpCode::Call(1));
                self.chunk.emit(OpCode::LessThan);

                // Jump to end if done
                let end_jump = self.chunk.emit(OpCode::JumpIfFalse(0));
                self.chunk.emit(OpCode::Pop);

                // Get current element: iterable[index]
                if let Some(slot) = iter_slot {
                    self.chunk.emit(OpCode::LoadLocal(slot));
                } else {
                    self.chunk.emit(OpCode::LoadVar(iter_var.clone()));
                }
                if let Some(slot) = index_slot {
                    self.chunk.emit(OpCode::LoadLocal(slot));
                } else {
                    self.chunk.emit(OpCode::LoadVar(index_var.clone()));
                }
                self.chunk.emit(OpCode::IndexGet);

                // Store in loop variable
                if let Some(slot) = loop_var_slot {
                    self.chunk.emit(OpCode::StoreLocal(slot));
                } else {
                    self.chunk.emit(OpCode::StoreVar(var.clone()));
                }

                // Compile body
                for stmt in body {
                    self.compile_stmt(stmt)?;
                }

                // Increment index
                if let Some(slot) = index_slot {
                    self.chunk.emit(OpCode::LoadLocal(slot));
                } else {
                    self.chunk.emit(OpCode::LoadVar(index_var.clone()));
                }
                let one_index = self.chunk.add_constant(Constant::Int(1));
                self.chunk.emit(OpCode::LoadConst(one_index));
                self.chunk.emit(OpCode::Add);
                if let Some(slot) = index_slot {
                    self.chunk.emit(OpCode::StoreLocal(slot));
                } else {
                    self.chunk.emit(OpCode::StoreVar(index_var.clone()));
                }

                // Jump back to start
                self.chunk.emit(OpCode::JumpBack(loop_start));

                // Patch end jump
                self.chunk.patch_jump(end_jump);
                self.chunk.emit(OpCode::Pop);

                // Patch all break statements
                if let Some(breaks) = self.loop_ends.pop() {
                    for break_jump in breaks {
                        self.chunk.patch_jump(break_jump);
                    }
                }
                self.loop_starts.pop();
                self.exit_scope();

                Ok(())
            }

            Stmt::Return(value) => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                    self.chunk.emit(OpCode::Return);
                } else {
                    self.chunk.emit(OpCode::ReturnNone);
                }
                Ok(())
            }

            Stmt::Break => {
                if !self.is_inside_loop() {
                    return Err("break can only be used inside a loop".to_string());
                }

                // Add a jump that will be patched later
                let jump_index = self.chunk.emit(OpCode::Jump(0));
                if let Some(breaks) = self.loop_ends.last_mut() {
                    breaks.push(jump_index);
                }
                Ok(())
            }

            Stmt::Continue => {
                if !self.is_inside_loop() {
                    return Err("continue can only be used inside a loop".to_string());
                }

                // Jump back to loop start
                if let Some(&loop_start) = self.loop_starts.last() {
                    self.chunk.emit(OpCode::JumpBack(loop_start));
                }
                Ok(())
            }

            Stmt::FuncDef { name, params, body, is_async, is_generator, .. } => {
                // Create a new compiler for the function body
                let mut func_compiler = Compiler::new();
                func_compiler.used_locals = Self::collect_used_variables(body);
                func_compiler.chunk.name = Some(name.clone());
                func_compiler.chunk.params = params.clone();
                func_compiler.chunk.is_async = *is_async;
                func_compiler.chunk.is_generator = *is_generator;
                func_compiler.scope_depth = 1; // Functions create a new scope (not global)
                func_compiler.uses_local_slots = true;

                // Add parameters as locals
                for param in params {
                    if func_compiler.has_local_in_current_scope(param) {
                        return Err(format!("Duplicate declaration in the same scope: {}", param));
                    }
                    func_compiler.add_local(param, 1, BytecodeBindingKind::Mutable);
                }

                // Analyze the function body to find free variables (captures)
                let free_vars = Self::find_free_variables(body, params, &self.locals);
                func_compiler.chunk.upvalues = free_vars.clone();

                func_compiler.upvalue_names = free_vars.iter().cloned().collect();

                // Compile function body
                for stmt in body {
                    func_compiler.compile_stmt(stmt)?;
                }

                // Ensure function returns None if no explicit return
                func_compiler.chunk.emit(OpCode::ReturnNone);

                // Record local slot count
                func_compiler.chunk.local_count = func_compiler.next_local_slot;

                // Add function as constant
                let func_index =
                    self.chunk.add_constant(Constant::Function(Box::new(func_compiler.chunk)));

                // Create closure and store in variable
                self.chunk.emit(OpCode::MakeClosure(func_index));
                self.chunk.emit(OpCode::StoreGlobal(name.clone()));

                Ok(())
            }

            Stmt::StructDef { name, fields: _, methods } => {
                // Compile struct methods into global bytecode functions
                for method_stmt in methods {
                    if let Stmt::FuncDef {
                        name: method_name,
                        params,
                        body,
                        is_async,
                        is_generator,
                        ..
                    } = method_stmt
                    {
                        if *is_generator {
                            return Err(unsupported_struct_generator_method_message(
                                name,
                                method_name,
                            ));
                        }

                        let mut func_compiler = Compiler::new();
                        func_compiler.used_locals = Self::collect_used_variables(body);
                        func_compiler.chunk.name = Some(format!("{}.{}", name, method_name));
                        func_compiler.chunk.params = params.clone();
                        func_compiler.chunk.is_async = *is_async;
                        func_compiler.chunk.is_generator = *is_generator;
                        func_compiler.scope_depth = 1;
                        func_compiler.uses_local_slots = true;

                        for param in params {
                            if func_compiler.has_local_in_current_scope(param) {
                                return Err(format!(
                                    "Duplicate declaration in the same scope: {}",
                                    param
                                ));
                            }
                            func_compiler.add_local(param, 1, BytecodeBindingKind::Mutable);
                        }

                        let free_vars = Self::find_free_variables(body, params, &self.locals);
                        func_compiler.chunk.upvalues = free_vars.clone();

                        func_compiler.upvalue_names = free_vars.iter().cloned().collect();

                        for stmt in body {
                            func_compiler.compile_stmt(stmt)?;
                        }

                        func_compiler.chunk.emit(OpCode::ReturnNone);

                        func_compiler.chunk.local_count = func_compiler.next_local_slot;
                        let func_index = self
                            .chunk
                            .add_constant(Constant::Function(Box::new(func_compiler.chunk)));

                        let global_name = format!("{}.{}", name, method_name);
                        self.chunk.emit(OpCode::MakeClosure(func_index));
                        self.chunk.emit(OpCode::StoreGlobal(global_name));
                    }
                }

                Ok(())
            }

            Stmt::Match { value, cases, default } => {
                // Compile the value to match
                self.compile_expr(value)?;

                let mut case_jumps = Vec::new();
                let mut end_jumps = Vec::new();

                for (pattern_name, body) in cases {
                    self.chunk.emit(OpCode::BeginCase);

                    // Duplicate the value for matching
                    self.chunk.emit(OpCode::Dup);

                    // Match with case-pattern aware semantics (including tag-style bindings).
                    self.chunk.emit(OpCode::MatchCasePattern(pattern_name.clone()));

                    // If match fails, jump to next case
                    let next_case_jump = self.chunk.emit(OpCode::JumpIfFalse(0));
                    self.chunk.emit(OpCode::Pop); // Pop match result

                    case_jumps.push(next_case_jump);

                    // Pop the original value since we matched
                    self.chunk.emit(OpCode::Pop);

                    // Compile case body
                    for stmt in body {
                        self.compile_stmt(stmt)?;
                    }

                    // Jump to end of match
                    let end_jump = self.chunk.emit(OpCode::Jump(0));
                    end_jumps.push(end_jump);

                    // Patch the jump to next case
                    self.chunk.patch_jump(next_case_jump);
                    self.chunk.emit(OpCode::Pop); // Pop match result

                    self.chunk.emit(OpCode::EndCase);
                }

                // Compile default case if present
                if let Some(default_body) = default {
                    self.chunk.emit(OpCode::Pop); // Pop the value
                    for stmt in default_body {
                        self.compile_stmt(stmt)?;
                    }
                } else {
                    // Pop the original value if no case matched
                    self.chunk.emit(OpCode::Pop);
                }

                // Patch all end jumps
                for end_jump in end_jumps {
                    self.chunk.patch_jump(end_jump);
                }

                Ok(())
            }

            Stmt::Loop { condition, body } => {
                let loop_start = self.chunk.instructions.len();
                self.loop_starts.push(loop_start);
                self.loop_ends.push(Vec::new());

                // If there's a condition, check it
                if let Some(cond_expr) = condition {
                    self.compile_expr(cond_expr)?;

                    // Jump to end if condition is false
                    let end_jump = self.chunk.emit(OpCode::JumpIfFalse(0));
                    self.chunk.emit(OpCode::Pop); // Pop condition

                    // Compile body
                    self.enter_scope();
                    for stmt in body {
                        self.compile_stmt(stmt)?;
                    }
                    self.exit_scope();

                    // Jump back to start
                    self.chunk.emit(OpCode::JumpBack(loop_start));

                    // Patch end jump
                    self.chunk.patch_jump(end_jump);
                    self.chunk.emit(OpCode::Pop); // Pop condition
                } else {
                    // Unconditional loop
                    self.enter_scope();
                    for stmt in body {
                        self.compile_stmt(stmt)?;
                    }
                    self.exit_scope();

                    // Jump back to start
                    self.chunk.emit(OpCode::JumpBack(loop_start));
                }

                // Patch all break statements
                if let Some(breaks) = self.loop_ends.pop() {
                    for break_jump in breaks {
                        self.chunk.patch_jump(break_jump);
                    }
                }
                self.loop_starts.pop();

                Ok(())
            }

            Stmt::TryExcept { try_block, except_var, except_block } => {
                self.has_exception_flow = true;
                // Set up exception handler
                let try_start = self.chunk.instructions.len();

                // Emit BeginTry with placeholder catch address
                let begin_try_index = self.chunk.emit(OpCode::BeginTry(0));

                // Compile try block
                for stmt in try_block {
                    self.compile_stmt(stmt)?;
                }

                // End try block
                self.chunk.emit(OpCode::EndTry);

                // Jump over catch block if no exception
                let end_jump = self.chunk.emit(OpCode::Jump(0));

                // Catch block starts here
                let catch_start = self.chunk.instructions.len();

                // Patch BeginTry with actual catch address
                self.chunk.set_jump_target(begin_try_index, catch_start);

                // Begin catch and bind exception to variable
                self.chunk.emit(OpCode::BeginCatch(except_var.clone()));

                // Compile catch block
                for stmt in except_block {
                    self.compile_stmt(stmt)?;
                }

                // End catch block
                self.chunk.emit(OpCode::EndCatch);

                // Patch the jump over catch block
                self.chunk.patch_jump(end_jump);

                // Record exception handler in metadata
                let try_end = catch_start - 1;
                self.chunk.exception_handlers.push(crate::bytecode::ExceptionHandler {
                    try_start,
                    try_end,
                    catch_start,
                    exception_var: except_var.clone(),
                });

                Ok(())
            }

            Stmt::Block(statements) => {
                // Enter new scope
                self.chunk.emit(OpCode::PushScope);
                self.enter_scope();

                // Compile block statements
                for stmt in statements {
                    self.compile_stmt(stmt)?;
                }

                // Exit scope
                self.exit_scope();
                self.chunk.emit(OpCode::PopScope);

                Ok(())
            }

            Stmt::Const { name, value, .. } => {
                self.compile_expr(value)?;
                if !self.uses_local_slots || self.scope_depth == 0 {
                    self.chunk.emit(OpCode::DefineGlobal(name.clone(), BytecodeBindingKind::Const));
                } else {
                    let slot = self.declare_local(name, BytecodeBindingKind::Const)?;
                    self.chunk.emit(OpCode::StoreLocal(slot));
                }

                Ok(())
            }

            Stmt::EnumDef { name: _, variants: _ } => {
                // Enum definitions are handled at runtime for now
                // They don't generate bytecode, just metadata
                Ok(())
            }

            Stmt::Export { stmt } => {
                // Export is just a marker - compile the inner statement
                self.compile_stmt(stmt)?;
                Ok(())
            }

            Stmt::Spawn { body } => {
                // Spawn creates a background thread
                // For now, compile as a lambda that gets executed asynchronously
                // This is simplified - full implementation needs runtime support

                // Create function for spawn body
                let mut spawn_compiler = Compiler::new();
                spawn_compiler.used_locals = Self::collect_used_variables(body);
                spawn_compiler.chunk.name = Some("<spawn>".to_string());
                spawn_compiler.scope_depth = 0;

                for stmt in body {
                    spawn_compiler.compile_stmt(stmt)?;
                }

                spawn_compiler.chunk.emit(OpCode::ReturnNone);

                let func_index =
                    self.chunk.add_constant(Constant::Function(Box::new(spawn_compiler.chunk)));

                // Load function and call it (runtime will handle thread spawning)
                self.chunk.emit(OpCode::MakeClosure(func_index));
                // Deferred post-v1 runtime backlog: dedicated SpawnThread opcode (see docs/V1_SCOPE.md deferred runtime execution section)
                // For now this will just create a closure
                self.chunk.emit(OpCode::Pop); // Pop the closure for now

                Ok(())
            }

            Stmt::Import { module, symbols } => {
                let import_module_const = self.chunk.add_constant(Constant::String(module.clone()));

                match symbols {
                    Some(symbol_list) => {
                        for symbol_name in symbol_list {
                            let import_symbol_const =
                                self.chunk.add_constant(Constant::String(symbol_name.clone()));

                            self.chunk.emit(OpCode::LoadConst(import_module_const));
                            self.chunk.emit(OpCode::LoadConst(import_symbol_const));
                            self.chunk
                                .emit(OpCode::CallNative("__vm_import_symbol".to_string(), 2));
                            self.chunk.emit(OpCode::Pop);
                        }
                    }
                    None => {
                        self.chunk.emit(OpCode::LoadConst(import_module_const));
                        self.chunk.emit(OpCode::CallNative("__vm_import_all".to_string(), 1));
                        self.chunk.emit(OpCode::Pop);
                    }
                }

                Ok(())
            }

            Stmt::Test { .. }
            | Stmt::TestSetup { .. }
            | Stmt::TestTeardown { .. }
            | Stmt::TestGroup { .. } => {
                // These are handled at parse/runtime for now
                // Test statements are executed by test runner
                Ok(())
            }
        }
    }

    /// Compile an expression
    fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Int(n) => {
                let index = self.chunk.add_constant(Constant::Int(*n));
                self.chunk.emit(OpCode::LoadConst(index));
                Ok(())
            }

            Expr::Float(f) => {
                let index = self.chunk.add_constant(Constant::Float(*f));
                self.chunk.emit(OpCode::LoadConst(index));
                Ok(())
            }

            Expr::String(s) => {
                let index = self.chunk.add_constant(Constant::String(s.clone()));
                self.chunk.emit(OpCode::LoadConst(index));
                Ok(())
            }

            Expr::Bool(b) => {
                let index = self.chunk.add_constant(Constant::Bool(*b));
                self.chunk.emit(OpCode::LoadConst(index));
                Ok(())
            }

            Expr::Identifier(name) => {
                if self.is_upvalue(name) {
                    self.chunk.emit(OpCode::LoadVar(name.clone()));
                } else if let Some(slot) = self.resolve_local_slot(name) {
                    self.chunk.emit(OpCode::LoadLocal(slot));
                } else if self.scope_depth == 0 {
                    self.chunk.emit(OpCode::LoadGlobal(name.clone()));
                } else {
                    self.chunk.emit(OpCode::LoadVar(name.clone()));
                }
                Ok(())
            }

            Expr::BinaryOp { left, op, right } => {
                if op == "&&" {
                    self.has_logical_short_circuit = true;
                    self.compile_expr(left)?;
                    let false_branch_jump = self.chunk.emit(OpCode::JumpIfFalse(0));
                    self.chunk.emit(OpCode::Pop);
                    let true_index = self.chunk.add_constant(Constant::Bool(true));
                    self.chunk.emit(OpCode::LoadConst(true_index));
                    self.compile_expr(right)?;
                    self.chunk.emit(OpCode::And);
                    let end_jump = self.chunk.emit(OpCode::Jump(0));

                    self.chunk.patch_jump(false_branch_jump);
                    self.chunk.emit(OpCode::Pop);
                    let false_index = self.chunk.add_constant(Constant::Bool(false));
                    self.chunk.emit(OpCode::LoadConst(false_index));
                    self.chunk.patch_jump(end_jump);
                    return Ok(());
                }

                if op == "||" {
                    self.has_logical_short_circuit = true;
                    self.compile_expr(left)?;
                    let true_branch_jump = self.chunk.emit(OpCode::JumpIfTrue(0));
                    self.chunk.emit(OpCode::Pop);
                    let false_index = self.chunk.add_constant(Constant::Bool(false));
                    self.chunk.emit(OpCode::LoadConst(false_index));
                    self.compile_expr(right)?;
                    self.chunk.emit(OpCode::Or);
                    let end_jump = self.chunk.emit(OpCode::Jump(0));

                    self.chunk.patch_jump(true_branch_jump);
                    self.chunk.emit(OpCode::Pop);
                    let true_index = self.chunk.add_constant(Constant::Bool(true));
                    self.chunk.emit(OpCode::LoadConst(true_index));
                    self.chunk.patch_jump(end_jump);
                    return Ok(());
                }

                if op == "??" {
                    self.compile_expr(left)?;
                    self.chunk.emit(OpCode::Dup);
                    let none_index = self.chunk.add_constant(Constant::None);
                    self.chunk.emit(OpCode::LoadConst(none_index));
                    self.chunk.emit(OpCode::Equal);

                    // If left is NOT null, keep it and skip RHS evaluation.
                    let left_non_null_jump = self.chunk.emit(OpCode::JumpIfFalse(0));

                    // left is null: drop condition + left and evaluate RHS.
                    self.chunk.emit(OpCode::Pop);
                    self.chunk.emit(OpCode::Pop);
                    self.compile_expr(right)?;
                    let end_jump = self.chunk.emit(OpCode::Jump(0));

                    // left is non-null: drop condition and keep left.
                    self.chunk.patch_jump(left_non_null_jump);
                    self.chunk.emit(OpCode::Pop);
                    self.chunk.patch_jump(end_jump);
                    return Ok(());
                }

                if op == "?." {
                    self.compile_expr(left)?;
                    self.chunk.emit(OpCode::Dup);
                    let none_index = self.chunk.add_constant(Constant::None);
                    self.chunk.emit(OpCode::LoadConst(none_index));
                    self.chunk.emit(OpCode::Equal);

                    // If left is NOT null, evaluate field access.
                    let left_non_null_jump = self.chunk.emit(OpCode::JumpIfFalse(0));

                    // left is null: drop condition + left, return null.
                    self.chunk.emit(OpCode::Pop);
                    self.chunk.emit(OpCode::Pop);
                    self.chunk.emit(OpCode::LoadConst(none_index));
                    let end_jump = self.chunk.emit(OpCode::Jump(0));

                    self.chunk.patch_jump(left_non_null_jump);
                    self.chunk.emit(OpCode::Pop);
                    match right.as_ref() {
                        Expr::String(field_name) => {
                            self.chunk.emit(OpCode::FieldGet(field_name.clone()))
                        }
                        _ => return Err(
                            "Optional chaining requires a field-name string on the right-hand side"
                                .to_string(),
                        ),
                    };
                    self.chunk.patch_jump(end_jump);
                    return Ok(());
                }

                if op == "|>" {
                    match right.as_ref() {
                        Expr::Call { function, args } => {
                            self.compile_expr(left)?;
                            for arg in args {
                                self.compile_expr(arg)?;
                            }
                            self.compile_expr(function)?;
                            self.chunk.emit(OpCode::Call(args.len() + 1));
                        }
                        _ => {
                            self.compile_expr(left)?;
                            self.compile_expr(right)?;
                            self.chunk.emit(OpCode::Call(1));
                        }
                    }
                    return Ok(());
                }

                // Compile operands
                self.compile_expr(left)?;
                self.compile_expr(right)?;

                // Emit operation
                match op.as_str() {
                    "+" => self.chunk.emit(OpCode::Add),
                    "-" => self.chunk.emit(OpCode::Sub),
                    "*" => self.chunk.emit(OpCode::Mul),
                    "/" => self.chunk.emit(OpCode::Div),
                    "%" => self.chunk.emit(OpCode::Mod),
                    "==" => self.chunk.emit(OpCode::Equal),
                    "!=" => self.chunk.emit(OpCode::NotEqual),
                    "<" => self.chunk.emit(OpCode::LessThan),
                    ">" => self.chunk.emit(OpCode::GreaterThan),
                    "<=" => self.chunk.emit(OpCode::LessEqual),
                    ">=" => self.chunk.emit(OpCode::GreaterEqual),
                    "&&" => self.chunk.emit(OpCode::And),
                    "||" => self.chunk.emit(OpCode::Or),
                    _ => return Err(format!("Unknown binary operator: {}", op)),
                };

                Ok(())
            }

            Expr::UnaryOp { op, operand } => {
                self.compile_expr(operand)?;

                match op.as_str() {
                    "-" => self.chunk.emit(OpCode::Negate),
                    "!" => self.chunk.emit(OpCode::Not),
                    _ => return Err(format!("Unknown unary operator: {}", op)),
                };

                Ok(())
            }

            Expr::Call { function, args } => {
                // Method-call sugar: obj.method(a, b) should lower to a receiver-aware
                // call path rather than a plain function call of FieldGet.
                if let Expr::FieldAccess { object, field } = function.as_ref() {
                    self.has_method_call_flow = true;
                    // Receiver becomes first argument.
                    self.compile_expr(object)?;
                    for arg in args {
                        self.compile_expr(arg)?;
                    }

                    // Re-load receiver for FieldGet so method lookup can resolve the member.
                    self.compile_expr(object)?;
                    self.chunk.emit(OpCode::FieldGet(field.clone()));

                    // +1 accounts for receiver argument.
                    self.chunk.emit(OpCode::Call(args.len() + 1));
                    return Ok(());
                }

                // Compile arguments (bottom to top on stack)
                for arg in args {
                    self.compile_expr(arg)?;
                }

                // Compile function expression
                self.compile_expr(function)?;

                // Emit call with argument count
                self.chunk.emit(OpCode::Call(args.len()));

                Ok(())
            }

            Expr::ArrayLiteral(elements) => {
                // Check if we have any spread operations
                let has_spread = elements.iter().any(|e| matches!(e, ArrayElement::Spread(_)));

                if has_spread {
                    // With spreads, push a marker first, then all elements
                    self.chunk.emit(OpCode::PushArrayMarker);
                }

                // Compile all elements and spreads
                for element in elements {
                    match element {
                        ArrayElement::Single(expr) => {
                            self.compile_expr(expr)?;
                        }
                        ArrayElement::Spread(expr) => {
                            self.compile_expr(expr)?;
                            self.chunk.emit(OpCode::SpreadArray);
                        }
                    }
                }

                // MakeArray will collect everything
                // If there was a marker, it collects until marker
                // Otherwise, it collects exactly 'count' elements
                if has_spread {
                    self.chunk.emit(OpCode::MakeArrayFromMarker);
                } else {
                    self.chunk.emit(OpCode::MakeArray(elements.len()));
                }

                Ok(())
            }

            Expr::DictLiteral(elements) => {
                let mut pair_count = 0;
                let mut const_keys = Vec::new();
                let mut all_string_keys = true;
                let mut has_spread = false;

                for element in elements {
                    match element {
                        DictElement::Pair(key, _value) => {
                            if let Expr::String(s) = key {
                                const_keys.push(Arc::from(s.clone()));
                            } else {
                                all_string_keys = false;
                            }
                            pair_count += 1;
                        }
                        DictElement::Spread(_) => {
                            has_spread = true;
                        }
                    }
                }

                if all_string_keys && !has_spread {
                    if const_keys.is_empty() {
                        self.chunk.emit(OpCode::MakeDict(0));
                        return Ok(());
                    }
                    for element in elements {
                        if let DictElement::Pair(_, value) = element {
                            self.compile_expr(value)?;
                        }
                    }
                    self.chunk.emit(OpCode::MakeDictWithKeys(Arc::new(const_keys)));
                    return Ok(());
                }

                if has_spread {
                    // Marker sentinel pair for MakeDictFromMarker
                    self.chunk.emit(OpCode::PushArrayMarker);
                    self.chunk.emit(OpCode::PushArrayMarker);
                }

                for element in elements {
                    match element {
                        DictElement::Pair(key, value) => {
                            self.compile_expr(key)?;
                            self.compile_expr(value)?;
                        }
                        DictElement::Spread(expr) => {
                            self.compile_expr(expr)?;
                            self.chunk.emit(OpCode::SpreadDict);
                        }
                    }
                }

                if has_spread {
                    self.chunk.emit(OpCode::MakeDictFromMarker);
                } else {
                    self.chunk.emit(OpCode::MakeDict(pair_count));
                }

                Ok(())
            }

            Expr::IndexAccess { object, index } => {
                // Optimization: if object is a local variable identifier, use IndexGetInPlace
                if let Expr::Identifier(name) = &**object {
                    if let Some(slot) = self.resolve_local_slot(name) {
                        // Compile index and emit optimized opcode
                        self.compile_expr(index)?;
                        self.chunk.emit(OpCode::IndexGetInPlace(slot));
                        return Ok(());
                    }
                }

                // Default path: load object, then index
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.chunk.emit(OpCode::IndexGet);
                Ok(())
            }

            Expr::FieldAccess { object, field } => {
                self.compile_expr(object)?;
                self.chunk.emit(OpCode::FieldGet(field.clone()));
                Ok(())
            }

            Expr::Function { params, body, .. } => {
                // Create anonymous function
                let mut func_compiler = Compiler::new();
                func_compiler.used_locals = Self::collect_used_variables(body);
                func_compiler.chunk.name = Some("<lambda>".to_string());
                func_compiler.chunk.params = params.clone();
                func_compiler.scope_depth = 1; // Functions create a new scope (not global)
                func_compiler.uses_local_slots = true;

                // Add parameters as locals
                for param in params {
                    if func_compiler.has_local_in_current_scope(param) {
                        return Err(format!("Duplicate declaration in the same scope: {}", param));
                    }
                    func_compiler.add_local(param, 1, BytecodeBindingKind::Mutable);
                }

                // Analyze the function body to find free variables (captures)
                let free_vars = Self::find_free_variables(body, params, &self.locals);
                func_compiler.chunk.upvalues = free_vars.clone();

                // Captured variables are resolved from the closure's captured map at runtime
                func_compiler.upvalue_names = free_vars.iter().cloned().collect();

                // Compile function body
                for stmt in body {
                    func_compiler.compile_stmt(stmt)?;
                }

                func_compiler.chunk.emit(OpCode::ReturnNone);

                func_compiler.chunk.local_count = func_compiler.next_local_slot;
                let func_index =
                    self.chunk.add_constant(Constant::Function(Box::new(func_compiler.chunk)));
                self.chunk.emit(OpCode::MakeClosure(func_index));

                Ok(())
            }

            Expr::Ok(value) => {
                self.compile_expr(value)?;
                self.chunk.emit(OpCode::MakeOk);
                Ok(())
            }

            Expr::Err(value) => {
                self.compile_expr(value)?;
                self.chunk.emit(OpCode::MakeErr);
                Ok(())
            }

            Expr::Some(value) => {
                self.compile_expr(value)?;
                self.chunk.emit(OpCode::MakeSome);
                Ok(())
            }

            Expr::None => {
                self.chunk.emit(OpCode::MakeNone);
                Ok(())
            }

            Expr::Try(expr) => {
                self.has_exception_flow = true;
                self.compile_expr(expr)?;
                self.chunk.emit(OpCode::TryUnwrap);
                Ok(())
            }

            Expr::StructInstance { name, fields } => {
                // Compile field values
                let mut field_names = Vec::new();
                for (field_name, field_value) in fields {
                    field_names.push(field_name.clone());
                    self.compile_expr(field_value)?;
                }

                self.chunk.emit(OpCode::MakeStruct(name.clone(), field_names));
                Ok(())
            }

            Expr::Tag(tag, values) => {
                // Special handling for throw - it's a control flow primitive
                if tag == "throw" {
                    self.has_exception_flow = true;
                    // Compile error value
                    if values.len() != 1 {
                        return Err("throw() requires exactly one argument".to_string());
                    }
                    self.compile_expr(&values[0])?;

                    // Emit throw instruction
                    self.chunk.emit(OpCode::Throw);

                    return Ok(());
                }

                // Canonical Result/Option tag constructors should map to dedicated
                // runtime value opcodes so VM match semantics can inspect structured values.
                if tag == "Result::Ok" {
                    if values.len() != 1 {
                        return Err("Result::Ok(...) requires exactly one argument".to_string());
                    }
                    self.compile_expr(&values[0])?;
                    self.chunk.emit(OpCode::MakeOk);
                    return Ok(());
                }

                if tag == "Result::Err" {
                    if values.len() != 1 {
                        return Err("Result::Err(...) requires exactly one argument".to_string());
                    }
                    self.compile_expr(&values[0])?;
                    self.chunk.emit(OpCode::MakeErr);
                    return Ok(());
                }

                if tag == "Option::Some" {
                    if values.len() != 1 {
                        return Err("Option::Some(...) requires exactly one argument".to_string());
                    }
                    self.compile_expr(&values[0])?;
                    self.chunk.emit(OpCode::MakeSome);
                    return Ok(());
                }

                if tag == "Option::None" {
                    if !values.is_empty() {
                        return Err("Option::None takes no arguments".to_string());
                    }
                    self.chunk.emit(OpCode::MakeNone);
                    return Ok(());
                }

                // Compile tag values for other tags
                for value in values {
                    self.compile_expr(value)?;
                }

                // For now, treat as array with tag name
                // Deferred post-v1 optimization backlog: specialized enum value layout/handling.
                let tag_index = self.chunk.add_constant(Constant::String(tag.clone()));
                self.chunk.emit(OpCode::LoadConst(tag_index));
                self.chunk.emit(OpCode::MakeArray(values.len() + 1));

                Ok(())
            }

            Expr::InterpolatedString(parts) => {
                // Compile each part and concatenate
                for part in parts {
                    match part {
                        crate::ast::InterpolatedStringPart::Text(s) => {
                            let index = self.chunk.add_constant(Constant::String(s.clone()));
                            self.chunk.emit(OpCode::LoadConst(index));
                        }
                        crate::ast::InterpolatedStringPart::Expr(e) => {
                            self.compile_expr(e)?;
                            // Convert to string using to_string builtin
                            self.chunk.emit(OpCode::LoadGlobal("to_string".to_string()));
                            self.chunk.emit(OpCode::Call(1));
                        }
                    }
                }

                // Concatenate all parts
                // Deferred post-v1 optimization backlog: dedicated string-builder opcode path.
                for _ in 1..parts.len() {
                    self.chunk.emit(OpCode::Add); // String concatenation
                }

                Ok(())
            }

            Expr::Yield(value_expr) => {
                // Compile the value to yield (or None if no value)
                if let Some(expr) = value_expr {
                    self.compile_expr(expr)?;
                } else {
                    let none_index = self.chunk.add_constant(Constant::None);
                    self.chunk.emit(OpCode::LoadConst(none_index));
                }

                // Emit yield instruction
                // This saves the current state and returns to caller
                self.chunk.emit(OpCode::Yield);

                // Mark the chunk as a generator
                self.chunk.is_generator = true;

                Ok(())
            }

            Expr::Await(promise_expr) => {
                // Compile the promise expression
                self.compile_expr(promise_expr)?;

                // Emit await instruction
                // This suspends execution until the promise resolves
                self.chunk.emit(OpCode::Await);

                // Mark the chunk as async
                self.chunk.is_async = true;

                Ok(())
            }

            Expr::MethodCall { object, method, args } => {
                self.has_method_call_flow = true;
                // Method calls are sugar for calling a method on an object
                // Translate: obj.method(a, b) -> method(obj, a, b)

                // Compile the object (becomes first argument)
                self.compile_expr(object)?;

                // Compile other arguments
                for arg in args {
                    self.compile_expr(arg)?;
                }

                // Load the method (it's either a field or a built-in method)
                // For built-in iterator methods (map, filter, etc.), use native calls
                match method.as_str() {
                    "map" | "filter" | "reduce" | "collect" | "take" | "skip" | "zip"
                    | "enumerate" | "flatten" | "chunk" => {
                        // These are native iterator functions
                        self.chunk.emit(OpCode::CallNative(method.clone(), args.len() + 1));
                    }
                    _ => {
                        // General method call: load field then call
                        self.compile_expr(object)?;
                        self.chunk.emit(OpCode::FieldGet(method.clone()));

                        // Move function to top of stack (after arguments)
                        // Stack: [obj, arg1, arg2, ..., method]
                        // Need: [obj, arg1, arg2, ..., method] for Call

                        self.chunk.emit(OpCode::Call(args.len() + 1)); // +1 for object as first arg
                    }
                }

                Ok(())
            }

            Expr::Spread(_) => {
                Err("Spread operator cannot be compiled as standalone expression".to_string())
            }
        }
    }

    /// Compile pattern binding (for let statements)
    fn compile_pattern_binding(
        &mut self,
        pattern: &Pattern,
        binding_kind: BytecodeBindingKind,
    ) -> Result<(), String> {
        match pattern {
            Pattern::Identifier(name) => {
                if !self.uses_local_slots || self.scope_depth == 0 {
                    self.chunk.emit(OpCode::DefineGlobal(name.clone(), binding_kind));
                } else if self.is_upvalue(name) {
                    self.chunk.emit(OpCode::StoreVar(name.clone()));
                } else {
                    let slot = self.declare_local(name, binding_kind)?;
                    self.chunk.emit(OpCode::StoreLocal(slot));
                }
                Ok(())
            }

            Pattern::Ignore => {
                // Just pop the value
                self.chunk.emit(OpCode::Pop);
                Ok(())
            }

            Pattern::Array { elements: _, rest: _ } => {
                // Use MatchPattern for complex binding
                let pattern_index = self.chunk.add_constant(Constant::Pattern(pattern.clone()));
                self.chunk.emit(OpCode::MatchPattern(pattern_index, binding_kind));
                self.chunk.emit(OpCode::Pop); // Pop the success bool
                Ok(())
            }

            Pattern::Dict { keys: _, rest: _ } => {
                // Use MatchPattern for complex binding
                let pattern_index = self.chunk.add_constant(Constant::Pattern(pattern.clone()));
                self.chunk.emit(OpCode::MatchPattern(pattern_index, binding_kind));
                self.chunk.emit(OpCode::Pop); // Pop the success bool
                Ok(())
            }
        }
    }

    /// Compile assignment target
    fn compile_assignment(&mut self, target: &Expr) -> Result<(), String> {
        match target {
            Expr::Identifier(name) => {
                if self.is_upvalue(name) {
                    self.chunk.emit(OpCode::StoreVar(name.clone()));
                } else if let Some(slot) = self.resolve_local_slot(name) {
                    self.chunk.emit(OpCode::StoreLocal(slot));
                } else if self.uses_local_slots && self.scope_depth > 0 {
                    // Mirror interpreter semantics for `:=` assignment:
                    // assign into an existing outer binding when present;
                    // otherwise define in the current runtime scope.
                    self.chunk.emit(OpCode::StoreVar(name.clone()));
                } else if self.scope_depth == 0 {
                    self.chunk.emit(OpCode::StoreGlobal(name.clone()));
                } else {
                    self.chunk.emit(OpCode::StoreVar(name.clone()));
                }
                Ok(())
            }

            Expr::IndexAccess { object, index } => {
                // Optimization: if object is a local variable identifier, use IndexSetInPlace
                if let Expr::Identifier(name) = &**object {
                    if let Some(slot) = self.resolve_local_slot(name) {
                        // Value is already on stack from assignment
                        // Compile index and emit optimized opcode
                        self.compile_expr(index)?;
                        self.chunk.emit(OpCode::IndexSetInPlace(slot));
                        return Ok(());
                    } else if self.scope_depth == 0 && !self.is_upvalue(name) {
                        // Global in-place mutation should preserve mutable-binding checks.
                        self.chunk.emit(OpCode::EnsureMutableGlobalForMutation(name.clone()));
                        self.compile_expr(object)?;
                        self.compile_expr(index)?;
                        self.chunk.emit(OpCode::IndexSet);
                        self.chunk.emit(OpCode::StoreGlobal(name.clone()));
                        return Ok(());
                    }
                }

                // Default path: load object, index, then set
                // Value is already on stack
                // Need: [value, object, index]
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.chunk.emit(OpCode::IndexSet);

                // IndexSet leaves the modified object on the stack
                // We need to store it back to the variable if object is an identifier
                if let Expr::Identifier(name) = &**object {
                    if self.is_upvalue(name) || self.is_local(name) {
                        self.chunk.emit(OpCode::StoreVar(name.clone()));
                    } else {
                        self.chunk.emit(OpCode::StoreGlobal(name.clone()));
                    }
                } else {
                    // For non-identifier objects (like nested access), just pop the result
                    self.chunk.emit(OpCode::Pop);
                }
                Ok(())
            }

            Expr::FieldAccess { object, field } => {
                // Value is already on stack
                self.compile_expr(object)?;
                self.chunk.emit(OpCode::FieldSet(field.clone()));
                Ok(())
            }

            _ => Err("Invalid assignment target".to_string()),
        }
    }

    /// Check if a variable is a local
    fn is_local(&self, name: &str) -> bool {
        self.resolve_local_slot(name).is_some()
    }

    /// Check if an expression is pure (no side effects) and safe to elide
    fn expr_is_pure(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Int(_) | Expr::Float(_) | Expr::String(_) | Expr::Bool(_) | Expr::None => true,
            Expr::Identifier(name) => {
                self.resolve_local_slot(name).is_some() || self.is_upvalue(name)
            }
            Expr::UnaryOp { operand, .. } => self.expr_is_pure(operand),
            Expr::BinaryOp { left, right, .. } => {
                self.expr_is_pure(left) && self.expr_is_pure(right)
            }
            Expr::ArrayLiteral(elements) => elements.iter().all(|elem| match elem {
                ArrayElement::Single(expr) | ArrayElement::Spread(expr) => self.expr_is_pure(expr),
            }),
            Expr::DictLiteral(entries) => entries.iter().all(|entry| match entry {
                DictElement::Pair(key, value) => self.expr_is_pure(key) && self.expr_is_pure(value),
                DictElement::Spread(expr) => self.expr_is_pure(expr),
            }),
            Expr::StructInstance { fields, .. } => {
                fields.iter().all(|(_, expr)| self.expr_is_pure(expr))
            }
            Expr::InterpolatedString(parts) => parts.iter().all(|part| match part {
                crate::ast::InterpolatedStringPart::Text(_) => true,
                crate::ast::InterpolatedStringPart::Expr(expr) => self.expr_is_pure(expr),
            }),
            Expr::Ok(expr) | Expr::Err(expr) | Expr::Some(expr) | Expr::Try(expr) => {
                self.expr_is_pure(expr)
            }
            Expr::Tag(_, values) => values.iter().all(|expr| self.expr_is_pure(expr)),
            _ => false,
        }
    }

    fn match_append_char_until_local_pattern(
        &self,
        condition: &Expr,
        body: &[Stmt],
    ) -> Option<(usize, usize, usize, char)> {
        if body.len() != 2 {
            return None;
        }

        let (index_name, limit_name) = match condition {
            Expr::BinaryOp { left, op, right } if op == "<" => match (&**left, &**right) {
                (Expr::Identifier(index_name), Expr::Identifier(limit_name)) => {
                    (index_name.as_str(), limit_name.as_str())
                }
                _ => return None,
            },
            _ => return None,
        };

        let (target_name, append_char) = match &body[0] {
            Stmt::Assign {
                target: Expr::Identifier(target_name),
                value: Expr::BinaryOp { left, op, right },
            } if op == "+" => match (&**left, &**right) {
                (Expr::Identifier(left_name), Expr::String(append_str))
                    if left_name == target_name =>
                {
                    let mut chars = append_str.chars();
                    match (chars.next(), chars.next()) {
                        (Some(ch), None) => (target_name.as_str(), ch),
                        _ => return None,
                    }
                }
                _ => return None,
            },
            _ => return None,
        };

        match &body[1] {
            Stmt::Assign {
                target: Expr::Identifier(target_name),
                value: Expr::BinaryOp { left, op, right },
            } if op == "+" && target_name == index_name => match (&**left, &**right) {
                (Expr::Identifier(left_name), Expr::Int(1)) if left_name == index_name => {}
                _ => return None,
            },
            _ => return None,
        }

        let target_slot = self.resolve_local_slot(target_name)?;
        let index_slot = self.resolve_local_slot(index_name)?;
        let limit_slot = self.resolve_local_slot(limit_name)?;

        Some((target_slot, index_slot, limit_slot, append_char))
    }

    fn match_sum_int_map_until_local_pattern(
        &self,
        condition: &Expr,
        body: &[Stmt],
    ) -> Option<(usize, usize, usize, usize)> {
        if body.len() != 2 {
            return None;
        }

        let (index_name, limit_name) = match condition {
            Expr::BinaryOp { left, op, right } if op == "<" => match (&**left, &**right) {
                (Expr::Identifier(index_name), Expr::Identifier(limit_name)) => {
                    (index_name.as_str(), limit_name.as_str())
                }
                _ => return None,
            },
            _ => return None,
        };

        let (sum_name, map_name) = match &body[0] {
            Stmt::Assign {
                target: Expr::Identifier(sum_name),
                value: Expr::BinaryOp { left, op, right },
            } if op == "+" => match (&**left, &**right) {
                (Expr::Identifier(left_sum_name), Expr::IndexAccess { object, index })
                    if left_sum_name == sum_name =>
                {
                    match (&**object, &**index) {
                        (Expr::Identifier(map_name), Expr::Identifier(index_var))
                            if index_var == index_name =>
                        {
                            (sum_name.as_str(), map_name.as_str())
                        }
                        _ => return None,
                    }
                }
                _ => return None,
            },
            _ => return None,
        };

        match &body[1] {
            Stmt::Assign {
                target: Expr::Identifier(target_name),
                value: Expr::BinaryOp { left, op, right },
            } if op == "+" && target_name == index_name => match (&**left, &**right) {
                (Expr::Identifier(left_name), Expr::Int(1)) if left_name == index_name => {}
                _ => return None,
            },
            _ => return None,
        }

        let map_slot = self.resolve_local_slot(map_name)?;
        let sum_slot = self.resolve_local_slot(sum_name)?;
        let index_slot = self.resolve_local_slot(index_name)?;
        let limit_slot = self.resolve_local_slot(limit_name)?;

        Some((map_slot, sum_slot, index_slot, limit_slot))
    }

    fn match_fill_int_map_with_double_until_local_pattern(
        &self,
        condition: &Expr,
        body: &[Stmt],
    ) -> Option<(usize, usize, usize)> {
        if body.len() != 2 {
            return None;
        }

        let (index_name, limit_name) = match condition {
            Expr::BinaryOp { left, op, right } if op == "<" => match (&**left, &**right) {
                (Expr::Identifier(index_name), Expr::Identifier(limit_name)) => {
                    (index_name.as_str(), limit_name.as_str())
                }
                _ => return None,
            },
            _ => return None,
        };

        let map_name = match &body[0] {
            Stmt::Assign {
                target: Expr::IndexAccess { object, index },
                value: Expr::BinaryOp { left, op, right },
            } if op == "*" => match (&**object, &**index, &**left, &**right) {
                (
                    Expr::Identifier(map_name),
                    Expr::Identifier(index_var),
                    Expr::Identifier(left_name),
                    Expr::Int(2),
                ) if index_var == index_name && left_name == index_name => map_name.as_str(),
                (
                    Expr::Identifier(map_name),
                    Expr::Identifier(index_var),
                    Expr::Int(2),
                    Expr::Identifier(right_name),
                ) if index_var == index_name && right_name == index_name => map_name.as_str(),
                _ => return None,
            },
            _ => return None,
        };

        match &body[1] {
            Stmt::Assign {
                target: Expr::Identifier(target_name),
                value: Expr::BinaryOp { left, op, right },
            } if op == "+" && target_name == index_name => match (&**left, &**right) {
                (Expr::Identifier(left_name), Expr::Int(1)) if left_name == index_name => {}
                _ => return None,
            },
            _ => return None,
        }

        let map_slot = self.resolve_local_slot(map_name)?;
        let index_slot = self.resolve_local_slot(index_name)?;
        let limit_slot = self.resolve_local_slot(limit_name)?;

        Some((map_slot, index_slot, limit_slot))
    }

    /// Collect variables that are read within the statement list
    fn collect_used_variables(body: &[Stmt]) -> HashSet<String> {
        let mut used_vars = HashSet::new();

        fn collect_expr_vars(expr: &Expr, used: &mut HashSet<String>) {
            match expr {
                Expr::Identifier(name) => {
                    used.insert(name.clone());
                }
                Expr::BinaryOp { left, right, .. } => {
                    collect_expr_vars(left, used);
                    collect_expr_vars(right, used);
                }
                Expr::UnaryOp { operand, .. } => {
                    collect_expr_vars(operand, used);
                }
                Expr::Call { function, args } => {
                    collect_expr_vars(function, used);
                    for arg in args {
                        collect_expr_vars(arg, used);
                    }
                }
                Expr::MethodCall { object, args, .. } => {
                    collect_expr_vars(object, used);
                    for arg in args {
                        collect_expr_vars(arg, used);
                    }
                }
                Expr::ArrayLiteral(elements) => {
                    for elem in elements {
                        match elem {
                            ArrayElement::Single(expr) | ArrayElement::Spread(expr) => {
                                collect_expr_vars(expr, used);
                            }
                        }
                    }
                }
                Expr::DictLiteral(entries) => {
                    for entry in entries {
                        match entry {
                            DictElement::Pair(key, value) => {
                                collect_expr_vars(key, used);
                                collect_expr_vars(value, used);
                            }
                            DictElement::Spread(expr) => {
                                collect_expr_vars(expr, used);
                            }
                        }
                    }
                }
                Expr::IndexAccess { object, index } => {
                    collect_expr_vars(object, used);
                    collect_expr_vars(index, used);
                }
                Expr::FieldAccess { object, .. } => {
                    collect_expr_vars(object, used);
                }
                Expr::Function { body, .. } => {
                    for stmt in body {
                        collect_stmt_vars(stmt, used);
                    }
                }
                Expr::Ok(expr) | Expr::Err(expr) | Expr::Some(expr) | Expr::Await(expr) => {
                    collect_expr_vars(expr, used);
                }
                Expr::Yield(Some(expr)) => {
                    collect_expr_vars(expr, used);
                }
                Expr::Try(expr) => {
                    collect_expr_vars(expr, used);
                }
                Expr::StructInstance { fields, .. } => {
                    for (_, expr) in fields {
                        collect_expr_vars(expr, used);
                    }
                }
                Expr::InterpolatedString(parts) => {
                    for part in parts {
                        if let crate::ast::InterpolatedStringPart::Expr(expr) = part {
                            collect_expr_vars(expr, used);
                        }
                    }
                }
                _ => {}
            }
        }

        fn collect_stmt_vars(stmt: &Stmt, used: &mut HashSet<String>) {
            match stmt {
                Stmt::Let { value, .. } => {
                    collect_expr_vars(value, used);
                }
                Stmt::Assign { target, value } => {
                    collect_expr_vars(value, used);
                    match target {
                        Expr::IndexAccess { object, index } => {
                            collect_expr_vars(object, used);
                            collect_expr_vars(index, used);
                        }
                        Expr::FieldAccess { object, .. } => {
                            collect_expr_vars(object, used);
                        }
                        _ => {}
                    }
                }
                Stmt::ExprStmt(expr) => {
                    collect_expr_vars(expr, used);
                }
                Stmt::If { condition, then_branch, else_branch } => {
                    collect_expr_vars(condition, used);
                    for stmt in then_branch {
                        collect_stmt_vars(stmt, used);
                    }
                    if let Some(else_stmts) = else_branch {
                        for stmt in else_stmts {
                            collect_stmt_vars(stmt, used);
                        }
                    }
                }
                Stmt::While { condition, body } => {
                    collect_expr_vars(condition, used);
                    for stmt in body {
                        collect_stmt_vars(stmt, used);
                    }
                }
                Stmt::For { iterable, body, .. } => {
                    collect_expr_vars(iterable, used);
                    for stmt in body {
                        collect_stmt_vars(stmt, used);
                    }
                }
                Stmt::Return(expr) => {
                    if let Some(expr) = expr {
                        collect_expr_vars(expr, used);
                    }
                }
                Stmt::Break | Stmt::Continue => {}
                Stmt::Match { value, cases, default } => {
                    collect_expr_vars(value, used);
                    for (_pattern, stmts) in cases {
                        for stmt in stmts {
                            collect_stmt_vars(stmt, used);
                        }
                    }
                    if let Some(default_stmts) = default {
                        for stmt in default_stmts {
                            collect_stmt_vars(stmt, used);
                        }
                    }
                }
                Stmt::FuncDef { body, .. } => {
                    for stmt in body {
                        collect_stmt_vars(stmt, used);
                    }
                }
                Stmt::TryExcept { try_block, except_block, .. } => {
                    for stmt in try_block {
                        collect_stmt_vars(stmt, used);
                    }
                    for stmt in except_block {
                        collect_stmt_vars(stmt, used);
                    }
                }
                Stmt::Block(stmts) => {
                    for stmt in stmts {
                        collect_stmt_vars(stmt, used);
                    }
                }
                Stmt::Const { value, .. } => {
                    collect_expr_vars(value, used);
                }
                Stmt::Export { stmt } => {
                    collect_stmt_vars(stmt, used);
                }
                Stmt::Spawn { body } => {
                    for stmt in body {
                        collect_stmt_vars(stmt, used);
                    }
                }
                Stmt::Loop { condition, body } => {
                    if let Some(cond) = condition {
                        collect_expr_vars(cond, used);
                    }
                    for stmt in body {
                        collect_stmt_vars(stmt, used);
                    }
                }
                Stmt::Test { body, .. }
                | Stmt::TestSetup { body }
                | Stmt::TestTeardown { body } => {
                    for stmt in body {
                        collect_stmt_vars(stmt, used);
                    }
                }
                Stmt::TestGroup { tests, .. } => {
                    for stmt in tests {
                        collect_stmt_vars(stmt, used);
                    }
                }
                Stmt::StructDef { .. } | Stmt::EnumDef { .. } | Stmt::Import { .. } => {}
            }
        }

        for stmt in body {
            collect_stmt_vars(stmt, &mut used_vars);
        }

        used_vars
    }

    /// Find free variables in a function body
    /// Free variables are variables that are used but not defined locally (not params or let bindings)
    fn find_free_variables(
        body: &[Stmt],
        params: &[String],
        _parent_locals: &[Local],
    ) -> Vec<String> {
        use std::collections::HashSet;

        let mut used_vars = HashSet::new();
        let mut defined_vars: HashSet<String> = params.iter().cloned().collect();

        // Helper function to collect variable usage from expressions
        fn collect_expr_vars(expr: &Expr, used: &mut HashSet<String>) {
            match expr {
                Expr::Identifier(name) => {
                    used.insert(name.clone());
                }
                Expr::BinaryOp { left, right, .. } => {
                    collect_expr_vars(left, used);
                    collect_expr_vars(right, used);
                }
                Expr::UnaryOp { operand, .. } => {
                    collect_expr_vars(operand, used);
                }
                Expr::Call { function, args } => {
                    collect_expr_vars(function, used);
                    for arg in args {
                        collect_expr_vars(arg, used);
                    }
                }
                Expr::MethodCall { object, args, .. } => {
                    collect_expr_vars(object, used);
                    for arg in args {
                        collect_expr_vars(arg, used);
                    }
                }
                Expr::ArrayLiteral(elements) => {
                    for elem in elements {
                        match elem {
                            ArrayElement::Single(e) | ArrayElement::Spread(e) => {
                                collect_expr_vars(e, used);
                            }
                        }
                    }
                }
                Expr::DictLiteral(entries) => {
                    for entry in entries {
                        match entry {
                            DictElement::Pair(k, v) => {
                                collect_expr_vars(k, used);
                                collect_expr_vars(v, used);
                            }
                            DictElement::Spread(e) => {
                                collect_expr_vars(e, used);
                            }
                        }
                    }
                }
                Expr::IndexAccess { object, index } => {
                    collect_expr_vars(object, used);
                    collect_expr_vars(index, used);
                }
                Expr::FieldAccess { object, .. } => {
                    collect_expr_vars(object, used);
                }
                Expr::Function { body, .. } => {
                    // Don't descend into nested functions - they have their own scope
                    for stmt in body {
                        collect_stmt_vars(stmt, used, &mut HashSet::new());
                    }
                }
                Expr::Ok(e) | Expr::Err(e) | Expr::Some(e) | Expr::Await(e) => {
                    collect_expr_vars(e, used);
                }
                Expr::Yield(Some(e)) => {
                    collect_expr_vars(e, used);
                }
                Expr::Try(e) => {
                    collect_expr_vars(e, used);
                }
                Expr::StructInstance { fields, .. } => {
                    for (_, expr) in fields {
                        collect_expr_vars(expr, used);
                    }
                }
                Expr::InterpolatedString(parts) => {
                    for part in parts {
                        if let crate::ast::InterpolatedStringPart::Expr(e) = part {
                            collect_expr_vars(e, used);
                        }
                    }
                }
                _ => {}
            }
        }

        // Helper function to collect variable definitions and usage from statements
        fn collect_stmt_vars(
            stmt: &Stmt,
            used: &mut HashSet<String>,
            defined: &mut HashSet<String>,
        ) {
            match stmt {
                Stmt::Let { pattern, value, .. } => {
                    collect_expr_vars(value, used);
                    // Add defined variables from pattern
                    if let Pattern::Identifier(name) = pattern {
                        defined.insert(name.clone());
                    }
                }
                Stmt::Assign { target, value } => {
                    collect_expr_vars(value, used);
                    match target {
                        Expr::Identifier(name) => {
                            // Assignment can mutate an outer captured binding.
                            // Treat identifier targets as usage so free-variable
                            // analysis preserves closure capture when needed.
                            used.insert(name.clone());
                        }
                        Expr::IndexAccess { object, index } => {
                            collect_expr_vars(object, used);
                            collect_expr_vars(index, used);
                        }
                        Expr::FieldAccess { object, .. } => {
                            collect_expr_vars(object, used);
                        }
                        _ => {
                            collect_expr_vars(target, used);
                        }
                    }
                }
                Stmt::ExprStmt(expr) => {
                    collect_expr_vars(expr, used);
                }
                Stmt::If { condition, then_branch, else_branch } => {
                    collect_expr_vars(condition, used);
                    for s in then_branch {
                        collect_stmt_vars(s, used, defined);
                    }
                    if let Some(else_stmts) = else_branch {
                        for s in else_stmts {
                            collect_stmt_vars(s, used, defined);
                        }
                    }
                }
                Stmt::While { condition, body } => {
                    collect_expr_vars(condition, used);
                    for s in body {
                        collect_stmt_vars(s, used, defined);
                    }
                }
                Stmt::For { var, iterable, body } => {
                    collect_expr_vars(iterable, used);
                    defined.insert(var.clone());
                    for s in body {
                        collect_stmt_vars(s, used, defined);
                    }
                }
                Stmt::Return(expr) => {
                    if let Some(e) = expr {
                        collect_expr_vars(e, used);
                    }
                }
                Stmt::Break | Stmt::Continue => {}
                Stmt::Match { value, cases, default } => {
                    collect_expr_vars(value, used);
                    for (_pattern, stmts) in cases {
                        for s in stmts {
                            collect_stmt_vars(s, used, defined);
                        }
                    }
                    if let Some(default_stmts) = default {
                        for s in default_stmts {
                            collect_stmt_vars(s, used, defined);
                        }
                    }
                }
                Stmt::FuncDef { name, body, .. } => {
                    defined.insert(name.clone());
                    // Don't descend into nested function bodies
                    for s in body {
                        collect_stmt_vars(s, used, &mut HashSet::new());
                    }
                }
                Stmt::TryExcept { try_block, except_block, .. } => {
                    for s in try_block {
                        collect_stmt_vars(s, used, defined);
                    }
                    for s in except_block {
                        collect_stmt_vars(s, used, defined);
                    }
                }
                Stmt::Block(stmts) => {
                    for s in stmts {
                        collect_stmt_vars(s, used, defined);
                    }
                }
                Stmt::Const { name, value, .. } => {
                    defined.insert(name.clone());
                    collect_expr_vars(value, used);
                }
                Stmt::Export { stmt } => {
                    collect_stmt_vars(stmt, used, defined);
                }
                Stmt::Spawn { body } => {
                    for s in body {
                        collect_stmt_vars(s, used, defined);
                    }
                }
                Stmt::StructDef { name, .. } => {
                    defined.insert(name.clone());
                }
                Stmt::EnumDef { name, .. } => {
                    defined.insert(name.clone());
                }
                Stmt::Import { module, symbols } => {
                    // Module itself becomes a variable
                    defined.insert(module.clone());
                    // Imported symbols also become variables
                    if let Some(syms) = symbols {
                        for sym in syms {
                            defined.insert(sym.clone());
                        }
                    }
                }
                Stmt::Loop { condition, body } => {
                    if let Some(cond) = condition {
                        collect_expr_vars(cond, used);
                    }
                    for s in body {
                        collect_stmt_vars(s, used, defined);
                    }
                }
                Stmt::Test { body, .. }
                | Stmt::TestSetup { body }
                | Stmt::TestTeardown { body } => {
                    for s in body {
                        collect_stmt_vars(s, used, defined);
                    }
                }
                Stmt::TestGroup { tests, .. } => {
                    for s in tests {
                        collect_stmt_vars(s, used, defined);
                    }
                }
            }
        }

        // Collect all variable usage and definitions
        for stmt in body {
            collect_stmt_vars(stmt, &mut used_vars, &mut defined_vars);
        }

        // Free variables are those used but not defined locally
        let mut free_vars: Vec<String> = used_vars.difference(&defined_vars).cloned().collect();

        // Sort for deterministic output
        free_vars.sort();
        free_vars
    }
}
