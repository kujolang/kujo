//! Upper bound on writes before a captured helper returns. Used only to decide
//! whether avoiding an owner copy can pay for a new-key publication table.
//! It never changes execution order or authorizes otherwise ineligible writes.
use super::Value;
use crate::ast::{Expr, Pattern, Stmt};

#[derive(Debug, PartialEq)]
enum Unbounded {
    Dynamic,
    Unsupported,
}
type Bound<T> = Result<T, Unbounded>;

fn integer(expr: &Expr, params: &[String], args: &[Value], depth: usize) -> Bound<i64> {
    if depth > 128 {
        return Err(Unbounded::Unsupported);
    }
    match expr {
        Expr::Int(n) => Ok(*n),
        Expr::Identifier(name) => params
            .iter()
            .position(|p| p == name)
            .and_then(|i| args.get(i))
            .and_then(|v| if let Value::Int(n) = v { Some(*n) } else { None })
            .ok_or(Unbounded::Dynamic),
        Expr::UnaryOp { op, operand } if op == "-" => {
            integer(operand, params, args, depth + 1)?.checked_neg().ok_or(Unbounded::Unsupported)
        }
        _ => Err(Unbounded::Unsupported),
    }
}

fn writes(body: &[Stmt], params: &[String], args: &[Value], depth: usize) -> Bound<usize> {
    if depth > 128 {
        return Err(Unbounded::Unsupported);
    }
    let mut total = 0usize;
    let mut i = 0;
    while i < body.len() {
        let count = match &body[i] {
            Stmt::Assign { target: Expr::IndexAccess { object, .. }, .. }
                if matches!(object.as_ref(), Expr::Identifier(_)) =>
            {
                1
            }
            Stmt::Return(_) | Stmt::ExprStmt(_) => 0,
            Stmt::Block(body) => writes(body, params, args, depth + 1)?,
            Stmt::If { then_branch, else_branch, .. } => {
                let a = writes(then_branch, params, args, depth + 1)?;
                let b = else_branch
                    .as_ref()
                    .map(|b| writes(b, params, args, depth + 1))
                    .transpose()?
                    .unwrap_or(0);
                a.max(b)
            }
            Stmt::Let {
                pattern: Pattern::Identifier(name),
                value,
                mutable: true,
                type_annotation: None,
            } => {
                // Consume a canonical counter declaration and its entire loop
                // together. No local/parameter assignments are otherwise admitted.
                let Some(Stmt::While { condition, body }) = body.get(i + 1) else {
                    return Err(Unbounded::Unsupported);
                };
                let Expr::BinaryOp { left, op, right } = condition else {
                    return Err(Unbounded::Unsupported);
                };
                if op != "<"
                    || !matches!(left.as_ref(), Expr::Identifier(n) if n == name)
                    || matches!(right.as_ref(), Expr::Identifier(n) if n == name)
                    || params.iter().any(|p| p == name)
                {
                    return Err(Unbounded::Unsupported);
                }
                let Some((
                    Stmt::Assign {
                        target: Expr::Identifier(n),
                        value: Expr::BinaryOp { left, op, right },
                    },
                    updates,
                )) = body.split_last()
                else {
                    return Err(Unbounded::Unsupported);
                };
                if n != name
                    || op != "+"
                    || !matches!(left.as_ref(), Expr::Identifier(n) if n == name)
                {
                    return Err(Unbounded::Unsupported);
                }
                let Expr::Int(step) = right.as_ref() else { return Err(Unbounded::Unsupported) };
                if *step <= 0 {
                    return Err(Unbounded::Unsupported);
                }
                // Reject all extra counter/local assignments, nested loops and
                // local declarations. Thus the trip count cannot be invalidated.
                fn updates_bound(body: &[Stmt], depth: usize) -> Bound<usize> {
                    if depth > 128 {
                        return Err(Unbounded::Unsupported);
                    }
                    let mut n = 0usize;
                    for s in body {
                        let k =
                            match s {
                                Stmt::Assign {
                                    target: Expr::IndexAccess { object, .. }, ..
                                } if matches!(object.as_ref(), Expr::Identifier(_)) => 1,
                                Stmt::If { then_branch, else_branch, .. } => {
                                    updates_bound(then_branch, depth + 1)?.max(
                                        else_branch
                                            .as_ref()
                                            .map(|b| updates_bound(b, depth + 1))
                                            .transpose()?
                                            .unwrap_or(0),
                                    )
                                }
                                Stmt::Block(b) => updates_bound(b, depth + 1)?,
                                Stmt::Return(_) | Stmt::Break | Stmt::Continue => 0,
                                _ => return Err(Unbounded::Unsupported),
                            };
                        n = n.checked_add(k).ok_or(Unbounded::Unsupported)?;
                    }
                    Ok(n)
                }
                let per_trip = updates_bound(updates, depth + 1)?;
                // Continue could bypass the increment. Reject it even inside a
                // branch; the simple bound must not assume progress that may fail.
                fn has_continue(body: &[Stmt]) -> bool {
                    body.iter().any(|s| match s {
                        Stmt::Continue => true,
                        Stmt::Block(b) => has_continue(b),
                        Stmt::If { then_branch, else_branch, .. } => {
                            has_continue(then_branch)
                                || else_branch.as_ref().is_some_and(|b| has_continue(b))
                        }
                        _ => false,
                    })
                }
                if has_continue(updates) {
                    return Err(Unbounded::Unsupported);
                }
                let start = integer(value, params, args, depth + 1)?;
                let Expr::BinaryOp { right: end, .. } = condition else { unreachable!() };
                let end = integer(end, params, args, depth + 1)?;
                let distance = end.checked_sub(start).ok_or(Unbounded::Unsupported)?;
                let trips = if distance <= 0 { 0 } else { 1 + (distance - 1) / step };
                trips
                    .checked_mul(*step)
                    .and_then(|n| start.checked_add(n))
                    .ok_or(Unbounded::Unsupported)?;
                i += 1;
                per_trip
                    .checked_mul(usize::try_from(trips).map_err(|_| Unbounded::Unsupported)?)
                    .ok_or(Unbounded::Unsupported)?
            }
            _ => return Err(Unbounded::Unsupported),
        };
        total = total.checked_add(count).ok_or(Unbounded::Unsupported)?;
        i += 1;
    }
    Ok(total)
}

