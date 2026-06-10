// File: src/optimizer.rs
//
// Bytecode optimization passes for the Kujo VM.
// Implements constant folding, dead code elimination, peephole optimizations,
// and other performance improvements.

use crate::bytecode::{BytecodeChunk, Constant, OpCode};
use std::collections::HashMap;

/// Main optimizer for bytecode chunks
pub struct Optimizer {
    /// Statistics about optimizations performed
    pub stats: OptimizationStats,
}

/// Statistics tracking what optimizations were performed
#[derive(Debug, Default, Clone)]
pub struct OptimizationStats {
    pub constants_folded: usize,
    pub dead_instructions_removed: usize,
    pub peephole_optimizations: usize,
    pub total_instructions_before: usize,
    pub total_instructions_after: usize,
}

impl Optimizer {
    pub fn new() -> Self {
        Self { stats: OptimizationStats::default() }
    }

    /// Run all optimization passes on a bytecode chunk
    pub fn optimize(&mut self, chunk: &mut BytecodeChunk) {
        self.stats.total_instructions_before = chunk.instructions.len();

        // Pass 1: Constant folding
        self.constant_folding_pass(chunk);

        // Pass 2: Dead code elimination
        self.dead_code_elimination_pass(chunk);

        // Pass 3: Peephole optimizations
        self.peephole_optimization_pass(chunk);

        // Also optimize nested functions in constants
        for constant in &mut chunk.constants {
            if let Constant::Function(func_chunk) = constant {
                self.optimize(func_chunk);
            }
        }

        self.stats.total_instructions_after = chunk.instructions.len();
    }

    /// Pass 1: Constant Folding
    /// Evaluates constant expressions at compile time
    fn constant_folding_pass(&mut self, chunk: &mut BytecodeChunk) {
        let mut new_instructions = Vec::new();
        let mut i = 0;

        while i < chunk.instructions.len() {
            // Look for pattern: LoadConst, LoadConst, BinaryOp
            if i + 2 < chunk.instructions.len() {
                if let (OpCode::LoadConst(idx1), OpCode::LoadConst(idx2), binary_op) =
                    (&chunk.instructions[i], &chunk.instructions[i + 1], &chunk.instructions[i + 2])
                {
                    // Try to fold constants
                    if let Some(folded) = self.try_fold_binary_op(
                        &chunk.constants[*idx1],
                        &chunk.constants[*idx2],
                        binary_op,
                    ) {
                        // Add the folded constant
                        let new_idx = chunk.add_constant(folded);
                        new_instructions.push(OpCode::LoadConst(new_idx));

                        self.stats.constants_folded += 1;
                        i += 3; // Skip the three instructions we just folded
                        continue;
                    }
                }
            }

            // Look for pattern: LoadConst, UnaryOp (like Negate, Not)
            if i + 1 < chunk.instructions.len() {
                if let (OpCode::LoadConst(idx), unary_op) =
                    (&chunk.instructions[i], &chunk.instructions[i + 1])
                {
                    if let Some(folded) = self.try_fold_unary_op(&chunk.constants[*idx], unary_op) {
                        let new_idx = chunk.add_constant(folded);
                        new_instructions.push(OpCode::LoadConst(new_idx));

                        self.stats.constants_folded += 1;
                        i += 2;
                        continue;
                    }
                }
            }

            // No folding possible, keep instruction as-is
            new_instructions.push(chunk.instructions[i].clone());
            i += 1;
        }

        chunk.instructions = new_instructions;
    }

