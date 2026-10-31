//! Profitability admission for scalar and collection row batches, including counted loops.
//! This does not authorize reordering: publication still handles every escape.
use super::{environment::Environment, Value};
use crate::ast::{ArrayElement, DictElement, Expr, Pattern, Stmt};
use std::cell::Cell;

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
    let (object, selector) = match object.as_ref() {
        Expr::FieldAccess { object, field } => (object, Selector::Field(field)),
        Expr::IndexAccess { object, index } => (
            object,
            match index.as_ref() {
                Expr::Int(n) => Selector::Integer(*n),
                Expr::String(s) => Selector::String(s),
                _ => return None,
            },
        ),
        _ => return None,
    };
    let Expr::Identifier(name) = object.as_ref() else { return None };
    Some((Owner { name, selector }, index))
}
fn first_owner(body: &[Stmt], depth: usize) -> Option<Owner<'_>> {
    if depth > 128 {
        return None;
    }
    body.iter().find_map(|s| match s {
        Stmt::Assign { target, .. } => path(target).map(|p| p.0),
        Stmt::While { body, .. } | Stmt::Block(body) => first_owner(body, depth + 1),
        Stmt::If { then_branch, else_branch, .. } => first_owner(then_branch, depth + 1)
            .or_else(|| else_branch.as_ref().and_then(|b| first_owner(b, depth + 1))),
        _ => None,
    })
}
fn scalar(v: &Value) -> bool {
    matches!(v, Value::Int(_) | Value::Float(_) | Value::Bool(_) | Value::Null)
}
struct LoopRange {
    last: i64,
    step: i64,
    trips: i64,
    scalar: [Cell<Option<bool>>; 2],
}
struct Region<'a> {
    owner: Owner<'a>,
    runtime: Option<(&'a [Value], &'a [String], &'a [Value], &'a Environment)>,
    counter: Option<(&'a str, i64)>,
    loop_range: Option<LoopRange>,
    writes: usize,
    collection_payload: bool,
    reads_row: Cell<bool>,
    #[cfg(test)]
    validation_reads: Cell<usize>,
    #[cfg(test)]
    validation_assignments: Cell<usize>,
}
impl Region<'_> {
    fn argument(&self, name: &str) -> Option<&Value> {
        let (_, params, args, env) = self.runtime?;
        if let Some(i) = params.iter().position(|p| p == name) {
            args.get(i)
        } else {
            env.nested_spill_proof_value(name)
        }
    }
    fn integer(&self, expr: &Expr, depth: usize) -> Option<i64> {
        if depth > 128 {
            return None;
        }
        match expr {
            Expr::Int(n) => Some(*n),
            Expr::Identifier(name) if name != self.owner.name => {
                if let Some((local, n)) = self.counter {
                    if local == name {
                        return Some(n);
                    }
                }
                if self.runtime.is_none() {
                    return Some(0);
                }
                match self.argument(name) {
                    Some(Value::Int(n)) => Some(*n),
                    _ => None,
                }
            }
            Expr::UnaryOp { op, operand } if op == "-" => {
                self.integer(operand, depth + 1)?.checked_neg()
            }
            _ => None,
        }
    }
    fn indexed(&self, expr: &Expr, reading: bool) -> bool {
        let Some((owner, index)) = path(expr) else { return false };
        if owner != self.owner {
            return false;
        }
        let index_expr = index;
        let Some(index) = self.integer(index_expr, 0) else { return false };
        let Some((row, ..)) = self.runtime else { return true };
        if let (Some(range), Some(negative)) =
            (&self.loop_range, self.counter_polarity(index_expr, 0))
        {
            let last = if negative { range.last.checked_neg() } else { Some(range.last) };
            let Some(last) = last else { return false };
            // A positive induction step visits a monotone interval. Checking
            // both endpoints also covers the negative/positive alias boundary.
            let valid = |i: i64| i >= -(row.len() as i64) && i < row.len() as i64;
            if !valid(index) || !valid(last) {
                return false;
            }
            if !reading {
                return true;
            }
            let cache = &range.scalar[usize::from(negative)];
            if let Some(result) = cache.get() {
                return result;
            }
            let mut current = self.counter.expect("loop counter").1;
            let result = (0..range.trips).all(|_| {
                let index = if negative { -current } else { current };
                current += range.step;
                let index = if index < 0 { row.len() as i64 + index } else { index };
                #[cfg(test)]
                self.validation_reads.set(self.validation_reads.get() + 1);
                scalar(&row[index as usize])
            });
            cache.set(Some(result));
            return result;
        }
        let index = if index < 0 { (row.len() as i64).checked_add(index) } else { Some(index) };
        index
            .filter(|i| *i >= 0)
            .and_then(|i| row.get(i as usize))
            .is_some_and(|v| !reading || scalar(v))
    }
    fn counter_polarity(&self, expr: &Expr, depth: usize) -> Option<bool> {
        if depth > 128 {
            return None;
        }
        match expr {
            Expr::Identifier(name) if self.counter.is_some_and(|(n, _)| n == name) => Some(false),
            Expr::UnaryOp { op, operand } if op == "-" => {
                self.counter_polarity(operand, depth + 1).map(|negative| !negative)
            }
            _ => None,
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
                self.counter.is_some_and(|(n, _)| n == name)
                    || self.runtime.is_none()
                    || self.argument(name).is_some_and(scalar)
            }
            Expr::IndexAccess { .. } => {
                self.reads_row.set(true);
                self.indexed(expr, true)
            }
            Expr::UnaryOp { operand, .. } => self.expression(operand, depth + 1),
            Expr::BinaryOp { left, right, .. } => {
                self.expression(left, depth + 1) && self.expression(right, depth + 1)
            }
            _ => false,
        }
    }
}
impl<'a> Region<'a> {
    fn new(
        owner: Owner<'a>,
        runtime: Option<(&'a [Value], &'a [String], &'a [Value], &'a Environment)>,
    ) -> Self {
        Self {
            owner,
            runtime,
            counter: None,
            loop_range: None,
            writes: 0,
            collection_payload: false,
            reads_row: Cell::new(false),
            #[cfg(test)]
            validation_reads: Cell::new(0),
            #[cfg(test)]
            validation_assignments: Cell::new(0),
        }
    }
    fn payload(&mut self, expr: &Expr, depth: usize) -> bool {
        if depth > 128 {
            return false;
        }
        match expr {
            Expr::ArrayLiteral(items) => {
                self.collection_payload = true;
                items.iter().all(|item| match item {
                    ArrayElement::Single(value) => self.payload(value, depth + 1),
                    _ => false,
                })
            }
            Expr::DictLiteral(items) => {
                self.collection_payload = true;
                items.iter().all(|item| match item {
                    DictElement::Pair(key, value) => {
                        (matches!(key, Expr::String(_)) || self.expression(key, depth + 1))
                            && self.payload(value, depth + 1)
                    }
                    _ => false,
                })
            }
            Expr::StructInstance { fields, .. } => {
                self.collection_payload = true;
                fields.iter().all(|(_, value)| self.payload(value, depth + 1))
            }
            Expr::Identifier(name)
                if name != self.owner.name
                    && name != "null"
                    && !self.counter.is_some_and(|(n, _)| n == name) =>
            {
                if self.runtime.is_none() {
                    // Static shape only: runtime admission checks the actual kind.
                    self.collection_payload = true;
                    return true;
                }
                let Some(value) = self.argument(name) else { return false };
                if !super::array_batch_publication::BatchValue::supports(value) {
                    return false;
                }
                if !scalar(value) {
                    self.collection_payload = true;
                }
                true
            }
            _ => self.expression(expr, depth + 1),
        }
    }

