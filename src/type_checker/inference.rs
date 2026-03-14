//! Type inference for expressions in the CAD-DSL type checker
//!
//! This module provides type inference functionality that determines the types
//! of expressions based on their structure and the types of their sub-expressions.

// Allow dead code for now since this module is not yet fully integrated
#![allow(dead_code)] // Planned for future type inference implementation

use super::context::TypeCheckContext;
use super::errors::TypeCheckError;
use crate::hir::expr::{ResolvedExpr, ResolvedExprKind};
use crate::hir::types::ResolvedType;

/// Infer the type of an expression
///
/// This function examines the expression kind and infers its type based on:
/// - The types of literal values
/// - The types of sub-expressions
/// - Type rules for operators and function calls
///
/// If type inference fails (e.g., due to type mismatches or unsupported operations),
/// an error is added to the context and None is returned.
pub fn infer_expr_type<'src, 'arena>(
    ctx: &mut TypeCheckContext<'src, 'arena>,
    expr: &ResolvedExpr<'src, 'arena>,
) -> Option<ResolvedType<'src, 'arena>> {
    match &expr.kind {
        // ====================================================================
        // Literals
        // ====================================================================
        ResolvedExprKind::IntLit { .. } => Some(ResolvedType::I32 { span: expr.span }),

        ResolvedExprKind::FloatLit { .. } => Some(ResolvedType::F64 { span: expr.span }),

        ResolvedExprKind::BoolLit { .. } => Some(ResolvedType::Bool { span: expr.span }),

        // ====================================================================
        // Variables
        // ====================================================================
        ResolvedExprKind::Var { definition, .. } => definition.var_type,

        // ====================================================================
        // Arithmetic Binary Operators (require numeric operands)
        // ====================================================================
        ResolvedExprKind::Add { lhs, rhs }
        | ResolvedExprKind::Sub { lhs, rhs }
        | ResolvedExprKind::Mul { lhs, rhs }
        | ResolvedExprKind::Div { lhs, rhs }
        | ResolvedExprKind::Mod { lhs, rhs }
        | ResolvedExprKind::Pow { lhs, rhs } => {
            let operator = match &expr.kind {
                ResolvedExprKind::Add { .. } => "+",
                ResolvedExprKind::Sub { .. } => "-",
                ResolvedExprKind::Mul { .. } => "*",
                ResolvedExprKind::Div { .. } => "/",
                ResolvedExprKind::Mod { .. } => "%",
                ResolvedExprKind::Pow { .. } => "^",
                _ => unreachable!(),
            };

            let lhs_ty = lhs.ty;
            let rhs_ty = rhs.ty;

            if !is_numeric_type(lhs_ty) {
                ctx.add_error(TypeCheckError::NonNumericOperand {
                    operator: operator.to_string(),
                    operand_type: type_name(lhs_ty),
                    span: lhs.span,
                });
                return None;
            }

            if !is_numeric_type(rhs_ty) {
                ctx.add_error(TypeCheckError::NonNumericOperand {
                    operator: operator.to_string(),
                    operand_type: type_name(rhs_ty),
                    span: rhs.span,
                });
                return None;
            }

            promote_numeric_type(lhs_ty, rhs_ty, expr.span)
        }

        // ====================================================================
        // Comparison Operators (require numeric operands, return bool)
        // ====================================================================
        ResolvedExprKind::Lt { lhs, rhs }
        | ResolvedExprKind::Gt { lhs, rhs }
        | ResolvedExprKind::LtEq { lhs, rhs }
        | ResolvedExprKind::GtEq { lhs, rhs } => {
            let operator = match &expr.kind {
                ResolvedExprKind::Lt { .. } => "<",
                ResolvedExprKind::Gt { .. } => ">",
                ResolvedExprKind::LtEq { .. } => "<=",
                ResolvedExprKind::GtEq { .. } => ">=",
                _ => unreachable!(),
            };

            let lhs_ty = lhs.ty;
            let rhs_ty = rhs.ty;

            if !is_numeric_type(lhs_ty) {
                ctx.add_error(TypeCheckError::NonNumericOperand {
                    operator: operator.to_string(),
                    operand_type: type_name(lhs_ty),
                    span: lhs.span,
                });
                return None;
            }

            if !is_numeric_type(rhs_ty) {
                ctx.add_error(TypeCheckError::NonNumericOperand {
                    operator: operator.to_string(),
                    operand_type: type_name(rhs_ty),
                    span: rhs.span,
                });
                return None;
            }

            Some(ResolvedType::Bool { span: expr.span })
        }

        // ====================================================================
        // Equality Operators (require compatible types, return bool)
        // ====================================================================
        ResolvedExprKind::Eq { lhs, rhs } | ResolvedExprKind::NotEq { lhs, rhs } => {
            let operator = match &expr.kind {
                ResolvedExprKind::Eq { .. } => "==",
                ResolvedExprKind::NotEq { .. } => "!=",
                _ => unreachable!(),
            };

            let lhs_ty = lhs.ty;
            let rhs_ty = rhs.ty;

            // Check if types are compatible (same type)
            if !types_equal(lhs_ty, rhs_ty) {
                ctx.add_error(TypeCheckError::IncompatibleTypes {
                    lhs_type: type_name(lhs_ty),
                    rhs_type: type_name(rhs_ty),
                    operation: operator.to_string(),
                    span: expr.span,
                });
                return None;
            }

            Some(ResolvedType::Bool { span: expr.span })
        }

        // ====================================================================
        // Logical Operators (require bool operands, return bool)
        // ====================================================================
        ResolvedExprKind::And { lhs, rhs } | ResolvedExprKind::Or { lhs, rhs } => {
            let lhs_ty = lhs.ty;
            let rhs_ty = rhs.ty;

            if !matches!(lhs_ty, ResolvedType::Bool { .. }) {
                ctx.add_error(TypeCheckError::NonBooleanCondition {
                    found_type: type_name(lhs_ty),
                    span: lhs.span,
                });
                return None;
            }

            if !matches!(rhs_ty, ResolvedType::Bool { .. }) {
                ctx.add_error(TypeCheckError::NonBooleanCondition {
                    found_type: type_name(rhs_ty),
                    span: rhs.span,
                });
                return None;
            }

            Some(ResolvedType::Bool { span: expr.span })
        }

        // ====================================================================
        // Unary Operators
        // ====================================================================
        ResolvedExprKind::Neg { inner } => {
            let inner_ty = inner.ty;

            if !is_numeric_type(inner_ty) {
                ctx.add_error(TypeCheckError::NonNumericOperand {
                    operator: "-".to_string(),
                    operand_type: type_name(inner_ty),
                    span: inner.span,
                });
                return None;
            }

            Some(*inner_ty)
        }

        ResolvedExprKind::Ref { inner } => {
            let inner_ty = inner.ty;
            Some(ResolvedType::Reference {
                inner: inner_ty,
                span: expr.span,
            })
        }

        ResolvedExprKind::Deref { inner } => {
            let inner_ty = inner.ty;

            // Dereference requires a reference type
            match inner_ty {
                ResolvedType::Reference {
                    inner: deref_ty, ..
                } => Some(**deref_ty),
                _ => {
                    ctx.add_error(TypeCheckError::TypeMismatch {
                        expected: "reference type (&T)".to_string(),
                        found: type_name(inner_ty),
                        span: inner.span,
                    });
                    None
                }
            }
        }

        // ====================================================================
        // Function Calls
        // ====================================================================
        ResolvedExprKind::FunctionCall { function, .. } => Some(function.return_type),

        ResolvedExprKind::MethodCall { method, .. } => Some(method.return_type),

        // ====================================================================
        // Field Access
        // ====================================================================
        ResolvedExprKind::FieldAccess { field, .. } => Some(field.field_type),

        // ====================================================================
        // Parenthesized Expressions
        // ====================================================================
        ResolvedExprKind::Paren { inner } => Some(*inner.ty),

        // ====================================================================
        // Struct Literals
        // ====================================================================
        ResolvedExprKind::StructLit { name, fields } => {
            // For struct literals, we need to get the struct definition
            // Since we don't have access to it in the current context,
            // we try to extract it from one of the field definitions
            if let Some(first_field) = fields.first() {
                match first_field {
                    crate::hir::expr::ResolvedStructLitField::Field { .. } => {
                        // We need to get the parent struct definition from the field
                        // This is a limitation of the current HIR design
                        // For now, we return an error
                        ctx.add_error(TypeCheckError::CannotInferType {
                            expr_kind: format!("struct literal '{}'", name),
                            span: expr.span,
                        });
                        None
                    }
                    crate::hir::expr::ResolvedStructLitField::ComputedProperty {
                        method_def,
                        ..
                    } => {
                        // Try to get the struct definition from the method's parent
                        if let Some(parent_struct) = method_def.parent_struct {
                            Some(ResolvedType::UserDefined {
                                name,
                                definition: parent_struct,
                                span: expr.span,
                            })
                        } else {
                            ctx.add_error(TypeCheckError::CannotInferType {
                                expr_kind: format!("struct literal '{}'", name),
                                span: expr.span,
                            });
                            None
                        }
                    }
                }
            } else {
                // Empty struct literal - cannot infer without struct definition
                ctx.add_error(TypeCheckError::CannotInferType {
                    expr_kind: format!("empty struct literal '{}'", name),
                    span: expr.span,
                });
                None
            }
        }

        // ====================================================================
        // Array Literals
        // ====================================================================
        ResolvedExprKind::ArrayLit { elements } => {
            // Arrays are not yet fully supported in the type system
            // The ResolvedType enum doesn't have an Array variant
            if elements.is_empty() {
                ctx.add_error(TypeCheckError::CannotInferType {
                    expr_kind: "empty array literal".to_string(),
                    span: expr.span,
                });
                return None;
            }

            // For now, return an error since arrays aren't in the type system
            ctx.add_error(TypeCheckError::CannotInferType {
                expr_kind: "array literal (arrays not yet supported in type system)".to_string(),
                span: expr.span,
            });
            None
        }

        // ====================================================================
        // Index Expressions
        // ====================================================================
        ResolvedExprKind::Index { array, index } => {
            // Check that array is actually an array type
            match array.ty {
                ResolvedType::Array { element_type, .. } => {
                    // Check that index is i32
                    let index_ty = index.ty;
                    if !matches!(index_ty, ResolvedType::I32 { .. }) {
                        ctx.add_error(TypeCheckError::TypeMismatch {
                            expected: "i32".to_string(),
                            found: type_name(index_ty),
                            span: index.span,
                        });
                        return None;
                    }
                    // Return the element type
                    Some(**element_type)
                }
                _ => {
                    // Not an array type
                    ctx.add_error(TypeCheckError::CannotIndex {
                        array_type: type_name(array.ty),
                        span: expr.span,
                    });
                    None
                }
            }
        }

        // ====================================================================
        // Range Expressions
        // ====================================================================
        ResolvedExprKind::Range { start, end } => {
            // Check that both start and end are i32
            let start_ty = start.ty;
            let end_ty = end.ty;

            if !matches!(start_ty, ResolvedType::I32 { .. }) {
                ctx.add_error(TypeCheckError::TypeMismatch {
                    expected: "i32".to_string(),
                    found: type_name(start_ty),
                    span: start.span,
                });
                return None;
            }

            if !matches!(end_ty, ResolvedType::I32 { .. }) {
                ctx.add_error(TypeCheckError::TypeMismatch {
                    expected: "i32".to_string(),
                    found: type_name(end_ty),
                    span: end.span,
                });
                return None;
            }

            // Ranges are not yet in the type system, so return i32 as the result
            // (or we could add an error)
            Some(ResolvedType::I32 { span: expr.span })
        }

        // ====================================================================
        // Closures
        // ====================================================================
        ResolvedExprKind::Closure { .. } => {
            // Closures/lambdas are not yet supported in the type system
            ctx.add_error(TypeCheckError::CannotInferType {
                expr_kind: "closure (closures not yet supported in type system)".to_string(),
                span: expr.span,
            });
            None
        }

        // ====================================================================
        // Container Field Access
        // ====================================================================
        ResolvedExprKind::ContainerFieldAccess { .. } => {
            // Container field access type inference requires more context
            ctx.add_error(TypeCheckError::CannotInferType {
                expr_kind: "container field access".to_string(),
                span: expr.span,
            });
            None
        }

        // ====================================================================
        // Rune Blocks
        // ====================================================================
        ResolvedExprKind::RuneBlock {
            params,
            body,
            return_type: _,
        } => {
            // Phase 3: Rune type checking integration
            // Compile Rune code with parameter types and infer return type
            use super::rune_integration::RuneTypeChecker;

            let rune_checker = match RuneTypeChecker::new() {
                Ok(checker) => checker,
                Err(e) => {
                    ctx.add_error(TypeCheckError::Rune {
                        message: format!("Failed to create Rune type checker: {}", e),
                        span: expr.span,
                    });
                    return None;
                }
            };

            let global_fns: Vec<String> = ctx.global_rune_fns().to_vec();
            match rune_checker.infer_return_type(body, params, &global_fns, expr.span) {
                Ok((inferred_type, diagnostics)) => {
                    // Add any warnings to the context (errors were already handled separately)
                    if !diagnostics.is_empty() {
                        ctx.add_warning(format!(
                            "Rune compilation warnings at line {}, column {}: {:?}",
                            expr.span.start.line, expr.span.start.column, diagnostics
                        ));
                    }
                    Some(inferred_type)
                }
                Err(e) => {
                    ctx.add_error(TypeCheckError::Rune {
                        message: e.to_string(),
                        span: expr.span,
                    });
                    None
                }
            }
        }
    }
}

