//! Conservative proof that a body cannot observe its captured environment.
//! Unknown syntax falls back to ordinary capture setup. Operator and iteration
//! paths additionally require primitive arguments: struct operators and generators
//! can invoke user code even without an explicit Call node.

use crate::ast::{ArrayElement, DictElement, Expr, InterpolatedStringPart, Pattern, Stmt};
use std::collections::{BTreeSet, HashSet};

#[derive(Clone)]
pub(super) struct Proof {
    pub required_parameters: Vec<String>,
    pub primitive_arguments: bool,
    pub read_only: bool,
    pub whole_value_mask: u32,
    pub sparse_write_region: bool,
}

struct Analyzer<'a> {
    scopes: Vec<HashSet<&'a str>>,
    required: BTreeSet<&'a str>,
    primitive_arguments: bool,
    read_only: bool,
}

pub(super) fn analyze(body: &[Stmt]) -> Option<Proof> {
    let mut analyzer = Analyzer {
        scopes: vec![HashSet::new()],
        required: BTreeSet::new(),
        primitive_arguments: false,
        read_only: true,
    };
    if !analyzer.statements(body, 0) {
        return None;
    }
    let required_parameters: Vec<String> =
        analyzer.required.into_iter().map(str::to_owned).collect();
    let mut region = SparseRegion { names: &required_parameters, whole_value_mask: 0 };
    let sparse_write_region = required_parameters.len() <= 32 && region.statements(body, 0);
    Some(Proof {
        whole_value_mask: region.whole_value_mask,
        sparse_write_region,
        required_parameters,
        primitive_arguments: analyzer.primitive_arguments,
        read_only: analyzer.read_only,
    })
}

// This is a profitability guard, not permission to change publication order.
// Reuse the existing proof's padding and names; no per-body allocation is added.
// Whole captured-value reads and opaque operations keep the single-key path.
struct SparseRegion<'a> {
    names: &'a [String],
    whole_value_mask: u32,
}

impl SparseRegion<'_> {
    fn expression(&mut self, expr: &Expr, depth: usize) -> bool {
        if depth > 128 {
            return false;
        }
        match expr {
            // The evaluator treats this spelling as a literal before lexical lookup.
            Expr::Identifier(name) if name == "null" => true,
            Expr::Identifier(name) => {
                if let Some(index) = self.names.iter().position(|value| value == name) {
                    let Some(bit) = 1u32.checked_shl(index as u32) else { return false };
                    self.whole_value_mask |= bit;
                }
                true
            }
            Expr::Int(_) | Expr::Float(_) | Expr::String(_) | Expr::Bool(_) => true,
            Expr::UnaryOp { operand, .. } => self.expression(operand, depth + 1),
            Expr::BinaryOp { left, right, .. } => {
                self.expression(left, depth + 1) && self.expression(right, depth + 1)
            }
            _ => false,
        }
    }

    // Indexed values have unknown runtime types and can invoke struct operators.
    // Permit an indexed destination or direct return, but do not feed such a
    // value into arithmetic or a local later used by arithmetic.
    fn indexed_access(&mut self, expr: &Expr, depth: usize) -> bool {
        if depth > 128 {
            return false;
        }
        match expr {
            Expr::IndexAccess { object, index }
                if matches!(object.as_ref(), Expr::Identifier(_)) =>
            {
                self.expression(index, depth + 1)
            }
            _ => self.expression(expr, depth + 1),
        }
    }

    fn statements(&mut self, stmts: &[Stmt], depth: usize) -> bool {
        if depth > 128 {
            return false;
        }
        stmts.iter().all(|stmt| match stmt {
            Stmt::Let { value, type_annotation: None, .. }
            | Stmt::Const { value, type_annotation: None, .. } => self.expression(value, depth + 1),
            Stmt::Assign { target, value } => {
                self.indexed_access(target, depth + 1) && self.expression(value, depth + 1)
            }
            Stmt::ExprStmt(expr) => self.expression(expr, depth + 1),
            Stmt::Return(Some(expr)) => self.indexed_access(expr, depth + 1),
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => true,
            Stmt::If { condition, then_branch, else_branch } => {
                self.expression(condition, depth + 1)
                    && self.statements(then_branch, depth + 1)
                    && else_branch.as_ref().is_none_or(|body| self.statements(body, depth + 1))
            }
            Stmt::While { condition, body } => {
                self.expression(condition, depth + 1) && self.statements(body, depth + 1)
            }
            Stmt::Loop { condition, body } => {
                condition.as_ref().is_none_or(|value| self.expression(value, depth + 1))
                    && self.statements(body, depth + 1)
            }
            Stmt::Block(body) => self.statements(body, depth + 1),
            _ => false,
        })
    }
}

