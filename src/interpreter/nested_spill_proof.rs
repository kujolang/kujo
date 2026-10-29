//! Profitability guard for nested dictionary overflow, separate from the direct
//! dictionary proof. It never grants permission to reorder observable execution.
use super::dictionary_publication::with_integer_key;
use super::environment::Environment;
use super::value::DictMap;
use super::Value;
use crate::ast::{Expr, Stmt};

/// Reserve hint and admission policy are separate: an unprofitable first insertion
/// must not disable batching when the executed branch first updates an existing key.
#[derive(Clone, Copy)]
pub(crate) struct SpillPolicy {
    pub(super) capacity: u16,
    pub(super) allow_initial_insertion: bool,
}
impl Default for SpillPolicy {
    fn default() -> Self {
        Self { capacity: 0, allow_initial_insertion: true }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]

enum Selector<'a> {
    Field(&'a str),
    String(&'a str),
    Integer(i64),
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Owner<'a> {
    name: &'a str,
    selector: Selector<'a>,
}

fn path(expr: &Expr) -> Option<(Owner<'_>, &Expr)> {
    let Expr::IndexAccess { object, index } = expr else { return None };
    let (name, selector) = match object.as_ref() {
        Expr::FieldAccess { object, field } => {
            let Expr::Identifier(name) = object.as_ref() else { return None };
            (name.as_str(), Selector::Field(field))
        }
        Expr::IndexAccess { object, index } => {
            let Expr::Identifier(name) = object.as_ref() else { return None };
            let selector = match index.as_ref() {
                Expr::Int(index) => Selector::Integer(*index),
                Expr::String(key) => Selector::String(key),
                _ => return None,
            };
            (name.as_str(), selector)
        }
        _ => return None,
    };
    Some((Owner { name, selector }, index))
}

fn first_owner(body: &[Stmt], depth: usize) -> Option<Owner<'_>> {
    if depth > 128 {
        return None;
    }
    for stmt in body {
        let owner = match stmt {
            Stmt::Assign { target, .. } => path(target).map(|(owner, _)| owner),
            Stmt::If { then_branch, else_branch, .. } => first_owner(then_branch, depth + 1)
                .or_else(|| else_branch.as_ref().and_then(|body| first_owner(body, depth + 1))),
            Stmt::Block(body) => first_owner(body, depth + 1),
            _ => None,
        };
        if owner.is_some() {
            return owner;
        }
    }
    None
}

fn scalar(value: &Value) -> bool {
    matches!(value, Value::Int(_) | Value::Float(_) | Value::Bool(_) | Value::Null)
}

struct Region<'a> {
    owner: Owner<'a>,
    runtime: Option<(&'a DictMap, &'a [String], &'a [Value], &'a Environment)>,
    writes: usize,
}

