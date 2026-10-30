//! Allocation-free admission for root array collection batches. This is a
//! profitability guard only; ordinary publication boundaries remain authoritative.
use super::{environment::Environment, Value};
use crate::ast::{ArrayElement, DictElement, Expr, Pattern, Stmt};
use std::cell::Cell;

fn destination(expr: &Expr) -> Option<(&str, &Expr)> {
    let Expr::IndexAccess { object, index } = expr else { return None };
    let Expr::Identifier(name) = object.as_ref() else { return None };
    Some((name, index))
}
fn first_owner<'a>(body: &'a [Stmt], depth: usize, budget: &mut usize) -> Option<&'a str> {
    if depth > 128 {
        return None;
    }
    for stmt in body {
        *budget = budget.checked_sub(1)?;
        let owner = match stmt {
            Stmt::Assign { target, .. } => return destination(target).map(|(name, _)| name),
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
struct Region<'a> {
    owner: &'a str,
    length: usize,
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
        if depth > 128 || !self.visit() {
            return None;
        }
        match expr {
            Expr::Int(n) => Some(*n),
            Expr::Identifier(name) if name != self.owner => {
                if let Some((counter, start)) = self.counter {
                    if counter == name {
                        return if last { self.last.or(Some(start)) } else { Some(start) };
                    }
                }
                match self.argument(name) {
                    Some(Value::Int(n)) => Some(*n),
                    _ => None,
                }
            }
            Expr::UnaryOp { op, operand } if op == "-" => {
                self.integer(operand, depth + 1, last)?.checked_neg()
            }
            // These expressions are affine in the single counted variable.
            // Both endpoints check every subexpression, including overflow.
            Expr::BinaryOp { left, op, right } if op == "+" || op == "-" => {
                let left = self.integer(left, depth + 1, last)?;
                let right = self.integer(right, depth + 1, last)?;
                if op == "+" {
                    left.checked_add(right)
                } else {
                    left.checked_sub(right)
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
            Expr::Identifier(name) if name != self.owner => {
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
            Expr::ArrayLiteral(items) => items.iter().all(|item| match item {
                ArrayElement::Single(value) => self.payload(value, depth + 1),
                _ => false,
            }),
            Expr::DictLiteral(items) => items.iter().all(|item| match item {
                DictElement::Pair(key, value) => {
                    self.scalar(key, depth + 1) && self.payload(value, depth + 1)
                }
                _ => false,
            }),
            Expr::StructInstance { fields, .. } => {
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
                let (name, index) = destination(target)?;
                if name != self.owner {
                    return None;
                }
                let valid =
                    |index: i64| index >= -(self.length as i64) && index < self.length as i64;
                if !valid(self.integer(index, 0, false)?)
                    || !valid(self.integer(index, 0, true)?)
                    || !self.payload(value, depth + 1)
                {
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
                } if self.counter.is_none() && name != self.owner => {
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

pub(super) fn eligible(body: &[Stmt], params: &[String], args: &[Value], env: &Environment) -> u16 {
    let Some(owner) = first_owner(body, 0, &mut 4096) else { return 0 };
    if params.len() != args.len()
        || params.len() > 64
        || params.iter().enumerate().any(|(i, name)| params[..i].contains(name))
    {
        return 0;
    }
    if params.iter().any(|p| p == owner) {
        return 0;
    }
    let Some(Value::Array(base)) = env.nested_spill_proof_value(owner) else { return 0 };
    if std::sync::Arc::strong_count(base) != 2 || base.len() < 2 {
        return 0;
    }
    let mut region = Region {
        owner,
        length: base.len(),
        params,
        args,
        env,
        counter: None,
        last: None,
        budget: Cell::new(4096),
    };
    match region.statements(body, 0) {
        Some(writes) if writes >= 2 => writes.min(base.len()).min(usize::from(u16::MAX)) as u16,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(source: &str) -> Vec<Stmt> {
        let parsed = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap())
            .parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        parsed.stmts
    }
    fn environment() -> (Environment, Value) {
        let owner = Value::array(vec![Value::Null; 64]);
        let mut env = Environment::new();
        env.define("state".into(), owner.clone());
        (env, owner)
    }
    #[test]
    fn admits_bounds_checked_counted_mixed_and_struct_batches() {
        let (env, _owner) = environment();
        for payload in [
            "state[j] = [n]",
            "state[j] = Item { value: n }",
            "if j % 2 == 0 { state[j] = [n] } else { state[j] = {\"v\": n} }",
        ] {
            let body =
                parse(&format!("mut j := 0 while j < limit {{ {payload} j += 1 }} return n"));
            for count in [2, 3, 8, 64] {
                assert_eq!(
                    eligible(
                        &body,
                        &["n".into(), "limit".into()],
                        &[Value::Int(7), Value::Int(count)],
                        &env
                    ),
                    count as u16
                );
            }
            for count in [0, 1, 65, i64::MAX] {
                assert_eq!(
                    eligible(
                        &body,
                        &["n".into(), "limit".into()],
                        &[Value::Int(7), Value::Int(count)],
                        &env
                    ),
                    0
                );
            }
            assert_eq!(
                eligible(
                    &body,
                    &["n".into(), "limit".into()],
                    &[Value::array(vec![]), Value::Int(8)],
                    &env
                ),
                0
            );
        }
        let explicit =
            parse("state[-64] = [n] state[1] = {\"value\": n} state[63] = Item { value: n }");
        assert_eq!(eligible(&explicit, &["n".into()], &[Value::Int(1)], &env), 3);
        let negative = parse("mut j := 1 while j < limit { state[-j] = [n] j += 2 } return n");
        assert_eq!(
            eligible(
                &negative,
                &["n".into(), "limit".into()],
                &[Value::Int(1), Value::Int(64)],
                &env
            ),
            32
        );
    }

    #[test]
    fn admits_scalar_batches_without_displacing_two_write_storage() {
        let (env, _owner) = environment();
        for body in [
            "state[0] = n state[1] = n state[2] = n",
            "state[0] = n state[1] = [n] state[2] = {\"value\": n}",
            "mut j := 0 while j < 3 { state[j] = n j += 1 }",
        ] {
            assert_eq!(eligible(&parse(body), &["n".into()], &[Value::Int(7)], &env), 3);
        }
        for body in [
            "state[0] = n state[1] = n state[2] = observe()",
            "state[0] = n state[1] = n state[99] = n",
        ] {
            assert_eq!(eligible(&parse(body), &["n".into()], &[Value::Int(7)], &env), 0);
        }
        assert_eq!(
            eligible(&parse("state[0] = n state[1] = n"), &["n".into()], &[Value::Int(7)], &env),
            2
        );
    }

    #[test]
    fn admits_two_element_owners_without_widening_observation_bounds() {
        let owner = Value::array(vec![Value::Null; 2]);
        let mut env = Environment::new();
        env.define("state".into(), owner.clone());
        for body in ["state[0] = n state[1] = [n]", "state[-2] = [n] state[-1] = n"] {
            assert_eq!(eligible(&parse(body), &["n".into()], &[Value::Int(7)], &env), 2);
        }
        for body in ["state[0] = n", "state[0] = n state[2] = n", "state[0] = n state[1] = read()"]
        {
            assert_eq!(eligible(&parse(body), &["n".into()], &[Value::Int(7)], &env), 0);
        }
    }
    #[test]
    fn admits_checked_affine_indices_and_rejects_overflowing_subexpressions() {
        let owner = Value::array(vec![Value::Null; 8]);
        let mut env = Environment::new();
        env.define("state".into(), owner.clone());
        for (body, offset) in [
            ("state[offset + 0] = n state[1 + offset] = [n] state[(offset + 1) + 1] = n", 0),
            ("state[offset - 0] = n state[offset - 1] = [n] state[offset - 2] = n", 2),
            ("state[offset + 0] = n state[offset + 1] = [n] state[offset + 2] = n", -8),
            ("mut j := 0 while j < 3 { state[offset + j] = [n] j += 1 }", 1),
            ("mut j := 0 while j < 3 { state[offset - j] = [n] j += 1 }", 3),
            ("mut j := 0 while j < 3 { state[-(offset + j)] = [n] j += 1 }", 1),
        ] {
            assert_eq!(
                eligible(
                    &parse(body),
                    &["n".into(), "offset".into()],
                    &[Value::Int(7), Value::Int(offset)],
                    &env
                ),
                3
            );
        }
        for (body, offset) in [
            ("state[0] = n state[(offset + 1) - offset] = [n] state[2] = n", i64::MAX),
            ("state[0] = n state[(offset - 1) - offset] = [n] state[2] = n", i64::MIN),
            ("state[0] = n state[-(-offset)] = [n] state[2] = n", i64::MIN),
            ("mut j := 0 while j < 3 { state[offset + j] = [n] j += 1 }", 7),
            ("mut j := 0 while j < 3 { state[offset - j] = [n] j += 1 }", -7),
            ("state[0] = n state[offset * offset] = [n] state[2] = n", 1),
            ("state[0] = n state[offset + observe()] = [n] state[2] = n", 1),
        ] {
            assert_eq!(
                eligible(
                    &parse(body),
                    &["n".into(), "offset".into()],
                    &[Value::Int(7), Value::Int(offset)],
                    &env
                ),
                0
            );
        }
    }

    #[test]
    fn rejects_observation_callbacks_uncertain_bounds_and_local_mutation() {
        let (mut env, _owner) = environment();
        for source in [
            "state[0] = [n] state[1] = [n] state[2] = [n] return state",
            "state[0] = [n] state[1] = [n] state[2] = [observe()]",
            "state[0] = [n] state[1] = [n] state[2] = [state[0]]",
            "state[0] = [n] state[1] = [n] state[2] = [...n]",
            "mut j := 0 while j < 3 { state[j] = [n] j += 0 }",
            "mut j := 0 while j < 3 { state[j] = [n] j -= 1 }",
            "mut j := 0 while j < 3 { state[j] = [n] n = 2 j += 1 }",
            "mut j := 0 while j < j { state[j] = [n] j += 1 }",
            "state[0] = [n] other[1] = [n] state[2] = [n]",
            "state[0] = [n] state[1] = [n] state[99] = [n]",
            "state[0] = [n] state[1] = [n] state[2] = [captured]",
            "if true { mut j := 0 } while j < 3 { state[j] = [n] j += 1 }",
        ] {
            assert_eq!(
                eligible(&parse(source), &["n".into()], &[Value::Int(1)], &env),
                0,
                "{source}"
            );
        }
        let body = parse("mut j := 0 while j < 8 { state[j] = [n] j += 1 }");
        env.define("j".into(), Value::Int(100));
        assert_eq!(eligible(&body, &["n".into()], &[Value::Int(1)], &env), 0);
        let (env, owner) = environment();
        let _snapshot = owner.clone();
        assert_eq!(eligible(&body, &["n".into()], &[Value::Int(1)], &env), 0);
    }

    #[test]
    fn inspection_is_bounded_and_duplicate_parameters_are_not_trusted() {
        let (env, _owner) = environment();
        let body = parse("state[0] = [n] state[1] = [n] state[2] = [n]");
        assert_eq!(
            eligible(
                &body,
                &["n".into(), "n".into()],
                &[Value::Int(1), Value::array(vec![])],
                &env
            ),
            0
        );
        let long = parse(&"state[0] = [n]\n".repeat(3000));
        assert_eq!(eligible(&long, &["n".into()], &[Value::Int(1)], &env), 0);
        let unreachable = parse("return n state[0] = [n] state[1] = [n] state[2] = [n]");
        assert_eq!(eligible(&unreachable, &["n".into()], &[Value::Int(1)], &env), 0);
    }
}
