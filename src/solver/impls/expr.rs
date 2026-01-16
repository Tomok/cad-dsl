//! Solvable trait implementations for expressions
//!
//! This module implements the `Solvable` trait for `ResolvedExpr` nodes,
//! converting HIR expressions into Z3 constraint expressions.

use crate::hir::expr::{ResolvedExpr, ResolvedExprKind};
use crate::solver::context::{SolverContext, Z3Primitive};
use crate::solver::{Solvable, SolverError, VariablePath};

/// Z3 expression result type
///
/// This enum represents the possible Z3 expression types that can result
/// from solving an expression.
#[derive(Debug, Clone)]
pub enum Z3Expr {
    /// Integer expression
    Int(z3::ast::Int),
    /// Real (floating-point) expression
    Real(z3::ast::Real),
    /// Boolean expression
    Bool(z3::ast::Bool),
}

impl Z3Expr {
    /// Convert to Int, with automatic type conversion if needed
    pub fn to_int(&self, _ctx: &z3::Context) -> z3::ast::Int {
        match self {
            Z3Expr::Int(i) => i.clone(),
            Z3Expr::Real(r) => r.to_int(),
            Z3Expr::Bool(b) => b.ite(&z3::ast::Int::from_i64(1), &z3::ast::Int::from_i64(0)),
        }
    }

    /// Convert to Real, with automatic type conversion if needed
    pub fn to_real(&self, _ctx: &z3::Context) -> z3::ast::Real {
        match self {
            Z3Expr::Int(i) => i.to_real(),
            Z3Expr::Real(r) => r.clone(),
            Z3Expr::Bool(b) => b
                .ite(&z3::ast::Int::from_i64(1), &z3::ast::Int::from_i64(0))
                .to_real(),
        }
    }

    /// Convert to Bool
    pub fn to_bool(&self, _ctx: &z3::Context) -> Result<z3::ast::Bool, SolverError> {
        match self {
            Z3Expr::Bool(b) => Ok(b.clone()),
            _ => Err(SolverError::UnsupportedExpression(
                "Cannot convert non-boolean expression to boolean".to_string(),
            )),
        }
    }
}

impl<'src, 'arena> Solvable<'src, 'arena> for ResolvedExpr<'src, 'arena> {
    type Output = Z3Expr;

