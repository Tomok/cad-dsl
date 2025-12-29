//! Type validation for statements in the CAD-DSL type checker
//!
//! This module provides validation functionality that checks types in statements,
//! ensuring type consistency and compatibility across variable declarations,
//! assignments, control flow, and function calls.

// Allow dead code for now since this module is not yet fully integrated
#![allow(dead_code)]

use crate::ast::types::Stmt;
use crate::hir_types::ResolvedType;
use crate::type_checker_context::TypeCheckContext;

/// Validate types in a statement
///
/// This function checks that all type uses in a statement are valid:
/// - Let statements have compatible initializer types
/// - Assignments have compatible value types
/// - Function calls have correct argument types and counts
/// - Conditionals have boolean conditions
/// - Control flow structures are well-typed
///
/// Errors are added to the context and this function returns nothing.
pub fn validate_stmt<'src, 'arena>(_ctx: &mut TypeCheckContext<'src, 'arena>, stmt: &Stmt<'src>) {
    match stmt {
        // ====================================================================
        // Let Statements
        // ====================================================================
        Stmt::Let {
            type_annotation,
            init,
            span,
            ..
        } => {
            // If there's both a type annotation and an initializer, check compatibility
            if let (Some(type_ann), Some(init_expr)) = (type_annotation, init) {
                // Try to infer the type of the initializer
                // Note: We need to convert the AST expression to a ResolvedExpr first
                // For now, we'll skip this validation as it requires resolved expressions
                // This would be implemented once we have full HIR with resolved statements

                // TODO: Once ResolvedStmt exists with ResolvedExpr, implement this:
                // if let Some(init_type) = infer_expr_type(ctx, resolved_init_expr) {
                //     if !types_compatible(&expected_type, &init_type) {
                //         ctx.add_error(TypeCheckError::TypeMismatch {
                //             expected: type_name(&expected_type),
                //             found: type_name(&init_type),
                //             span: init_expr.span(),
                //         });
                //     }
                // }

                // Placeholder: mark as used to avoid warnings
                let _ = (type_ann, init_expr, span);
            }
        }

        // ====================================================================
        // Assignment Statements
        // ====================================================================
        Stmt::Assignment {
            name, value, span, ..
        } => {
            // Validate that the value type is compatible with the variable type
            // This requires looking up the variable definition and checking types

            // TODO: Once ResolvedStmt exists, implement this:
            // 1. Look up variable definition
            // 2. Infer type of value expression
            // 3. Check compatibility

            // Placeholder
            let _ = (name, value, span);
        }

        // ====================================================================
        // Field Assignment Statements
        // ====================================================================
        Stmt::FieldAssignment {
            field_path,
            value,
            span,
            ..
        } => {
            // Similar to Assignment but for field access
            // TODO: Implement once ResolvedStmt exists
            let _ = (field_path, value, span);
        }

        // ====================================================================
        // If Statements
        // ====================================================================
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            // Validate that the condition is a boolean
            // TODO: Once ResolvedStmt exists with ResolvedExpr:
            // if let Some(cond_type) = infer_expr_type(ctx, resolved_condition) {
            //     if !matches!(cond_type, ResolvedType::Bool { .. }) {
            //         ctx.add_error(TypeCheckError::NonBooleanCondition {
            //             found_type: type_name(&cond_type),
            //             span: condition.span(),
            //         });
            //     }
            // }

            // Recursively validate branches
            for stmt in then_branch {
                validate_stmt(_ctx, stmt);
            }

            if let Some(else_stmts) = else_branch {
                for stmt in else_stmts {
                    validate_stmt(_ctx, stmt);
                }
            }

            // Placeholder
            let _ = condition;
        }

        // ====================================================================
        // For Loops
        // ====================================================================
        Stmt::For { iterator, body, .. } => {
            // Validate that iterator is a Range or Array type
            // TODO: Implement once ResolvedStmt exists

            // Recursively validate body
            for stmt in body {
                validate_stmt(_ctx, stmt);
            }

            // Placeholder
            let _ = iterator;
        }

        // ====================================================================
        // Return Statements
        // ====================================================================
        Stmt::Return { value, span } => {
            // Validate that return value type matches function return type
            // This requires function context which we don't have yet
            // TODO: Implement once we have function context in TypeCheckContext
            let _ = (value, span);
        }

        // ====================================================================
        // Expression Statements
        // ====================================================================
        Stmt::Expression { expr, .. } => {
            // Validate the expression
            // TODO: Once ResolvedStmt exists, call infer_expr_type on resolved expression
            let _ = expr;
        }

        // ====================================================================
        // Block Statements
        // ====================================================================
        Stmt::Block { statements, .. } => {
            // Recursively validate all statements in the block
            for stmt in statements {
                validate_stmt(_ctx, stmt);
            }
        }

        // ====================================================================
        // With Statements
        // ====================================================================
        Stmt::With { body, .. } => {
            // Recursively validate all statements in the with body
            for stmt in body {
                validate_stmt(_ctx, stmt);
            }
        }

        // ====================================================================
        // Function Definitions
        // ====================================================================
        Stmt::FunctionDef { body, .. } => {
            // Recursively validate function body
            for stmt in body {
                validate_stmt(_ctx, stmt);
            }
        }

        // ====================================================================
        // Struct Definitions
        // ====================================================================
        Stmt::StructDef { methods, .. } => {
            // Recursively validate method definitions
            for method in methods {
                validate_stmt(_ctx, method);
            }
        }
    }
}

