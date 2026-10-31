//! Bounded admission for nested dictionary collection writes.
//! Owner selectors are stable literals/fields; payload checks never admit callbacks.
use super::{environment::Environment, Value};
use crate::ast::{ArrayElement, DictElement, Expr, Pattern, Stmt};
use std::cell::Cell;

fn owner_name(expr: &Expr, depth: usize) -> Option<&str> {
    if depth > 128 {
        return None;
    }
    match expr {
        Expr::Identifier(name) => Some(name),
        Expr::FieldAccess { object, .. } => owner_name(object, depth + 1),
        Expr::IndexAccess { object, index }
            if matches!(index.as_ref(), Expr::Int(_) | Expr::String(_)) =>
        {
            owner_name(object, depth + 1)
        }
        _ => None,
    }
}
fn destination(expr: &Expr) -> Option<(&Expr, &Expr)> {
    let Expr::IndexAccess { object, index } = expr else { return None };
    if !matches!(object.as_ref(), Expr::FieldAccess { .. } | Expr::IndexAccess { .. }) {
        return None;
    }
    owner_name(object, 0)?;
    Some((object, index))
}
fn same_owner(a: &Expr, b: &Expr, depth: usize) -> bool {
    if depth > 128 {
        return false;
    }
    match (a, b) {
        (Expr::Identifier(a), Expr::Identifier(b)) => a == b,
        (Expr::FieldAccess { object: a, field: x }, Expr::FieldAccess { object: b, field: y }) => {
            x == y && same_owner(a, b, depth + 1)
        }
        (Expr::IndexAccess { object: a, index: x }, Expr::IndexAccess { object: b, index: y }) => {
            let same = match (x.as_ref(), y.as_ref()) {
                (Expr::Int(x), Expr::Int(y)) => x == y,
                (Expr::String(x), Expr::String(y)) => x == y,
                _ => false,
            };
            same && same_owner(a, b, depth + 1)
        }
        _ => false,
    }
}
fn first_owner<'a>(body: &'a [Stmt], depth: usize, budget: &mut usize) -> Option<&'a Expr> {
    if depth > 128 {
        return None;
    }
    for stmt in body {
        *budget = budget.checked_sub(1)?;
        let owner = match stmt {
            Stmt::Assign { target, .. } => return destination(target).map(|(owner, _)| owner),
            Stmt::While { body, .. } | Stmt::Block(body) => first_owner(body, depth + 1, budget),
            Stmt::If { then_branch, else_branch, .. } => {
                first_owner(then_branch, depth + 1, budget).or_else(|| {
                    else_branch.as_ref().and_then(|body| first_owner(body, depth + 1, budget))
                })
            }
            Stmt::Let { mutable: true, .. } => None,
            _ => return None,
        };
        if owner.is_some() {
            return owner;
        }
    }
    None
}
fn owner_value<'a>(expr: &Expr, env: &'a Environment, depth: usize) -> Option<&'a Value> {
    if depth > 128 {
        return None;
    }
    let value = match expr {
        Expr::Identifier(name) => env.nested_spill_proof_value(name)?,
        Expr::FieldAccess { object, field } => {
            let Value::Struct { fields, .. } = owner_value(object, env, depth + 1)? else {
                return None;
            };
            fields.get(field)?
        }
        Expr::IndexAccess { object, index } => {
            let parent = owner_value(object, env, depth + 1)?;
            match (parent, index.as_ref()) {
                (Value::Dict(map), Expr::String(key)) => map.get(key.as_str())?,
                (Value::Dict(map), Expr::Int(key)) => {
                    super::dictionary_publication::with_integer_key(*key, |key| map.get(key))?
                }
                (Value::Array(items), Expr::Int(index)) => {
                    let length = i64::try_from(items.len()).ok()?;
                    let index = if *index < 0 { length.checked_add(*index)? } else { *index };
                    items.get(usize::try_from(index).ok()?)?
                }
                _ => return None,
            }
        }
        _ => return None,
    };
    let count = match value {
        Value::Dict(map) => std::sync::Arc::strong_count(map),
        Value::Array(items) => std::sync::Arc::strong_count(items),
        Value::Struct { fields, .. } => std::sync::Arc::strong_count(fields),
        _ => return None,
    };
    (count == if matches!(expr, Expr::Identifier(_)) { 2 } else { 1 }).then_some(value)
}
struct Region<'a> {
    owner: &'a Expr,
    name: &'a str,
    collections: bool,
    params: &'a [String],
    args: &'a [Value],
    env: &'a Environment,
    counter: Option<(&'a str, i64)>,
    last: Option<i64>,
    budget: Cell<usize>,
}
impl Region<'_> {
    fn visit(&self) -> bool {
        let Some(next) = self.budget.get().checked_sub(1) else { return false };
        self.budget.set(next);
        true
    }
    fn argument(&self, name: &str) -> Option<&Value> {
        self.params.iter().position(|p| p == name).and_then(|i| self.args.get(i))
    }
    fn integer(&self, expr: &Expr, depth: usize, last: bool) -> Option<i64> {
        self.integer_expression(expr, depth, last, true)
    }
    fn integer_expression(
        &self,
        expr: &Expr,
        depth: usize,
        last: bool,
        allow_counter: bool,
    ) -> Option<i64> {
        if depth > 128 || !self.visit() {
            return None;
        }
        match expr {
            Expr::Int(n) => Some(*n),
            Expr::Identifier(name) if name != self.name => {
                if let Some((counter, start)) = self.counter {
                    if counter == name {
                        if !allow_counter {
                            return None;
                        }
                        return if last { self.last.or(Some(start)) } else { Some(start) };
                    }
                }
                match self.argument(name) {
                    Some(Value::Int(n)) => Some(*n),
                    _ => None,
                }
            }
            Expr::UnaryOp { op, operand } if op == "-" => {
                self.integer_expression(operand, depth + 1, last, allow_counter)?.checked_neg()
            }
            // These expressions are affine in the single counted variable.
            // Both endpoints check every subexpression, including overflow.
            Expr::BinaryOp { left, op, right } if op == "+" || op == "-" => {
                let left = self.integer_expression(left, depth + 1, last, allow_counter)?;
                let right = self.integer_expression(right, depth + 1, last, allow_counter)?;
                if op == "+" {
                    left.checked_add(right)
                } else {
                    left.checked_sub(right)
                }
            }
            // Multiplication/division are admitted only for invariant integer
            // operands. Nonlinear counter expressions cannot be proven safe by
            // checking endpoints (interior values may overflow or divide by zero).
            Expr::BinaryOp { left, op, right } if op == "*" || op == "/" => {
                let left = self.integer_expression(left, depth + 1, last, false)?;
                let right = self.integer_expression(right, depth + 1, last, false)?;
                if op == "*" {
                    left.checked_mul(right)
                } else {
                    left.checked_div(right)
                }
            }
            _ => None,
        }
    }
    fn scalar(&self, expr: &Expr, depth: usize) -> bool {
        if depth > 128 || !self.visit() {
            return false;
        }
        match expr {
            Expr::Int(_) | Expr::Float(_) | Expr::Bool(_) | Expr::String(_) | Expr::None => true,
            Expr::Identifier(name) if name == "null" => true,
            Expr::Identifier(name) if name != self.name => {
                self.counter.is_some_and(|(n, _)| n == name)
                    || self.argument(name).is_some_and(super::value::is_capture_primitive)
            }
            Expr::UnaryOp { operand, .. } => self.scalar(operand, depth + 1),
            Expr::BinaryOp { left, right, .. } => {
                self.scalar(left, depth + 1) && self.scalar(right, depth + 1)
            }
            _ => false,
        }
    }
    fn payload(&mut self, expr: &Expr, depth: usize) -> bool {
        if depth > 128 || !self.visit() {
            return false;
        }
        match expr {
            Expr::String(_) => {
                self.collections = true;
                true
            }
            Expr::Identifier(name)
                if matches!(self.argument(name), Some(Value::Str(_) | Value::Bytes(_))) =>
            {
                self.collections = true;
                self.scalar(expr, depth + 1)
            }
            Expr::ArrayLiteral(items) => {
                self.collections = true;
                items.iter().all(|item| match item {
                    ArrayElement::Single(value) => self.payload(value, depth + 1),
                    _ => false,
                })
            }
            Expr::DictLiteral(items) => {
                self.collections = true;
                items.iter().all(|item| match item {
                    DictElement::Pair(key, value) => {
                        self.scalar(key, depth + 1) && self.payload(value, depth + 1)
                    }
                    _ => false,
                })
            }
            Expr::StructInstance { fields, .. } => {
                self.collections = true;
                fields.iter().all(|(_, value)| self.payload(value, depth + 1))
            }
            _ => self.scalar(expr, depth + 1),
        }
    }
    fn update(&mut self, stmt: &Stmt, depth: usize) -> Option<usize> {
        if depth > 128 || !self.visit() {
            return None;
        }
        match stmt {
            Stmt::Assign { target, value } => {
                let (owner, index) = destination(target)?;
                if !same_owner(owner, self.owner, 0) {
                    return None;
                }
                let valid_key = match index {
                    Expr::String(_) => true,
                    Expr::Identifier(name)
                        if self.counter.is_none_or(|(counter, _)| counter != name)
                            && matches!(self.argument(name), Some(Value::Str(_))) =>
                    {
                        true
                    }
                    _ => {
                        self.integer(index, 0, false).is_some()
                            && self.integer(index, 0, true).is_some()
                    }
                };
                if !valid_key || !self.payload(value, depth + 1) {
                    return None;
                }
                Some(1)
            }
            Stmt::If { condition, then_branch, else_branch } => {
                if !self.scalar(condition, depth + 1) {
                    return None;
                }
                let a = self.updates(then_branch, depth + 1)?;
                let b = match else_branch {
                    Some(body) => self.updates(body, depth + 1)?,
                    None => 0,
                };
                Some(a.max(b))
            }
            Stmt::Block(body) => self.updates(body, depth + 1),
            _ => None,
        }
    }
    fn updates(&mut self, body: &[Stmt], depth: usize) -> Option<usize> {
        body.iter().try_fold(0usize, |n, stmt| Some(n.saturating_add(self.update(stmt, depth)?)))
    }
    fn counted_loop(&mut self, condition: &Expr, body: &[Stmt], depth: usize) -> Option<usize> {
        let (name, start) = self.counter?;
        let Expr::BinaryOp { left, op, right } = condition else { return None };
        if op != "<" || !matches!(left.as_ref(), Expr::Identifier(n) if n == name) {
            return None;
        }
        // Bounds must be invariant throughout the loop.
        if !match right.as_ref() {
            Expr::Int(_) => true,
            Expr::Identifier(n) => n != name,
            Expr::UnaryOp { op, operand } => op == "-" && matches!(operand.as_ref(), Expr::Int(_)),
            _ => false,
        } {
            return None;
        }
        let end = self.integer(right, 0, false)?;
        let (increment, updates) = body.split_last()?;
        let Stmt::Assign {
            target: Expr::Identifier(target),
            value: Expr::BinaryOp { left, op, right },
        } = increment
        else {
            return None;
        };
        if target != name || op != "+" || !matches!(left.as_ref(), Expr::Identifier(n) if n == name)
        {
            return None;
        }
        let Expr::Int(step) = right.as_ref() else { return None };
        if *step <= 0 || updates.is_empty() {
            return None;
        }
        let distance = end.checked_sub(start)?;
        // Reject empty loops instead of validating unreachable body operands.
        if distance <= 0 {
            return None;
        }
        let trips = 1 + (distance - 1) / step;
        if trips > i64::from(u16::MAX) {
            return None;
        }
        let final_counter = trips.checked_mul(*step)?.checked_add(start)?;
        self.last = Some(final_counter - step);
        let writes = self.updates(updates, depth + 1)?;
        self.last = None;
        self.counter = Some((name, final_counter));
        Some(writes.saturating_mul(trips as usize))
    }
}
impl<'a> Region<'a> {
    fn statements(&mut self, body: &'a [Stmt], depth: usize) -> Option<usize> {
        if depth > 128 {
            return None;
        }
        let mut writes = 0usize;
        for stmt in body {
            if !self.visit() {
                return None;
            }
            match stmt {
                Stmt::Let {
                    pattern: Pattern::Identifier(name),
                    value,
                    mutable: true,
                    type_annotation: None,
                } if self.counter.is_none() && name != self.name => {
                    if self.params.contains(name)
                        || self.env.nested_spill_proof_value(name).is_some()
                    {
                        return None;
                    }
                    self.counter = Some((name, self.integer(value, 0, false)?));
                }
                Stmt::While { condition, body } => {
                    writes = writes.saturating_add(self.counted_loop(condition, body, depth + 1)?)
                }
                Stmt::Return(Some(value)) if self.scalar(value, depth + 1) => return Some(writes),
                Stmt::Return(None) => return Some(writes),
                Stmt::Block(body) => writes = writes.saturating_add(self.updates(body, depth + 1)?),
                stmt => writes = writes.saturating_add(self.update(stmt, depth + 1)?),
            }
        }
        Some(writes)
    }
}