    fn solve(&self, ctx: &mut SolverContext<'src, 'arena>) -> Result<Self::Output, SolverError> {
        match &self.kind {
            // Literals
            ResolvedExprKind::IntLit { value } => {
                Ok(Z3Expr::Int(z3::ast::Int::from_i64((*value) as i64)))
            }

            ResolvedExprKind::FloatLit { value } => {
                // Convert to rational representation (numerator, denominator)
                let value_i64 = (*value * 1000000.0) as i64; // Use 6 decimal places of precision
                Ok(Z3Expr::Real(z3::ast::Real::from_rational(
                    value_i64, 1000000,
                )))
            }

            ResolvedExprKind::BoolLit { value } => {
                Ok(Z3Expr::Bool(z3::ast::Bool::from_bool(*value)))
            }

            // Variable reference
            ResolvedExprKind::Var { name, .. } => {
                let path = VariablePath::from_name(name);
                let var_node = ctx
                    .get_variable(&path)
                    .ok_or_else(|| SolverError::UndefinedVariable(name.to_string()))?;

                let z3_var = var_node
                    .as_primitive()
                    .ok_or(SolverError::NotAPrimitiveType)?;

                Ok(match z3_var {
                    Z3Primitive::Int(z3_int) => Z3Expr::Int(z3_int.clone()),
                    Z3Primitive::Real(z3_real) => Z3Expr::Real(z3_real.clone()),
                    Z3Primitive::Bool(z3_bool) => Z3Expr::Bool(z3_bool.clone()),
                })
            }

            // Field access
            ResolvedExprKind::FieldAccess {
                receiver,
                field_name,
                ..
            } => {
                // Recursively build the path
                let base_path = self.build_variable_path(receiver, ctx)?;
                let full_path = base_path.with_field(field_name);

                let var_node = ctx.get_variable(&full_path).ok_or_else(|| {
                    SolverError::UndefinedVariable(format!("{}.{}", base_path, field_name))
                })?;

                let z3_var = var_node
                    .as_primitive()
                    .ok_or(SolverError::NotAPrimitiveType)?;

                Ok(match z3_var {
                    Z3Primitive::Int(z3_int) => Z3Expr::Int(z3_int.clone()),
                    Z3Primitive::Real(z3_real) => Z3Expr::Real(z3_real.clone()),
                    Z3Primitive::Bool(z3_bool) => Z3Expr::Bool(z3_bool.clone()),
                })
            }

            // Array index
            ResolvedExprKind::Index { array, index } => {
                // Evaluate index to constant
                let index_val = self.evaluate_const_expr(index, ctx)?;
                let base_path = self.build_variable_path(array, ctx)?;
                let full_path = base_path.with_index(index_val as usize);

                let var_node = ctx.get_variable(&full_path).ok_or_else(|| {
                    SolverError::UndefinedVariable(format!("{}[{}]", base_path, index_val))
                })?;

                let z3_var = var_node
                    .as_primitive()
                    .ok_or(SolverError::NotAPrimitiveType)?;

                Ok(match z3_var {
                    Z3Primitive::Int(z3_int) => Z3Expr::Int(z3_int.clone()),
                    Z3Primitive::Real(z3_real) => Z3Expr::Real(z3_real.clone()),
                    Z3Primitive::Bool(z3_bool) => Z3Expr::Bool(z3_bool.clone()),
                })
            }

            // Binary operations - Arithmetic
            ResolvedExprKind::Add { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Int(l + r)),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l + r)),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l.to_real() + r)),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Real(l + r.to_real())),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for addition".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Sub { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Int(l - r)),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l - r)),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l.to_real() - r)),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Real(l - r.to_real())),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for subtraction".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Mul { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Int(l * r)),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l * r)),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l.to_real() * r)),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Real(l * r.to_real())),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for multiplication".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Div { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Int(l / r)),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l / r)),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l.to_real() / r)),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Real(l / r.to_real())),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for division".to_string(),
                    )),
                }
            }

            // Binary operations - Comparisons
            ResolvedExprKind::Eq { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.eq(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.eq(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().eq(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.eq(r.to_real()))),
                    (Z3Expr::Bool(l), Z3Expr::Bool(r)) => Ok(Z3Expr::Bool(l.eq(&r))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for equality comparison".to_string(),
                    )),
                }
            }

            ResolvedExprKind::NotEq { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.eq(&r).not())),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.eq(&r).not())),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().eq(&r).not())),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.eq(r.to_real()).not())),
                    (Z3Expr::Bool(l), Z3Expr::Bool(r)) => Ok(Z3Expr::Bool(l.eq(&r).not())),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for not-equal comparison".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Lt { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.lt(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.lt(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().lt(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.lt(r.to_real()))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for less-than comparison".to_string(),
                    )),
                }
            }

            ResolvedExprKind::LtEq { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.le(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.le(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().le(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.le(r.to_real()))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for less-than-or-equal comparison".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Gt { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.gt(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.gt(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().gt(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.gt(r.to_real()))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for greater-than comparison".to_string(),
                    )),
                }
            }

            ResolvedExprKind::GtEq { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.ge(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.ge(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().ge(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.ge(r.to_real()))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for greater-than-or-equal comparison".to_string(),
                    )),
                }
            }

            // Unary operations
            ResolvedExprKind::Neg { inner } => {
                let operand_z3 = inner.solve(ctx)?;
                match operand_z3 {
                    Z3Expr::Int(i) => Ok(Z3Expr::Int(-i)),
                    Z3Expr::Real(r) => Ok(Z3Expr::Real(-r)),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Cannot negate boolean expression".to_string(),
                    )),
                }
            }

            // Logical operations
            ResolvedExprKind::And { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Bool(l), Z3Expr::Bool(r)) => {
                        Ok(Z3Expr::Bool(z3::ast::Bool::and(&[&l, &r])))
                    }
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for logical AND".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Or { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Bool(l), Z3Expr::Bool(r)) => {
                        Ok(Z3Expr::Bool(z3::ast::Bool::or(&[&l, &r])))
                    }
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for logical OR".to_string(),
                    )),
                }
            }

            // Function and Method Calls (Phase 3c)
            ResolvedExprKind::FunctionCall {
                name,
                function,
                args,
            } => {
                // Inline the function immediately by substituting parameters with arguments
                // We do NOT check if arguments are known - Z3 can handle symbolic variables
                self.inline_function(name, function, args, ctx)
            }

            ResolvedExprKind::MethodCall {
                receiver,
                method_name,
                method,
                args,
            } => {
                // Inline the method immediately by substituting parameters and self
                // We do NOT check if receiver/arguments are known - Z3 can handle symbolic variables
                self.inline_method(receiver, method_name, method, args, ctx)
            }

            // Unsupported expressions
            _ => Err(SolverError::UnsupportedExpression(format!(
                "{:?}",
                self.kind
            ))),
        }
    }
}