impl<'a> Analyzer<'a> {
    fn reference(&mut self, name: &'a str) {
        if !self.scopes.iter().rev().any(|scope| scope.contains(name)) {
            self.required.insert(name);
        }
    }

    fn pattern(&mut self, pattern: &'a Pattern, depth: usize) -> bool {
        if depth > 128 {
            return false;
        }
        match pattern {
            Pattern::Identifier(name) => {
                self.scopes.last_mut().unwrap().insert(name);
            }
            Pattern::Ignore => {}
            Pattern::Array { elements, rest } => {
                if !elements.iter().all(|p| self.pattern(p, depth + 1)) {
                    return false;
                }
                if let Some(name) = rest {
                    self.scopes.last_mut().unwrap().insert(name);
                }
            }
            Pattern::Dict { keys, rest } => {
                self.scopes.last_mut().unwrap().extend(keys.iter().map(String::as_str));
                if let Some(name) = rest {
                    self.scopes.last_mut().unwrap().insert(name);
                }
            }
        }
        true
    }

    fn expression(&mut self, expression: &'a Expr, depth: usize) -> bool {
        if depth > 128 {
            return false;
        }
        match expression {
            Expr::Identifier(name) => {
                self.reference(name);
                true
            }
            Expr::Int(_) | Expr::Float(_) | Expr::String(_) | Expr::Bool(_) | Expr::None => true,
            Expr::UnaryOp { operand, .. } => {
                self.primitive_arguments = true;
                self.expression(operand, depth + 1)
            }
            Expr::BinaryOp { left, right, .. } => {
                self.primitive_arguments = true;
                self.expression(left, depth + 1) && self.expression(right, depth + 1)
            }
            Expr::IndexAccess { object, index } => {
                self.expression(object, depth + 1) && self.expression(index, depth + 1)
            }
            Expr::FieldAccess { object, .. } => self.expression(object, depth + 1),
            Expr::Ok(value) | Expr::Err(value) | Expr::Some(value) | Expr::Try(value) => {
                self.expression(value, depth + 1)
            }
            Expr::InterpolatedString(parts) => {
                self.primitive_arguments = true;
                parts.iter().all(|part| match part {
                    InterpolatedStringPart::Text(_) => true,
                    InterpolatedStringPart::Expr(value) => self.expression(value, depth + 1),
                })
            }
            Expr::ArrayLiteral(elements) => elements.iter().all(|element| match element {
                ArrayElement::Single(value) => self.expression(value, depth + 1),
                ArrayElement::Spread(_) => false,
            }),
            Expr::DictLiteral(elements) => elements.iter().all(|element| match element {
                DictElement::Pair(key, value) => {
                    self.expression(key, depth + 1) && self.expression(value, depth + 1)
                }
                DictElement::Spread(_) => false,
            }),
            // Calls, methods, constructors, closures and suspension can observe
            // lexical state or execute callbacks. Never infer purity from names.
            _ => false,
        }
    }

    fn scoped(&mut self, body: &'a [Stmt], binding: Option<&'a str>, depth: usize) -> bool {
        self.scopes.push(HashSet::new());
        if let Some(name) = binding {
            self.scopes.last_mut().unwrap().insert(name);
        }
        let result = self.statements(body, depth + 1);
        self.scopes.pop();
        result
    }