    /// Try to fold a binary operation on two constants
    fn try_fold_binary_op(
        &self,
        left: &Constant,
        right: &Constant,
        op: &OpCode,
    ) -> Option<Constant> {
        match (left, right, op) {
            // Integer arithmetic
            (Constant::Int(a), Constant::Int(b), OpCode::Add) => {
                a.checked_add(*b).map(Constant::Int)
            }
            (Constant::Int(a), Constant::Int(b), OpCode::Sub) => {
                a.checked_sub(*b).map(Constant::Int)
            }
            (Constant::Int(a), Constant::Int(b), OpCode::Mul) => {
                a.checked_mul(*b).map(Constant::Int)
            }
            (Constant::Int(a), Constant::Int(b), OpCode::Div) if *b != 0 => {
                a.checked_div(*b).map(Constant::Int)
            }
            (Constant::Int(a), Constant::Int(b), OpCode::Mod) if *b != 0 => {
                a.checked_rem(*b).map(Constant::Int)
            }

            // Float arithmetic
            (Constant::Float(a), Constant::Float(b), OpCode::Add) => Some(Constant::Float(a + b)),
            (Constant::Float(a), Constant::Float(b), OpCode::Sub) => Some(Constant::Float(a - b)),
            (Constant::Float(a), Constant::Float(b), OpCode::Mul) => Some(Constant::Float(a * b)),
            (Constant::Float(a), Constant::Float(b), OpCode::Div) if *b != 0.0 => {
                Some(Constant::Float(a / b))
            }
            (Constant::Float(a), Constant::Float(b), OpCode::Mod) if *b != 0.0 => {
                Some(Constant::Float(a % b))
            }

            // Mixed int/float arithmetic (promote to float)
            (Constant::Int(a), Constant::Float(b), OpCode::Add) => {
                Some(Constant::Float(*a as f64 + b))
            }
            (Constant::Float(a), Constant::Int(b), OpCode::Add) => {
                Some(Constant::Float(a + *b as f64))
            }
            (Constant::Int(a), Constant::Float(b), OpCode::Sub) => {
                Some(Constant::Float(*a as f64 - b))
            }
            (Constant::Float(a), Constant::Int(b), OpCode::Sub) => {
                Some(Constant::Float(a - *b as f64))
            }
            (Constant::Int(a), Constant::Float(b), OpCode::Mul) => {
                Some(Constant::Float(*a as f64 * b))
            }
            (Constant::Float(a), Constant::Int(b), OpCode::Mul) => {
                Some(Constant::Float(a * *b as f64))
            }
            (Constant::Int(a), Constant::Float(b), OpCode::Div) if *b != 0.0 => {
                Some(Constant::Float(*a as f64 / b))
            }
            (Constant::Float(a), Constant::Int(b), OpCode::Div) if *b != 0 => {
                Some(Constant::Float(a / *b as f64))
            }

            // String concatenation
            (Constant::String(a), Constant::String(b), OpCode::Add) => {
                Some(Constant::String(format!("{}{}", a, b)))
            }

            // Boolean operations
            (Constant::Bool(a), Constant::Bool(b), OpCode::And) => Some(Constant::Bool(*a && *b)),
            (Constant::Bool(a), Constant::Bool(b), OpCode::Or) => Some(Constant::Bool(*a || *b)),

            // Comparison operations on integers
            (Constant::Int(a), Constant::Int(b), OpCode::Equal) => Some(Constant::Bool(a == b)),
            (Constant::Int(a), Constant::Int(b), OpCode::NotEqual) => Some(Constant::Bool(a != b)),
            (Constant::Int(a), Constant::Int(b), OpCode::LessThan) => Some(Constant::Bool(a < b)),
            (Constant::Int(a), Constant::Int(b), OpCode::GreaterThan) => {
                Some(Constant::Bool(a > b))
            }
            (Constant::Int(a), Constant::Int(b), OpCode::LessEqual) => Some(Constant::Bool(a <= b)),
            (Constant::Int(a), Constant::Int(b), OpCode::GreaterEqual) => {
                Some(Constant::Bool(a >= b))
            }

            // Comparison operations on floats
            (Constant::Float(a), Constant::Float(b), OpCode::Equal) => Some(Constant::Bool(a == b)),
            (Constant::Float(a), Constant::Float(b), OpCode::NotEqual) => {
                Some(Constant::Bool(a != b))
            }
            (Constant::Float(a), Constant::Float(b), OpCode::LessThan) => {
                Some(Constant::Bool(a < b))
            }
            (Constant::Float(a), Constant::Float(b), OpCode::GreaterThan) => {
                Some(Constant::Bool(a > b))
            }
            (Constant::Float(a), Constant::Float(b), OpCode::LessEqual) => {
                Some(Constant::Bool(a <= b))
            }
            (Constant::Float(a), Constant::Float(b), OpCode::GreaterEqual) => {
                Some(Constant::Bool(a >= b))
            }

            // Comparison operations on booleans
            (Constant::Bool(a), Constant::Bool(b), OpCode::Equal) => Some(Constant::Bool(a == b)),
            (Constant::Bool(a), Constant::Bool(b), OpCode::NotEqual) => {
                Some(Constant::Bool(a != b))
            }

            // Comparison operations on strings
            (Constant::String(a), Constant::String(b), OpCode::Equal) => {
                Some(Constant::Bool(a == b))
            }
            (Constant::String(a), Constant::String(b), OpCode::NotEqual) => {
                Some(Constant::Bool(a != b))
            }

            _ => None,
        }
    }

