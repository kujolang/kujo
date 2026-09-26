//! Normalize expression yields into explicit statement suspension points.
//! Synthetic bindings use an unlexable prefix and are local to each generator.
use crate::ast::{ArrayElement, DictElement, Expr, InterpolatedStringPart, Pattern, Stmt};

pub(super) fn lower(body: &[Stmt]) -> Vec<Stmt> {
    Lowering { next: 0 }.block(body.to_vec())
}
struct Lowering {
    next: usize,
}
impl Lowering {
    fn save(&mut self, expr: Expr, out: &mut Vec<Stmt>) -> Expr {
        let name = format!("\0generator_operand_{}", self.next);
        self.next += 1;
        out.push(Stmt::Let {
            pattern: Pattern::Identifier(name.clone()),
            value: expr,
            mutable: false,
            type_annotation: None,
        });
        Expr::Identifier(name)
    }
    fn operand(&mut self, expr: Expr, out: &mut Vec<Stmt>) -> Expr {
        let expr = self.expression(expr, out);
        self.save(expr, out)
    }
    fn expression(&mut self, expr: Expr, out: &mut Vec<Stmt>) -> Expr {
        if !has_yield(&expr) {
            return expr;
        }
        match expr {
            Expr::Yield(value) => {
                let value = value
                    .map(|v| self.expression(*v, out))
                    .unwrap_or(Expr::Identifier("null".into()));
                let saved = self.save(value, out);
                out.push(Stmt::ExprStmt(Expr::Yield(Some(Box::new(saved.clone())))));
                saved
            }
            Expr::BinaryOp { left, op, right } if op == "&&" || op == "||" => {
                let left = self.operand(*left, out);
                let name = format!("\0generator_operand_{}", self.next);
                self.next += 1;
                let result = Expr::Identifier(name.clone());
                out.push(Stmt::Let {
                    pattern: Pattern::Identifier(name),
                    value: boolean(left),
                    mutable: true,
                    type_annotation: None,
                });
                let mut branch = Vec::new();
                let right = self.expression(*right, &mut branch);
                branch.push(Stmt::Assign { target: result.clone(), value: boolean(right) });
                let condition = if op == "&&" {
                    result.clone()
                } else {
                    Expr::UnaryOp { op: "!".into(), operand: Box::new(result.clone()) }
                };
                out.push(Stmt::If { condition, then_branch: branch, else_branch: None });
                result
            }
            Expr::BinaryOp { left, op, right } if op == "??" => {
                let left = self.operand(*left, out);
                let name = format!("\0generator_operand_{}", self.next);
                self.next += 1;
                let result = Expr::Identifier(name.clone());
                out.push(Stmt::Let {
                    pattern: Pattern::Identifier(name),
                    value: left,
                    mutable: true,
                    type_annotation: None,
                });
                let mut branch = Vec::new();
                let right = self.expression(*right, &mut branch);
                branch.push(Stmt::Assign { target: result.clone(), value: right });
                out.push(Stmt::If {
                    condition: Expr::BinaryOp {
                        left: Box::new(result.clone()),
                        op: "==".into(),
                        right: Box::new(Expr::Identifier("null".into())),
                    },
                    then_branch: branch,
                    else_branch: None,
                });
                result
            }
            Expr::BinaryOp { left, op, right } => Expr::BinaryOp {
                left: Box::new(self.operand(*left, out)),
                op,
                right: Box::new(self.operand(*right, out)),
            },
            Expr::UnaryOp { op, operand } => {
                Expr::UnaryOp { op, operand: Box::new(self.expression(*operand, out)) }
            }
            Expr::Call { function, args } => {
                // Named calls carry native dispatch/mutation metadata. Preserve
                // their syntax; non-name callees are evaluated before arguments.
                let function = match *function {
                    name @ Expr::Identifier(_) => name,
                    other => self.operand(other, out),
                };
                Expr::Call {
                    function: Box::new(function),
                    args: args.into_iter().map(|e| self.operand(e, out)).collect(),
                }
            }
            Expr::MethodCall { object, method, args } => {
                let object = match *object {
                    name @ Expr::Identifier(_) => name,
                    other => self.operand(other, out),
                };
                Expr::MethodCall {
                    object: Box::new(object),
                    method,
                    args: args.into_iter().map(|e| self.operand(e, out)).collect(),
                }
            }
            Expr::Tag(tag, args) => {
                Expr::Tag(tag, args.into_iter().map(|e| self.operand(e, out)).collect())
            }
            Expr::StructInstance { name, fields } => Expr::StructInstance {
                name,
                fields: fields.into_iter().map(|(k, v)| (k, self.operand(v, out))).collect(),
            },
            Expr::FieldAccess { object, field } => {
                Expr::FieldAccess { object: Box::new(self.expression(*object, out)), field }
            }
            Expr::IndexAccess { object, index } => Expr::IndexAccess {
                object: Box::new(self.operand(*object, out)),
                index: Box::new(self.operand(*index, out)),
            },
            Expr::ArrayLiteral(items) => Expr::ArrayLiteral(
                items
                    .into_iter()
                    .map(|e| match e {
                        ArrayElement::Single(e) => ArrayElement::Single(self.operand(e, out)),
                        ArrayElement::Spread(e) => ArrayElement::Spread(self.operand(e, out)),
                    })
                    .collect(),
            ),
            Expr::DictLiteral(items) => Expr::DictLiteral(
                items
                    .into_iter()
                    .map(|e| match e {
                        DictElement::Pair(k, v) => {
                            DictElement::Pair(self.operand(k, out), self.operand(v, out))
                        }
                        DictElement::Spread(e) => DictElement::Spread(self.operand(e, out)),
                    })
                    .collect(),
            ),
            Expr::InterpolatedString(parts) => Expr::InterpolatedString(
                parts
                    .into_iter()
                    .map(|p| match p {
                        InterpolatedStringPart::Expr(e) => {
                            InterpolatedStringPart::Expr(Box::new(self.operand(*e, out)))
                        }
                        text => text,
                    })
                    .collect(),
            ),
            Expr::Some(e) => Expr::Some(Box::new(self.expression(*e, out))),
            Expr::Ok(e) => Expr::Ok(Box::new(self.expression(*e, out))),
            Expr::Err(e) => Expr::Err(Box::new(self.expression(*e, out))),
            Expr::Try(e) => Expr::Try(Box::new(self.expression(*e, out))),
            Expr::Await(e) => Expr::Await(Box::new(self.expression(*e, out))),
            Expr::Spread(e) => Expr::Spread(Box::new(self.expression(*e, out))),
            other => other,
        }
    }
    fn block(&mut self, body: Vec<Stmt>) -> Vec<Stmt> {
        let mut out = Vec::new();
        for stmt in body {
            self.statement(stmt, &mut out);
        }
        out
    }
    fn statement(&mut self, stmt: Stmt, out: &mut Vec<Stmt>) {
        let stmt = match stmt {
            Stmt::ExprStmt(Expr::Yield(value)) => {
                Stmt::ExprStmt(Expr::Yield(value.map(|v| Box::new(self.expression(*v, out)))))
            }
            Stmt::ExprStmt(e) => Stmt::ExprStmt(self.expression(e, out)),
            Stmt::Let { pattern, value, mutable, type_annotation } => {
                Stmt::Let { pattern, value: self.expression(value, out), mutable, type_annotation }
            }
            Stmt::Const { name, value, type_annotation } => {
                Stmt::Const { name, value: self.expression(value, out), type_annotation }
            }
            Stmt::Assign { target, value } => Stmt::Assign {
                target: self.target(target, out),
                value: self.expression(value, out),
            },
            Stmt::Return(e) => Stmt::Return(e.map(|e| self.expression(e, out))),
            Stmt::If { condition, then_branch, else_branch } => Stmt::If {
                condition: self.expression(condition, out),
                then_branch: self.block(then_branch),
                else_branch: else_branch.map(|b| self.block(b)),
            },
            Stmt::While { condition, body } => self.loop_stmt(Some(condition), body),
            Stmt::Loop { condition, body } => self.loop_stmt(condition, body),
            Stmt::For { var, iterable, body } => {
                Stmt::For { var, iterable: self.expression(iterable, out), body: self.block(body) }
            }
            Stmt::Block(body) => Stmt::Block(self.block(body)),
            Stmt::TryExcept { try_block, except_var, except_block } => Stmt::TryExcept {
                try_block: self.block(try_block),
                except_var,
                except_block: self.block(except_block),
            },
            Stmt::Match { value, cases, default } => Stmt::Match {
                value: self.expression(value, out),
                cases: cases.into_iter().map(|(p, b)| (p, self.block(b))).collect(),
                default: default.map(|b| self.block(b)),
            },
            // Function/spawn bodies belong to their own execution owner.
            other => other,
        };
        out.push(stmt);
    }
    fn target(&mut self, target: Expr, out: &mut Vec<Stmt>) -> Expr {
        match target {
            Expr::IndexAccess { object, index } => Expr::IndexAccess {
                object: Box::new(self.target(*object, out)),
                index: Box::new(self.expression(*index, out)),
            },
            Expr::FieldAccess { object, field } => {
                Expr::FieldAccess { object: Box::new(self.target(*object, out)), field }
            }
            other => other,
        }
    }
    fn loop_stmt(&mut self, condition: Option<Expr>, body: Vec<Stmt>) -> Stmt {
        let body = self.block(body);
        match condition {
            Some(condition) if has_yield(&condition) => {
                let mut prefix = Vec::new();
                let condition = self.expression(condition, &mut prefix);
                prefix.push(Stmt::If {
                    condition,
                    then_branch: body,
                    else_branch: Some(vec![Stmt::Break]),
                });
                Stmt::Loop { condition: None, body: prefix }
            }
            condition => Stmt::Loop { condition, body },
        }
    }
}
fn boolean(e: Expr) -> Expr {
    Expr::BinaryOp { left: Box::new(e), op: "&&".into(), right: Box::new(Expr::Bool(true)) }
}

