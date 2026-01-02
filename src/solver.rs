//! Solver Pipeline for Constraint Solving
//!
//! This module orchestrates the complete constraint solving pipeline, integrating:
//! - Constraint extraction from HIR
//! - Z3 solver bridge
//! - Solution formatting
//!
//! # Purpose
//!
//! The solver pipeline provides a high-level API for solving constraint problems
//! defined in the CAD-DSL language. It takes HIR statements as input and produces
//! formatted variable assignments as output.
//!
//! # Pipeline Stages
//!
//! 1. **Constraint Extraction**: Extract variables and constraints from HIR statements
//! 2. **Z3 Translation**: Translate HIR expressions to Z3 assertions
//! 3. **Solving**: Run Z3 solver to find a satisfying assignment
//! 4. **Formatting**: Format the solution for human-readable output
//!
//! # Example
//!
//! ```ignore
//! // Given HIR statements for:
//! // let x;
//! // let y = 10;
//! // x + y == 20;
//!
//! let result = solve(&statements);
//! // result == Ok("x = 10\ny = 10\n")
//! ```
//!
//! # Error Handling
//!
//! The pipeline can fail at any stage:
//! - Empty program (no statements)
//! - Constraint extraction errors (unsupported statements)
//! - Z3 translation errors (unsupported types/expressions)
//! - Solving errors (UNSAT, unknown)
//! - Formatting errors (evaluation failures)

#![allow(dead_code)] // Public API for future constraint solving integration

// ============================================================================
// Submodule Declarations
// ============================================================================

pub mod constraint_extractor;
pub mod solution_formatter;
pub mod z3_bridge;

// ============================================================================
// Public Re-exports
// ============================================================================

#[allow(unused_imports)]
pub use constraint_extractor::{ConstraintExtractorError, extract_constraints};
#[allow(unused_imports)]
pub use solution_formatter::{SolutionFormatter, SolutionFormatterError};
#[allow(unused_imports)]
pub use z3_bridge::{Z3Bridge, Z3BridgeError};

// ============================================================================
// Imports
// ============================================================================

use crate::hir::expr::ResolvedStmt;
use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during the solving pipeline
#[derive(Debug, Clone, PartialEq)]
pub enum SolverError {
    /// Empty program - no statements provided
    EmptyProgram,

    /// No variables found in the program
    NoVariables,

    /// No constraints found in the program
    NoConstraints,

    /// Constraint extraction failed
    ConstraintExtraction {
        errors: Vec<ConstraintExtractorError>,
    },

    /// Z3 bridge translation failed
    Z3Bridge { error: Z3BridgeError },

    /// Solution formatting failed
    SolutionFormatting { error: SolutionFormatterError },
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolverError::EmptyProgram => {
                write!(f, "Empty program: no statements provided")
            }
            SolverError::NoVariables => {
                write!(
                    f,
                    "No variables found: program must declare at least one variable"
                )
            }
            SolverError::NoConstraints => {
                write!(
                    f,
                    "No constraints found: program must include at least one constraint"
                )
            }
            SolverError::ConstraintExtraction { errors } => {
                writeln!(
                    f,
                    "Constraint extraction failed with {} error(s):",
                    errors.len()
                )?;
                for (i, err) in errors.iter().enumerate() {
                    writeln!(f, "  {}. {}", i + 1, err)?;
                }
                Ok(())
            }
            SolverError::Z3Bridge { error } => {
                write!(f, "Z3 translation failed: {}", error)
            }
            SolverError::SolutionFormatting { error } => {
                write!(f, "Solution formatting failed: {}", error)
            }
        }
    }
}

impl std::error::Error for SolverError {}

// Convert from individual error types
impl From<Z3BridgeError> for SolverError {
    fn from(error: Z3BridgeError) -> Self {
        SolverError::Z3Bridge { error }
    }
}

impl From<SolutionFormatterError> for SolverError {
    fn from(error: SolutionFormatterError) -> Self {
        SolverError::SolutionFormatting { error }
    }
}

// ============================================================================
// Solver Pipeline
// ============================================================================