    /// Try to fold a unary operation on a constant
    fn try_fold_unary_op(&self, operand: &Constant, op: &OpCode) -> Option<Constant> {
        match (operand, op) {
            (Constant::Int(n), OpCode::Negate) => n.checked_neg().map(Constant::Int),
            (Constant::Float(f), OpCode::Negate) => Some(Constant::Float(-f)),
            (Constant::Bool(b), OpCode::Not) => Some(Constant::Bool(!b)),
            _ => None,
        }
    }

    /// Pass 2: Dead Code Elimination
    /// Removes unreachable instructions
    fn dead_code_elimination_pass(&mut self, chunk: &mut BytecodeChunk) {
        let mut reachable = vec![false; chunk.instructions.len()];

        // Mark all reachable instructions starting from entry point
        self.mark_reachable(chunk, &mut reachable, 0);

        // Also mark all exception handler entry points as reachable
        for handler in &chunk.exception_handlers {
            self.mark_reachable(chunk, &mut reachable, handler.catch_start);
        }

        // Build a mapping from old instruction indices to new ones
        let mut index_map = HashMap::new();
        let mut new_instructions = Vec::new();
        let mut new_index = 0;

        for (old_index, instruction) in chunk.instructions.iter().enumerate() {
            if reachable[old_index] {
                index_map.insert(old_index, new_index);
                new_instructions.push(instruction.clone());
                new_index += 1;
            } else {
                self.stats.dead_instructions_removed += 1;
            }
        }

        // Update all jump targets to use new indices
        for instruction in &mut new_instructions {
            match instruction {
                OpCode::Jump(ref mut target)
                | OpCode::JumpIfFalse(ref mut target)
                | OpCode::JumpIfTrue(ref mut target)
                | OpCode::JumpBack(ref mut target)
                | OpCode::BeginTry(ref mut target) => {
                    if let Some(&new_target) = index_map.get(target) {
                        *target = new_target;
                    }
                }
                _ => {}
            }
        }

        // Update exception handler indices
        for handler in &mut chunk.exception_handlers {
            if let Some(&new_start) = index_map.get(&handler.try_start) {
                handler.try_start = new_start;
            }
            if let Some(&new_end) = index_map.get(&handler.try_end) {
                handler.try_end = new_end;
            }
            if let Some(&new_catch) = index_map.get(&handler.catch_start) {
                handler.catch_start = new_catch;
            }
        }

        chunk.instructions = new_instructions;
    }

    /// Mark all reachable instructions starting from a given index
    fn mark_reachable(&self, chunk: &BytecodeChunk, reachable: &mut [bool], start: usize) {
        if start >= chunk.instructions.len() || reachable[start] {
            return; // Already visited or out of bounds
        }

        let mut i = start;
        while i < chunk.instructions.len() {
            if reachable[i] {
                break; // Already visited this path
            }

            reachable[i] = true;

            match &chunk.instructions[i] {
                // Unconditional jump - follow it, stop current path
                OpCode::Jump(target) => {
                    self.mark_reachable(chunk, reachable, *target);
                    break;
                }

                // Conditional jumps - follow both paths
                OpCode::JumpIfFalse(target) | OpCode::JumpIfTrue(target) => {
                    self.mark_reachable(chunk, reachable, *target);
                    // Continue with next instruction (fall-through)
                }

                // Backward jump - follow it
                OpCode::JumpBack(target) => {
                    self.mark_reachable(chunk, reachable, *target);
                    // Continue with next instruction
                }

                // Try block - mark catch block as reachable
                OpCode::BeginTry(catch_target) => {
                    self.mark_reachable(chunk, reachable, *catch_target);
                    // Continue with next instruction
                }

                // Return/Throw stop execution in current block
                OpCode::Return | OpCode::ReturnNone | OpCode::Throw => {
                    break;
                }

                _ => {
                    // Regular instruction, continue to next
                }
            }

            i += 1;
        }
    }