impl<'src, 'arena> ResolvedExpr<'src, 'arena> {
    /// Build a variable path from an expression
    ///
    /// This handles complex paths like `p.x` or `arr[0].field`
    fn build_variable_path(
        &self,
        base: &ResolvedExpr<'src, 'arena>,
        ctx: &SolverContext<'src, 'arena>,
    ) -> Result<VariablePath<'src>, SolverError> {
        match &base.kind {
            ResolvedExprKind::Var { name, .. } => Ok(VariablePath::from_name(name)),

            ResolvedExprKind::FieldAccess {
                receiver,
                field_name,
                ..
            } => {
                let inner_path = self.build_variable_path(receiver, ctx)?;
                Ok(inner_path.with_field(field_name))
            }

            ResolvedExprKind::Index { array, index } => {
                let index_val = self.evaluate_const_expr(index, ctx)?;
                let inner_path = self.build_variable_path(array, ctx)?;
                Ok(inner_path.with_index(index_val as usize))
            }

            _ => Err(SolverError::UnsupportedExpression(
                "Cannot build variable path from this expression".to_string(),
            )),
        }
    }

    /// Evaluate an expression to a constant integer value
    ///
    /// This is used for array indices and loop ranges.
    /// Returns an error if the expression cannot be evaluated to a constant.
    fn evaluate_const_expr(
        &self,
        expr: &ResolvedExpr<'src, 'arena>,
        ctx: &SolverContext<'src, 'arena>,
    ) -> Result<i64, SolverError> {
        match &expr.kind {
            ResolvedExprKind::IntLit { value } => Ok((*value) as i64),

            ResolvedExprKind::Var { name, .. } => {
                // Try to get value from current solution
                if let Some(solution) = ctx.get_current_solution() {
                    let path = VariablePath::from_name(name);
                    if let Some(value) = solution.assignments.get(&path) {
                        match value {
                            crate::solver::Value::Int(v) => Ok(*v),
                            _ => Err(SolverError::UnsupportedExpression(
                                "Variable is not an integer".to_string(),
                            )),
                        }
                    } else {
                        Err(SolverError::UndefinedVariable(format!(
                            "Variable '{}' not yet resolved",
                            name
                        )))
                    }
                } else {
                    Err(SolverError::UndefinedVariable(format!(
                        "Variable '{}' not yet resolved (no solution available)",
                        name
                    )))
                }
            }

            ResolvedExprKind::Add { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr(lhs, ctx)?;
                let rhs_val = self.evaluate_const_expr(rhs, ctx)?;
                Ok(lhs_val + rhs_val)
            }

            ResolvedExprKind::Sub { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr(lhs, ctx)?;
                let rhs_val = self.evaluate_const_expr(rhs, ctx)?;
                Ok(lhs_val - rhs_val)
            }

            ResolvedExprKind::Mul { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr(lhs, ctx)?;
                let rhs_val = self.evaluate_const_expr(rhs, ctx)?;
                Ok(lhs_val * rhs_val)
            }

            ResolvedExprKind::Div { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr(lhs, ctx)?;
                let rhs_val = self.evaluate_const_expr(rhs, ctx)?;
                if rhs_val == 0 {
                    return Err(SolverError::UnsupportedExpression(
                        "Division by zero".to_string(),
                    ));
                }
                Ok(lhs_val / rhs_val)
            }

            ResolvedExprKind::Neg { inner } => {
                let val = self.evaluate_const_expr(inner, ctx)?;
                Ok(-val)
            }

            _ => Err(SolverError::UnsupportedExpression(format!(
                "Cannot evaluate expression to constant: {:?}",
                expr.kind
            ))),
        }
    }

    /// Try to evaluate an expression to check if it depends on unknown variables
    ///
    /// This is used to determine if function calls can be inlined or need to be deferred.
    /// Returns Ok if the expression can be evaluated, Err if it depends on unknown variables.
    fn try_evaluate_expr(
        &self,
        expr: &ResolvedExpr<'src, 'arena>,
        ctx: &SolverContext<'src, 'arena>,
    ) -> Result<(), SolverError> {
        match &expr.kind {
            // Literals are always evaluable
            ResolvedExprKind::IntLit { .. }
            | ResolvedExprKind::FloatLit { .. }
            | ResolvedExprKind::BoolLit { .. } => Ok(()),

            // Variables are evaluable if they have a known value
            ResolvedExprKind::Var { name, .. } => {
                if ctx.is_variable_known(name) {
                    Ok(())
                } else {
                    Err(SolverError::UndefinedVariable(format!(
                        "Variable '{}' not yet resolved",
                        name
                    )))
                }
            }

            // Field access is evaluable if the base is evaluable
            ResolvedExprKind::FieldAccess { receiver, .. } => self.try_evaluate_expr(receiver, ctx),

            // Array index is evaluable if both array and index are evaluable
            ResolvedExprKind::Index { array, index } => {
                self.try_evaluate_expr(array, ctx)?;
                self.try_evaluate_expr(index, ctx)
            }

            // Binary operations are evaluable if both operands are
            ResolvedExprKind::Add { lhs, rhs }
            | ResolvedExprKind::Sub { lhs, rhs }
            | ResolvedExprKind::Mul { lhs, rhs }
            | ResolvedExprKind::Div { lhs, rhs }
            | ResolvedExprKind::Eq { lhs, rhs }
            | ResolvedExprKind::NotEq { lhs, rhs }
            | ResolvedExprKind::Lt { lhs, rhs }
            | ResolvedExprKind::LtEq { lhs, rhs }
            | ResolvedExprKind::Gt { lhs, rhs }
            | ResolvedExprKind::GtEq { lhs, rhs }
            | ResolvedExprKind::And { lhs, rhs }
            | ResolvedExprKind::Or { lhs, rhs } => {
                self.try_evaluate_expr(lhs, ctx)?;
                self.try_evaluate_expr(rhs, ctx)
            }

            // Unary operations are evaluable if the operand is
            ResolvedExprKind::Neg { inner } => self.try_evaluate_expr(inner, ctx),

            // Parenthesized expressions are evaluable if the inner expression is
            ResolvedExprKind::Paren { inner } => self.try_evaluate_expr(inner, ctx),

            // For now, other expressions are considered not evaluable
            _ => Err(SolverError::UnsupportedExpression(format!(
                "Cannot check if expression is evaluable: {:?}",
                expr.kind
            ))),
        }
    }

    /// Inline a function call by substituting parameters with arguments
    ///
    /// This works by:
    /// 1. Getting the return expression from the solver context
    /// 2. Creating a parameter->argument mapping
    /// 3. Recursively substituting parameters with arguments in the return expression
    fn inline_function(
        &self,
        function_name: &'src str,
        function: &'arena crate::hir::definitions::FunctionDefinition<'src, 'arena>,
        args: &[&'arena ResolvedExpr<'src, 'arena>],
        ctx: &mut SolverContext<'src, 'arena>,
    ) -> Result<Z3Expr, SolverError> {
        use std::collections::HashMap;

        // Get the return expression from the context
        // (it was registered during the pre-pass in solve())
        let return_expr = ctx.get_function_return(function_name).ok_or_else(|| {
            SolverError::UnsupportedExpression(format!(
                "Function '{}' has no return expression registered",
                function_name
            ))
        })?;

        // Create parameter substitution map
        let mut param_map: HashMap<&'src str, &'arena ResolvedExpr<'src, 'arena>> = HashMap::new();
        for (param, arg) in function.params.iter().zip(args.iter()) {
            param_map.insert(param.name, *arg);
        }

        // Substitute parameters in the return expression
        let inlined_expr = self.substitute_parameters(return_expr, &param_map, ctx)?;

        // Solve the inlined expression
        inlined_expr.solve(ctx)
    }

    /// Inline a method call by substituting parameters and self with arguments
    ///
    /// Similar to inline_function but also handles the receiver (self) binding.
    fn inline_method(
        &self,
        receiver: &'arena ResolvedExpr<'src, 'arena>,
        method_name: &'src str,
        method: &'arena crate::hir::definitions::FunctionDefinition<'src, 'arena>,
        args: &[&'arena ResolvedExpr<'src, 'arena>],
        ctx: &mut SolverContext<'src, 'arena>,
    ) -> Result<Z3Expr, SolverError> {
        use std::collections::HashMap;

        // Get the return expression from the context
        // For methods, we need to use qualified name (StructName::method_name)
        let qualified_name = if let Some(parent) = method.parent_struct {
            format!("{}::{}", parent.name, method_name)
        } else {
            method_name.to_string()
        };

        let return_expr = ctx.get_function_return(&qualified_name).ok_or_else(|| {
            SolverError::UnsupportedExpression(format!(
                "Method '{}' has no return expression registered",
                method_name
            ))
        })?;

        // Create parameter substitution map
        // First, map "self" to the receiver
        let mut param_map: HashMap<&'src str, &'arena ResolvedExpr<'src, 'arena>> = HashMap::new();
        param_map.insert("self", receiver);

        // Then, map the explicit parameters to the arguments
        for (param, arg) in method.params.iter().zip(args.iter()) {
            param_map.insert(param.name, *arg);
        }

        // Substitute parameters in the return expression
        let inlined_expr = self.substitute_parameters(return_expr, &param_map, ctx)?;

        // Solve the inlined expression
        inlined_expr.solve(ctx)
    }

    /// Substitute parameters with argument expressions in an expression
    ///
    /// This recursively walks the expression tree and replaces variable references
    /// to parameters with the corresponding argument expressions.
    fn substitute_parameters(
        &self,
        expr: &'arena ResolvedExpr<'src, 'arena>,
        param_map: &std::collections::HashMap<&'src str, &'arena ResolvedExpr<'src, 'arena>>,
        ctx: &SolverContext<'src, 'arena>,
    ) -> Result<&'arena ResolvedExpr<'src, 'arena>, SolverError> {
        let kind = match &expr.kind {
            // Variable reference - check if it's a parameter
            ResolvedExprKind::Var { name, .. } => {
                // If this variable name is in the parameter map, substitute it
                if let Some(arg_expr) = param_map.get(name) {
                    // Return the argument expression directly - no need to create new node
                    return Ok(*arg_expr);
                } else {
                    // Not a parameter, keep as is
                    return Ok(expr);
                }
            }

            // Binary operations - recursively substitute in both operands
            ResolvedExprKind::Add { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::Add {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::Sub { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::Sub {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::Mul { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::Mul {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::Div { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::Div {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::Mod { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::Mod {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::Pow { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::Pow {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            // Comparison operations
            ResolvedExprKind::Eq { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::Eq {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::NotEq { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::NotEq {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::Lt { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::Lt {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::LtEq { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::LtEq {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::Gt { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::Gt {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::GtEq { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::GtEq {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            // Logical operations
            ResolvedExprKind::And { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::And {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            ResolvedExprKind::Or { lhs, rhs } => {
                let sub_lhs = self.substitute_parameters(lhs, param_map, ctx)?;
                let sub_rhs = self.substitute_parameters(rhs, param_map, ctx)?;
                ResolvedExprKind::Or {
                    lhs: sub_lhs,
                    rhs: sub_rhs,
                }
            }

            // Unary operations
            ResolvedExprKind::Neg { inner } => {
                let sub_inner = self.substitute_parameters(inner, param_map, ctx)?;
                ResolvedExprKind::Neg { inner: sub_inner }
            }

            ResolvedExprKind::Ref { inner } => {
                let sub_inner = self.substitute_parameters(inner, param_map, ctx)?;
                ResolvedExprKind::Ref { inner: sub_inner }
            }

            ResolvedExprKind::Paren { inner } => {
                let sub_inner = self.substitute_parameters(inner, param_map, ctx)?;
                ResolvedExprKind::Paren { inner: sub_inner }
            }

            // Field access - substitute in receiver
            ResolvedExprKind::FieldAccess {
                receiver,
                field_name,
                field,
            } => {
                let sub_receiver = self.substitute_parameters(receiver, param_map, ctx)?;
                ResolvedExprKind::FieldAccess {
                    receiver: sub_receiver,
                    field_name,
                    field,
                }
            }

            // Array indexing - substitute in array and index
            ResolvedExprKind::Index { array, index } => {
                let sub_array = self.substitute_parameters(array, param_map, ctx)?;
                let sub_index = self.substitute_parameters(index, param_map, ctx)?;
                ResolvedExprKind::Index {
                    array: sub_array,
                    index: sub_index,
                }
            }

            // Nested function calls - substitute in arguments
            ResolvedExprKind::FunctionCall {
                name,
                function,
                args,
            } => {
                let sub_args = args
                    .iter()
                    .map(|arg| self.substitute_parameters(arg, param_map, ctx))
                    .collect::<Result<Vec<_>, _>>()?;
                ResolvedExprKind::FunctionCall {
                    name,
                    function,
                    args: sub_args,
                }
            }

            // Method calls - substitute in receiver and arguments
            ResolvedExprKind::MethodCall {
                receiver,
                method_name,
                method,
                args,
            } => {
                let sub_receiver = self.substitute_parameters(receiver, param_map, ctx)?;
                let sub_args = args
                    .iter()
                    .map(|arg| self.substitute_parameters(arg, param_map, ctx))
                    .collect::<Result<Vec<_>, _>>()?;
                ResolvedExprKind::MethodCall {
                    receiver: sub_receiver,
                    method_name,
                    method,
                    args: sub_args,
                }
            }

            // Literals - no substitution needed
            ResolvedExprKind::IntLit { .. }
            | ResolvedExprKind::FloatLit { .. }
            | ResolvedExprKind::BoolLit { .. } => {
                // Return original expression for literals
                return Ok(expr);
            }

            // Struct literals, array literals, ranges, closures - not needed for basic function calls
            _ => {
                return Err(SolverError::UnsupportedExpression(format!(
                    "Parameter substitution not supported for this expression type: {:?}",
                    expr.kind
                )));
            }
        };

        // Allocate new expression node with the substituted kind
        // Use the arena from the solver context
        Ok(ctx.arena.alloc(ResolvedExpr {
            span: expr.span,
            kind,
            ty: expr.ty,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_evaluate_expr_basic_structure() {
        // This is a basic compilation test to ensure the new methods compile correctly
        // Full functional tests will be in the integration test suite
        let arena = bumpalo::Bump::new();
        let z3_solver = z3::Solver::new();
        let z3_ctx = z3_solver.get_context().clone();
        let ctx = SolverContext::new(z3_ctx, z3_solver, &arena);

        // Just verify basic context creation works
        assert_eq!(ctx.scope_level(), 0);
    }
}