/// Check if a type is numeric (i32, f64, real, or algebraic)
pub fn is_numeric_type(ty: &ResolvedType) -> bool {
    ty.is_numeric()
}

/// Promote two numeric types to their common type
///
/// Type promotion rules:
/// - i32 + i32 → i32
/// - i32 + f64 → f64
/// - f64 + i32 → f64
/// - f64 + f64 → f64
/// - real and algebraic promote similarly to f64
///
/// Returns None if the types cannot be promoted (e.g., non-numeric types)
pub fn promote_numeric_type<'src, 'arena>(
    lhs: &ResolvedType<'src, 'arena>,
    rhs: &ResolvedType<'src, 'arena>,
    span: crate::lexer::Span,
) -> Option<ResolvedType<'src, 'arena>> {
    use ResolvedType::*;

    match (lhs, rhs) {
        // Same types
        (I32 { .. }, I32 { .. }) => Some(I32 { span }),
        (F64 { .. }, F64 { .. }) => Some(F64 { span }),
        (Real { .. }, Real { .. }) => Some(Real { span }),
        (Algebraic { .. }, Algebraic { .. }) => Some(Algebraic { span }),

        // i32 promotes to f64
        (I32 { .. }, F64 { .. }) | (F64 { .. }, I32 { .. }) => Some(F64 { span }),

        // i32 promotes to real
        (I32 { .. }, Real { .. }) | (Real { .. }, I32 { .. }) => Some(Real { span }),

        // i32 promotes to algebraic
        (I32 { .. }, Algebraic { .. }) | (Algebraic { .. }, I32 { .. }) => Some(Algebraic { span }),

        // f64 promotes to real
        (F64 { .. }, Real { .. }) | (Real { .. }, F64 { .. }) => Some(Real { span }),

        // f64 promotes to algebraic
        (F64 { .. }, Algebraic { .. }) | (Algebraic { .. }, F64 { .. }) => Some(Algebraic { span }),

        // algebraic promotes to real
        (Algebraic { .. }, Real { .. }) | (Real { .. }, Algebraic { .. }) => Some(Real { span }),

        // Non-numeric types cannot be promoted
        _ => None,
    }
}