    /// Pass 3: Peephole Optimizations
    /// Optimizes small sequences of instructions
    fn peephole_optimization_pass(&mut self, chunk: &mut BytecodeChunk) {
        let mut new_instructions = Vec::new();
        let mut i = 0;

        while i < chunk.instructions.len() {
            let mut optimized = false;

            // Pattern 1: LoadConst followed by Pop (useless load)
            if i + 1 < chunk.instructions.len() {
                if matches!(chunk.instructions[i], OpCode::LoadConst(_))
                    && matches!(chunk.instructions[i + 1], OpCode::Pop)
                {
                    // Skip both instructions
                    self.stats.peephole_optimizations += 1;
                    i += 2;
                    optimized = true;
                }
            }

            // Pattern 2: StoreVar followed by LoadVar of same variable
            if !optimized && i + 1 < chunk.instructions.len() {
                if let (OpCode::StoreVar(var1), OpCode::LoadVar(var2)) =
                    (&chunk.instructions[i], &chunk.instructions[i + 1])
                {
                    if var1 == var2 {
                        // Replace with: StoreVar, Dup (to leave value on stack)
                        // Actually, StoreVar consumes the value, so we need: Dup, StoreVar
                        // But the original pattern already stored it, so just add a Load back
                        // Keep both for now - this is a minor optimization
                        // Better approach: Dup before StoreVar
                        new_instructions.push(OpCode::Dup);
                        new_instructions.push(chunk.instructions[i].clone());
                        self.stats.peephole_optimizations += 1;
                        i += 2;
                        optimized = true;
                    }
                }
            }

            // Pattern 3: Pop followed by Pop -> could combine but not worth complexity
            // Pattern 4: Double Jump optimization
            if !optimized && i + 1 < chunk.instructions.len() {
                if let (OpCode::Jump(target1), _) =
                    (&chunk.instructions[i], &chunk.instructions[i + 1])
                {
                    // Check if target is also a jump
                    if *target1 < chunk.instructions.len() {
                        if let OpCode::Jump(target2) = chunk.instructions[*target1] {
                            // Replace with direct jump to final target
                            new_instructions.push(OpCode::Jump(target2));
                            self.stats.peephole_optimizations += 1;
                            i += 1;
                            optimized = true;
                        }
                    }
                }
            }

            // Pattern 5: LoadVar immediately after StoreVar (value still on stack conceptually)
            // This is actually handled by pattern 2 above

            if !optimized {
                new_instructions.push(chunk.instructions[i].clone());
                i += 1;
            }
        }

        chunk.instructions = new_instructions;
    }

