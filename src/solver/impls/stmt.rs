//! Solvable trait implementations for statements
//!
//! This module implements the `Solvable` trait for `ResolvedStmt` nodes,
//! processing HIR statements and adding constraints to the Z3 solver.

use crate::hir::expr::{ResolvedExpr, ResolvedExprKind, ResolvedStmt, ResolvedStmtKind};
use crate::solver::context::{SolverContext, WithContextInfo};
use crate::solver::impls::expr::Z3Expr;
use crate::solver::{Solvable, SolverError, VariablePath};

impl<'src, 'arena> Solvable<'src, 'arena> for ResolvedStmt<'src, 'arena> {
    type Output = ();

    fn solve(&self, ctx: &mut SolverContext<'src, 'arena>) -> Result<Self::Output, SolverError> {
        match &self.kind {
            // Let statement - declare variable
            ResolvedStmtKind::Let {
                dot_prefix,
                name_path,
                var_def,
                init,
                ..
            } => {
                // Determine the full variable name
                let _full_name = if *dot_prefix {
                    // Dot-prefix variable in with-statement
                    match ctx.current_with_context() {
                        Some(WithContextInfo::Container {
                            container_path,
                            container_field,
                        }) => {
                            // Construct path: container.field.varname
                            let var_name = name_path.first().map(|(n, _)| *n).ok_or_else(|| {
                                SolverError::ContextError("Empty name path".to_string())
                            })?;

                            let full_path = container_path
                                .with_field(container_field.name)
                                .with_field(var_name);

                            // Declare variable at the constructed path
                            let var_type = var_def.var_type.as_ref().ok_or_else(|| {
                                SolverError::ContextError("Variable type not resolved".to_string())
                            })?;
                            ctx.declare_variable_at_path(&full_path, var_type)?;

                            // If there's an initializer, add constraint
                            if let Some(init_expr) = init {
                                let z3_value = init_expr.solve(ctx)?;
                                let z3_var = self.get_variable_z3(ctx, &full_path)?;

                                // Add equality constraint
                                let constraint = match (z3_var, z3_value) {
                                    (Z3Expr::Int(var), Z3Expr::Int(val)) => var.eq(&val),
                                    (Z3Expr::Real(var), Z3Expr::Real(val)) => var.eq(&val),
                                    (Z3Expr::Bool(var), Z3Expr::Bool(val)) => var.eq(&val),
                                    (Z3Expr::Int(var), Z3Expr::Real(val)) => var.to_real().eq(&val),
                                    (Z3Expr::Real(var), Z3Expr::Int(val)) => var.eq(val.to_real()),
                                    _ => {
                                        return Err(SolverError::UnsupportedExpression(
                                            "Type mismatch in initialization".to_string(),
                                        ));
                                    }
                                };

                                ctx.z3_solver.assert(&constraint);
                            }

                            full_path.to_z3_name()
                        }
                        _ => {
                            return Err(SolverError::ContextError(
                                "Dot-prefix variable outside with-statement".to_string(),
                            ));
                        }
                    }
                } else {
                    // Regular variable
                    let var_name = name_path
                        .first()
                        .map(|(n, _)| *n)
                        .ok_or_else(|| SolverError::ContextError("Empty name path".to_string()))?;

                    // Declare variable
                    let var_type = var_def.var_type.as_ref().ok_or_else(|| {
                        SolverError::ContextError("Variable type not resolved".to_string())
                    })?;
                    ctx.declare_variable(var_name, var_type)?;

                    // If there's an initializer, add constraint
                    if let Some(init_expr) = init {
                        let z3_value = init_expr.solve(ctx)?;
                        let path = VariablePath::from_name(var_name);
                        let z3_var = self.get_variable_z3(ctx, &path)?;

                        // Add equality constraint
                        let constraint = match (z3_var, z3_value) {
                            (Z3Expr::Int(var), Z3Expr::Int(val)) => var.eq(&val),
                            (Z3Expr::Real(var), Z3Expr::Real(val)) => var.eq(&val),
                            (Z3Expr::Bool(var), Z3Expr::Bool(val)) => var.eq(&val),
                            (Z3Expr::Int(var), Z3Expr::Real(val)) => var.to_real().eq(&val),
                            (Z3Expr::Real(var), Z3Expr::Int(val)) => var.eq(val.to_real()),
                            _ => {
                                return Err(SolverError::UnsupportedExpression(
                                    "Type mismatch in initialization".to_string(),
                                ));
                            }
                        };

                        ctx.z3_solver.assert(&constraint);
                    }

                    var_name.to_string()
                };

                Ok(())
            }

            // Expression statement - add as constraint
            ResolvedStmtKind::Expression { expr, .. } => {
                let z3_expr = expr.solve(ctx)?;

                // Expression statements must evaluate to boolean (constraints)
                match z3_expr {
                    Z3Expr::Bool(constraint) => {
                        ctx.z3_solver.assert(&constraint);
                        Ok(())
                    }
                    _ => Err(SolverError::UnsupportedStatement(
                        "Expression statement must evaluate to boolean".to_string(),
                    )),
                }
            }

            // For loop - with deferral support
            ResolvedStmtKind::For {
                loop_var_def,
                iterator,
                body,
                ..
            } => {
                // Try to evaluate the range
                match self.evaluate_range(iterator, ctx) {
                    Ok((start, end)) => {
                        // Range is known - unroll the loop immediately
                        self.unroll_loop(ctx, loop_var_def.name, start, end, body)?;
                        Ok(())
                    }
                    Err(SolverError::UndefinedVariable(var_name)) => {
                        // Range depends on unknown variable - defer this loop
                        // Note: We need to leak the string to get a 'src lifetime reference
                        // This is safe because defer_constraint stores the description as String
                        let var_name_static = Box::leak(var_name.into_boxed_str());
                        ctx.defer_constraint(
                            vec![var_name_static],
                            format!(
                                "for-loop range depends on unknown variable '{}'",
                                var_name_static
                            ),
                        );
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }

            // If statement
            ResolvedStmtKind::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                // Solve condition
                let cond_z3 = condition.solve(ctx)?;
                let cond_bool = cond_z3.to_bool(&ctx.z3_ctx).map_err(|_| {
                    SolverError::UnsupportedExpression("If condition must be boolean".to_string())
                })?;

                // Process then branch - wrap constraints with condition
                for stmt in then_branch {
                    self.process_conditional_stmt(ctx, stmt, &cond_bool, false)?;
                }

                // Process else branch if present
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.process_conditional_stmt(ctx, stmt, &cond_bool, true)?;
                    }
                }

                Ok(())
            }

            // Block statement - process all statements in the block
            ResolvedStmtKind::Block { statements, .. } => {
                for stmt in statements {
                    stmt.solve(ctx)?;
                }
                Ok(())
            }

            // With statement - set context and process body
            ResolvedStmtKind::With {
                with_context, body, ..
            } => {
                // Enter the with-context
                ctx.push_with_context(with_context);

                // Process all statements in the body
                for stmt in body {
                    stmt.solve(ctx)?;
                }

                // Exit the with-context
                ctx.pop_with_context();

                Ok(())
            }

            // StructDef and FunctionDef - skip these, they're definitions not executable statements
            // They've already been processed during semantic analysis
            ResolvedStmtKind::StructDef { .. } | ResolvedStmtKind::FunctionDef { .. } => {
                // Skip definitions - they don't contribute constraints
                Ok(())
            }

            // Unsupported statements
            _ => Err(SolverError::UnsupportedStatement(format!(
                "{:?}",
                self.kind
            ))),
        }
    }
}