    fn assignment(&mut self, stmt: &Stmt, depth: usize) -> bool {
        #[cfg(test)]
        self.validation_assignments.set(self.validation_assignments.get() + 1);
        let Stmt::Assign { target, value } = stmt else { return false };
        if !self.indexed(target, false) || !self.payload(value, depth + 1) {
            return false;
        }
        self.writes = self.writes.saturating_add(1);
        true
    }
    fn counted_loop(&mut self, condition: &'a Expr, body: &'a [Stmt], depth: usize) -> bool {
        let Some((name, start)) = self.counter else { return false };
        let Expr::BinaryOp { left, op, right } = condition else { return false };
        if op != "<" || !matches!(left.as_ref(), Expr::Identifier(n) if n == name) {
            return false;
        }
        // The bound cannot depend on the changing counter.
        if !match right.as_ref() {
            Expr::Int(_) => true,
            Expr::Identifier(n) => n != name,
            Expr::UnaryOp { op, operand } => op == "-" && matches!(operand.as_ref(), Expr::Int(_)),
            _ => false,
        } {
            return false;
        }
        let Some(end) = self.integer(right, 0) else { return false };
        let Some((last, updates)) = body.split_last() else { return false };
        if updates.is_empty() {
            return false;
        }
        let Stmt::Assign { target: Expr::Identifier(n), value: Expr::BinaryOp { left, op, right } } =
            last
        else {
            return false;
        };
        if n != name || op != "+" || !matches!(left.as_ref(), Expr::Identifier(n) if n == name) {
            return false;
        }
        let Expr::Int(step) = right.as_ref() else { return false };
        if *step <= 0 {
            return false;
        }
        if self.runtime.is_none() {
            if !updates.iter().all(|s| self.assignment(s, depth + 1)) {
                return false;
            }
            self.writes = self.writes.max(5);
            return true;
        }
        let Some(distance) = end.checked_sub(start) else { return false };
        if distance <= 0 {
            return true;
        }
        let trips = 1 + (distance - 1) / step;
        // Bound admission work independently of source loop duration. Larger or
        // uncertain loops keep ordinary publication instead of slowing setup.
        if trips > 65535 {
            return false;
        }
        let Some(final_counter) = trips.checked_mul(*step).and_then(|n| start.checked_add(n))
        else {
            return false;
        };
        let before = self.writes;
        self.loop_range = Some(LoopRange {
            last: final_counter - step,
            step: *step,
            trips,
            scalar: [Cell::new(None), Cell::new(None)],
        });
        let valid = updates.iter().all(|s| self.assignment(s, depth + 1));
        self.loop_range = None;
        if !valid {
            return false;
        }
        let per_iteration = self.writes - before;
        self.writes = before.saturating_add(per_iteration.saturating_mul(trips as usize));
        self.counter = Some((name, final_counter));
        true
    }
    fn statements(&mut self, body: &'a [Stmt], depth: usize) -> bool {
        if depth > 128 {
            return false;
        }
        body.iter().all(|stmt| match stmt {
            Stmt::Let {
                pattern: Pattern::Identifier(name),
                value,
                mutable: true,
                type_annotation: None,
            } if self.counter.is_none() && name != self.owner.name => {
                // Do not shadow a parameter/capture: proof reads must denote
                // exactly the same local values as ordinary execution.
                if self.runtime.is_some_and(|(_, params, _, _)| params.iter().any(|p| p == name))
                    || self.argument(name).is_some()
                {
                    return false;
                }
                let Some(n) = self.integer(value, 0) else { return false };
                self.counter = Some((name, n));
                true
            }
            Stmt::While { condition, body } => self.counted_loop(condition, body, depth + 1),
            Stmt::Assign { .. } => self.assignment(stmt, depth),
            Stmt::Return(Some(value)) => self.expression(value, depth + 1),
            Stmt::Return(None) => true,
            // Conditional local scope/control is not simulated by this guard.
            _ => false,
        })
    }
}