// Cached syntax admission only; every invocation still validates owners and operands.
pub(super) fn shape(body: &[Stmt]) -> bool {
    first_owner(body, 0, &mut 4096).is_some()
}

pub(super) fn eligible(
    body: &[Stmt],
    params: &[String],
    args: &[Value],
    env: &Environment,
) -> super::nested_spill_proof::SpillPolicy {
    let fallback = super::nested_spill_proof::SpillPolicy::default;
    let Some(owner) = first_owner(body, 0, &mut 4096) else { return fallback() };
    let Some(name) = owner_name(owner, 0) else { return fallback() };
    if params.len() != args.len()
        || params.len() > 64
        || params.iter().enumerate().any(|(i, name)| params[..i].contains(name))
        || params.iter().any(|p| p == name)
    {
        return fallback();
    }
    let Some(Value::Dict(base)) = owner_value(owner, env, 0) else { return fallback() };
    if base.is_empty() {
        return fallback();
    }
    let mut region = Region {
        owner,
        name,
        collections: false,
        params,
        args,
        env,
        counter: None,
        last: None,
        budget: Cell::new(4096),
    };
    match region.statements(body, 0) {
        Some(writes) if region.collections && (2..=usize::from(u16::MAX)).contains(&writes) => {
            super::nested_spill_proof::SpillPolicy {
                capacity: writes as u16,
                collection_rows: true,
                allow_initial_insertion: super::nested_publication::collection_insertion_can_pay(
                    base,
                    writes as u16,
                ),
            }
        }
        _ => fallback(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    fn parse(source: &str) -> Vec<Stmt> {
        let parsed = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap())
            .parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        parsed.stmts
    }
    fn leaf() -> Value {
        Value::dict((0..128).map(|i| (Arc::from(i.to_string()), Value::Int(0))).collect())
    }
    fn root(kind: usize) -> (Value, &'static str) {
        match kind {
            0 => (
                Value::dict([(Arc::from("inner"), leaf())].into_iter().collect()),
                "state[\"inner\"]",
            ),
            1 => (
                Value::Struct {
                    name: "State".into(),
                    fields: Arc::new([("inner".into(), leaf())].into_iter().collect()),
                },
                "state.inner",
            ),
            2 => (
                Value::dict(
                    [(
                        Arc::from("outer"),
                        Value::dict([(Arc::from("inner"), leaf())].into_iter().collect()),
                    )]
                    .into_iter()
                    .collect(),
                ),
                "state[\"outer\"][\"inner\"]",
            ),
            _ => (Value::array(vec![leaf()]), "state[0]"),
        }
    }
    fn check(source: &str, env: &Environment) -> super::super::nested_spill_proof::SpillPolicy {
        eligible(&parse(source), &["n".into()], &[Value::Int(7)], env)
    }
    #[test]
    fn accepts_stable_deep_owners_and_mixed_payloads_without_changing_policy_layout() {
        for kind in 0..4 {
            let (owner, path) = root(kind);
            let mut env = Environment::new();
            env.define("state".into(), owner.clone());
            let policy = check(
                &format!("{path}[0]=[n] {path}[1]={{\"value\":n}} {path}[2]=n return n"),
                &env,
            );
            assert_eq!(policy.capacity, 3);
            assert!(policy.collection_rows);
            let policy = check(&format!("{path}[0]=Cell {{ value:n }} {path}[1]=null"), &env);
            assert_eq!(policy.capacity, 2);
            let policy = check(&format!("{path}[0]=\"left\" {path}[1]=\"right\""), &env);
            assert_eq!(policy.capacity, 2);
            for value in [Value::Str(Arc::new("text".into())), Value::Bytes(Arc::new(vec![0, 255]))]
            {
                let policy = eligible(
                    &parse(&format!("{path}[0]=n {path}[1]=n")),
                    &["n".into()],
                    &[value],
                    &env,
                );
                assert_eq!(policy.capacity, 2);
                assert!(policy.collection_rows);
            }
        }
        assert_eq!(std::mem::size_of::<super::super::nested_spill_proof::SpillPolicy>(), 4);
    }
    #[test]
    fn counted_loops_bound_work_but_do_not_preallocate_that_bound() {
        let (owner, path) = root(0);
        let mut env = Environment::new();
        env.define("state".into(), owner.clone());
        let p = check(
            &format!(
                "mut i:=0 while i<64 {{ {path}[0]=[n+i] {path}[1]={{\"v\":n+i}} i+=1 }} return n"
            ),
            &env,
        );
        assert_eq!(p.capacity, 128);
        assert!(p.collection_rows);
        assert_eq!(
            check(&format!("mut i:=0 while i<8 {{ {path}[i]=[n] i+=1 }}"), &env).capacity,
            8
        );
        for end in [0, 65536] {
            assert_eq!(
                check(&format!("mut i:=0 while i<{end} {{ {path}[i]=[n] i+=1 }}"), &env).capacity,
                0
            );
        }
    }
    #[test]
    fn observations_callbacks_aliases_and_replacements_keep_fallback() {
        let (owner, path) = root(0);
        let mut env = Environment::new();
        env.define("state".into(), owner.clone());
        for suffix in [
            "return state",
            "return state[\"inner\"][0]",
            "state = {}",
            "state[\"inner\"] = {}",
            "observe()",
            "let alias := state",
            "return n + callback()",
        ] {
            assert_eq!(
                check(&format!("{path}[0]=[n] {path}[1]=[n] {suffix}"), &env).capacity,
                0,
                "{suffix}"
            );
        }
        for source in [
            "state[key][0]=[n] state[key][1]=[n]",
            "state[\"inner\"][0]=[n] state[\"other\"][1]=[n]",
            "state[\"inner\"][0]=[observe()] state[\"inner\"][1]=[n]",
            "state[\"inner\"][0]=n state[\"inner\"][1]=n",
        ] {
            assert_eq!(check(source, &env).capacity, 0, "{source}");
        }
        let extra = owner.clone();
        assert_eq!(check(&format!("{path}[0]=[n] {path}[1]=[n]"), &env).capacity, 0);
        drop(extra);
        let Value::Dict(map) = &owner else { panic!() };
        let alias = map["inner"].clone();
        assert_eq!(check(&format!("{path}[0]=[n] {path}[1]=[n]"), &env).capacity, 0);
        drop(alias);
    }
    #[test]
    fn keys_parameters_and_counter_mutation_are_checked() {
        let (owner, path) = root(0);
        let mut env = Environment::new();
        env.define("state".into(), owner.clone());
        let source = parse(&format!("{path}[key]=[n] {path}[1]=n"));
        assert_eq!(
            eligible(
                &source,
                &["key".into(), "n".into()],
                &[Value::Str(Arc::new("new".into())), Value::Int(2)],
                &env
            )
            .capacity,
            2
        );
        for args in [vec![Value::Float(1.0), Value::Int(2)], vec![Value::Int(0)]] {
            assert_eq!(eligible(&source, &["key".into(), "n".into()], &args, &env).capacity, 0);
        }
        for source in [
            format!("mut i:=0 while i<8 {{ {path}[i]=[n] i+=0 }}"),
            format!("mut i:=0 while i<i {{ {path}[i]=[n] i+=1 }}"),
            format!("mut i:=0 while i<8 {{ {path}[i]=[n] n=0 i+=1 }}"),
        ] {
            assert_eq!(check(&source, &env).capacity, 0);
        }
    }
}