impl<'src, 'arena> ResolvedStmt<'src, 'arena> {
    /// Evaluate a range expression to concrete (start, end) values
    ///
    /// Returns an error if the range depends on unknown variables.
    fn evaluate_range(
        &self,
        iterator: &ResolvedExpr<'src, 'arena>,
        ctx: &SolverContext<'src, 'arena>,
    ) -> Result<(i64, i64), SolverError> {
        match &iterator.kind {
            ResolvedExprKind::Range { start, end } => {
                let start_val = self.evaluate_const_expr(start, ctx)?;
                let end_val = self.evaluate_const_expr(end, ctx)?;
                Ok((start_val, end_val))
            }
            _ => Err(SolverError::UnsupportedExpression(
                "For loop iterator must be a range expression".to_string(),
            )),
        }
    }

    /// Evaluate an expression to a constant integer value
    ///
    /// This is used for loop ranges. Returns the variable name as an error
    /// if the expression depends on an unknown variable.
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
                        // Variable exists but not yet resolved - return var name as error
                        Err(SolverError::UndefinedVariable(name.to_string()))
                    }
                } else {
                    // No solution available yet - return var name as error
                    Err(SolverError::UndefinedVariable(name.to_string()))
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

            ResolvedExprKind::Paren { inner } => {
                // Parenthesized expressions - just evaluate the inner expression
                self.evaluate_const_expr(inner, ctx)
            }

            _ => Err(SolverError::UnsupportedExpression(format!(
                "Cannot evaluate expression to constant: {:?}",
                expr.kind
            ))),
        }
    }

    /// Unroll a for loop with known range bounds
    ///
    /// Creates constraints for each iteration of the loop.
    fn unroll_loop(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        loop_var_name: &'src str,
        start: i64,
        end: i64,
        body: &[&'arena ResolvedStmt<'src, 'arena>],
    ) -> Result<(), SolverError> {
        // Unroll the loop
        for i in start..end {
            // For each iteration, we need to substitute the loop variable
            // with the current iteration value in the body

            // For now, we'll create constraints assuming the loop variable
            // is used in a simple way. This is a simplified implementation
            // that handles common cases.

            // TODO: Full implementation would need to:
            // 1. Create a temporary variable for the loop iteration
            // 2. Substitute it in all expressions in the body
            // 3. Process the modified body statements

            for stmt in body {
                // Process statements, substituting loop variable with current value
                self.process_loop_body_stmt(ctx, stmt, loop_var_name, i)?;
            }
        }

        Ok(())
    }

    /// Process a statement from a loop body, substituting the loop variable
    ///
    /// This is a helper for loop unrolling that handles variable substitution.
    fn process_loop_body_stmt(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        stmt: &'arena ResolvedStmt<'src, 'arena>,
        loop_var_name: &'src str,
        loop_value: i64,
    ) -> Result<(), SolverError> {
        match &stmt.kind {
            ResolvedStmtKind::Expression { expr, .. } => {
                // Substitute loop variable and solve
                let z3_expr =
                    self.solve_expr_with_substitution(ctx, expr, loop_var_name, loop_value)?;

                // Add as constraint
                match z3_expr {
                    Z3Expr::Bool(constraint) => {
                        ctx.z3_solver.assert(&constraint);
                        Ok(())
                    }
                    _ => Err(SolverError::UnsupportedStatement(
                        "Loop body expression must evaluate to boolean".to_string(),
                    )),
                }
            }
            _ => Err(SolverError::UnsupportedStatement(
                "Only constraint expressions allowed in loop body".to_string(),
            )),
        }
    }

    /// Solve an expression with loop variable substitution
    ///
    /// Replaces references to the loop variable with the concrete loop value.
    fn solve_expr_with_substitution(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        expr: &'arena ResolvedExpr<'src, 'arena>,
        loop_var_name: &'src str,
        loop_value: i64,
    ) -> Result<Z3Expr, SolverError> {
        match &expr.kind {
            // If this is the loop variable, return the constant value
            ResolvedExprKind::Var { name, .. } if *name == loop_var_name => {
                Ok(Z3Expr::Int(z3::ast::Int::from_i64(loop_value)))
            }

            // For all other cases, recursively substitute in subexpressions
            ResolvedExprKind::FieldAccess {
                receiver,
                field_name,
                ..
            } => {
                // For field access, we need to handle the case where the base uses the loop var
                // For example: points[i].x where i is the loop variable
                let base_path = self.build_var_path_with_substitution(
                    receiver,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                let full_path = base_path.with_field(field_name);

                let var_node = ctx.get_variable(&full_path).ok_or_else(|| {
                    SolverError::UndefinedVariable(format!("{}.{}", base_path, field_name))
                })?;

                let z3_var = var_node
                    .as_primitive()
                    .ok_or(SolverError::NotAPrimitiveType)?;

                Ok(match z3_var {
                    crate::solver::context::Z3Primitive::Int(z3_int) => Z3Expr::Int(z3_int.clone()),
                    crate::solver::context::Z3Primitive::Real(z3_real) => {
                        Z3Expr::Real(z3_real.clone())
                    }
                    crate::solver::context::Z3Primitive::Bool(z3_bool) => {
                        Z3Expr::Bool(z3_bool.clone())
                    }
                })
            }

            ResolvedExprKind::Index { array, index } => {
                // Evaluate index with substitution
                let index_val = self.evaluate_const_expr_with_substitution(
                    index,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;

                let base_path =
                    self.build_var_path_with_substitution(array, ctx, loop_var_name, loop_value)?;
                let full_path = base_path.with_index(index_val as usize);

                let var_node = ctx.get_variable(&full_path).ok_or_else(|| {
                    SolverError::UndefinedVariable(format!("{}[{}]", base_path, index_val))
                })?;

                let z3_var = var_node
                    .as_primitive()
                    .ok_or(SolverError::NotAPrimitiveType)?;

                Ok(match z3_var {
                    crate::solver::context::Z3Primitive::Int(z3_int) => Z3Expr::Int(z3_int.clone()),
                    crate::solver::context::Z3Primitive::Real(z3_real) => {
                        Z3Expr::Real(z3_real.clone())
                    }
                    crate::solver::context::Z3Primitive::Bool(z3_bool) => {
                        Z3Expr::Bool(z3_bool.clone())
                    }
                })
            }

            // Binary operations - recursively substitute
            ResolvedExprKind::Add { lhs, rhs } => {
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

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
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

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
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

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
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

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

            // Comparison operations
            ResolvedExprKind::Eq { lhs, rhs } => {
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.eq(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.eq(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().eq(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.eq(r.to_real()))),
                    (Z3Expr::Bool(l), Z3Expr::Bool(r)) => Ok(Z3Expr::Bool(l.eq(&r))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for equality comparison in loop".to_string(),
                    )),
                }
            }

            // Remaining comparison operations
            ResolvedExprKind::NotEq { lhs, rhs } => {
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.eq(&r).not())),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.eq(&r).not())),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().eq(&r).not())),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.eq(r.to_real()).not())),
                    (Z3Expr::Bool(l), Z3Expr::Bool(r)) => Ok(Z3Expr::Bool(l.eq(&r).not())),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for not-equal comparison in loop".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Lt { lhs, rhs } => {
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.lt(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.lt(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().lt(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.lt(r.to_real()))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for less-than comparison in loop".to_string(),
                    )),
                }
            }

            ResolvedExprKind::LtEq { lhs, rhs } => {
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.le(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.le(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().le(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.le(r.to_real()))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for less-or-equal comparison in loop".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Gt { lhs, rhs } => {
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.gt(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.gt(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().gt(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.gt(r.to_real()))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for greater-than comparison in loop".to_string(),
                    )),
                }
            }

            ResolvedExprKind::GtEq { lhs, rhs } => {
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.ge(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.ge(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Bool(l.to_real().ge(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Bool(l.ge(r.to_real()))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for greater-or-equal comparison in loop".to_string(),
                    )),
                }
            }

            // Logical operations
            ResolvedExprKind::And { lhs, rhs } => {
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Bool(l), Z3Expr::Bool(r)) => {
                        Ok(Z3Expr::Bool(z3::ast::Bool::and(&[&l, &r])))
                    }
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for logical AND in loop".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Or { lhs, rhs } => {
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Bool(l), Z3Expr::Bool(r)) => {
                        Ok(Z3Expr::Bool(z3::ast::Bool::or(&[&l, &r])))
                    }
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for logical OR in loop".to_string(),
                    )),
                }
            }

            // Unary operations
            ResolvedExprKind::Neg { inner } => {
                let operand_z3 =
                    self.solve_expr_with_substitution(ctx, inner, loop_var_name, loop_value)?;
                match operand_z3 {
                    Z3Expr::Int(i) => Ok(Z3Expr::Int(-i)),
                    Z3Expr::Real(r) => Ok(Z3Expr::Real(-r)),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Cannot negate boolean expression in loop".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Ref { inner } => {
                // Reference expressions: just recurse into the inner expression
                self.solve_expr_with_substitution(ctx, inner, loop_var_name, loop_value)
            }

            // Additional binary operations
            ResolvedExprKind::Mod { lhs, rhs } => {
                // Recursively substitute to handle loop variables in operands
                let _lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let _rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                todo!(
                    "Modulo operator not yet implemented in constraint solver. \
                     This will cause incorrect behavior. Please report this case."
                )
            }

            ResolvedExprKind::Pow { lhs, rhs } => {
                // Recursively substitute to handle loop variables in operands
                let _lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let _rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                todo!(
                    "Power operator not yet implemented in constraint solver. \
                     This will cause incorrect behavior. Please report this case."
                )
            }

            // Parenthesized expressions - just unwrap and recurse
            ResolvedExprKind::Paren { inner } => {
                self.solve_expr_with_substitution(ctx, inner, loop_var_name, loop_value)
            }

            // Literals - these don't contain the loop variable, so just evaluate normally
            ResolvedExprKind::IntLit { .. }
            | ResolvedExprKind::FloatLit { .. }
            | ResolvedExprKind::BoolLit { .. } => expr.solve(ctx),

            // For all other expressions that cannot contain the loop variable,
            // use the regular solve without substitution
            _ => expr.solve(ctx),
        }
    }

    /// Build variable path with loop variable substitution
    fn build_var_path_with_substitution(
        &self,
        expr: &ResolvedExpr<'src, 'arena>,
        ctx: &SolverContext<'src, 'arena>,
        loop_var_name: &'src str,
        loop_value: i64,
    ) -> Result<VariablePath<'src>, SolverError> {
        match &expr.kind {
            ResolvedExprKind::Var { name, .. } => Ok(VariablePath::from_name(name)),

            ResolvedExprKind::FieldAccess {
                receiver,
                field_name,
                ..
            } => {
                let base_path = self.build_var_path_with_substitution(
                    receiver,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                Ok(base_path.with_field(field_name))
            }

            ResolvedExprKind::Index { array, index } => {
                let index_val = self.evaluate_const_expr_with_substitution(
                    index,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                let base_path =
                    self.build_var_path_with_substitution(array, ctx, loop_var_name, loop_value)?;
                Ok(base_path.with_index(index_val as usize))
            }

            ResolvedExprKind::Paren { inner } => {
                // Unwrap parentheses and recurse
                self.build_var_path_with_substitution(inner, ctx, loop_var_name, loop_value)
            }

            _ => Err(SolverError::UnsupportedExpression(
                "Cannot build variable path from this expression".to_string(),
            )),
        }
    }

    /// Evaluate expression to constant with loop variable substitution
    fn evaluate_const_expr_with_substitution(
        &self,
        expr: &ResolvedExpr<'src, 'arena>,
        ctx: &SolverContext<'src, 'arena>,
        loop_var_name: &'src str,
        loop_value: i64,
    ) -> Result<i64, SolverError> {
        match &expr.kind {
            ResolvedExprKind::IntLit { value } => Ok((*value) as i64),

            ResolvedExprKind::Var { name, .. } if *name == loop_var_name => {
                // This is the loop variable - return the substitution value
                Ok(loop_value)
            }

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
                        Err(SolverError::UndefinedVariable(name.to_string()))
                    }
                } else {
                    Err(SolverError::UndefinedVariable(name.to_string()))
                }
            }

            ResolvedExprKind::Add { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr_with_substitution(
                    lhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                let rhs_val = self.evaluate_const_expr_with_substitution(
                    rhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                Ok(lhs_val + rhs_val)
            }

            ResolvedExprKind::Sub { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr_with_substitution(
                    lhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                let rhs_val = self.evaluate_const_expr_with_substitution(
                    rhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                Ok(lhs_val - rhs_val)
            }

            ResolvedExprKind::Mul { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr_with_substitution(
                    lhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                let rhs_val = self.evaluate_const_expr_with_substitution(
                    rhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                Ok(lhs_val * rhs_val)
            }

            ResolvedExprKind::Div { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr_with_substitution(
                    lhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                let rhs_val = self.evaluate_const_expr_with_substitution(
                    rhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                if rhs_val == 0 {
                    return Err(SolverError::UnsupportedExpression(
                        "Division by zero".to_string(),
                    ));
                }
                Ok(lhs_val / rhs_val)
            }

            ResolvedExprKind::Neg { inner } => {
                let val = self.evaluate_const_expr_with_substitution(
                    inner,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                Ok(-val)
            }

            ResolvedExprKind::Mod { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr_with_substitution(
                    lhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                let rhs_val = self.evaluate_const_expr_with_substitution(
                    rhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                if rhs_val == 0 {
                    return Err(SolverError::UnsupportedExpression(
                        "Modulo by zero".to_string(),
                    ));
                }
                Ok(lhs_val % rhs_val)
            }

            ResolvedExprKind::Pow { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr_with_substitution(
                    lhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                let rhs_val = self.evaluate_const_expr_with_substitution(
                    rhs,
                    ctx,
                    loop_var_name,
                    loop_value,
                )?;
                if rhs_val < 0 {
                    return Err(SolverError::UnsupportedExpression(
                        "Negative exponent not supported for integer power".to_string(),
                    ));
                }
                Ok(lhs_val.pow(rhs_val as u32))
            }

            ResolvedExprKind::Paren { inner } => {
                self.evaluate_const_expr_with_substitution(inner, ctx, loop_var_name, loop_value)
            }

            ResolvedExprKind::Ref { inner } => {
                self.evaluate_const_expr_with_substitution(inner, ctx, loop_var_name, loop_value)
            }

            _ => Err(SolverError::UnsupportedExpression(format!(
                "Cannot evaluate expression to constant: {:?}",
                expr.kind
            ))),
        }
    }

    /// Get Z3 variable from a variable path
    fn get_variable_z3(
        &self,
        ctx: &SolverContext<'src, 'arena>,
        path: &VariablePath<'src>,
    ) -> Result<Z3Expr, SolverError> {
        let var_node = ctx
            .get_variable(path)
            .ok_or_else(|| SolverError::UndefinedVariable(path.to_z3_name()))?;

        let z3_var = var_node
            .as_primitive()
            .ok_or(SolverError::NotAPrimitiveType)?;

        Ok(match z3_var {
            crate::solver::context::Z3Primitive::Int(z3_int) => Z3Expr::Int(z3_int.clone()),
            crate::solver::context::Z3Primitive::Real(z3_real) => Z3Expr::Real(z3_real.clone()),
            crate::solver::context::Z3Primitive::Bool(z3_bool) => Z3Expr::Bool(z3_bool.clone()),
        })
    }

    /// Process a statement inside a conditional branch (if-statement)
    ///
    /// Wraps the statement's constraints with the condition using implication.
    /// If `negate` is true, uses !condition (for else branch).
    fn process_conditional_stmt(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        stmt: &'arena ResolvedStmt<'src, 'arena>,
        condition: &z3::ast::Bool,
        negate: bool,
    ) -> Result<(), SolverError> {
        let actual_condition = if negate {
            condition.not()
        } else {
            condition.clone()
        };

        match &stmt.kind {
            // Expression statement - add as conditional constraint
            ResolvedStmtKind::Expression { expr, .. } => {
                let constraint = expr.solve(ctx)?;
                let constraint_bool = constraint.to_bool(&ctx.z3_ctx)?;

                // Add implication: condition => constraint
                let implication = actual_condition.implies(&constraint_bool);
                ctx.z3_solver.assert(&implication);
                Ok(())
            }

            // Assignment statement - create conditional constraint
            ResolvedStmtKind::Assignment { var_def, value, .. } => {
                let var_name = var_def.name;
                let path = VariablePath::from_name(var_name);
                let z3_var = self.get_variable_z3(ctx, &path)?;
                let z3_value = value.solve(ctx)?;

                // Create equality constraint
                let equality = match (z3_var, z3_value) {
                    (Z3Expr::Int(var), Z3Expr::Int(val)) => var.eq(&val),
                    (Z3Expr::Real(var), Z3Expr::Real(val)) => var.eq(&val),
                    (Z3Expr::Bool(var), Z3Expr::Bool(val)) => var.eq(&val),
                    (Z3Expr::Int(var), Z3Expr::Real(val)) => var.to_real().eq(&val),
                    (Z3Expr::Real(var), Z3Expr::Int(val)) => var.eq(val.to_real()),
                    _ => {
                        return Err(SolverError::UnsupportedExpression(
                            "Type mismatch in conditional assignment".to_string(),
                        ));
                    }
                };

                // Add implication: condition => (var = value)
                let implication = actual_condition.implies(&equality);
                ctx.z3_solver.assert(&implication);
                Ok(())
            }

            // Field assignment statement - create conditional constraint
            ResolvedStmtKind::FieldAssignment { target, value, .. } => {
                // Build the path for the target field
                let path = self.build_var_path(target, ctx)?;
                let z3_var = self.get_variable_z3(ctx, &path)?;
                let z3_value = value.solve(ctx)?;

                // Create equality constraint
                let equality = match (z3_var, z3_value) {
                    (Z3Expr::Int(var), Z3Expr::Int(val)) => var.eq(&val),
                    (Z3Expr::Real(var), Z3Expr::Real(val)) => var.eq(&val),
                    (Z3Expr::Bool(var), Z3Expr::Bool(val)) => var.eq(&val),
                    (Z3Expr::Int(var), Z3Expr::Real(val)) => var.to_real().eq(&val),
                    (Z3Expr::Real(var), Z3Expr::Int(val)) => var.eq(val.to_real()),
                    _ => {
                        return Err(SolverError::UnsupportedExpression(
                            "Type mismatch in conditional field assignment".to_string(),
                        ));
                    }
                };

                // Add implication: condition => (field = value)
                let implication = actual_condition.implies(&equality);
                ctx.z3_solver.assert(&implication);
                Ok(())
            }

            // Nested if statement - recursively process
            ResolvedStmtKind::If {
                condition: inner_cond,
                then_branch,
                else_branch,
                ..
            } => {
                // Solve inner condition
                let inner_cond_z3 = inner_cond.solve(ctx)?;
                let inner_cond_bool = inner_cond_z3.to_bool(&ctx.z3_ctx).map_err(|_| {
                    SolverError::UnsupportedExpression(
                        "Nested if condition must be boolean".to_string(),
                    )
                })?;

                // Process then branch: outer_condition => (inner_condition => inner_constraint)
                // Which is equivalent to: (outer_condition AND inner_condition) => inner_constraint
                let combined_then_cond = z3::ast::Bool::and(&[&actual_condition, &inner_cond_bool]);
                for inner_stmt in then_branch {
                    self.process_conditional_stmt(ctx, inner_stmt, &combined_then_cond, false)?;
                }

                // Process else branch: outer_condition => (!inner_condition => inner_constraint)
                // Which is equivalent to: (outer_condition AND !inner_condition) => inner_constraint
                if let Some(else_stmts) = else_branch {
                    let combined_else_cond =
                        z3::ast::Bool::and(&[&actual_condition, &inner_cond_bool.not()]);
                    for inner_stmt in else_stmts {
                        self.process_conditional_stmt(ctx, inner_stmt, &combined_else_cond, false)?;
                    }
                }

                Ok(())
            }

            // Block statement - process all statements in the block
            ResolvedStmtKind::Block { statements, .. } => {
                for inner_stmt in statements {
                    self.process_conditional_stmt(ctx, inner_stmt, condition, negate)?;
                }
                Ok(())
            }

            // Let statements are not allowed in conditional branches
            // (they would need scoping semantics we don't support)
            ResolvedStmtKind::Let { .. } => Err(SolverError::UnsupportedStatement(
                "Variable declarations (let) are not allowed inside if-statement branches"
                    .to_string(),
            )),

            // Other statements are not supported in conditional branches
            _ => Err(SolverError::UnsupportedStatement(format!(
                "Statement type not supported in if-statement branches: {:?}",
                stmt.kind
            ))),
        }
    }

    /// Build a variable path from an expression
    fn build_var_path(
        &self,
        expr: &ResolvedExpr<'src, 'arena>,
        ctx: &SolverContext<'src, 'arena>,
    ) -> Result<VariablePath<'src>, SolverError> {
        match &expr.kind {
            ResolvedExprKind::Var { name, .. } => Ok(VariablePath::from_name(name)),

            ResolvedExprKind::FieldAccess {
                receiver,
                field_name,
                ..
            } => {
                let base_path = self.build_var_path(receiver, ctx)?;
                Ok(base_path.with_field(field_name))
            }

            ResolvedExprKind::Index { array, index } => {
                // Evaluate index to constant
                let index_val = self.evaluate_const_expr(index, ctx)?;
                let base_path = self.build_var_path(array, ctx)?;
                Ok(base_path.with_index(index_val as usize))
            }

            ResolvedExprKind::Paren { inner } => self.build_var_path(inner, ctx),

            _ => Err(SolverError::UnsupportedExpression(
                "Cannot build variable path from this expression".to_string(),
            )),
        }
    }
}