pub(super) fn shape(body: &[Stmt]) -> bool {
    let Some(owner) = first_owner(body, 0) else { return false };
    let mut r = Region::new(owner, None);
    r.statements(body, 0) && (r.writes > 4 || (r.collection_payload && r.writes >= 2))
}
pub(super) fn eligible_policy(
    body: &[Stmt],
    params: &[String],
    args: &[Value],
    env: &Environment,
) -> super::nested_spill_proof::SpillPolicy {
    let Some(owner) = first_owner(body, 0) else { return Default::default() };
    if params.len() != args.len()
        || params.iter().any(|n| n == owner.name)
        || params.iter().enumerate().any(|(i, n)| params[..i].contains(n))
    {
        return Default::default();
    }
    let Some(root) = env.nested_spill_proof_value(owner.name) else { return Default::default() };
    let leaf = match (owner.selector, root) {
        (Selector::Field(k), Value::Struct { fields, .. }) => fields.get(k),
        (Selector::String(k), Value::Dict(map)) => map.get(k),
        (Selector::Integer(k), Value::Dict(map)) => {
            super::dictionary_publication::with_integer_key(k, |k| map.get(k))
        }
        (Selector::Integer(k), Value::Array(rows)) => {
            let k = if k < 0 { rows.len() as i64 + k } else { k };
            if k < 0 {
                None
            } else {
                rows.get(k as usize)
            }
        }
        _ => None,
    };
    let Some(Value::Array(row)) = leaf else { return Default::default() };
    let mut r = Region::new(owner, Some((row, params, args, env)));
    if r.statements(body, 0)
        && (r.writes > 4 || (r.collection_payload && r.writes >= 2))
        && !(r.collection_payload && r.reads_row.get())
    {
        super::nested_spill_proof::SpillPolicy {
            capacity: r.writes.min(row.len()).min(u16::MAX as usize) as u16,
            collection_rows: r.collection_payload,
            ..Default::default()
        }
    } else {
        Default::default()
    }
}