impl Region<'_> {
    fn argument(&self, name: &str) -> Option<&Value> {
        let (_, params, args, env) = self.runtime?;
        if let Some(index) = params.iter().position(|param| param == name) {
            args.get(index)
        } else {
            env.nested_spill_proof_value(name)
        }
    }

    fn indexed(&self, expr: &Expr, reading: bool) -> bool {
        let Some((owner, index)) = path(expr) else { return false };
        if owner != self.owner {
            return false;
        }
        match index {
            Expr::Int(index) => self.runtime.is_none_or(|(map, ..)| {
                !reading || with_integer_key(*index, |key| map.get(key).is_some_and(scalar))
            }),
            Expr::String(key) => self
                .runtime
                .is_none_or(|(map, ..)| !reading || map.get(key.as_str()).is_some_and(scalar)),
            Expr::Identifier(name) if name != self.owner.name => {
                let Some((map, ..)) = self.runtime else { return true };
                match self.argument(name) {
                    Some(Value::Int(index)) => {
                        !reading || with_integer_key(*index, |key| map.get(key).is_some_and(scalar))
                    }
                    Some(Value::Str(key)) => !reading || map.get(key.as_str()).is_some_and(scalar),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn expression(&self, expr: &Expr, depth: usize) -> bool {
        if depth > 128 {
            return false;
        }
        match expr {
            Expr::Int(_) | Expr::Float(_) | Expr::Bool(_) => true,
            Expr::Identifier(name) if name == "null" => true,
            Expr::Identifier(name) if name != self.owner.name => {
                self.runtime.is_none() || self.argument(name).is_some_and(scalar)
            }
            Expr::IndexAccess { .. } => self.indexed(expr, true),
            Expr::UnaryOp { operand, .. } => self.expression(operand, depth + 1),
            Expr::BinaryOp { left, right, .. } => {
                self.expression(left, depth + 1) && self.expression(right, depth + 1)
            }
            _ => false,
        }
    }

    fn statements(&mut self, body: &[Stmt], depth: usize) -> bool {
        if depth > 128 {
            return false;
        }
        body.iter().all(|stmt| match stmt {
            Stmt::Assign { target, value } => {
                self.writes += 1;
                self.indexed(target, false) && self.expression(value, depth + 1)
            }
            Stmt::Return(Some(value)) => self.expression(value, depth + 1),
            Stmt::Return(None) => true,
            Stmt::If { condition, then_branch, else_branch } => {
                self.expression(condition, depth + 1)
                    && self.statements(then_branch, depth + 1)
                    && else_branch.as_ref().is_none_or(|body| self.statements(body, depth + 1))
            }
            Stmt::Block(body) => self.statements(body, depth + 1),
            // Local mutation, loops, calls and uncertain syntax retain fallback.
            _ => false,
        })
    }
}

pub(super) fn shape(body: &[Stmt]) -> bool {
    let Some(owner) = first_owner(body, 0) else { return false };
    let mut region = Region { owner, runtime: None, writes: 0 };
    region.statements(body, 0) && region.writes > 4
}

pub(super) fn eligible(
    body: &[Stmt],
    params: &[String],
    args: &[Value],
    env: &Environment,
) -> SpillPolicy {
    let Some(owner) = first_owner(body, 0) else { return SpillPolicy::default() };
    // A parameter can shadow the captured root; do not inspect the wrong value.
    if params.iter().any(|name| name == owner.name) {
        return SpillPolicy::default();
    }
    let Some(root) = env.nested_spill_proof_value(owner.name) else {
        return SpillPolicy::default();
    };
    let leaf = match (owner.selector, root) {
        (Selector::Field(field), Value::Struct { fields, .. }) => fields.get(field),
        (Selector::String(key), Value::Dict(map)) => map.get(key),
        (Selector::Integer(index), Value::Dict(map)) => with_integer_key(index, |key| map.get(key)),
        (Selector::Integer(index), Value::Array(rows)) => {
            let index = if index < 0 { rows.len() as i64 + index } else { index };
            if index < 0 {
                None
            } else {
                rows.get(index as usize)
            }
        }
        _ => None,
    };
    let Some(Value::Dict(map)) = leaf else { return SpillPolicy::default() };
    let mut region = Region { owner, runtime: Some((map, params, args, env)), writes: 0 };
    if region.statements(body, 0) && region.writes > 4 {
        // Bound preallocation by the leaf and source work, not an arbitrary
        // maximum batch size. Further insertions still grow normally.
        SpillPolicy {
            capacity: region.writes.min(map.len().max(5)).min(u16::MAX as usize) as u16,
            allow_initial_insertion: super::dictionary_publication::insertion_can_pay(
                map,
                u16::try_from(region.writes).unwrap_or(0),
            ),
        }
    } else {
        SpillPolicy::default()
    }
}

#[cfg(test)]
mod tests {
    fn eligible(
        body: &[crate::ast::Stmt],
        params: &[String],
        args: &[super::Value],
        env: &super::Environment,
    ) -> bool {
        super::eligible(body, params, args, env).capacity != 0
    }

    use super::*;
    use std::sync::Arc;

    fn parse(source: &str) -> Vec<Stmt> {
        let parsed = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap())
            .parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        parsed.stmts
    }
    fn dictionary() -> Value {
        Value::dict(DictMap::from_iter((0..300).map(|i| (Arc::from(i.to_string()), Value::Int(i)))))
    }
    fn edits(row: &str, count: usize) -> String {
        (0..count).map(|i| format!("{row}[{i}] += n\n")).collect()
    }

    #[test]
    fn insertion_batches_respect_owner_budget_without_disabling_existing_key_batches() {
        for width in [2, 128, 2048] {
            let mut env = Environment::new();
            let map = Value::dict(
                (0..width).map(|i| (Arc::from(i.to_string()), Value::Int(i))).collect(),
            );
            env.define("state".into(), Value::array(vec![map]));
            for prefix in
                ["", "state[0][0] = n ", "if n > 0 { state[0][0] = n } else { state[0][2] = n }"]
            {
                let source = format!(
                    "{prefix}{} return n",
                    (0..64).map(|i| format!("state[0][{}] = n ", width + i)).collect::<String>()
                );
                let body = parse(&source);
                assert!(shape(&body));
                let policy = super::eligible(&body, &["n".into()], &[Value::Int(1)], &env);
                assert!(policy.capacity > 0, "Existing first writes retain batching");
                assert_eq!(policy.allow_initial_insertion, width >= 128);
            }
        }
    }

    #[test]
    fn accepts_scalar_compound_batches_and_preserves_owner_reference_counts() {
        for (row, root) in [
            ("state[0]", Value::array(vec![dictionary()])),
            ("state[\"row\"]", Value::dict(DictMap::from_iter([(Arc::from("row"), dictionary())]))),
            (
                "state.row",
                Value::Struct {
                    name: "State".into(),
                    fields: Arc::new(std::collections::HashMap::from([(
                        "row".into(),
                        dictionary(),
                    )])),
                },
            ),
        ] {
            let mut env = Environment::new();
            env.define("state".into(), root);
            for count in [5, 16, 64, 257] {
                let source = edits(row, count) + &format!("return {row}[0]");
                let body = parse(&source);
                assert!(shape(&body));
                assert!(eligible(&body, &["n".into()], &[Value::Int(1)], &env), "{row}/{count}");
                assert!(!eligible(&body, &["n".into()], &[Value::array(vec![])], &env));
            }
            let root = env.get("state").unwrap();
            let count = match root {
                Value::Array(v) => Arc::strong_count(&v),
                Value::Dict(v) => Arc::strong_count(&v),
                Value::Struct { fields, .. } => Arc::strong_count(&fields),
                _ => unreachable!(),
            };
            assert_eq!(count, 2, "guard retained an owner reference");
        }
    }

    #[test]
    fn rejects_all_measured_escape_boundaries_and_small_batches() {
        let prefix = edits("state[0]", 5);
        for tail in [
            "return observe()",
            "return state",
            "return [state]",
            "return state[0]",
            "let saved := state state[0][0] = n return saved",
            "state[1][0] = n return state[1][0]",
            "return state[1][0]",
            "state[0][0] = [n] return state[0][0][0]",
            "return state[state][0]",
            "return state[-state][0]",
            "return state[0][state]",
            "return state[0][-state]",
            "let minimum := -9223372036854775807 - 1 return state[-minimum][0]",
            "if false { return observe() } return state[0][0]",
        ] {
            assert!(!shape(&parse(&(prefix.clone() + tail))), "{tail}");
        }
        assert!(!shape(&parse(&(edits("state[0]", 4) + "return state[0][0]"))));
    }

    #[test]
    fn rejects_missing_or_nonscalar_reads_and_parameter_shadowing() {
        let body = parse(&(edits("state[0]", 5) + "return state[0][0]"));
        for value in [
            Value::array(vec![]),
            Value::dict(DictMap::default()),
            Value::str_ref("text"),
            Value::Struct {
                name: "Operator".into(),
                fields: Arc::new(std::collections::HashMap::new()),
            },
        ] {
            let mut map =
                DictMap::from_iter((0..5).map(|i| (Arc::from(i.to_string()), Value::Int(i))));
            map.insert(Arc::from("4"), value);
            let mut env = Environment::new();
            env.define("state".into(), Value::array(vec![Value::dict(map)]));
            assert!(!eligible(&body, &["n".into()], &[Value::Int(1)], &env));
        }
        let mut env = Environment::new();
        env.define("state".into(), Value::array(vec![dictionary()]));
        assert!(!eligible(
            &body,
            &["state".into(), "n".into()],
            &[Value::Null, Value::Int(1)],
            &env
        ));
        let missing = parse(&(edits("state[0]", 5) + "return state[0][999]"));
        assert!(!eligible(&missing, &["n".into()], &[Value::Int(1)], &env));
        let insert = parse(&(edits("state[0]", 5) + "state[0][\"new\"] = n return state[0][0]"));
        assert!(eligible(&insert, &["n".into()], &[Value::Int(1)], &env));
    }

    #[test]
    fn runtime_selector_arguments_require_actual_scalar_keys() {
        let body = parse(&(edits("state[0]", 5) + "return state[0][key]"));
        let mut env = Environment::new();
        env.define("state".into(), Value::array(vec![dictionary()]));
        let params = ["n".into(), "key".into()];
        for key in [Value::Int(0), Value::str_ref("0")] {
            assert!(eligible(&body, &params, &[Value::Int(1), key], &env));
        }
        for key in [Value::Null, Value::Bool(true), Value::array(vec![]), Value::Int(999)] {
            assert!(!eligible(&body, &params, &[Value::Int(1), key], &env));
        }
    }
}
