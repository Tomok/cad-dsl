//! Solvable trait implementations for statements
//!
//! This module implements the `Solvable` trait for `ResolvedStmt` nodes,
//! processing HIR statements and adding constraints to the Z3 solver.

use crate::hir::definitions::VarDefinitionKind;
use crate::hir::expr::{ResolvedExpr, ResolvedExprKind, ResolvedStmt, ResolvedStmtKind};
use crate::hir::types::ResolvedType;
use crate::solver::context::SolverContext;
use crate::solver::impls::expr::Z3Expr;
use crate::solver::{Solvable, SolverError, VariablePath};

impl<'src, 'arena> Solvable<'src, 'arena> for ResolvedStmt<'src, 'arena> {
    type Output = ();

    fn solve(&self, ctx: &mut SolverContext<'src, 'arena>) -> Result<Self::Output, SolverError> {
        match &self.kind {
            // Let statement - declare variable
            ResolvedStmtKind::Let {
                dot_prefix: _,
                name_path: _,
                var_def,
                init,
                ..
            } => {
                // Build the variable path from the identifier structure
                // This correctly handles all identifier variants including TransformedView
                let var_path = ctx.build_var_path_from_identifier(var_def.identifier)?;

                // Check if this is a rune block initialization
                if let Some(init_expr) = init
                    && let ResolvedExprKind::RuneBlock {
                        params,
                        body,
                        return_type,
                    } = &init_expr.kind
                {
                    // Register the rune block for execution after Z3 solving
                    // This compiles the rune code once and caches it
                    ctx.register_rune_block(var_path.clone(), params.clone(), body, return_type)?;

                    // Don't create a Z3 variable - the value will be computed by rune
                    return Ok(());
                }

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
                    ctx.register_alias(var_path, target);
                } else {
                    // Regular variable declaration - use HIR VarDefinitionKind
                    self.solve_variable_by_kind(ctx, &var_path, var_def)?;
                }

                Ok(())
            }

            // Expression statement - add as constraint
            ResolvedStmtKind::Expression { expr, .. } => {
                #[cfg(feature = "solver-debug")]
                eprintln!(
                    "[SOLVER-DEBUG] Adding constraint from expression (span: {:?})",
                    expr.span
                );

                let z3_expr = expr.solve(ctx)?;

                // Expression statements must evaluate to boolean (constraints)
                match z3_expr {
                    Z3Expr::Bool(constraint) => {
                        #[cfg(feature = "solver-debug")]
                        eprintln!("[SOLVER-DEBUG]   Constraint: {}", constraint);
                        ctx.z3_optimizer.assert(&constraint);
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
                        // Range is known - unroll the loop immediately.
                        // Pass the raw pointer of this for-loop's HIR node as a
                        // stable identity for LoopFrame; arena-allocated nodes
                        // never move so the address is valid for the solve lifetime.
                        let for_stmt_ptr = self as *const ResolvedStmt as usize;
                        self.unroll_loop(ctx, for_stmt_ptr, loop_var_def.name(), start, end, body)?;
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

                // Process then branch - wrap constraints with condition.
                // Snapshot aliases so that scoped `let` variables introduced inside
                // the branch are removed afterward and not visible outside.
                let then_snapshot = ctx.alias_map_snapshot();
                for stmt in then_branch {
                    self.process_conditional_stmt(ctx, stmt, &cond_bool, false)?;
                }
                ctx.restore_alias_map(then_snapshot);

                // Process else branch if present
                if let Some(else_stmts) = else_branch {
                    let else_snapshot = ctx.alias_map_snapshot();
                    for stmt in else_stmts {
                        self.process_conditional_stmt(ctx, stmt, &cond_bool, true)?;
                    }
                    ctx.restore_alias_map(else_snapshot);
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

            // Assignment statement - add as equality constraint (var == value)
            ResolvedStmtKind::Assignment { var_def, value, .. } => {
                let qualified_name = var_def.identifier.to_qualified_name();
                let name_ref: &'static str = Box::leak(qualified_name.into_boxed_str());
                let path = VariablePath::from_name(name_ref);
                let z3_var = self.get_variable_z3(ctx, &path)?;
                let z3_value = value.solve(ctx)?;

                let constraint = match (z3_var, z3_value) {
                    (Z3Expr::Int(var), Z3Expr::Int(val)) => var.eq(&val),
                    (Z3Expr::Real(var), Z3Expr::Real(val)) => var.eq(&val),
                    (Z3Expr::Bool(var), Z3Expr::Bool(val)) => var.eq(&val),
                    (Z3Expr::Int(var), Z3Expr::Real(val)) => var.to_real().eq(&val),
                    (Z3Expr::Real(var), Z3Expr::Int(val)) => var.eq(val.to_real()),
                    _ => {
                        return Err(SolverError::UnsupportedExpression(
                            "Type mismatch in assignment".to_string(),
                        ));
                    }
                };

                ctx.z3_optimizer.assert(&constraint);
                Ok(())
            }

            // StructDef and FunctionDef - skip these, they're definitions not executable statements
            // They've already been processed during semantic analysis
            ResolvedStmtKind::StructDef { .. } | ResolvedStmtKind::FunctionDef { .. } => {
                // Skip definitions - they don't contribute constraints
                Ok(())
            }

            // Optimize block - register minimize/maximize objectives
            ResolvedStmtKind::Optimize { directives, .. } => {
                use crate::hir::expr::ResolvedOptimizeDirectiveKind;

                for directive in directives {
                    let z3_expr = directive.expr.solve(ctx)?;

                    match directive.kind {
                        ResolvedOptimizeDirectiveKind::Minimize => match &z3_expr {
                            Z3Expr::Int(z3_int) => ctx.z3_optimizer.minimize(z3_int),
                            Z3Expr::Real(z3_real) => ctx.z3_optimizer.minimize(z3_real),
                            Z3Expr::Bool(_) => {
                                return Err(SolverError::UnsupportedStatement(
                                    "Cannot minimize a boolean expression".to_string(),
                                ));
                            }
                        },
                        ResolvedOptimizeDirectiveKind::Maximize => match &z3_expr {
                            Z3Expr::Int(z3_int) => ctx.z3_optimizer.maximize(z3_int),
                            Z3Expr::Real(z3_real) => ctx.z3_optimizer.maximize(z3_real),
                            Z3Expr::Bool(_) => {
                                return Err(SolverError::UnsupportedStatement(
                                    "Cannot maximize a boolean expression".to_string(),
                                ));
                            }
                        },
                    }
                }

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
    ///
    /// `for_stmt_ptr` is the raw address of the HIR `ResolvedStmt` node for
    /// this for-loop. It is used as the stable identity component of the
    /// `LoopFrame` that is pushed onto `ctx.loop_context_stack` for each
    /// iteration, ensuring that scoped `let` variables declared in the body
    /// receive a context-aware cache key (see `get_or_create_scoped_var_path`).
    fn unroll_loop(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        for_stmt_ptr: usize,
        loop_var_name: &'src str,
        start: i64,
        end: i64,
        body: &[&'arena ResolvedStmt<'src, 'arena>],
    ) -> Result<(), SolverError> {
        use crate::solver::context::LoopFrame;

        for i in start..end {
            // Push this iteration's frame before processing the body.
            // Any scoped `let` declarations inside the body will snapshot
            // the stack (including this frame) to form their `ScopedVarKey`.
            ctx.push_loop_frame(LoopFrame {
                for_stmt_ptr,
                iteration_value: i,
            });

            // Process body statements inside a closure so the frame is
            // always popped regardless of whether an error occurs.
            let body_result: Result<(), SolverError> = (|| {
                for stmt in body {
                    self.process_loop_body_stmt(ctx, stmt, loop_var_name, i)?;
                }
                Ok(())
            })();

            ctx.pop_loop_frame();
            body_result?;
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
                        ctx.z3_optimizer.assert(&constraint);
                        Ok(())
                    }
                    _ => Err(SolverError::UnsupportedStatement(
                        "Loop body expression must evaluate to boolean".to_string(),
                    )),
                }
            }
            ResolvedStmtKind::Let { var_def, .. } => {
                let base_path = ctx.build_var_path_from_identifier(var_def.identifier)?;

                // Obtain a stable, context-aware path for this scoped variable.
                //
                // `get_or_create_scoped_var_path` keys on the raw address of this
                // HIR `let` node plus a snapshot of the full loop-context stack, so:
                //
                // - The same HIR node under the same loop values (across outer solver
                //   iterations) always returns the same path → `declare_variable_at_path`'s
                //   existence guard then reuses the correct Z3 variable.
                // - The same HIR node under different outer-loop values (e.g. i=0 vs i=1
                //   in a nested loop) returns different paths → no variable aliasing.
                // - Two distinct HIR nodes with the same variable name (two different loops
                //   both declaring `let x`) return different paths → no collision.
                let let_stmt_ptr = stmt as *const ResolvedStmt as usize;
                let var_path =
                    ctx.get_or_create_scoped_var_path(let_stmt_ptr, &base_path.to_z3_name());

                let var_type = var_def.var_type.as_ref().ok_or_else(|| {
                    SolverError::ContextError("Variable type not resolved".to_string())
                })?;
                ctx.declare_variable_at_path(&var_path, var_type)?;

                // If there's an initializer, add equality constraint with loop var substituted
                if let VarDefinitionKind::Initialized { init } = &var_def.definition_kind {
                    let z3_value =
                        self.solve_expr_with_substitution(ctx, init, loop_var_name, loop_value)?;
                    let z3_var = self.get_variable_z3(ctx, &var_path)?;

                    let constraint = match (z3_var, z3_value) {
                        (Z3Expr::Int(var), Z3Expr::Int(val)) => var.eq(&val),
                        (Z3Expr::Real(var), Z3Expr::Real(val)) => var.eq(&val),
                        (Z3Expr::Bool(var), Z3Expr::Bool(val)) => var.eq(&val),
                        (Z3Expr::Int(var), Z3Expr::Real(val)) => var.to_real().eq(&val),
                        (Z3Expr::Real(var), Z3Expr::Int(val)) => var.eq(val.to_real()),
                        _ => {
                            return Err(SolverError::UnsupportedExpression(
                                "Type mismatch in loop let initialization".to_string(),
                            ));
                        }
                    };
                    ctx.z3_optimizer.assert(&constraint);
                }

                Ok(())
            }
            _ => Err(SolverError::UnsupportedStatement(format!(
                "Statement type not supported in for-loop body: {:?}",
                stmt.kind
            ))),
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
                ctx.z3_optimizer.assert(&implication);
                Ok(())
            }

            // Assignment statement - create conditional constraint
            ResolvedStmtKind::Assignment { var_def, value, .. } => {
                let qualified_name = var_def.identifier.to_qualified_name();
                // Leak the string to get a 'static reference (coercible to 'src)
                // This is intentional - we need the name to persist for the solver context
                let name_ref: &'static str = Box::leak(qualified_name.into_boxed_str());
                let path = VariablePath::from_name(name_ref);
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
                ctx.z3_optimizer.assert(&implication);
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
                ctx.z3_optimizer.assert(&implication);
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
                let nested_then_snapshot = ctx.alias_map_snapshot();
                for inner_stmt in then_branch {
                    self.process_conditional_stmt(ctx, inner_stmt, &combined_then_cond, false)?;
                }
                ctx.restore_alias_map(nested_then_snapshot);

                // Process else branch: outer_condition => (!inner_condition => inner_constraint)
                // Which is equivalent to: (outer_condition AND !inner_condition) => inner_constraint
                if let Some(else_stmts) = else_branch {
                    let combined_else_cond =
                        z3::ast::Bool::and(&[&actual_condition, &inner_cond_bool.not()]);
                    let nested_else_snapshot = ctx.alias_map_snapshot();
                    for inner_stmt in else_stmts {
                        self.process_conditional_stmt(ctx, inner_stmt, &combined_else_cond, false)?;
                    }
                    ctx.restore_alias_map(nested_else_snapshot);
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

            // Let statement in conditional branch - create a uniquely-scoped Z3 variable
            // and register an alias from the original name so subsequent references
            // within the same branch resolve correctly.  The alias is removed by the
            // caller after the branch finishes, so the variable is not visible outside.
            ResolvedStmtKind::Let { var_def, .. } => {
                let base_path = ctx.build_var_path_from_identifier(var_def.identifier)?;

                // Unique scoped name (same counter used for for-loop bodies)
                let unique_id = ctx.scoped_let_counter;
                ctx.scoped_let_counter += 1;
                let scoped_name = format!("{}_{}", base_path.to_z3_name(), unique_id);
                let scoped_name_ref: &'static str = Box::leak(scoped_name.into_boxed_str());
                let scoped_path = VariablePath::from_name(scoped_name_ref);

                let var_type = var_def.var_type.as_ref().ok_or_else(|| {
                    SolverError::ContextError("Variable type not resolved".to_string())
                })?;
                ctx.declare_variable_at_path(&scoped_path, var_type)?;

                // Alias original name -> scoped name so references inside this branch work
                ctx.register_alias(base_path, scoped_path.clone());

                // If there's an initializer, add an unconditional equality.
                // The scoped variable is unique to this branch so there is no
                // conflict; no implication wrapper is needed.
                if let VarDefinitionKind::Initialized { init } = &var_def.definition_kind {
                    let z3_value = init.solve(ctx)?;
                    let z3_var = self.get_variable_z3(ctx, &scoped_path)?;

                    let equality = match (z3_var, z3_value) {
                        (Z3Expr::Int(var), Z3Expr::Int(val)) => var.eq(&val),
                        (Z3Expr::Real(var), Z3Expr::Real(val)) => var.eq(&val),
                        (Z3Expr::Bool(var), Z3Expr::Bool(val)) => var.eq(&val),
                        (Z3Expr::Int(var), Z3Expr::Real(val)) => var.to_real().eq(&val),
                        (Z3Expr::Real(var), Z3Expr::Int(val)) => var.eq(val.to_real()),
                        _ => {
                            return Err(SolverError::UnsupportedExpression(
                                "Type mismatch in conditional let initialization".to_string(),
                            ));
                        }
                    };

                    ctx.z3_optimizer.assert(&equality);
                }

                Ok(())
            }

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

                        ctx.z3_optimizer.assert(&constraint);
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

    /// Handle a variable declaration based on its VarDefinitionKind
    ///
    /// This function processes variables according to how they are defined in the HIR:
    /// - Uninitialized: Declares a free variable for the solver
    /// - Initialized: Declares variable and adds equality constraint
    /// - TransformedView: Handles container+view variable pair with transform constraint
    fn solve_variable_by_kind(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        var_path: &VariablePath<'src>,
        var_def: &'arena crate::hir::definitions::VarDefinition<'src, 'arena>,
    ) -> Result<(), SolverError> {
        #[cfg(feature = "solver-debug")]
        eprintln!(
            "[SOLVER-DEBUG] solve_variable_by_kind: path={}, kind={:?}",
            var_path,
            match &var_def.definition_kind {
                VarDefinitionKind::Uninitialized => "Uninitialized",
                VarDefinitionKind::Initialized { .. } => "Initialized",
                VarDefinitionKind::TransformedView { .. } => "TransformedView",
            }
        );

        match &var_def.definition_kind {
            VarDefinitionKind::Uninitialized => {
                // Free variable - just declare it
                let var_type = var_def.var_type.as_ref().ok_or_else(|| {
                    SolverError::ContextError("Variable type not resolved".to_string())
                })?;
                #[cfg(feature = "solver-debug")]
                eprintln!(
                    "[SOLVER-DEBUG]   Declaring uninitialized variable: {} (type: {:?})",
                    var_path, var_type
                );
                ctx.declare_variable_at_path(var_path, var_type)?;
                Ok(())
            }

            VarDefinitionKind::Initialized { init } => {
                // Variable with initialization expression
                let var_type = var_def.var_type.as_ref().ok_or_else(|| {
                    SolverError::ContextError("Variable type not resolved".to_string())
                })?;
                ctx.declare_variable_at_path(var_path, var_type)?;

                // Add constraint: var == init
                // Special handling for struct literals
                if let ResolvedExprKind::StructLit { fields, .. } = &init.kind {
                    self.process_struct_literal_init(ctx, var_path, fields)?;
                } else {
                    let z3_value = init.solve(ctx)?;
                    let z3_var = self.get_variable_z3(ctx, var_path)?;

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

                    ctx.z3_optimizer.assert(&constraint);
                }

                Ok(())
            }

            VarDefinitionKind::TransformedView {
                container_var,
                transform_expr,
                ..
            } => {
                // View variable with transform relationship
                // 1. First, handle the container variable (it should be Uninitialized)
                let container_path = self.build_var_path_for_definition(ctx, container_var)?;
                #[cfg(feature = "solver-debug")]
                eprintln!(
                    "[SOLVER-DEBUG]   TransformedView: view_path={}, container_path={}",
                    var_path, container_path
                );
                self.solve_variable_by_kind(ctx, &container_path, container_var)?;

                // 2. Declare the view variable
                let view_type = var_def.var_type.as_ref().ok_or_else(|| {
                    SolverError::ContextError("View variable type not resolved".to_string())
                })?;
                #[cfg(feature = "solver-debug")]
                eprintln!(
                    "[SOLVER-DEBUG]   Declaring view variable: {} (type: {:?})",
                    var_path, view_type
                );
                ctx.declare_variable_at_path(var_path, view_type)?;

                // 3. Add constraints: view.field == transform_expr.field for each field
                // Transform expressions are typically MethodCalls that return structs.
                // We create field-access expressions for each field and evaluate those.

                // Get the struct definition from the view type to know which fields to constrain
                let struct_def = match view_type {
                    ResolvedType::UserDefined { definition, .. } => definition,
                    _ => {
                        return Err(SolverError::UnsupportedExpression(
                            "TransformedView with non-struct view type".to_string(),
                        ));
                    }
                };

                // For each field in the struct, create constraint: view.field == transform_expr.field
                for field_def in &struct_def.fields {
                    // Create field access expression: transform_expr.field_name
                    use crate::hir::expr::ResolvedExprKind;
                    let transform_field_expr = ctx.arena.alloc(ResolvedExpr {
                        span: transform_expr.span,
                        kind: ResolvedExprKind::FieldAccess {
                            receiver: transform_expr,
                            field_name: field_def.name,
                            field: field_def,
                        },
                        ty: ctx.arena.alloc(field_def.field_type),
                    });

                    // Evaluate the field expression to Z3
                    let field_z3 = transform_field_expr.solve(ctx)?;

                    // Get the view variable's field
                    let field_path = var_path.with_field(field_def.name);
                    let view_field_z3 = self.get_variable_z3(ctx, &field_path)?;

                    #[cfg(feature = "solver-debug")]
                    eprintln!(
                        "[SOLVER-DEBUG]   Adding transform constraint: {}.{} == transform_expr.{}",
                        var_path, field_def.name, field_def.name
                    );

                    // Add constraint: view.field == transform_expr.field
                    let constraint = match (view_field_z3, field_z3) {
                        (Z3Expr::Int(var), Z3Expr::Int(val)) => var.eq(&val),
                        (Z3Expr::Real(var), Z3Expr::Real(val)) => var.eq(&val),
                        (Z3Expr::Bool(var), Z3Expr::Bool(val)) => var.eq(&val),
                        (Z3Expr::Int(var), Z3Expr::Real(val)) => var.to_real().eq(&val),
                        (Z3Expr::Real(var), Z3Expr::Int(val)) => var.eq(val.to_real()),
                        _ => {
                            return Err(SolverError::UnsupportedExpression(format!(
                                "Type mismatch in transform field '{}'",
                                field_def.name
                            )));
                        }
                    };

                    ctx.z3_optimizer.assert(&constraint);
                }

                Ok(())
            }
        }
    }

    /// Build a VariablePath from a VarDefinition's identifier
    fn build_var_path_for_definition(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        var_def: &'arena crate::hir::definitions::VarDefinition<'src, 'arena>,
    ) -> Result<VariablePath<'src>, SolverError> {
        ctx.build_var_path_from_identifier(var_def.identifier)
    }
}