/// Zero means ineligible; 255 requests a bounded runtime calculation. The extra
/// byte fits the existing cached proof's padding (covered by a layout test).
pub(super) fn cached_bound(body: &[Stmt]) -> u8 {
    match writes(body, &[], &[], 0) {
        Ok(n) => u8::try_from(n).unwrap_or(u8::MAX),
        Err(Unbounded::Dynamic) => u8::MAX,
        Err(Unbounded::Unsupported) => 0,
    }
}
pub(super) fn runtime_bound(body: &[Stmt], params: &[String], args: &[Value]) -> u16 {
    writes(body, params, args, 0).ok().and_then(|n| u16::try_from(n).ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(source: &str) -> Vec<Stmt> {
        let p = crate::parser::Parser::new(crate::lexer::tokenize(source).unwrap())
            .parse_with_diagnostics();
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        p.stmts
    }
    #[test]
    fn bounds_explicit_conditional_and_counted_writes() {
        for (s, n) in [
            ("data[k] = k data[k+1] = k+1 return k", 2),
            ("if k > 0 { data[k]=1 data[k+1]=2 } else { data[k]=3 }", 2),
            ("mut j := 0 while j < 64 { data[k+j]=k+j j+=1 } return k", 64),
            ("mut j := 1 while j < 16 { data[j]=1 j+=2 }", 8),
            ("mut j := 16 while j < 1 { data[j]=1 j+=2 }", 0),
        ] {
            let body = parse(s);
            assert_eq!(runtime_bound(&body, &[], &[]), n, "{s}");
            assert_eq!(cached_bound(&body), n as u8);
        }
    }
    #[test]
    fn dynamic_bounds_are_checked_without_traversing_iterations() {
        let b = parse("mut j := 0 while j < end { data[j]=1 j+=1 }");
        assert_eq!(cached_bound(&b), 255);
        for (v, n) in [
            (Value::Int(64), 64),
            (Value::Int(65535), 65535),
            (Value::Int(65536), 0),
            (Value::Int(-1), 0),
            (Value::Null, 0),
        ] {
            assert_eq!(runtime_bound(&b, &["end".into()], &[v]), n);
        }
    }
    #[test]
    fn uncertain_or_nonprogressing_loops_and_mutated_parameters_fall_back() {
        for s in [
            "mut j:=0 while j<64 { data[j]=1 j+=0 }",
            "mut j:=0 while j<64 { data[j]=1 if j>2 { continue } j+=1 }",
            "mut j:=0 while j<64 { data[j]=1 j=0 j+=1 }",
            "mut j:=0 while j<64 { data[j]=1 end=128 j+=1 }",
            "end=128 mut j:=0 while j<end { data[j]=1 j+=1 }",
            "mut j:=0 while j<64 { data[j]=1 mut j:=0 j+=1 }",
            "mut j:=0 while j<j { data[j]=1 j+=1 }",
            "mut j:=9223372036854775806 while j<9223372036854775807 { data[j]=1 j+=2 }",
        ] {
            assert_eq!(runtime_bound(&parse(s), &["end".into()], &[Value::Int(1)]), 0, "{s}");
        }
    }
}