    fn statements(&mut self, statements: &'a [Stmt], depth: usize) -> bool {
        if depth > 128 {
            return false;
        }
        statements.iter().all(|statement| match statement {
            Stmt::Let { pattern, value, type_annotation: None, .. } => {
                self.expression(value, depth + 1) && self.pattern(pattern, depth + 1)
            }
            Stmt::Const { name, value, type_annotation: None } => {
                if !self.expression(value, depth + 1) {
                    return false;
                }
                self.scopes.last_mut().unwrap().insert(name);
                true
            }
            Stmt::Assign { target, value } => {
                self.read_only = false;
                self.expression(target, depth + 1) && self.expression(value, depth + 1)
            }
            Stmt::ExprStmt(value) | Stmt::Return(Some(value)) => self.expression(value, depth + 1),
            Stmt::Return(None) | Stmt::Break | Stmt::Continue => true,
            Stmt::If { condition, then_branch, else_branch } => {
                self.expression(condition, depth + 1)
                    && self.scoped(then_branch, None, depth)
                    && else_branch.as_ref().is_none_or(|body| self.scoped(body, None, depth))
            }
            Stmt::While { condition, body } => {
                self.expression(condition, depth + 1) && self.scoped(body, None, depth)
            }
            Stmt::Loop { condition, body } => {
                condition.as_ref().is_none_or(|value| self.expression(value, depth + 1))
                    && self.scoped(body, None, depth)
            }
            Stmt::For { var, iterable, body } => {
                self.primitive_arguments = true;
                self.expression(iterable, depth + 1) && self.scoped(body, Some(var), depth)
            }
            // Catch constructs a named Error struct, whose operators can resolve
            // through the lexical environment even with primitive arguments.
            Stmt::TryExcept { .. } => false,
            Stmt::Block(body) => self.scoped(body, None, depth),
            _ => false,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::interpreter::value::{LeakyFunctionBody, Value};
    use std::sync::Arc;

    fn body(source: &str) -> LeakyFunctionBody {
        let parsed = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap())
            .parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        LeakyFunctionBody::new(parsed.stmts)
    }

    fn eligible(source: &str, names: &[&str], args: &[Value]) -> bool {
        body(source).get().can_skip_capture_setup(
            &names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>(),
            args,
        )
    }

    #[test]
    fn projection_accepts_containers_without_inspecting_or_copying_elements() {
        let value = Value::Array(Arc::new(vec![Value::Struct {
            name: "Box".into(),
            fields: Arc::new(Default::default()),
        }]));
        assert!(eligible("return items[0]", &["items"], &[value]));
        assert!(eligible(
            "items[0] = value return items",
            &["items", "value"],
            &[Value::Array(Arc::new(vec![Value::Int(1)])), Value::Int(9)]
        ));
    }

    #[test]
    fn operators_require_primitive_arguments_even_inside_containers() {
        let source = "return a + b";
        assert!(eligible(source, &["a", "b"], &[Value::Int(1), Value::Int(2)]));
        let custom = Value::Struct { name: "Box".into(), fields: Arc::new(Default::default()) };
        assert!(!eligible(source, &["a", "b"], &[custom.clone(), custom.clone()]));
        assert!(!eligible(
            "return items[0] + 1",
            &["items"],
            &[Value::Array(Arc::new(vec![custom]))]
        ));
    }

    #[test]
    fn cached_proof_rechecks_actual_callable_parameter_names_and_argument_kinds() {
        let function = body("return a + 1");
        let body = function.get();
        assert!(body.can_skip_capture_setup(&["a".into()], &[Value::Int(1)]));
        assert!(!body.can_skip_capture_setup(&["b".into()], &[Value::Int(1)]));
        assert!(!body.can_skip_capture_setup(&["a".into()], &[Value::Array(Arc::new(vec![]))]));
        assert!(body.can_skip_capture_setup(&["a".into()], &[Value::Int(2)]));
    }

    #[test]
    fn local_loops_are_independent_but_scope_exit_does_not_hide_globals() {
        assert!(eligible(
            "mut total := 0 mut i := 0 while i < n { total += i i += 1 } return total",
            &["n"],
            &[Value::Int(10)]
        ));
        for source in [
            "if flag { let hidden := 1 } return hidden",
            "let hidden := hidden return hidden",
            "global = value return value",
            "return value + offset",
        ] {
            assert!(
                !eligible(source, &["flag", "value"], &[Value::Bool(true), Value::Int(1)]),
                "{source}"
            );
        }
    }

    #[test]
    fn callback_and_implicit_named_error_paths_keep_capture_setup() {
        for source in [
            "return f(x)",
            "return x.method()",
            "return Box { value: x }",
            "return func() { return x }",
            "try { return x } except e { return e + e }",
            "let x: int := 1 return x",
            "return [ ...x ]",
        ] {
            assert!(!eligible(source, &["f", "x"], &[Value::Int(1), Value::Int(2)]), "{source}");
        }
        assert!(!eligible(
            "for x in items { return x }",
            &["items"],
            &[Value::Array(Arc::new(vec![Value::Int(1)]))]
        ));
    }
}

#[cfg(test)]
mod read_projection_tests {
    use super::*;
    fn proof(source: &str) -> Option<Proof> {
        let parsed = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap())
            .parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty());
        analyze(&parsed.stmts)
    }
    #[test]
    fn read_projection_rejects_all_assignments_and_callback_paths() {
        for source in ["state[0] = 1 return state", "mut x := 0 x += 1 return x"] {
            assert!(!proof(source).unwrap().read_only);
        }
        for source in ["return helper()", "return state.method()", "return func() { return state }"]
        {
            assert!(proof(source).is_none());
        }
    }
    #[test]
    fn read_dependencies_keep_outer_reads_and_exclude_local_names() {
        let p = proof("let copied := state return copied[index]").unwrap();
        assert!(p.read_only);
        assert_eq!(p.required_parameters, vec!["index", "state"]);
        assert!(!p.primitive_arguments);
        assert!(proof("return state + index").unwrap().primitive_arguments);
    }
}