/// Get a human-readable name for a type
fn type_name(ty: &ResolvedType) -> String {
    match ty {
        ResolvedType::Bool { .. } => "bool".to_string(),
        ResolvedType::I32 { .. } => "i32".to_string(),
        ResolvedType::F64 { .. } => "f64".to_string(),
        ResolvedType::Real { .. } => "real".to_string(),
        ResolvedType::Algebraic { .. } => "algebraic".to_string(),
        ResolvedType::Reference { inner, .. } => format!("&{}", type_name(inner)),
        ResolvedType::UserDefined { name, .. } => name.to_string(),
        ResolvedType::Array {
            element_type, size, ..
        } => format!("[{}; {}]", type_name(element_type), size),
    }
}

/// Check if two types are equal (ignoring spans)
fn types_equal(lhs: &ResolvedType, rhs: &ResolvedType) -> bool {
    use ResolvedType::*;

    match (lhs, rhs) {
        (Bool { .. }, Bool { .. }) => true,
        (I32 { .. }, I32 { .. }) => true,
        (F64 { .. }, F64 { .. }) => true,
        (Real { .. }, Real { .. }) => true,
        (Algebraic { .. }, Algebraic { .. }) => true,
        (
            Reference {
                inner: lhs_inner, ..
            },
            Reference {
                inner: rhs_inner, ..
            },
        ) => types_equal(lhs_inner, rhs_inner),
        (
            Array {
                element_type: lhs_elem,
                size: lhs_size,
                ..
            },
            Array {
                element_type: rhs_elem,
                size: rhs_size,
                ..
            },
        ) => lhs_size == rhs_size && types_equal(lhs_elem, rhs_elem),
        (
            UserDefined {
                name: lhs_name,
                definition: lhs_def,
                ..
            },
            UserDefined {
                name: rhs_name,
                definition: rhs_def,
                ..
            },
        ) => lhs_name == rhs_name && std::ptr::eq(*lhs_def, *rhs_def),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::definitions::{FunctionDefinition, VarDefinition};
    use crate::hir::expr::ResolvedExpr;
    use crate::lexer::{LineColumn, Span};
    use bumpalo::Bump;

    fn make_span(line: usize, column: usize) -> Span {
        Span {
            start: LineColumn { line, column },
            lines: 0,
            end_column: column + 5,
        }
    }

    fn make_i32_type(span: Span) -> ResolvedType<'static, 'static> {
        ResolvedType::I32 { span }
    }

    fn make_f64_type(span: Span) -> ResolvedType<'static, 'static> {
        ResolvedType::F64 { span }
    }

    fn make_bool_type(span: Span) -> ResolvedType<'static, 'static> {
        ResolvedType::Bool { span }
    }

    #[test]
    fn test_is_numeric_type() {
        let span = make_span(1, 1);
        assert!(is_numeric_type(&ResolvedType::I32 { span }));
        assert!(is_numeric_type(&ResolvedType::F64 { span }));
        assert!(is_numeric_type(&ResolvedType::Real { span }));
        assert!(is_numeric_type(&ResolvedType::Algebraic { span }));
        assert!(!is_numeric_type(&ResolvedType::Bool { span }));
    }

    #[test]
    fn test_promote_numeric_type_same_types() {
        let span = make_span(1, 1);
        let i32_ty = ResolvedType::I32 { span };
        let f64_ty = ResolvedType::F64 { span };

        // i32 + i32 → i32
        let result = promote_numeric_type(&i32_ty, &i32_ty, span);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::I32 { .. }));

        // f64 + f64 → f64
        let result = promote_numeric_type(&f64_ty, &f64_ty, span);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::F64 { .. }));
    }

    #[test]
    fn test_promote_numeric_type_mixed() {
        let span = make_span(1, 1);
        let i32_ty = ResolvedType::I32 { span };
        let f64_ty = ResolvedType::F64 { span };

        // i32 + f64 → f64
        let result = promote_numeric_type(&i32_ty, &f64_ty, span);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::F64 { .. }));

        // f64 + i32 → f64
        let result = promote_numeric_type(&f64_ty, &i32_ty, span);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::F64 { .. }));
    }

    #[test]
    fn test_promote_numeric_type_non_numeric() {
        let span = make_span(1, 1);
        let i32_ty = ResolvedType::I32 { span };
        let bool_ty = ResolvedType::Bool { span };

        // i32 + bool → None
        let result = promote_numeric_type(&i32_ty, &bool_ty, span);
        assert!(result.is_none());

        // bool + i32 → None
        let result = promote_numeric_type(&bool_ty, &i32_ty, span);
        assert!(result.is_none());
    }

    #[test]
    fn test_type_name() {
        let span = make_span(1, 1);

        assert_eq!(type_name(&ResolvedType::Bool { span }), "bool");
        assert_eq!(type_name(&ResolvedType::I32 { span }), "i32");
        assert_eq!(type_name(&ResolvedType::F64 { span }), "f64");
        assert_eq!(type_name(&ResolvedType::Real { span }), "real");
        assert_eq!(type_name(&ResolvedType::Algebraic { span }), "algebraic");
    }

    #[test]
    fn test_type_name_reference() {
        let span = make_span(1, 1);
        let i32_ty = ResolvedType::I32 { span };
        let ref_ty = ResolvedType::Reference {
            inner: &i32_ty,
            span,
        };

        assert_eq!(type_name(&ref_ty), "&i32");
    }

    #[test]
    fn test_types_equal_primitives() {
        let span1 = make_span(1, 1);
        let span2 = make_span(2, 2);

        assert!(types_equal(
            &ResolvedType::I32 { span: span1 },
            &ResolvedType::I32 { span: span2 }
        ));
        assert!(types_equal(
            &ResolvedType::F64 { span: span1 },
            &ResolvedType::F64 { span: span2 }
        ));
        assert!(types_equal(
            &ResolvedType::Bool { span: span1 },
            &ResolvedType::Bool { span: span2 }
        ));

        assert!(!types_equal(
            &ResolvedType::I32 { span: span1 },
            &ResolvedType::F64 { span: span2 }
        ));
        assert!(!types_equal(
            &ResolvedType::Bool { span: span1 },
            &ResolvedType::I32 { span: span2 }
        ));
    }

    #[test]
    fn test_infer_int_lit() {
        let arena = Bump::new();
        let source = "42";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));
        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 42 },
            ty: i32_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::I32 { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_float_lit() {
        let arena = Bump::new();
        let source = "3.14";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let f64_ty = arena.alloc(make_f64_type(span));
        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::FloatLit { value: 3.14 },
            ty: f64_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::F64 { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_bool_lit() {
        let arena = Bump::new();
        let source = "true";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let bool_ty = arena.alloc(make_bool_type(span));
        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::BoolLit { value: true },
            ty: bool_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::Bool { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_var_ref() {
        let arena = Bump::new();
        let source = "x";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = make_i32_type(span);
        let identifier = arena.alloc(crate::hir::definitions::VariableIdentifier::Simple("x"));
        let var_def = arena.alloc(VarDefinition::new(
            identifier,
            "x",
            span,
            Some(i32_ty),
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            span,
        ));

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::Var {
                name: "x",
                definition: var_def,
            },
            ty: &i32_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::I32 { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_add_same_types() {
        let arena = Bump::new();
        let source = "1 + 2";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));

        let lhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 1 },
            ty: i32_ty,
        });

        let rhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 2 },
            ty: i32_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::Add { lhs, rhs },
            ty: i32_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::I32 { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_add_type_promotion() {
        let arena = Bump::new();
        let source = "1 + 2.0";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));
        let f64_ty = arena.alloc(make_f64_type(span));

        let lhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 1 },
            ty: i32_ty,
        });

        let rhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::FloatLit { value: 2.0 },
            ty: f64_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::Add { lhs, rhs },
            ty: f64_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::F64 { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_add_non_numeric() {
        let arena = Bump::new();
        let source = "true + false";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let bool_ty = arena.alloc(make_bool_type(span));

        let lhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::BoolLit { value: true },
            ty: bool_ty,
        });

        let rhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::BoolLit { value: false },
            ty: bool_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::Add { lhs, rhs },
            ty: bool_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_none());
        assert!(ctx.has_errors());
    }

    #[test]
    fn test_infer_comparison() {
        let arena = Bump::new();
        let source = "1 < 2";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));
        let bool_ty = arena.alloc(make_bool_type(span));

        let lhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 1 },
            ty: i32_ty,
        });

        let rhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 2 },
            ty: i32_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::Lt { lhs, rhs },
            ty: bool_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::Bool { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_equality() {
        let arena = Bump::new();
        let source = "1 == 1";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));
        let bool_ty = arena.alloc(make_bool_type(span));

        let lhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 1 },
            ty: i32_ty,
        });

        let rhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 1 },
            ty: i32_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::Eq { lhs, rhs },
            ty: bool_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::Bool { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_logical_and() {
        let arena = Bump::new();
        let source = "true && false";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let bool_ty = arena.alloc(make_bool_type(span));

        let lhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::BoolLit { value: true },
            ty: bool_ty,
        });

        let rhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::BoolLit { value: false },
            ty: bool_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::And { lhs, rhs },
            ty: bool_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::Bool { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_logical_and_non_bool() {
        let arena = Bump::new();
        let source = "1 && 2";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));
        let bool_ty = arena.alloc(make_bool_type(span));

        let lhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 1 },
            ty: i32_ty,
        });

        let rhs = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 2 },
            ty: i32_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::And { lhs, rhs },
            ty: bool_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_none());
        assert!(ctx.has_errors());
    }

    #[test]
    fn test_infer_negation() {
        let arena = Bump::new();
        let source = "-42";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));

        let inner = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 42 },
            ty: i32_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::Neg { inner },
            ty: i32_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::I32 { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_reference() {
        let arena = Bump::new();
        let source = "&x";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));
        let ref_ty = arena.alloc(ResolvedType::Reference {
            inner: i32_ty,
            span,
        });

        let identifier = arena.alloc(crate::hir::definitions::VariableIdentifier::Simple("x"));
        let var_def = arena.alloc(VarDefinition::new(
            identifier,
            "x",
            span,
            Some(*i32_ty),
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            0,
            span,
        ));

        let inner = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::Var {
                name: "x",
                definition: var_def,
            },
            ty: i32_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::Ref { inner },
            ty: ref_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::Reference { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_function_call() {
        let arena = Bump::new();
        let source = "foo()";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let f64_ty = arena.alloc(make_f64_type(span));

        let func_def = arena.alloc(FunctionDefinition::new(
            "foo",
            span,
            vec![],
            *f64_ty,
            vec![],
            None,
            span,
        ));

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::FunctionCall {
                name: "foo",
                function: func_def,
                args: vec![],
            },
            ty: f64_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::F64 { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_paren() {
        let arena = Bump::new();
        let source = "(42)";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));

        let inner = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 42 },
            ty: i32_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::Paren { inner },
            ty: i32_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::I32 { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_range() {
        let arena = Bump::new();
        let source = "1..10";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));

        let start = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 1 },
            ty: i32_ty,
        });

        let end = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 10 },
            ty: i32_ty,
        });

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::Range { start, end },
            ty: i32_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_some());
        assert!(matches!(result.unwrap(), ResolvedType::I32 { .. }));
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_infer_empty_array_lit() {
        let arena = Bump::new();
        let source = "[]";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let span = make_span(1, 1);
        let i32_ty = arena.alloc(make_i32_type(span));

        let expr = ResolvedExpr {
            span,
            kind: ResolvedExprKind::ArrayLit { elements: vec![] },
            ty: i32_ty,
        };

        let result = infer_expr_type(&mut ctx, &expr);
        assert!(result.is_none());
        assert!(ctx.has_errors());
    }
}