fn has_yield(e: &Expr) -> bool {
    match e {
        Expr::Yield(_) => true,
        Expr::BinaryOp { left, right, .. } => has_yield(left) || has_yield(right),
        Expr::UnaryOp { operand, .. }
        | Expr::Some(operand)
        | Expr::Ok(operand)
        | Expr::Err(operand)
        | Expr::Try(operand)
        | Expr::Await(operand)
        | Expr::Spread(operand) => has_yield(operand),
        Expr::Call { function, args } => has_yield(function) || args.iter().any(has_yield),
        Expr::MethodCall { object, args, .. } => has_yield(object) || args.iter().any(has_yield),
        Expr::Tag(_, args) => args.iter().any(has_yield),
        Expr::StructInstance { fields, .. } => fields.iter().any(|(_, e)| has_yield(e)),
        Expr::FieldAccess { object, .. } => has_yield(object),
        Expr::IndexAccess { object, index } => has_yield(object) || has_yield(index),
        Expr::ArrayLiteral(items) => items.iter().any(|e| match e {
            ArrayElement::Single(e) | ArrayElement::Spread(e) => has_yield(e),
        }),
        Expr::DictLiteral(items) => items.iter().any(|e| match e {
            DictElement::Pair(k, v) => has_yield(k) || has_yield(v),
            DictElement::Spread(e) => has_yield(e),
        }),
        Expr::InterpolatedString(parts) => {
            parts.iter().any(|p| matches!(p, InterpolatedStringPart::Expr(e) if has_yield(e)))
        }
        _ => false,
    }
}