    /// Get a summary of optimization results
    /// Infrastructure for optimization reporting
    #[allow(dead_code)]
    pub fn summary(&self) -> String {
        let reduction = if self.stats.total_instructions_before > 0 {
            let diff = self.stats.total_instructions_before - self.stats.total_instructions_after;
            let percent = (diff as f64 / self.stats.total_instructions_before as f64) * 100.0;
            format!("{:.1}%", percent)
        } else {
            "0%".to_string()
        };

        format!(
            "Optimization Summary:\n\
             - Constants folded: {}\n\
             - Dead instructions removed: {}\n\
             - Peephole optimizations: {}\n\
             - Instructions: {} -> {} (reduced by {})",
            self.stats.constants_folded,
            self.stats.dead_instructions_removed,
            self.stats.peephole_optimizations,
            self.stats.total_instructions_before,
            self.stats.total_instructions_after,
            reduction
        )
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_folding_arithmetic() {
        let mut chunk = BytecodeChunk::new();

        // Create: 2 + 3 (should fold to 5)
        let idx1 = chunk.add_constant(Constant::Int(2));
        let idx2 = chunk.add_constant(Constant::Int(3));
        chunk.emit(OpCode::LoadConst(idx1));
        chunk.emit(OpCode::LoadConst(idx2));
        chunk.emit(OpCode::Add);

        let mut optimizer = Optimizer::new();
        optimizer.optimize(&mut chunk);

        // Should be optimized to single LoadConst(5)
        assert_eq!(chunk.instructions.len(), 1);
        assert!(matches!(chunk.instructions[0], OpCode::LoadConst(_)));
        assert_eq!(optimizer.stats.constants_folded, 1);
    }

    #[test]
    fn test_constant_folding_string_concat() {
        let mut chunk = BytecodeChunk::new();

        // Create: "hello" + " world"
        let idx1 = chunk.add_constant(Constant::String("hello".to_string()));
        let idx2 = chunk.add_constant(Constant::String(" world".to_string()));
        chunk.emit(OpCode::LoadConst(idx1));
        chunk.emit(OpCode::LoadConst(idx2));
        chunk.emit(OpCode::Add);

        let mut optimizer = Optimizer::new();
        optimizer.optimize(&mut chunk);

        assert_eq!(chunk.instructions.len(), 1);
        assert_eq!(optimizer.stats.constants_folded, 1);
    }

    #[test]
    fn test_constant_folding_boolean() {
        let mut chunk = BytecodeChunk::new();

        // Create: true && false (should fold to false)
        let idx1 = chunk.add_constant(Constant::Bool(true));
        let idx2 = chunk.add_constant(Constant::Bool(false));
        chunk.emit(OpCode::LoadConst(idx1));
        chunk.emit(OpCode::LoadConst(idx2));
        chunk.emit(OpCode::And);

        let mut optimizer = Optimizer::new();
        optimizer.optimize(&mut chunk);

        assert_eq!(chunk.instructions.len(), 1);
        assert_eq!(optimizer.stats.constants_folded, 1);
    }

    #[test]
    fn test_dead_code_elimination() {
        let mut chunk = BytecodeChunk::new();

        // Create code with unreachable instruction after return
        let idx = chunk.add_constant(Constant::Int(42));
        chunk.emit(OpCode::LoadConst(idx));
        chunk.emit(OpCode::Return);
        chunk.emit(OpCode::LoadConst(idx)); // Dead code
        chunk.emit(OpCode::Pop); // Dead code

        let mut optimizer = Optimizer::new();
        optimizer.optimize(&mut chunk);

        // Dead code after return should be removed
        assert!(optimizer.stats.dead_instructions_removed > 0);
    }

    #[test]
    fn test_peephole_load_pop() {
        let mut chunk = BytecodeChunk::new();

        // Create: LoadConst followed by Pop (useless)
        let idx = chunk.add_constant(Constant::Int(42));
        chunk.emit(OpCode::LoadConst(idx));
        chunk.emit(OpCode::Pop);

        let mut optimizer = Optimizer::new();
        optimizer.optimize(&mut chunk);

        // Both instructions should be removed
        assert_eq!(chunk.instructions.len(), 0);
        assert_eq!(optimizer.stats.peephole_optimizations, 1);
    }

    #[test]
    fn test_no_division_by_zero_folding() {
        let mut chunk = BytecodeChunk::new();

        // Create: 10 / 0 (should NOT fold - runtime error)
        let idx1 = chunk.add_constant(Constant::Int(10));
        let idx2 = chunk.add_constant(Constant::Int(0));
        chunk.emit(OpCode::LoadConst(idx1));
        chunk.emit(OpCode::LoadConst(idx2));
        chunk.emit(OpCode::Div);

        let mut optimizer = Optimizer::new();
        optimizer.optimize(&mut chunk);

        // Should NOT be optimized (div by zero)
        assert_eq!(chunk.instructions.len(), 3);
        assert_eq!(optimizer.stats.constants_folded, 0);
    }
}