#[cfg(test)]
fn eligible(body: &[Stmt], params: &[String], args: &[Value], env: &Environment) -> u16 {
    eligible_policy(body, params, args, env).capacity
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(source: &str) -> Vec<Stmt> {
        let result = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap())
            .parse_with_diagnostics();
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        result.stmts
    }
    fn env() -> Environment {
        let mut env = Environment::new();
        env.define(
            "state".into(),
            Value::array(vec![Value::array((0..128).map(Value::Int).collect())]),
        );
        env
    }
    #[test]
    fn admits_explicit_and_counted_batches_without_retaining_owners() {
        let env = env();
        for count in [5, 16, 64] {
            let explicit = (0..count).map(|i| format!("state[0][{i}] += n\n")).collect::<String>();
            let loops = format!("mut j := 0 while j < {count} {{ state[0][j] += n j += 1 }}");
            for body in [explicit, loops] {
                let body = parse(&(body + " state[0][-128] += n return state[0][0]"));
                assert!(shape(&body));
                assert_eq!(eligible(&body, &["n".into()], &[Value::Int(1)], &env), count + 1);
                assert_eq!(eligible(&body, &["n".into()], &[Value::array(vec![])], &env), 0);
            }
        }
        let root = env.get("state").unwrap();
        let Value::Array(root) = root else { unreachable!() };
        assert_eq!(std::sync::Arc::strong_count(&root), 2);
    }
    #[test]
    fn collection_rows_admit_mixed_batches_but_reject_reads_and_callbacks() {
        let env = env();
        for source in [
            "state[0][0] = n state[0][1] = [n] state[0][2] = {\"v\": n} return n",
            "state[0][0] = [n] state[0][1] = n return n",
            "mut j := 0 while j < 32 { state[0][j] = [n + j] j += 1 } return n",
        ] {
            let body = parse(source);
            assert!(shape(&body), "{source}");
            let policy = eligible_policy(&body, &["n".into()], &[Value::Int(7)], &env);
            assert!(policy.collection_rows, "{source}");
            assert!(policy.capacity >= 2, "{source}");
        }
        for source in [
            "state[0][0] = [n] state[0][1] = state[0][0] return n",
            "state[0][0] = [n] state[0][1] = [observe()] return n",
            "state[0][0] = [n] state[0][128] = [n] return n",
        ] {
            assert_eq!(eligible(&parse(source), &["n".into()], &[Value::Int(7)], &env), 0);
        }
        let scalar = parse("mut j := 0 while j < 8 { state[0][j] += n j += 1 } return n");
        let policy = eligible_policy(&scalar, &["n".into()], &[Value::Int(7)], &env);
        assert_eq!(policy.capacity, 8);
        assert!(!policy.collection_rows);
    }

    #[test]
    fn rejects_escape_uncertain_loops_and_invalid_reads() {
        for body in [
            "mut j := -8 while j < -j { state[0][0] += 1 j += 1 }",
            "mut j := 0 while j < 8 { state[0][j] += 1 j += 0 }",
            "mut j := 0 while j < 8 { state[0][j] += 1 j -= 1 }",
            "mut j := 0 while j < 8 { state[0][j] += 1 observe() j += 1 }",
            "mut j := 0 while j < 8 { state[0][j] += 1 j += 1 } return state",
            "mut j := 0 while j < 8 { state[0][j] += 1 j += 1 } return state[0]",
            "mut j := 0 while j < 8 { state[0][j] = [observe()] j += 1 }",
            "mut j := 0 while j < 129 { state[0][j] += 1 j += 1 }",
            "mut j := 0 while j < 65536 { state[0][0] += 1 j += 1 }",
            "mut j := 0 while j < 8 { state[0][j] += 1 j += 1 } return state[0][128]",
        ] {
            assert_eq!(eligible(&parse(body), &[], &[], &env()), 0, "{body}");
        }
    }
    #[test]
    fn dynamic_bounds_steps_and_shadowing_are_checked() {
        let source =
            parse("mut j := 1 while j < end { state[0][j] += 1 j += 2 } return state[0][1]");
        assert!(shape(&source));
        assert_eq!(eligible(&source, &["end".into()], &[Value::Int(16)], &env()), 8);
        assert_eq!(eligible(&source, &["end".into()], &[Value::Int(0)], &env()), 0);
        assert_eq!(eligible(&source, &["end".into()], &[Value::str_ref("16")], &env()), 0);
        assert_eq!(
            eligible(
                &source,
                &["end".into(), "j".into()],
                &[Value::Int(16), Value::Int(0)],
                &env()
            ),
            0
        );
        let mut env = env();
        env.define("j".into(), Value::Int(0));
        assert_eq!(eligible(&source, &["end".into()], &[Value::Int(16)], &env), 0);
    }
}