/// Check if two types are compatible for assignment
///
/// Compatibility rules:
/// - Exact match: Same primitive types (i32 == i32, f64 == f64, bool == bool)
/// - Numeric promotion: i32 is compatible with f64 (can be promoted)
/// - Reference compatibility: &T is compatible with &U if T is compatible with U
/// - User-defined types: UserDefined types are compatible if they reference the same struct definition
pub fn types_compatible<'src, 'arena>(
    lhs: &ResolvedType<'src, 'arena>,
    rhs: &ResolvedType<'src, 'arena>,
) -> bool {
    use ResolvedType::*;

    match (lhs, rhs) {
        // Exact matches for primitive types
        (Bool { .. }, Bool { .. }) => true,
        (I32 { .. }, I32 { .. }) => true,
        (F64 { .. }, F64 { .. }) => true,
        (Real { .. }, Real { .. }) => true,
        (Algebraic { .. }, Algebraic { .. }) => true,

        // Numeric promotion: i32 can be promoted to f64
        (F64 { .. }, I32 { .. }) => true,

        // Numeric promotion: i32 can be promoted to real
        (Real { .. }, I32 { .. }) => true,
        (Real { .. }, F64 { .. }) => true,

        // Numeric promotion: i32 and f64 can be promoted to algebraic
        (Algebraic { .. }, I32 { .. }) => true,
        (Algebraic { .. }, F64 { .. }) => true,
        (Algebraic { .. }, Real { .. }) => true,
        (Real { .. }, Algebraic { .. }) => true,

        // Reference compatibility: &T is compatible with &U if T is compatible with U
        (
            Reference {
                inner: lhs_inner, ..
            },
            Reference {
                inner: rhs_inner, ..
            },
        ) => types_compatible(lhs_inner, rhs_inner),

        // User-defined types: must reference the same struct definition
        (
            UserDefined {
                definition: lhs_def,
                ..
            },
            UserDefined {
                definition: rhs_def,
                ..
            },
        ) => std::ptr::eq(*lhs_def, *rhs_def),

        // All other combinations are incompatible
        _ => false,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir_definitions::StructDefinition;
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

    fn make_real_type(span: Span) -> ResolvedType<'static, 'static> {
        ResolvedType::Real { span }
    }

    fn make_algebraic_type(span: Span) -> ResolvedType<'static, 'static> {
        ResolvedType::Algebraic { span }
    }

    // ========================================================================
    // Type Compatibility Tests
    // ========================================================================

    #[test]
    fn test_types_compatible_exact_match_primitives() {
        let span = make_span(1, 1);

        assert!(types_compatible(&make_i32_type(span), &make_i32_type(span)));
        assert!(types_compatible(&make_f64_type(span), &make_f64_type(span)));
        assert!(types_compatible(
            &make_bool_type(span),
            &make_bool_type(span)
        ));
        assert!(types_compatible(
            &make_real_type(span),
            &make_real_type(span)
        ));
        assert!(types_compatible(
            &make_algebraic_type(span),
            &make_algebraic_type(span)
        ));
    }

    #[test]
    fn test_types_compatible_numeric_promotion_i32_to_f64() {
        let span = make_span(1, 1);
        let i32_ty = make_i32_type(span);
        let f64_ty = make_f64_type(span);

        // f64 = i32 should work (i32 promotes to f64)
        assert!(types_compatible(&f64_ty, &i32_ty));

        // i32 = f64 should NOT work (no demotion)
        assert!(!types_compatible(&i32_ty, &f64_ty));
    }

    #[test]
    fn test_types_compatible_numeric_promotion_to_real() {
        let span = make_span(1, 1);
        let i32_ty = make_i32_type(span);
        let f64_ty = make_f64_type(span);
        let real_ty = make_real_type(span);

        // real = i32 should work
        assert!(types_compatible(&real_ty, &i32_ty));

        // real = f64 should work
        assert!(types_compatible(&real_ty, &f64_ty));

        // i32 = real should NOT work
        assert!(!types_compatible(&i32_ty, &real_ty));

        // f64 = real should NOT work
        assert!(!types_compatible(&f64_ty, &real_ty));
    }

    #[test]
    fn test_types_compatible_numeric_promotion_to_algebraic() {
        let span = make_span(1, 1);
        let i32_ty = make_i32_type(span);
        let f64_ty = make_f64_type(span);
        let algebraic_ty = make_algebraic_type(span);

        // algebraic = i32 should work
        assert!(types_compatible(&algebraic_ty, &i32_ty));

        // algebraic = f64 should work
        assert!(types_compatible(&algebraic_ty, &f64_ty));
    }

    #[test]
    fn test_types_compatible_real_algebraic_bidirectional() {
        let span = make_span(1, 1);
        let real_ty = make_real_type(span);
        let algebraic_ty = make_algebraic_type(span);

        // real and algebraic are compatible both ways
        assert!(types_compatible(&real_ty, &algebraic_ty));
        assert!(types_compatible(&algebraic_ty, &real_ty));
    }

    #[test]
    fn test_types_incompatible_bool_with_numeric() {
        let span = make_span(1, 1);
        let bool_ty = make_bool_type(span);
        let i32_ty = make_i32_type(span);
        let f64_ty = make_f64_type(span);

        assert!(!types_compatible(&bool_ty, &i32_ty));
        assert!(!types_compatible(&bool_ty, &f64_ty));
        assert!(!types_compatible(&i32_ty, &bool_ty));
        assert!(!types_compatible(&f64_ty, &bool_ty));
    }

    #[test]
    fn test_types_compatible_reference_same_inner() {
        let span = make_span(1, 1);
        let i32_ty = make_i32_type(span);

        let ref_i32_1 = ResolvedType::Reference {
            inner: &i32_ty,
            span,
        };
        let ref_i32_2 = ResolvedType::Reference {
            inner: &i32_ty,
            span,
        };

        assert!(types_compatible(&ref_i32_1, &ref_i32_2));
    }

    #[test]
    fn test_types_compatible_reference_different_inner() {
        let span = make_span(1, 1);
        let i32_ty = make_i32_type(span);
        let bool_ty = make_bool_type(span);

        let ref_i32 = ResolvedType::Reference {
            inner: &i32_ty,
            span,
        };
        let ref_bool = ResolvedType::Reference {
            inner: &bool_ty,
            span,
        };

        assert!(!types_compatible(&ref_i32, &ref_bool));
    }

    #[test]
    fn test_types_compatible_reference_with_promotion() {
        let span = make_span(1, 1);
        let i32_ty = make_i32_type(span);
        let f64_ty = make_f64_type(span);

        let ref_f64 = ResolvedType::Reference {
            inner: &f64_ty,
            span,
        };
        let ref_i32 = ResolvedType::Reference {
            inner: &i32_ty,
            span,
        };

        // &f64 = &i32 should work (i32 promotes to f64)
        assert!(types_compatible(&ref_f64, &ref_i32));
    }

    #[test]
    fn test_types_compatible_user_defined_same_struct() {
        let arena = Bump::new();
        let span = make_span(1, 1);

        let struct_def = arena.alloc(StructDefinition::new(
            "Point",
            span,
            vec![],
            vec![],
            None,
            span,
        ));

        let ty1 = ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def,
            span,
        };
        let ty2 = ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def,
            span,
        };

        assert!(types_compatible(&ty1, &ty2));
    }

    #[test]
    fn test_types_compatible_user_defined_different_structs() {
        let arena = Bump::new();
        let span = make_span(1, 1);

        let struct_def1 = arena.alloc(StructDefinition::new(
            "Point",
            span,
            vec![],
            vec![],
            None,
            span,
        ));
        let struct_def2 = arena.alloc(StructDefinition::new(
            "Point",
            span,
            vec![],
            vec![],
            None,
            span,
        ));

        let ty1 = ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def1,
            span,
        };
        let ty2 = ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def2,
            span,
        };

        // Different struct definitions, even with same name
        assert!(!types_compatible(&ty1, &ty2));
    }

    #[test]
    fn test_type_name_primitives() {
        let span = make_span(1, 1);

        assert_eq!(type_name(&make_bool_type(span)), "bool");
        assert_eq!(type_name(&make_i32_type(span)), "i32");
        assert_eq!(type_name(&make_f64_type(span)), "f64");
        assert_eq!(type_name(&make_real_type(span)), "real");
        assert_eq!(type_name(&make_algebraic_type(span)), "algebraic");
    }

    #[test]
    fn test_type_name_reference() {
        let span = make_span(1, 1);
        let i32_ty = make_i32_type(span);
        let ref_ty = ResolvedType::Reference {
            inner: &i32_ty,
            span,
        };

        assert_eq!(type_name(&ref_ty), "&i32");
    }

    #[test]
    fn test_type_name_nested_reference() {
        let span = make_span(1, 1);
        let i32_ty = make_i32_type(span);
        let ref_ty = ResolvedType::Reference {
            inner: &i32_ty,
            span,
        };
        let ref_ref_ty = ResolvedType::Reference {
            inner: &ref_ty,
            span,
        };

        assert_eq!(type_name(&ref_ref_ty), "&&i32");
    }

    #[test]
    fn test_type_name_user_defined() {
        let arena = Bump::new();
        let span = make_span(1, 1);

        let struct_def = arena.alloc(StructDefinition::new(
            "Point",
            span,
            vec![],
            vec![],
            None,
            span,
        ));

        let ty = ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def,
            span,
        };

        assert_eq!(type_name(&ty), "Point");
    }

    // ========================================================================
    // Statement Validation Tests
    // ========================================================================

    #[test]
    fn test_validate_block_stmt() {
        let arena = Bump::new();
        let source = "{ let x = 1; }";
        let mut ctx = TypeCheckContext::new(&arena, source);
        let span = make_span(1, 1);

        let stmt = Stmt::Block {
            statements: vec![],
            span,
        };

        validate_stmt(&mut ctx, &stmt);

        // Empty block should not generate errors
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_validate_if_stmt_recursion() {
        let arena = Bump::new();
        let source = "if true { let x = 1; } else { let y = 2; }";
        let mut ctx = TypeCheckContext::new(&arena, source);
        let span = make_span(1, 1);

        let stmt = Stmt::If {
            condition: crate::ast::expr::Expr::BoolLit { value: true, span },
            then_branch: vec![],
            else_branch: Some(vec![]),
            span,
        };

        validate_stmt(&mut ctx, &stmt);

        // Should recursively validate branches without errors
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_validate_for_stmt_recursion() {
        let arena = Bump::new();
        let source = "for i in 0..10 { let x = i; }";
        let mut ctx = TypeCheckContext::new(&arena, source);
        let span = make_span(1, 1);

        let stmt = Stmt::For {
            loop_var: "i",
            loop_var_span: span,
            iterator: crate::ast::expr::Expr::IntLit { value: 0, span },
            body: vec![],
            span,
        };

        validate_stmt(&mut ctx, &stmt);

        // Should recursively validate body without errors
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_validate_function_def_recursion() {
        let arena = Bump::new();
        let source = "fn foo() -> i32 { return 42; }";
        let mut ctx = TypeCheckContext::new(&arena, source);
        let span = make_span(1, 1);

        let stmt = Stmt::FunctionDef {
            name: "foo".to_string(),
            name_span: span,
            params: vec![],
            return_type: crate::ast::types::Type::I32 { span },
            body: vec![],
            return_expr: None,
            span,
        };

        validate_stmt(&mut ctx, &stmt);

        // Should recursively validate body without errors
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_validate_with_stmt_recursion() {
        let arena = Bump::new();
        let source = "with transform { let x = 1; }";
        let mut ctx = TypeCheckContext::new(&arena, source);
        let span = make_span(1, 1);

        let stmt = Stmt::With {
            context_expr: crate::ast::expr::Expr::IntLit { value: 0, span },
            body: vec![],
            span,
        };

        validate_stmt(&mut ctx, &stmt);

        // Should recursively validate body without errors
        assert!(!ctx.has_errors());
    }
}
