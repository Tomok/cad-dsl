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
                // TODO: Apply transforms when accessing variables in transform context
                // For now, just get the variable's Z3 value
                // Transform implementation will be completed in a future iteration
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
                // Special case: field access on a struct literal (e.g., from inlined function)
                // Instead of treating it as a variable path, extract the field value and solve it
                if let ResolvedExprKind::StructLit { fields, .. } = &receiver.kind {
                    use crate::hir::expr::ResolvedStructLitField;

                    // Find the field in the struct literal
                    for field in fields {
                        if let ResolvedStructLitField::Field { name, value, .. } = field
                            && name == field_name
                        {
                            // Recursively solve the field value
                            return value.solve(ctx);
                        }
                    }

                    return Err(SolverError::UnsupportedExpression(format!(
                        "Field '{}' not found in struct literal",
                        field_name
                    )));
                }

                // Normal case: field access on a variable
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

            ResolvedExprKind::Mod { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Int(l.modulo(&r))),
                    // Modulo is only defined for integers in Z3
                    _ => Err(SolverError::UnsupportedExpression(
                        "Modulo operation only supported for integer types".to_string(),
                    )),
                }
            }

            ResolvedExprKind::Pow { lhs, rhs } => {
                let lhs_z3 = lhs.solve(ctx)?;
                let rhs_z3 = rhs.solve(ctx)?;

                // Power operations in Z3 always return Real type
                match (lhs_z3, rhs_z3) {
                    (Z3Expr::Int(l), Z3Expr::Int(r)) => Ok(Z3Expr::Real(l.power(&r))),
                    (Z3Expr::Real(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l.power(&r))),
                    (Z3Expr::Int(l), Z3Expr::Real(r)) => Ok(Z3Expr::Real(l.to_real().power(&r))),
                    (Z3Expr::Real(l), Z3Expr::Int(r)) => Ok(Z3Expr::Real(l.power(r.to_real()))),
                    _ => Err(SolverError::UnsupportedExpression(
                        "Invalid types for power operation".to_string(),
                    )),
                }
            }

            // Binary operations - Comparisons
            ResolvedExprKind::Eq { lhs, rhs } => {
                // First, check if RHS is a function call that might return a struct
                // If so, inline it (parameter substitution only) to get the actual struct literal
                let rhs_resolved = if let ResolvedExprKind::FunctionCall {
                    name,
                    function,
                    args,
                } = &rhs.kind
                {
                    use std::collections::HashMap;

                    // Get the return expression from the context
                    let return_expr = ctx.get_function_return(name).ok_or_else(|| {
                        SolverError::UnsupportedExpression(format!(
                            "Function '{}' has no return expression registered",
                            name
                        ))
                    })?;

                    // Create parameter substitution map
                    let mut param_map: HashMap<&'src str, &'arena ResolvedExpr<'src, 'arena>> =
                        HashMap::new();
                    for (param, arg) in function.params.iter().zip(args.iter()) {
                        param_map.insert(param.name, *arg);
                    }

                    // Substitute parameters in the return expression (but don't solve yet)
                    self.substitute_parameters(return_expr, &param_map, ctx)?
                } else {
                    rhs
                };

                // Apply transforms to the RHS if we're in a transform context
                // This handles cases like: cubes[1] == new_cube() inside a transform with-block
                let rhs_transformed = if let ResolvedExprKind::StructLit { .. } = &rhs_resolved.kind
                {
                    self.apply_transforms_to_struct_literal(ctx, rhs_resolved)?
                } else {
                    rhs_resolved
                };

                // Special handling for struct equality: if RHS is a struct literal,
                // we need to create field-wise equality constraints instead of
                // trying to solve the struct as a primitive
                if let ResolvedExprKind::StructLit { fields, .. } = &rhs_transformed.kind {
                    use crate::hir::expr::ResolvedStructLitField;

                    // Build the LHS path (must be a variable path)
                    let lhs_path = self.build_variable_path(lhs, ctx)?;

                    // Create a boolean constraint combining all field equalities
                    let mut field_constraints: Vec<z3::ast::Bool> = Vec::new();

                    for field in fields {
                        match field {
                            ResolvedStructLitField::Field { name, value, .. } => {
                                let field_path = lhs_path.with_field(name);

                                // Check if this field value is itself a struct literal (nested struct)
                                if let ResolvedExprKind::StructLit {
                                    fields: nested_fields,
                                    ..
                                } = &value.kind
                                {
                                    // Recursively handle nested struct literal
                                    for nested_field in nested_fields {
                                        if let ResolvedStructLitField::Field {
                                            name: nested_name,
                                            value: nested_value,
                                            ..
                                        } = nested_field
                                        {
                                            let nested_path = field_path.with_field(nested_name);
                                            let nested_z3_value = nested_value.solve(ctx)?;
                                            let nested_z3_var =
                                                self.get_variable_z3(ctx, &nested_path)?;

                                            let nested_eq = match (nested_z3_var, nested_z3_value) {
                                                (Z3Expr::Int(v), Z3Expr::Int(val)) => v.eq(&val),
                                                (Z3Expr::Real(v), Z3Expr::Real(val)) => v.eq(&val),
                                                (Z3Expr::Int(v), Z3Expr::Real(val)) => {
                                                    v.to_real().eq(&val)
                                                }
                                                (Z3Expr::Real(v), Z3Expr::Int(val)) => {
                                                    v.eq(val.to_real())
                                                }
                                                (Z3Expr::Bool(v), Z3Expr::Bool(val)) => v.eq(&val),
                                                _ => {
                                                    return Err(SolverError::UnsupportedExpression(
                                                        format!(
                                                            "Type mismatch in nested struct field equality for field '{}.{}'",
                                                            name, nested_name
                                                        ),
                                                    ));
                                                }
                                            };
                                            field_constraints.push(nested_eq);
                                        }
                                    }
                                } else {
                                    // Regular primitive field
                                    let field_z3_value = value.solve(ctx)?;
                                    let field_z3_var = self.get_variable_z3(ctx, &field_path)?;

                                    let field_eq = match (field_z3_var, field_z3_value) {
                                        (Z3Expr::Int(v), Z3Expr::Int(val)) => v.eq(&val),
                                        (Z3Expr::Real(v), Z3Expr::Real(val)) => v.eq(&val),
                                        (Z3Expr::Int(v), Z3Expr::Real(val)) => v.to_real().eq(&val),
                                        (Z3Expr::Real(v), Z3Expr::Int(val)) => v.eq(val.to_real()),
                                        (Z3Expr::Bool(v), Z3Expr::Bool(val)) => v.eq(&val),
                                        _ => {
                                            return Err(SolverError::UnsupportedExpression(
                                                format!(
                                                    "Type mismatch in struct field equality for field '{}'",
                                                    name
                                                ),
                                            ));
                                        }
                                    };
                                    field_constraints.push(field_eq);
                                }
                            }
                            ResolvedStructLitField::ComputedProperty { .. } => {
                                return Err(SolverError::UnsupportedExpression(
                                    "Computed properties in struct equality not supported"
                                        .to_string(),
                                ));
                            }
                        }
                    }

                    // Combine all field constraints with AND
                    if field_constraints.is_empty() {
                        // Empty struct - always true
                        Ok(Z3Expr::Bool(z3::ast::Bool::from_bool(true)))
                    } else {
                        let combined =
                            z3::ast::Bool::and(&field_constraints.iter().collect::<Vec<_>>());
                        Ok(Z3Expr::Bool(combined))
                    }
                } else {
                    // Normal primitive equality
                    let lhs_z3 = lhs.solve(ctx)?;
                    let rhs_z3 = rhs_resolved.solve(ctx)?;

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

            // Reference operations
            // In the constraint solver, references are transparent - we simply solve
            // the inner expression. The reference semantics are handled at the type
            // system level, but for Z3 constraint generation we just pass through.
            ResolvedExprKind::Ref { inner } => inner.solve(ctx),

            // Dereference operations
            // In the constraint solver, dereference is also transparent - we simply
            // solve the inner expression. The reference/dereference semantics are
            // handled at the type system level.
            ResolvedExprKind::Deref { inner } => inner.solve(ctx),

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

            // Function and Method Calls
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

            // Container field access (dot-prefix variables in with-statements)
            ResolvedExprKind::ContainerFieldAccess {
                resolved_path,
                with_context,
                ..
            } => {
                // Build the full path: container.container_field.resolved_path
                // Extract container variable name from with_context.context_expr
                let container_name = match &with_context.context_expr.kind {
                    ResolvedExprKind::Var { name, .. } => name,
                    _ => {
                        return Err(SolverError::ContextError(
                            "Container field access requires variable context".to_string(),
                        ));
                    }
                };

                // Get container field name
                let container_field_name = with_context
                    .container_field
                    .ok_or_else(|| {
                        SolverError::ContextError(
                            "Container field access without container field".to_string(),
                        )
                    })?
                    .name;

                // Build the full path: container.container_field.resolved_path
                let mut full_path = VariablePath::from_name(container_name);
                full_path = full_path.with_field(container_field_name);
                for field in resolved_path {
                    full_path = full_path.with_field(field);
                }

                // Get the variable
                let var_node = ctx
                    .get_variable(&full_path)
                    .ok_or_else(|| SolverError::UndefinedVariable(format!("{}", full_path)))?;

                let z3_var = var_node
                    .as_primitive()
                    .ok_or(SolverError::NotAPrimitiveType)?;

                Ok(match z3_var {
                    Z3Primitive::Int(z3_int) => Z3Expr::Int(z3_int.clone()),
                    Z3Primitive::Real(z3_real) => Z3Expr::Real(z3_real.clone()),
                    Z3Primitive::Bool(z3_bool) => Z3Expr::Bool(z3_bool.clone()),
                })
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

            _ => Err(SolverError::UnsupportedExpression(format!(
                "Cannot build variable path from this expression: {:?}",
                base.kind
            ))),
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

            ResolvedExprKind::Mod { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr(lhs, ctx)?;
                let rhs_val = self.evaluate_const_expr(rhs, ctx)?;
                if rhs_val == 0 {
                    return Err(SolverError::UnsupportedExpression(
                        "Modulo by zero".to_string(),
                    ));
                }
                Ok(lhs_val % rhs_val)
            }

            ResolvedExprKind::Pow { lhs, rhs } => {
                let lhs_val = self.evaluate_const_expr(lhs, ctx)?;
                let rhs_val = self.evaluate_const_expr(rhs, ctx)?;
                if rhs_val < 0 {
                    return Err(SolverError::UnsupportedExpression(
                        "Negative exponents not supported in constant evaluation".to_string(),
                    ));
                }
                // Use checked_pow to prevent overflow
                lhs_val.checked_pow(rhs_val as u32).ok_or_else(|| {
                    SolverError::UnsupportedExpression("Power operation overflow".to_string())
                })
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
    pub(crate) fn substitute_parameters(
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

            ResolvedExprKind::Deref { inner } => {
                let sub_inner = self.substitute_parameters(inner, param_map, ctx)?;
                ResolvedExprKind::Deref { inner: sub_inner }
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

            // Struct literals - substitute in field value expressions
            ResolvedExprKind::StructLit { name, fields } => {
                use crate::hir::expr::ResolvedStructLitField;

                let sub_fields = fields
                    .iter()
                    .map(|field| match field {
                        ResolvedStructLitField::Field {
                            name,
                            value,
                            field_def,
                            span,
                        } => {
                            let sub_value = self.substitute_parameters(value, param_map, ctx)?;
                            Ok(ResolvedStructLitField::Field {
                                name,
                                value: sub_value,
                                field_def,
                                span: *span,
                            })
                        }
                        ResolvedStructLitField::ComputedProperty {
                            name,
                            value,
                            method_def,
                            span,
                        } => {
                            let sub_value = self.substitute_parameters(value, param_map, ctx)?;
                            Ok(ResolvedStructLitField::ComputedProperty {
                                name,
                                value: sub_value,
                                method_def,
                                span: *span,
                            })
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                ResolvedExprKind::StructLit {
                    name,
                    fields: sub_fields,
                }
            }

            // Array literals, ranges, closures - not needed for basic function calls
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

    /// Get Z3 variable from a variable path
    ///
    /// Helper method to retrieve the Z3 expression for a variable at the given path.
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

    /// Apply transforms to a struct literal expression
    ///
    /// Recursively applies the active transform stack to all fields in a struct literal
    /// that match transform input types. This is used when assigning struct values
    /// inside transform with-blocks.
    ///
    /// # Parameters
    /// - `ctx`: The solver context (contains transform stack)
    /// - `struct_lit_expr`: The struct literal expression to transform
    ///
    /// # Returns
    /// The transformed struct literal expression, or the original if no transforms apply
    fn apply_transforms_to_struct_literal(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        struct_lit_expr: &'arena ResolvedExpr<'src, 'arena>,
    ) -> Result<&'arena ResolvedExpr<'src, 'arena>, SolverError> {
        use crate::hir::expr::ResolvedStructLitField;
        use crate::solver::context::WithContextInfo;

        // Get the current transform context
        let (transforms, context_expr) = match ctx.current_with_context() {
            Some(WithContextInfo::Transform {
                transforms,
                context_expr,
                ..
            }) => (transforms.clone(), *context_expr),
            Some(WithContextInfo::Container {
                transforms,
                context_expr,
                ..
            }) => {
                if transforms.is_empty() {
                    return Ok(struct_lit_expr); // No transforms to apply
                }
                (transforms.clone(), *context_expr)
            }
            None => return Ok(struct_lit_expr), // Not in transform context
        };

        if transforms.is_empty() {
            return Ok(struct_lit_expr);
        }

        // Only process struct literals
        let ResolvedExprKind::StructLit { name, fields } = &struct_lit_expr.kind else {
            return Ok(struct_lit_expr);
        };

        // First, check if there are transforms that apply to this struct type itself
        // (e.g., __transform__(p: &Point3D) -> Point3D)
        let struct_type = struct_lit_expr.ty;

        // Apply ALL matching transforms in sequence (for nested with-blocks)
        // The transforms are ordered from outermost to innermost in the stack,
        // but we want to apply them from innermost to outermost (reverse order).
        let matching_transforms: Vec<_> = transforms
            .iter()
            .filter(|t| {
                use crate::hir::TransformMethodKind;
                // Only use standard transforms for struct literal field transforms
                matches!(t.kind, TransformMethodKind::Standard)
                    && self.types_match_semantically(t.output_type, struct_type)
            })
            .collect();

        if !matching_transforms.is_empty() {
            // Apply transforms in sequence
            // NOTE: The transforms list is ordered [outermost, ..., innermost]
            // We want to apply innermost first, so iterate in reverse
            let mut result = struct_lit_expr;
            for transform_method in matching_transforms.iter().rev() {
                result =
                    self.inline_transform_method(ctx, context_expr, transform_method, &[result])?;
            }
            return Ok(result);
        }

        // TODO: Nested transform contexts are not fully supported yet.
        // The transform stack from the SolverContext only contains the transforms
        // from the current with-block, not the accumulated transforms from outer blocks.
        // This needs to be fixed in the WithContext management to properly accumulate
        // transforms as we enter nested with-blocks.
        // Current behavior: Only applies the innermost transform
        // Expected behavior: Apply all transforms in the stack (outermost to innermost)

        // Transform each field recursively (for nested structs)
        let transformed_fields: Vec<ResolvedStructLitField<'src, 'arena>> = fields
            .iter()
            .map(|field| match field {
                ResolvedStructLitField::Field {
                    name: field_name,
                    value,
                    field_def,
                    span: field_span,
                } => {
                    // Get the type of this field
                    let field_type = &field_def.field_type;

                    // Check if there's a transform for this field type
                    match self.select_transform_method(&transforms, field_type)? {
                        Some(transform_method) => {
                            // Apply the transform to this field value
                            let transformed_value =
                                if let ResolvedExprKind::StructLit { .. } = &value.kind {
                                    // Recursively transform nested struct literals
                                    self.apply_transforms_to_struct_literal(ctx, value)?
                                } else {
                                    value
                                };

                            // Inline the transform: context.__transform__(field_value)
                            let inlined = self.inline_transform_method(
                                ctx,
                                context_expr,
                                transform_method,
                                &[transformed_value],
                            )?;

                            Ok(ResolvedStructLitField::Field {
                                name: field_name,
                                value: inlined,
                                field_def,
                                span: *field_span,
                            })
                        }
                        None => {
                            // No transform for this field type, keep as is
                            // But still recursively check nested struct literals
                            if let ResolvedExprKind::StructLit { .. } = &value.kind {
                                let transformed_value =
                                    self.apply_transforms_to_struct_literal(ctx, value)?;
                                Ok(ResolvedStructLitField::Field {
                                    name: field_name,
                                    value: transformed_value,
                                    field_def,
                                    span: *field_span,
                                })
                            } else {
                                Ok(field.clone())
                            }
                        }
                    }
                }
                ResolvedStructLitField::ComputedProperty { .. } => {
                    Err(SolverError::UnsupportedExpression(
                        "Computed properties in struct literals not supported".to_string(),
                    ))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Create a new struct literal with transformed fields
        let transformed_lit = ctx.arena.alloc(ResolvedExpr {
            kind: ResolvedExprKind::StructLit {
                name,
                fields: transformed_fields,
            },
            ty: struct_lit_expr.ty,
            span: struct_lit_expr.span,
        });

        Ok(transformed_lit)
    }

    /// Select appropriate transform for a given type
    ///
    /// Returns the matching transform method, or None if no transform matches
    fn select_transform_method<'t>(
        &self,
        transforms: &'t [crate::hir::TransformMethod<'src, 'arena>],
        target_type: &'arena crate::hir::types::ResolvedType<'src, 'arena>,
    ) -> Result<Option<&'t crate::hir::TransformMethod<'src, 'arena>>, SolverError> {
        use crate::hir::TransformMethodKind;

        // Filter transforms by output type match
        let matching: Vec<_> = transforms
            .iter()
            .filter(|t| self.types_match_semantically(t.output_type, target_type))
            .collect();

        if matching.is_empty() {
            return Ok(None);
        }

        // For struct literal field transforms, we only use standard transforms
        let standard_methods: Vec<_> = matching
            .iter()
            .filter(|t| matches!(t.kind, TransformMethodKind::Standard))
            .copied()
            .collect();

        if standard_methods.len() > 1 {
            return Err(SolverError::ContextError(format!(
                "Multiple __transform__ methods found for type {:?}. \
                 Transform methods must have unique output types.",
                target_type
            )));
        }

        Ok(standard_methods.first().copied())
    }

    /// Inline a transform method call
    ///
    /// Similar to function inlining, but specifically for __transform__ methods
    fn inline_transform_method(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
        receiver_expr: &'arena ResolvedExpr<'src, 'arena>,
        transform: &crate::hir::TransformMethod<'src, 'arena>,
        args: &[&'arena ResolvedExpr<'src, 'arena>],
    ) -> Result<&'arena ResolvedExpr<'src, 'arena>, SolverError> {
        use std::collections::HashMap;

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
        let mut param_map: HashMap<&'src str, &'arena ResolvedExpr<'src, 'arena>> = HashMap::new();

        // Bind "self" to the receiver expression
        param_map.insert("self", receiver_expr);

        // Bind parameters to arguments
        for (param, arg) in method.params.iter().zip(args.iter()) {
            param_map.insert(param.name, *arg);
        }

        // Substitute parameters in the return expression
        let inlined_expr = self.substitute_parameters(return_expr, &param_map, ctx)?;

        Ok(inlined_expr)
    }

    /// Compare two types semantically (ignoring span information)
    fn types_match_semantically(
        &self,
        type1: &crate::hir::types::ResolvedType<'src, 'arena>,
        type2: &crate::hir::types::ResolvedType<'src, 'arena>,
    ) -> bool {
        use crate::hir::types::ResolvedType;

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
                name1 == name2 && std::ptr::eq(*def1 as *const _, *def2 as *const _)
            }
            (
                ResolvedType::Reference { inner: inner1, .. },
                ResolvedType::Reference { inner: inner2, .. },
            ) => self.types_match_semantically(inner1, inner2),
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
            ) => size1 == size2 && self.types_match_semantically(elem1, elem2),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_context_creation() {
        // This is a basic compilation test to ensure context creation works correctly
        // Full functional tests will be in the integration test suite
        let arena = bumpalo::Bump::new();
        let z3_solver = z3::Solver::new();
        let z3_ctx = z3_solver.get_context().clone();
        let _ctx = SolverContext::new(z3_ctx, z3_solver, &arena);

        // Context creation successful
    }
}