#[cfg(test)]
mod inspection_cost_tests {
    use super::*;
    fn parse(source: &str) -> Vec<Stmt> {
        let parsed = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap())
            .parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty());
        parsed.stmts
    }
    #[test]
    fn repeated_body_statements_do_not_multiply_row_inspection() {
        let row = (0..10000).map(Value::Int).collect::<Vec<_>>();
        let env = Environment::new();
        let params = vec!["zero".to_string()];
        let args = vec![Value::Int(0)];
        for (assignment, expected_reads) in
            [("state[0][j] = 1 / zero\n", 0), ("state[0][j] = state[0][j] + 1\n", 10000)]
        {
            let source = "mut j := 0 while j < 10000 {\n".to_owned()
                + &assignment.repeat(100)
                + "j += 1 } return state[0][0]";
            let body = parse(&source);
            let owner = first_owner(&body, 0).unwrap();
            let mut region = Region::new(owner, Some((&row, &params, &args, &env)));
            assert!(region.statements(&body, 0));
            assert_eq!(region.writes, 1_000_000);
            assert_eq!(region.validation_assignments.get(), 100);
            assert_eq!(region.validation_reads.get(), expected_reads);
        }
    }
    #[test]
    fn stride_and_negative_ranges_check_interior_values_without_checking_unvisited_cells() {
        let source = parse("mut j := 1 while j < 16 { state[0][j] += 1 state[0][-j] += 1 j += 2 } return state[0][1]");
        for (index, accepted) in [(2, true), (7, false), (121, false)] {
            let mut row = (0..128).map(Value::Int).collect::<Vec<_>>();
            row[index] = Value::array(vec![]);
            let mut env = Environment::new();
            env.define("state".into(), Value::array(vec![Value::array(row)]));
            assert_eq!(eligible(&source, &[], &[], &env) > 0, accepted, "index {index}");
        }
        let mut env = Environment::new();
        env.define(
            "state".into(),
            Value::array(vec![Value::array((0..128).map(Value::Int).collect())]),
        );
        let crossing =
            parse("mut j := -8 while j < 8 { state[0][j] += 1 j += 2 } return state[0][0]");
        assert_eq!(eligible(&crossing, &[], &[], &env), 8);
        let outside =
            parse("mut j := 1 while j < 130 { state[0][j] += 1 j += 2 } return state[0][0]");
        assert_eq!(eligible(&outside, &[], &[], &env), 0);
    }
}