#[cfg(test)]
mod sparse_region_tests {
    use super::Proof;
    use crate::interpreter::value::{LeakyFunctionBody, Value};
    use std::sync::OnceLock;

    fn eligible(source: &str, params: &[&str], args: &[Value]) -> bool {
        let parsed = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap())
            .parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty());
        LeakyFunctionBody::new(parsed.stmts).get().can_batch_dictionary_writes(
            &params.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>(),
            args,
        )
    }

    #[test]
    fn sparse_region_keeps_existing_cached_proof_layout() {
        #[allow(dead_code)]
        struct Previous {
            required_parameters: Vec<String>,
            primitive_arguments: bool,
            read_only: bool,
        }
        assert_eq!(std::mem::size_of::<Previous>(), std::mem::size_of::<Proof>());
        assert_eq!(std::mem::align_of::<Previous>(), std::mem::align_of::<Proof>());
        assert_eq!(
            std::mem::size_of::<OnceLock<Option<Previous>>>(),
            std::mem::size_of::<OnceLock<Option<Proof>>>()
        );
    }

    #[test]
    fn sparse_region_accepts_entry_updates_and_checks_actual_arguments() {
        let source = "mut key := 0 while key < n { state[key] = n + key key += 1 } return state[0]";
        assert!(eligible(source, &["n"], &[Value::Int(16)]));
        assert!(!eligible(source, &[], &[]));
        assert!(!eligible(source, &["n"], &[Value::array(vec![])]));
        assert!(eligible(
            "state[0] = n if n > 0 { state[1] = false } else { state[1] = null } return state[0]",
            &["n"],
            &[Value::Int(1)]
        ));
    }

    #[test]
    fn sparse_region_rejects_observation_and_opaque_boundaries() {
        for tail in [
            "return state",
            "let saved := state state[0] = n return saved",
            "return [state[0]]",
            "opaque() return state[0]",
            "return state[state]",
            "return state.row",
            "state[2] = None return state[0]",
            "return state[2] + n",
            "let value := state[2] return value + n",
            "state[2] += n return state[0]",
            "return -state[2]",
            "if false { let saved := state } return state[0]",
        ] {
            assert!(
                !eligible(&format!("state[0] = n state[1] = n {tail}"), &["n"], &[Value::Int(1)]),
                "{tail}"
            );
        }
        let many = (0..40).map(|i| format!("x{i}")).collect::<Vec<_>>().join(" + ");
        assert!(!eligible(&format!("state[0] = 1 state[1] = 2 return {many}"), &[], &[]));
    }
}
