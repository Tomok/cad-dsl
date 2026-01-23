//! Solvable trait implementations for statements
//!
//! This module implements the `Solvable` trait for `ResolvedStmt` nodes,
//! processing HIR statements and adding constraints to the Z3 solver.

use crate::hir::expr::{ResolvedExpr, ResolvedExprKind, ResolvedStmt, ResolvedStmtKind};
use crate::hir::types::ResolvedType;
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
                            ..
                        }) => {
                            // Construct path: container.field.varname
                            let var_name = name_path.first().map(|(n, _)| *n).ok_or_else(|| {
                                SolverError::ContextError("Empty name path".to_string())
                            })?;

                            let full_path = container_path
                                .with_field(container_field.name)
                                .with_field(var_name);

                            // Check if this is a reference alias (let .r = &x or let .r = get_ref())
                            // This now supports type-based alias tracking for function/method returns
                            let target_path = if let Some(init_expr) = init {
                                self.extract_reference_target_with_inlining(init_expr, ctx)
                            } else {
                                None
                            };

                            if let Some(target) = target_path {
                                // This is an alias declaration
                                // Don't create a variable, just register the alias
                                ctx.register_alias(full_path.clone(), target);
                            } else {
                                // Regular variable declaration
                                let var_type = var_def.var_type.as_ref().ok_or_else(|| {
                                    SolverError::ContextError(
                                        "Variable type not resolved".to_string(),
                                    )
                                })?;
                                ctx.declare_variable_at_path(&full_path, var_type)?;

                                // Check if we should apply a transform (no initializer in transform context)
                                if init.is_none() {
                                    // Try to apply transform if we're in a transform context
                                    self.apply_transform_to_variable(ctx, &full_path, var_type)?;
                                }

                                // If there's an initializer, add constraint
                                if let Some(init_expr) = init {
                                    // Special handling for struct literals
                                    if let ResolvedExprKind::StructLit { fields, .. } =
                                        &init_expr.kind
                                    {
                                        // Handle struct literal with potential reference fields
                                        self.process_struct_literal_init(ctx, &full_path, fields)?;
                                    } else {
                                        // Regular primitive initialization
                                        let z3_value = init_expr.solve(ctx)?;
                                        let z3_var = self.get_variable_z3(ctx, &full_path)?;

                                        // Add equality constraint
                                        let constraint = match (z3_var, z3_value) {
                                            (Z3Expr::Int(var), Z3Expr::Int(val)) => var.eq(&val),
                                            (Z3Expr::Real(var), Z3Expr::Real(val)) => var.eq(&val),
                                            (Z3Expr::Bool(var), Z3Expr::Bool(val)) => var.eq(&val),
                                            (Z3Expr::Int(var), Z3Expr::Real(val)) => {
                                                var.to_real().eq(&val)
                                            }
                                            (Z3Expr::Real(var), Z3Expr::Int(val)) => {
                                                var.eq(val.to_real())
                                            }
                                            _ => {
                                                return Err(SolverError::UnsupportedExpression(
                                                    "Type mismatch in initialization".to_string(),
                                                ));
                                            }
                                        };

                                        ctx.z3_solver.assert(&constraint);
                                    }
                                }
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

                    // Check if this is a reference alias (let r = &x or let r = get_ref())
                    // This now supports type-based alias tracking for function/method returns
                    let target_path = if let Some(init_expr) = init {
                        self.extract_reference_target_with_inlining(init_expr, ctx)
                    } else {
                        None
                    };

                    if let Some(target) = target_path {
                        // This is an alias declaration
                        // Don't create a variable, just register the alias
                        let alias_path = VariablePath::from_name(var_name);
                        ctx.register_alias(alias_path, target);
                    } else {
                        // Regular variable declaration
                        let var_type = var_def.var_type.as_ref().ok_or_else(|| {
                            SolverError::ContextError("Variable type not resolved".to_string())
                        })?;
                        ctx.declare_variable(var_name, var_type)?;

                        // If there's an initializer, add constraint
                        if let Some(init_expr) = init {
                            // Special handling for struct literals
                            if let ResolvedExprKind::StructLit { fields, .. } = &init_expr.kind {
                                // Handle struct literal with potential reference fields
                                let base_path = VariablePath::from_name(var_name);
                                self.process_struct_literal_init(ctx, &base_path, fields)?;
                            } else {
                                // Regular primitive initialization
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
                        }
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
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Int(l.modulo(&r))),
                    // Modulo is only defined for integers in Z3
                    _ => Err(SolverError::UnsupportedExpression(
                        "Modulo operation only supported for integer types in loop".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Pow { lhs, rhs } => {
                // Recursively substitute to handle loop variables in operands
                let lhs_z3 =
                    self.solve_expr_with_substitution(ctx, lhs, loop_var_name, loop_value)?;
                let rhs_z3 =
                    self.solve_expr_with_substitution(ctx, rhs, loop_var_name, loop_value)?;

                // Power operations in Z3 always return Real type
                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Real(l.power(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l.power(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l.to_real().power(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Real(l.power(r.to_real()))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for power operation in loop".to_string(),
                    )),
                }
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

    /// Check if a type is a reference type
    fn is_reference_type(ty: &ResolvedType<'src, 'arena>) -> bool {
        matches!(ty, ResolvedType::Reference { .. })
    }

    /// Extract the target path from a reference expression or reference-returning function
    ///
    /// This method supports type-based alias tracking by checking function return types.
    /// It handles:
    /// - Explicit references: `&x`, `&p.x`, `&arr[0]`
    /// - Variables with reference types: `p` where `p: &Type`
    /// - Functions returning references: `get_ref()` where `fn get_ref() -> &Type`
    /// - Methods returning references: `obj.get_ref()` where method returns `&Type`
    ///
    /// When a function/method returns a reference, this method inlines it and recursively
    /// extracts the target from the return expression.
    fn extract_reference_target_with_inlining(
        &self,
        expr: &ResolvedExpr<'src, 'arena>,
        ctx: &mut SolverContext<'src, 'arena>,
    ) -> Option<VariablePath<'src>> {
        match &expr.kind {
            // Explicit reference expression (&x)
            ResolvedExprKind::Ref { inner } => self.build_var_path(inner, ctx).ok(),

            // Variable with reference type - the variable itself is the target
            // This handles cases like: `fn f(p: &T) -> Container { return Container { ref: p }; }`
            // where `p` has a reference type
            ResolvedExprKind::Var { .. } if expr.ty.is_reference() => {
                self.build_var_path(expr, ctx).ok()
            }

            // Parentheses - unwrap and recurse
            ResolvedExprKind::Paren { inner } => {
                self.extract_reference_target_with_inlining(inner, ctx)
            }

            // Function call - check if it returns a reference type
            ResolvedExprKind::FunctionCall {
                name,
                function,
                args,
            } => {
                // Check if function returns a reference type
                if Self::is_reference_type(&function.return_type) {
                    // Inline the function and extract the target from the return expression
                    self.inline_and_extract_reference(name, function, args, None, ctx)
                } else {
                    None
                }
            }

            // Method call - check if it returns a reference type
            ResolvedExprKind::MethodCall {
                receiver,
                method_name,
                method,
                args,
            } => {
                // Check if method returns a reference type
                if Self::is_reference_type(&method.return_type) {
                    // Inline the method and extract the target from the return expression
                    self.inline_and_extract_reference(
                        method_name,
                        method,
                        args,
                        Some(*receiver),
                        ctx,
                    )
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    /// Inline a function/method and extract the reference target from its return expression
    ///
    /// This performs parameter substitution and then recursively extracts the reference
    /// target from the inlined return expression.
    fn inline_and_extract_reference(
        &self,
        function_name: &'src str,
        function: &'arena crate::hir::definitions::FunctionDefinition<'src, 'arena>,
        args: &[&'arena ResolvedExpr<'src, 'arena>],
        receiver: Option<&'arena ResolvedExpr<'src, 'arena>>,
        ctx: &mut SolverContext<'src, 'arena>,
    ) -> Option<VariablePath<'src>> {
        use std::collections::HashMap;

        // Get the qualified name (for methods: StructName::method_name)
        let qualified_name = if let Some(parent) = function.parent_struct {
            format!("{}::{}", parent.name, function_name)
        } else {
            function_name.to_string()
        };

        // Get the return expression
        let return_expr = ctx.get_function_return(&qualified_name)?;

        // Create parameter substitution map
        let mut param_map: HashMap<&'src str, &'arena ResolvedExpr<'src, 'arena>> = HashMap::new();

        // If this is a method, map "self" to the receiver
        if let Some(recv) = receiver {
            param_map.insert("self", recv);
        }

        // Map parameters to arguments
        for (param, arg) in function.params.iter().zip(args.iter()) {
            param_map.insert(param.name, *arg);
        }

        // Substitute parameters in the return expression
        // We need to use the ResolvedExpr's substitute_parameters method
        // Since it's on a different type, we'll need to access it through the expr module
        let inlined_expr = self
            .substitute_params_in_expr(return_expr, &param_map, ctx)
            .ok()?;

        // Recursively extract the reference target from the inlined expression
        self.extract_reference_target_with_inlining(inlined_expr, ctx)
    }

    /// Substitute parameters in an expression (wrapper around expr module's implementation)
    fn substitute_params_in_expr(
        &self,
        expr: &'arena ResolvedExpr<'src, 'arena>,
        param_map: &std::collections::HashMap<&'src str, &'arena ResolvedExpr<'src, 'arena>>,
        ctx: &SolverContext<'src, 'arena>,
    ) -> Result<&'arena ResolvedExpr<'src, 'arena>, SolverError> {
        // Delegate to the expression's substitute_parameters method
        // Note: self here is a ResolvedStmt, but we need to call the method on a ResolvedExpr
        // We use the expr itself as the receiver
        expr.substitute_parameters(expr, param_map, ctx)
    }

    /// Process a struct literal initialization, handling reference fields
    ///
    /// This method processes each field in a struct literal:
    /// - For reference-typed fields with reference values: creates an alias
    /// - For other fields: creates normal constraints
    fn process_struct_literal_init(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        base_path: &VariablePath<'src>,
        fields: &[crate::hir::expr::ResolvedStructLitField<'src, 'arena>],
    ) -> Result<(), SolverError> {
        use crate::hir::expr::ResolvedStructLitField;

        for field in fields {
            match field {
                ResolvedStructLitField::Field {
                    name,
                    value,
                    field_def,
                    ..
                } => {
                    let field_path = base_path.with_field(name);

                    // Check if this is a reference-typed field
                    if field_def.field_type.is_reference() {
                        // This is a reference field - try to extract the target
                        if let Some(target_path) =
                            self.extract_reference_target_with_inlining(value, ctx)
                        {
                            // Create an alias
                            ctx.register_alias(field_path, target_path);
                        } else {
                            // Reference field without a clear target - this shouldn't happen
                            // in well-typed code, but handle it gracefully
                            return Err(SolverError::ContextError(format!(
                                "Reference field '{}' must be initialized with a reference expression",
                                name
                            )));
                        }
                    } else {
                        // Regular (non-reference) field - solve and add constraint
                        let z3_value = value.solve(ctx)?;
                        let z3_var = self.get_variable_z3(ctx, &field_path)?;

                        // Add equality constraint
                        let constraint = match (z3_var, z3_value) {
                            (Z3Expr::Int(var), Z3Expr::Int(val)) => var.eq(&val),
                            (Z3Expr::Real(var), Z3Expr::Real(val)) => var.eq(&val),
                            (Z3Expr::Bool(var), Z3Expr::Bool(val)) => var.eq(&val),
                            (Z3Expr::Int(var), Z3Expr::Real(val)) => var.to_real().eq(&val),
                            (Z3Expr::Real(var), Z3Expr::Int(val)) => var.eq(val.to_real()),
                            _ => {
                                return Err(SolverError::UnsupportedExpression(format!(
                                    "Type mismatch in field '{}' initialization",
                                    name
                                )));
                            }
                        };

                        ctx.z3_solver.assert(&constraint);
                    }
                }
                ResolvedStructLitField::ComputedProperty { .. } => {
                    // Computed properties are not supported in initialization
                    return Err(SolverError::UnsupportedExpression(
                        "Computed properties in struct literals are not supported".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Process a reference field in a transform result
    ///
    /// When a transform returns a struct literal with reference fields,
    /// those references need special handling:
    /// - If the reference target's type matches the field's inner type: create direct alias
    /// - If the types differ: apply appropriate transform to create a new shadow variable
    ///
    /// This implements "transparent" reference transformation semantics.
    fn process_reference_field_in_transform(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        base_path: &VariablePath<'src>,
        field: &crate::hir::expr::ResolvedStructLitField<'src, 'arena>,
        transforms: &[crate::hir::TransformMethod<'src, 'arena>],
        context_expr: &'arena ResolvedExpr<'src, 'arena>,
    ) -> Result<(), SolverError> {
        use crate::hir::expr::ResolvedStructLitField;

        // Extract field information
        let (field_name, field_value, field_def) = match field {
            ResolvedStructLitField::Field {
                name,
                value,
                field_def,
                ..
            } => (name, value, field_def),
            _ => {
                return Err(SolverError::UnsupportedExpression(
                    "Only regular fields supported in transform struct literals".to_string(),
                ));
            }
        };

        // Extract the inner type from the reference
        let field_inner_type = field_def.field_type.as_reference().ok_or_else(|| {
            SolverError::ContextError("Expected reference type for reference field".to_string())
        })?;

        // Extract reference target recursively
        let mut target_path = self
            .extract_reference_target_with_inlining(field_value, ctx)
            .ok_or_else(|| {
                SolverError::ContextError(format!(
                    "Reference field '{}' must have a clear reference target",
                    field_name
                ))
            })?;

        // Follow alias chain for nested references
        target_path = ctx.resolve_alias(&target_path);

        // Get the target variable's type
        // We need to infer the type from the variable node structure
        // For shadow variables, we can look at what they point to
        // For regular variables, we need to traverse the path to get the type

        // For now, let's assume we can get the type from the variable path
        // We'll need to determine the target type somehow...
        // Actually, looking at the existing code, I don't see a direct way to get the type
        // from a variable path. Let me think about this differently.

        // The issue is: we have a target_path (e.g., "__shadow_0" or "sketch.origin")
        // and we need to know its type to decide if we need to transform it.

        // One approach: Check if there's a transform that takes the field_inner_type
        // as input and outputs the same field_inner_type. If not, we might need to
        // look for a transform.

        // But actually, the simpler approach is: try to find a transform from
        // some input type to field_inner_type. If we find one, we know we need
        // to transform. If not, we assume types match.

        // Even simpler: let's just try to solve the field_value to see if it works.
        // If it's a simple reference to a same-typed variable, create an alias.
        // If it requires transformation, we'll need to handle it.

        // Actually, re-reading the plan and user's intent: the user wants us to
        // transform references when they point to shadow variables of a different type.

        // The challenge is determining the target's type. Let me look for clues...
        // Looking at the shadow variable creation code, shadows are created with a type,
        // but that type isn't stored in the variable tree.

        // Wait! I have an idea. Let me check if the target is a struct by trying to
        // access it in the variable tree and seeing what kind of node it is.
        // Then I can look at the transforms to see if any of them have an input type
        // that would match a struct.

        // Actually, there's a simpler approach: I can try both strategies:
        // 1. First, try to create a direct alias (assume types match)
        // 2. If that fails or if we detect it's a shadow variable with known transform,
        //    then apply the transform

        // Let me use a heuristic: if the target variable name starts with "__shadow_",
        // it's likely a shadow variable that might need transformation.
        // Otherwise, create a direct alias.

        let target_name = target_path.to_z3_name();
        let is_shadow = target_name.starts_with("__shadow_");

        if !is_shadow {
            // Not a shadow variable - create direct alias
            let field_path = base_path.with_field(field_name);
            ctx.register_alias(field_path, target_path);
            return Ok(());
        }

        // Shadow variable - we might need to transform it
        // Try to find a transform that produces the field's inner type
        // We'll try all transforms and see which ones match

        // Find transforms that output the field_inner_type
        let matching_transforms: Vec<_> = transforms
            .iter()
            .filter(|t| Self::types_match_semantically(t.output_type, field_inner_type))
            .collect();

        if matching_transforms.is_empty() {
            // No matching transform - assume types match and create direct alias
            let field_path = base_path.with_field(field_name);
            ctx.register_alias(field_path, target_path);
            return Ok(());
        }

        if matching_transforms.len() > 1 {
            return Err(SolverError::ContextError(format!(
                "Multiple transforms found for reference field '{}' with type {:?}",
                field_name, field_inner_type
            )));
        }

        let transform_method = matching_transforms[0];

        // Create new shadow variable for the transformed reference
        let new_shadow_path = self.create_shadow_variable(ctx, field_inner_type)?;

        // Create reference expression for the original target
        let target_ref_expr =
            self.create_var_ref_expr(ctx, &target_path, transform_method.input_type)?;

        // Apply transform to link the new shadow to the original target
        let transform_result =
            self.inline_transform_method(ctx, context_expr, transform_method, &[target_ref_expr])?;

        // Add constraint linking new shadow to transform result
        // Handle struct literals specially
        use crate::hir::expr::ResolvedExprKind;
        match &transform_result.kind {
            ResolvedExprKind::StructLit { fields, .. } => {
                // For struct literals, create field-wise equality constraints
                for result_field in fields {
                    use crate::hir::expr::ResolvedStructLitField;
                    match result_field {
                        ResolvedStructLitField::Field { name, value, .. } => {
                            let field_z3 = value.solve(ctx)?;
                            let shadow_field_path = new_shadow_path.with_field(name);
                            self.add_struct_equality_constraint(
                                ctx,
                                &shadow_field_path,
                                &field_z3,
                            )?;
                        }
                        ResolvedStructLitField::ComputedProperty { .. } => {
                            return Err(SolverError::UnsupportedExpression(
                                "Computed properties not supported in reference field transforms"
                                    .to_string(),
                            ));
                        }
                    }
                }
            }
            _ => {
                // For non-struct, solve to Z3 and add constraint
                let result_z3 = transform_result.solve(ctx)?;
                self.add_struct_equality_constraint(ctx, &new_shadow_path, &result_z3)?;
            }
        }

        // Register alias from reference field to the transformed shadow
        let field_path = base_path.with_field(field_name);
        ctx.register_alias(field_path, new_shadow_path);

        Ok(())
    }

    /// Apply transform to a variable declaration in transform context
    ///
    /// Creates a shadow variable and links it to the declared variable
    /// via the appropriate __transform__ method.
    ///
    /// This is called when a variable is declared without an initializer
    /// inside a transform with-statement.
    fn apply_transform_to_variable(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        var_path: &VariablePath<'src>,
        declared_type: &'arena ResolvedType<'src, 'arena>,
    ) -> Result<(), SolverError> {
        // 1. Get transform context info
        let (transforms, context_expr, is_pure_transform) = match ctx.current_with_context() {
            Some(WithContextInfo::Transform {
                transforms,
                context_expr,
                ..
            }) => (transforms.clone(), *context_expr, true),
            Some(WithContextInfo::Container {
                transforms,
                context_expr,
                ..
            }) => (transforms.clone(), *context_expr, false),
            _ => return Ok(()), // Not in transform context, nothing to do
        };

        // If there are no transforms, return early
        if transforms.is_empty() {
            return Ok(());
        }

        // 2. Find matching transform method (output type == declared type)
        // Use semantic type comparison (ignoring spans) instead of direct ==
        let matching_transforms: Vec<_> = transforms
            .iter()
            .filter(|t| Self::types_match_semantically(t.output_type, declared_type))
            .collect();

        if matching_transforms.is_empty() {
            // No matching transform found
            if is_pure_transform {
                // For pure transform contexts, this is an error
                return Err(SolverError::ContextError(format!(
                    "No transform found for type {:?} in transform context",
                    declared_type
                )));
            } else {
                // For container contexts, no transform is okay - just use the variable as-is
                return Ok(());
            }
        }

        if matching_transforms.len() > 1 {
            // Multiple matching transforms - this is an error
            return Err(SolverError::ContextError(format!(
                "Multiple transforms found for type {:?} in transform context. \
                 Transform methods must have unique output types.",
                declared_type
            )));
        }

        let transform_method = matching_transforms[0];

        // 3. Get input type from transform method's first parameter
        let input_type = &transform_method.input_type;

        // 4. Create shadow variable with input type
        let shadow_path = self.create_shadow_variable(ctx, input_type)?;

        // 5. Create a reference expression for the shadow variable
        // We need to create a &'arena ResolvedExpr that references the shadow
        let shadow_ref_expr = self.create_var_ref_expr(ctx, &shadow_path, input_type)?;

        // 6. Inline the transform method call
        // Call: context_expr.__transform__(&shadow)
        let transform_result =
            self.inline_transform_method(ctx, context_expr, transform_method, &[shadow_ref_expr])?;

        // 7. Add constraint: var_path == transform_result
        // Handle struct literals specially by creating field-wise constraints
        use crate::hir::expr::{ResolvedExprKind, ResolvedStructLitField};
        match &transform_result.kind {
            ResolvedExprKind::StructLit { fields, .. } => {
                // Two-pass processing for struct literal fields:
                // Pass 1: Process non-reference fields
                for field in fields {
                    match field {
                        ResolvedStructLitField::Field {
                            name,
                            value,
                            field_def,
                            ..
                        } => {
                            // Skip reference fields - they'll be handled in pass 2
                            if field_def.field_type.is_reference() {
                                continue;
                            }

                            // Solve the field value expression to Z3
                            let field_z3 = value.solve(ctx)?;

                            // Create path to the struct field
                            let field_path = var_path.with_field(name);

                            // Add equality constraint for this field
                            self.add_struct_equality_constraint(ctx, &field_path, &field_z3)?;
                        }
                        ResolvedStructLitField::ComputedProperty { .. } => {
                            return Err(SolverError::UnsupportedExpression(
                                "Computed properties in transform results not supported"
                                    .to_string(),
                            ));
                        }
                    }
                }

                // Pass 2: Process reference fields with transform-aware aliasing
                for field in fields {
                    if let ResolvedStructLitField::Field { field_def, .. } = field {
                        // Only process reference fields
                        if field_def.field_type.is_reference() {
                            self.process_reference_field_in_transform(
                                ctx,
                                var_path,
                                field,
                                &transforms,
                                context_expr,
                            )?;
                        }
                    } // Computed properties already handled in pass 1
                }
            }
            _ => {
                // For non-struct expressions, solve to Z3 and add constraint
                let result_z3 = transform_result.solve(ctx)?;
                self.add_struct_equality_constraint(ctx, var_path, &result_z3)?;
            }
        }

        Ok(())
    }

    /// Create a shadow variable in the current scope
    ///
    /// Shadow variables are used to link transformed variables to their
    /// source values via transform constraints.
    fn create_shadow_variable(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        shadow_type: &'arena ResolvedType<'src, 'arena>,
    ) -> Result<VariablePath<'src>, SolverError> {
        // Generate unique name for shadow variable
        let shadow_id = ctx.next_shadow_id();
        let shadow_name = format!("__shadow_{}", shadow_id);

        // Leak string to get 'src lifetime (safe for solver duration)
        let shadow_name_static: &'src str = Box::leak(shadow_name.into_boxed_str());

        // Create variable in current scope
        ctx.declare_variable(shadow_name_static, shadow_type)?;

        Ok(VariablePath::from_name(shadow_name_static))
    }

    /// Create a variable reference expression in the arena
    ///
    /// This creates a ResolvedExpr node that references a variable by path.
    /// The type must be provided since VariableNode doesn't store full type information.
    fn create_var_ref_expr(
        &self,
        ctx: &SolverContext<'src, 'arena>,
        var_path: &VariablePath<'src>,
        var_type: &'arena ResolvedType<'src, 'arena>,
    ) -> Result<&'arena ResolvedExpr<'src, 'arena>, SolverError> {
        // Extract the root name from the path
        let root_name = match var_path.components().first() {
            Some(crate::solver::PathComponent::Field(name)) => *name,
            _ => {
                return Err(SolverError::ContextError(
                    "Invalid variable path for reference expression".to_string(),
                ));
            }
        };

        // Create a dummy span for generated code
        let dummy_span = crate::lexer::Span {
            start: crate::lexer::LineColumn { line: 0, column: 0 },
            lines: 0,
            end_column: 0,
        };

        // Create a dummy var definition in the arena
        use crate::hir::definitions::VarDefinition;
        let dummy_var_def = ctx.arena.alloc(VarDefinition {
            name: root_name,
            name_span: dummy_span,
            var_type: Some(*var_type), // Copy the type (ResolvedType is Copy)
            init: None,
            scope_level: 0,
            span: dummy_span,
        });

        // Create a Var expression
        let expr = ctx.arena.alloc(ResolvedExpr {
            span: dummy_span,
            kind: ResolvedExprKind::Var {
                name: root_name,
                definition: dummy_var_def,
            },
            ty: var_type,
        });

        Ok(expr)
    }

    /// Inline a transform method call
    ///
    /// Similar to inline_method in expr.rs, but used specifically for transforms.
    /// Returns the inlined expression (not yet converted to Z3).
    fn inline_transform_method(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        receiver_expr: &'arena ResolvedExpr<'src, 'arena>,
        transform: &crate::hir::TransformMethod<'src, 'arena>,
        args: &[&'arena ResolvedExpr<'src, 'arena>],
    ) -> Result<&'arena ResolvedExpr<'src, 'arena>, SolverError> {
        // Get the method from the transform
        let method = transform.function;

        // Get the qualified name (for methods: StructName::__transform__)
        let qualified_name = if let Some(parent) = method.parent_struct {
            format!("{}::{}", parent.name, "__transform__")
        } else {
            "__transform__".to_string()
        };

        // Get the return expression
        let return_expr = ctx.get_function_return(&qualified_name).ok_or_else(|| {
            SolverError::UnsupportedExpression(format!(
                "Transform method '{}' has no return expression registered",
                qualified_name
            ))
        })?;

        // Create parameter substitution map
        use std::collections::HashMap;
        let mut param_map: HashMap<&'src str, &'arena ResolvedExpr<'src, 'arena>> = HashMap::new();

        // Bind "self" to the receiver expression
        param_map.insert("self", receiver_expr);

        // Bind parameters to arguments
        for (param, arg) in method.params.iter().zip(args.iter()) {
            param_map.insert(param.name, *arg);
        }

        // Substitute parameters in the return expression
        // Note: substitute_parameters is a method on ResolvedExpr, so we call it on return_expr
        let inlined_expr = return_expr.substitute_parameters(return_expr, &param_map, ctx)?;

        // Return the inlined expression (caller will handle conversion to Z3)
        Ok(inlined_expr)
    }

    /// Add a constraint that a variable equals a Z3 expression
    ///
    /// Handles both primitive types and struct types (field-wise equality).
    fn add_struct_equality_constraint(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        var_path: &VariablePath<'src>,
        z3_value: &Z3Expr,
    ) -> Result<(), SolverError> {
        // Get the variable
        let var_node = ctx
            .get_variable(var_path)
            .ok_or_else(|| SolverError::UndefinedVariable(var_path.to_z3_name()))?;

        // For primitives, create simple equality
        if let Some(z3_var) = var_node.as_primitive() {
            let constraint = match (z3_var, z3_value) {
                (crate::solver::context::Z3Primitive::Int(var), Z3Expr::Int(val)) => var.eq(val),
                (crate::solver::context::Z3Primitive::Real(var), Z3Expr::Real(val)) => var.eq(val),
                (crate::solver::context::Z3Primitive::Bool(var), Z3Expr::Bool(val)) => var.eq(val),
                (crate::solver::context::Z3Primitive::Int(var), Z3Expr::Real(val)) => {
                    var.to_real().eq(val)
                }
                (crate::solver::context::Z3Primitive::Real(var), Z3Expr::Int(val)) => {
                    var.eq(val.to_real())
                }
                _ => {
                    return Err(SolverError::UnsupportedExpression(
                        "Type mismatch in transform constraint".to_string(),
                    ));
                }
            };
            ctx.z3_solver.assert(&constraint);
            return Ok(());
        }

        // For structs, we need field-wise equality
        // But Z3Expr doesn't directly support struct values from inlined expressions
        // The inlined expression should have been a struct literal that was already
        // solved field-by-field, so this case shouldn't occur.
        Err(SolverError::UnsupportedExpression(
            "Transform result must be a primitive or solved struct literal".to_string(),
        ))
    }

    /// Compare two types semantically (ignoring span information)
    ///
    /// This is used for matching transform output types with declared types,
    /// where the same logical type may have been parsed at different source locations.
    fn types_match_semantically(
        type1: &ResolvedType<'src, 'arena>,
        type2: &ResolvedType<'src, 'arena>,
    ) -> bool {
        match (type1, type2) {
            (ResolvedType::I32 { .. }, ResolvedType::I32 { .. }) => true,
            (ResolvedType::F64 { .. }, ResolvedType::F64 { .. }) => true,
            (ResolvedType::Bool { .. }, ResolvedType::Bool { .. }) => true,
            (
                ResolvedType::UserDefined {
                    name: name1,
                    definition: def1,
                    ..
                },
                ResolvedType::UserDefined {
                    name: name2,
                    definition: def2,
                    ..
                },
            ) => {
                // Compare by struct name and definition pointer
                // If they point to the same definition, they're the same type
                name1 == name2 && std::ptr::eq(*def1 as *const _, *def2 as *const _)
            }
            (
                ResolvedType::Reference { inner: inner1, .. },
                ResolvedType::Reference { inner: inner2, .. },
            ) => Self::types_match_semantically(inner1, inner2),
            (
                ResolvedType::Array {
                    element_type: elem1,
                    size: size1,
                    ..
                },
                ResolvedType::Array {
                    element_type: elem2,
                    size: size2,
                    ..
                },
            ) => size1 == size2 && Self::types_match_semantically(elem1, elem2),
            _ => false,
        }
    }
}