/// Solve a constraint problem defined by HIR statements
///
/// This is the main entry point for the constraint solving pipeline.
/// It takes a slice of HIR statements and returns a formatted solution
/// or an error if the problem cannot be solved.
///
/// # Arguments
///
/// * `statements` - Slice of HIR statements (from semantic analyzer + type checker)
///
/// # Returns
///
/// * `Ok(String)` - Formatted solution with variable assignments
/// * `Err(SolverError)` - Error at any stage of the pipeline
///
/// # Example
///
/// ```ignore
/// let arena = Bump::new();
/// let statements = vec![/* HIR statements */];
/// let solution = solve(&statements)?;
/// println!("{}", solution);
/// // Output:
/// // x = 10
/// // y = 20
/// ```
pub fn solve<'src, 'arena>(
    statements: &[&'arena ResolvedStmt<'src, 'arena>],
) -> Result<String, SolverError> {
    // Step 0: Validate input
    if statements.is_empty() {
        return Err(SolverError::EmptyProgram);
    }

    // Step 1: Extract constraints from HIR
    let problem = extract_constraints(statements)
        .map_err(|errors| SolverError::ConstraintExtraction { errors })?;

    // Validate that we have variables and constraints
    if problem.variables.is_empty() {
        return Err(SolverError::NoVariables);
    }

    // Update constraint validation to include conditional constraints
    if problem.constraints.is_empty() && problem.conditional_constraints.is_empty() {
        return Err(SolverError::NoConstraints);
    }

    // Step 2: Create Z3 bridge and add the problem
    let mut bridge = Z3Bridge::new();
    bridge.add_problem(&problem)?;

    // Step 3: Get the solver (already has all assertions added)
    let solver = bridge.solver();

    // Step 4: Format the solution
    let var_list: Vec<_> = problem.variables.iter().collect();
    let formatter = SolutionFormatter::new(bridge.variables(), var_list);
    let solution = formatter.format_solution(solver)?;

    Ok(solution)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::definitions::VarDefinition;
    use crate::hir::expr::{ResolvedExpr, ResolvedExprKind, ResolvedStmt, ResolvedStmtKind};
    use crate::hir::types::ResolvedType;
    use crate::lexer::{LineColumn, Span};
    use assert_matches::assert_matches;
    use bumpalo::Bump;

    /// Helper to create a test span
    fn test_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 10,
        }
    }

    /// Helper to create a resolved expression
    fn make_expr<'arena>(
        arena: &'arena Bump,
        kind: ResolvedExprKind<'static, 'arena>,
        ty: &'arena ResolvedType<'static, 'arena>,
    ) -> &'arena ResolvedExpr<'static, 'arena> {
        arena.alloc(ResolvedExpr {
            span: test_span(),
            kind,
            ty,
        })
    }

    /// Helper to create a resolved statement
    fn make_stmt<'arena>(
        arena: &'arena Bump,
        kind: ResolvedStmtKind<'static, 'arena>,
    ) -> &'arena ResolvedStmt<'static, 'arena> {
        arena.alloc(ResolvedStmt {
            span: test_span(),
            kind,
        })
    }

    #[test]
    fn test_solve_simple_linear_equation() {
        // Test: let x; x + 10 == 20;
        // Expected: x = 10
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // let x;
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let stmt_x = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def: var_def_x,
                init: None,
                span: test_span(),
            },
        );

        // x + 10 == 20
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let ten = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let sum = make_expr(
            &arena,
            ResolvedExprKind::Add {
                lhs: x_ref,
                rhs: ten,
            },
            int_ty,
        );
        let twenty = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: sum,
                rhs: twenty,
            },
            bool_ty,
        );
        let stmt_constraint = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint,
                span: test_span(),
            },
        );

        let result = solve(&[stmt_x, stmt_constraint]);
        assert!(result.is_ok());
        let solution = result.unwrap();
        assert_eq!(solution.trim(), "x = 10");
    }

    #[test]
    fn test_solve_multiple_variables() {
        // Test: let x; let y = 10; x + y == 20;
        // Expected: x = 10, y = 10
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // let x;
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let stmt_x = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def: var_def_x,
                init: None,
                span: test_span(),
            },
        );

        // let y = 10;
        let init_y = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let var_def_y = arena.alloc(VarDefinition {
            name: "y",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: Some(init_y),
            scope_level: 0,
            span: test_span(),
        });
        let stmt_y = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("y", test_span())],
                var_def: var_def_y,
                init: Some(init_y),
                span: test_span(),
            },
        );

        // x + y == 20
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let y_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let sum = make_expr(
            &arena,
            ResolvedExprKind::Add {
                lhs: x_ref,
                rhs: y_ref,
            },
            int_ty,
        );
        let twenty = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: sum,
                rhs: twenty,
            },
            bool_ty,
        );
        let stmt_constraint = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint,
                span: test_span(),
            },
        );

        let result = solve(&[stmt_x, stmt_y, stmt_constraint]);
        assert!(result.is_ok());
        let solution = result.unwrap();
        // Variables should be sorted alphabetically
        assert_eq!(solution, "x = 10\ny = 10\n");
    }

    #[test]
    fn test_solve_multiple_constraints() {
        // Test: let x; let y; x + y == 20; x - y == 0;
        // Expected: x = 10, y = 10
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // let x;
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let stmt_x = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def: var_def_x,
                init: None,
                span: test_span(),
            },
        );

        // let y;
        let var_def_y = arena.alloc(VarDefinition {
            name: "y",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let stmt_y = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("y", test_span())],
                var_def: var_def_y,
                init: None,
                span: test_span(),
            },
        );

        // x + y == 20
        let x_ref1 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let y_ref1 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let sum = make_expr(
            &arena,
            ResolvedExprKind::Add {
                lhs: x_ref1,
                rhs: y_ref1,
            },
            int_ty,
        );
        let twenty = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let constraint1 = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: sum,
                rhs: twenty,
            },
            bool_ty,
        );
        let stmt_constraint1 = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint1,
                span: test_span(),
            },
        );

        // x - y == 0
        let x_ref2 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let y_ref2 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let diff = make_expr(
            &arena,
            ResolvedExprKind::Sub {
                lhs: x_ref2,
                rhs: y_ref2,
            },
            int_ty,
        );
        let zero = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let constraint2 = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: diff,
                rhs: zero,
            },
            bool_ty,
        );
        let stmt_constraint2 = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint2,
                span: test_span(),
            },
        );

        let result = solve(&[stmt_x, stmt_y, stmt_constraint1, stmt_constraint2]);
        assert!(result.is_ok());
        let solution = result.unwrap();
        assert_eq!(solution, "x = 10\ny = 10\n");
    }

    #[test]
    fn test_solve_float_constraint() {
        // Test: let x; x * 2.0 == 6.28;
        // Expected: x = 3.14
        let arena = Bump::new();
        let real_ty = arena.alloc(ResolvedType::F64 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // let x;
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*real_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let stmt_x = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def: var_def_x,
                init: None,
                span: test_span(),
            },
        );

        // x * 2.0 == 6.28
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            real_ty,
        );
        let two = make_expr(&arena, ResolvedExprKind::FloatLit { value: 2.0 }, real_ty);
        let product = make_expr(
            &arena,
            ResolvedExprKind::Mul {
                lhs: x_ref,
                rhs: two,
            },
            real_ty,
        );
        let target = make_expr(&arena, ResolvedExprKind::FloatLit { value: 6.28 }, real_ty);
        let constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: product,
                rhs: target,
            },
            bool_ty,
        );
        let stmt_constraint = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint,
                span: test_span(),
            },
        );

        let result = solve(&[stmt_x, stmt_constraint]);
        assert!(result.is_ok());
        let solution = result.unwrap();
        assert!(solution.contains("x = 3.14"));
    }

    #[test]
    fn test_solve_bool_constraint() {
        // Test: let x; x == true;
        // Expected: x = true
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // let x;
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*bool_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let stmt_x = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def: var_def_x,
                init: None,
                span: test_span(),
            },
        );

        // x == true
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            bool_ty,
        );
        let true_lit = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);
        let constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: x_ref,
                rhs: true_lit,
            },
            bool_ty,
        );
        let stmt_constraint = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint,
                span: test_span(),
            },
        );

        let result = solve(&[stmt_x, stmt_constraint]);
        assert!(result.is_ok());
        let solution = result.unwrap();
        assert_eq!(solution.trim(), "x = true");
    }

    #[test]
    fn test_solve_unsat_constraints() {
        // Test: let x; x == 10; x == 20;
        // Expected: UNSAT
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // let x;
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let stmt_x = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def: var_def_x,
                init: None,
                span: test_span(),
            },
        );

        // x == 10
        let x_ref1 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let ten = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let constraint1 = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: x_ref1,
                rhs: ten,
            },
            bool_ty,
        );
        let stmt_constraint1 = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint1,
                span: test_span(),
            },
        );

        // x == 20
        let x_ref2 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let twenty = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let constraint2 = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: x_ref2,
                rhs: twenty,
            },
            bool_ty,
        );
        let stmt_constraint2 = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint2,
                span: test_span(),
            },
        );

        let result = solve(&[stmt_x, stmt_constraint1, stmt_constraint2]);
        assert!(result.is_err());
        assert_matches!(
            result.unwrap_err(),
            SolverError::SolutionFormatting {
                error: SolutionFormatterError::Unsat
            }
        );
    }

    #[test]
    fn test_solve_under_constrained() {
        // Test: let x; let y; x + y == 20;
        // Expected: Z3 picks one solution (e.g., x = 0, y = 20 or x = 10, y = 10)
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // let x;
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let stmt_x = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def: var_def_x,
                init: None,
                span: test_span(),
            },
        );

        // let y;
        let var_def_y = arena.alloc(VarDefinition {
            name: "y",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let stmt_y = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("y", test_span())],
                var_def: var_def_y,
                init: None,
                span: test_span(),
            },
        );

        // x + y == 20
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let y_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let sum = make_expr(
            &arena,
            ResolvedExprKind::Add {
                lhs: x_ref,
                rhs: y_ref,
            },
            int_ty,
        );
        let twenty = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: sum,
                rhs: twenty,
            },
            bool_ty,
        );
        let stmt_constraint = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint,
                span: test_span(),
            },
        );

        let result = solve(&[stmt_x, stmt_y, stmt_constraint]);
        assert!(result.is_ok());
        let solution = result.unwrap();
        // Solution should have x and y values that sum to 20
        // We can't predict exactly what Z3 will choose, but we can verify it's valid
        assert!(solution.contains("x = "));
        assert!(solution.contains("y = "));
    }

    #[test]
    fn test_error_empty_program() {
        let result = solve(&[]);
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), SolverError::EmptyProgram);
    }

    #[test]
    fn test_error_no_variables() {
        // Test: 1 + 1 == 2; (no variables)
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        let one = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty);
        let two = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty);
        let sum = make_expr(&arena, ResolvedExprKind::Add { lhs: one, rhs: one }, int_ty);
        let constraint = make_expr(&arena, ResolvedExprKind::Eq { lhs: sum, rhs: two }, bool_ty);
        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint,
                span: test_span(),
            },
        );

        let result = solve(&[stmt]);
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), SolverError::NoVariables);
    }

    #[test]
    fn test_error_no_constraints() {
        // Test: let x; (no constraints)
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let var_def = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def,
                init: None,
                span: test_span(),
            },
        );

        let result = solve(&[stmt]);
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), SolverError::NoConstraints);
    }

    #[test]
    fn test_error_unsupported_statement() {
        // Test: if true { }
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        let condition = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);
        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::If {
                condition,
                then_branch: vec![],
                else_branch: None,
                span: test_span(),
            },
        );

        let result = solve(&[stmt]);
        assert!(result.is_err());
        assert_matches!(
            result.unwrap_err(),
            SolverError::ConstraintExtraction { .. }
        );
    }

    #[test]
    fn test_error_display_empty_program() {
        let error = SolverError::EmptyProgram;
        let display = format!("{}", error);
        assert!(display.contains("Empty program"));
    }

    #[test]
    fn test_error_display_no_variables() {
        let error = SolverError::NoVariables;
        let display = format!("{}", error);
        assert!(display.contains("No variables found"));
    }

    #[test]
    fn test_error_display_no_constraints() {
        let error = SolverError::NoConstraints;
        let display = format!("{}", error);
        assert!(display.contains("No constraints found"));
    }
}
