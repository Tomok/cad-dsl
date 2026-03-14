//! Type validation for statements in the CAD-DSL type checker
#![allow(dead_code)] // Validation functions for future phases
//!
//! This module provides validation functionality that checks types in statements,
//! ensuring type consistency and compatibility across variable declarations,
//! assignments, control flow, and function calls.

use super::context::TypeCheckContext;
use crate::hir::expr::{ResolvedStmt, ResolvedStmtKind};
use crate::hir::types::ResolvedType;

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
pub fn validate_stmt<'src, 'arena>(
    _ctx: &mut TypeCheckContext<'src, 'arena>,
    stmt: &'arena ResolvedStmt<'src, 'arena>,
) {
    match &stmt.kind {
        // ====================================================================
        // Let Statements
        // ====================================================================
        ResolvedStmtKind::Let { var_def, init, .. } => {
            // If there's both a type annotation and an initializer, check compatibility
            if let (Some(var_type), Some(init_expr)) = (&var_def.var_type, init) {
                // Check if the initializer type is compatible with the variable type
                let init_type = init_expr.ty;
                if !types_compatible(var_type, init_type) {
                    // TODO: Add error to context
                    // For now, we'll skip error reporting as the type checker is still being integrated
                    let _ = (var_type, init_type);
                }
            }
        }

        // ====================================================================
        // Assignment Statements
        // ====================================================================
        ResolvedStmtKind::Assignment { var_def, value, .. } => {
            // Validate that the value type is compatible with the variable type
            if let Some(var_type) = &var_def.var_type {
                let value_type = value.ty;
                if !types_compatible(var_type, value_type) {
                    // TODO: Add error to context
                    let _ = (var_type, value_type);
                }
            }
        }

        // ====================================================================
        // Field Assignment Statements
        // ====================================================================
        ResolvedStmtKind::FieldAssignment { target, value, .. } => {
            // Validate that the value type is compatible with the field type
            let target_type = target.ty;
            let value_type = value.ty;
            if !types_compatible(target_type, value_type) {
                // TODO: Add error to context
                let _ = (target_type, value_type);
            }
        }

        // ====================================================================
        // If Statements
        // ====================================================================
        ResolvedStmtKind::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            // Validate that the condition is a boolean
            let cond_type = condition.ty;
            if !matches!(cond_type, ResolvedType::Bool { .. }) {
                // TODO: Add error to context
                let _ = cond_type;
            }

            // Recursively validate branches
            for stmt in then_branch {
                validate_stmt(_ctx, stmt);
            }

            if let Some(else_stmts) = else_branch {
                for stmt in else_stmts {
                    validate_stmt(_ctx, stmt);
                }
            }
        }

        // ====================================================================
        // For Loops
        // ====================================================================
        ResolvedStmtKind::For { iterator, body, .. } => {
            // Validate that iterator is a Range or Array type
            // TODO: Implement proper iterator type validation
            let _ = iterator;

            // Recursively validate body
            for stmt in body {
                validate_stmt(_ctx, stmt);
            }
        }

        // ====================================================================
        // Return Statements
        // ====================================================================
        ResolvedStmtKind::Return { value, .. } => {
            // Validate that return value type matches function return type
            // This requires function context which we don't have yet
            // TODO: Implement once we have function context in TypeCheckContext
            let _ = value;
        }

        // ====================================================================
        // Expression Statements
        // ====================================================================
        ResolvedStmtKind::Expression { expr, .. } => {
            // Validate the expression
            // The expression already has a resolved type from semantic analysis
            let _ = expr;
        }

        // ====================================================================
        // Block Statements
        // ====================================================================
        ResolvedStmtKind::Block { statements, .. } => {
            // Recursively validate all statements in the block
            for stmt in statements {
                validate_stmt(_ctx, stmt);
            }
        }

        // ====================================================================
        // With Statements
        // ====================================================================
        ResolvedStmtKind::With { body, .. } => {
            // Recursively validate all statements in the with body
            for stmt in body {
                validate_stmt(_ctx, stmt);
            }
        }

        // ====================================================================
        // Function Definitions
        // ====================================================================
        ResolvedStmtKind::FunctionDef { body, .. } => {
            // Recursively validate function body
            for stmt in body {
                validate_stmt(_ctx, stmt);
            }
        }

        // ====================================================================
        // Struct Definitions
        // ====================================================================
        ResolvedStmtKind::StructDef { methods, .. } => {
            // Recursively validate method definitions
            for method in methods {
                validate_stmt(_ctx, method);
            }
        }

        // ====================================================================
        // Optimize Block
        // ====================================================================
        ResolvedStmtKind::Optimize { directives, .. } => {
            // Validate that each directive expression has a numeric type
            for directive in directives {
                let expr_type = directive.expr.ty;
                // Optimize objectives must be numeric (i32 or f64)
                // Non-numeric types (bool, structs, arrays) are not valid objectives
                let is_numeric = matches!(
                    expr_type,
                    crate::hir::types::ResolvedType::I32 { .. }
                        | crate::hir::types::ResolvedType::F64 { .. }
                );
                if !is_numeric {
                    // TODO: Add error to context when error reporting is integrated
                    let _ = expr_type;
                }
            }
        }

        // Global rune function declarations need no validation — they are pure Rune code
        // that will be type-checked by the Rune compiler when injected into rune blocks.
        ResolvedStmtKind::GlobalRuneFn { .. } => {}
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
        ResolvedType::Array {
            element_type, size, ..
        } => format!("[{}; {}]", type_name(element_type), size),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::definitions::{StructDefinition, VarDefinition};
    use crate::hir::expr::{ResolvedExpr, ResolvedExprKind};
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

        let stmt = arena.alloc(ResolvedStmt {
            span,
            kind: ResolvedStmtKind::Block {
                statements: vec![],
                span,
            },
        });

        validate_stmt(&mut ctx, stmt);

        // Empty block should not generate errors
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_validate_if_stmt_recursion() {
        let arena = Bump::new();
        let source = "if true { let x = 1; } else { let y = 2; }";
        let mut ctx = TypeCheckContext::new(&arena, source);
        let span = make_span(1, 1);

        let bool_type = arena.alloc(ResolvedType::Bool { span });
        let condition = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::BoolLit { value: true },
            ty: bool_type,
        });

        let stmt = arena.alloc(ResolvedStmt {
            span,
            kind: ResolvedStmtKind::If {
                condition,
                then_branch: vec![],
                else_branch: Some(vec![]),
                span,
            },
        });

        validate_stmt(&mut ctx, stmt);

        // Should recursively validate branches without errors
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_validate_for_stmt_recursion() {
        let arena = Bump::new();
        let source = "for i in 0..10 { let x = i; }";
        let mut ctx = TypeCheckContext::new(&arena, source);
        let span = make_span(1, 1);

        let i32_type = arena.alloc(ResolvedType::I32 { span });
        let identifier = arena.alloc(crate::hir::definitions::VariableIdentifier::Simple("i"));
        let loop_var_def = arena.alloc(VarDefinition::new(
            identifier,
            "i",
            span,
            Some(ResolvedType::I32 { span }),
            crate::hir::definitions::VarDefinitionKind::Uninitialized,
            1,
            span,
        ));
        let iterator = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 0 },
            ty: i32_type,
        });

        let stmt = arena.alloc(ResolvedStmt {
            span,
            kind: ResolvedStmtKind::For {
                loop_var_def,
                iterator,
                body: vec![],
                span,
            },
        });

        validate_stmt(&mut ctx, stmt);

        // Should recursively validate body without errors
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_validate_function_def_recursion() {
        let arena = Bump::new();
        let source = "fn foo() -> i32 { return 42; }";
        let mut ctx = TypeCheckContext::new(&arena, source);
        let span = make_span(1, 1);

        let func_def = arena.alloc(crate::hir::definitions::FunctionDefinition {
            name: "foo",
            name_span: span,
            params: vec![],
            return_type: ResolvedType::I32 { span },
            body: vec![],
            parent_struct: None,
            span,
        });

        let stmt = arena.alloc(ResolvedStmt {
            span,
            kind: ResolvedStmtKind::FunctionDef {
                func_def,
                body: vec![],
                return_expr: None,
                span,
            },
        });

        validate_stmt(&mut ctx, stmt);

        // Should recursively validate body without errors
        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_validate_with_stmt_recursion() {
        let arena = Bump::new();
        let source = "with transform { let x = 1; }";
        let mut ctx = TypeCheckContext::new(&arena, source);
        let span = make_span(1, 1);

        let i32_type = arena.alloc(ResolvedType::I32 { span });
        let context_expr = arena.alloc(ResolvedExpr {
            span,
            kind: ResolvedExprKind::IntLit { value: 0 },
            ty: i32_type,
        });
        let with_context = arena.alloc(crate::hir::context::WithContext {
            context_expr,
            container_field: None,
            transforms: vec![],
        });

        let stmt = arena.alloc(ResolvedStmt {
            span,
            kind: ResolvedStmtKind::With {
                with_context,
                body: vec![],
                span,
            },
        });

        validate_stmt(&mut ctx, stmt);

        // Should recursively validate body without errors
        assert!(!ctx.has_errors());
    }
}
