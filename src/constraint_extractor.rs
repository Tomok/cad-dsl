//! Constraint Extractor for Z3 Integration
//!
//! This module extracts constraint equations from the HIR for solving with Z3.
//! It identifies variables and constraint expressions that can be passed to the
//! Z3 constraint solver.
//!
//! # Purpose
//!
//! The constraint extractor processes HIR statements to:
//! - Identify known variables (with initializers): `let y = 10;`
//! - Identify unknown variables (without initializers): `let x;`
//! - Extract constraint expressions (comparisons): `x + y == 20`
//! - Report errors for unsupported constructs (control flow, structs, functions)
//!
//! # Workflow
//!
//! 1. Walk the HIR statements
//! 2. Collect variable declarations with their types and initial values
//! 3. Collect constraint expressions from expression statements
//! 4. Build a `ConstraintProblem` that can be passed to Z3
//!
//! # Supported Constructs
//!
//! - `let` statements (both initialized and uninitialized)
//! - Expression statements with comparison operators (==, !=, <, >, <=, >=)
//!
//! # Unsupported Constructs
//!
//! - Control flow: if, for, return
//! - Definitions: struct, function
//! - Advanced features: with blocks, field assignments
//!
//! These will generate errors as they're out of scope for basic constraint solving.

#![allow(dead_code)] // Public API for future constraint solving implementation

use crate::hir_expr::{ResolvedExpr, ResolvedExprKind, ResolvedStmt, ResolvedStmtKind};
use crate::hir_types::ResolvedType;
use crate::lexer::Span;
use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during constraint extraction
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintExtractorError {
    /// Unsupported statement type (control flow, definitions, etc.)
    UnsupportedStatement {
        statement_type: String,
        span: Span,
        message: String,
    },

    /// Expression statement that is not a constraint (not a comparison)
    NotAConstraint { span: Span },

    /// Variable has no type information
    MissingTypeInfo { var_name: String, span: Span },
}

impl fmt::Display for ConstraintExtractorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstraintExtractorError::UnsupportedStatement {
                statement_type,
                span,
                message,
            } => {
                write!(
                    f,
                    "Unsupported statement type '{}' at line {}, column {}: {}",
                    statement_type, span.start.line, span.start.column, message
                )
            }
            ConstraintExtractorError::NotAConstraint { span } => {
                write!(
                    f,
                    "Expression statement at line {}, column {} is not a constraint (must be a comparison)",
                    span.start.line, span.start.column
                )
            }
            ConstraintExtractorError::MissingTypeInfo { var_name, span } => {
                write!(
                    f,
                    "Variable '{}' at line {}, column {} has no type information",
                    var_name, span.start.line, span.start.column
                )
            }
        }
    }
}

impl std::error::Error for ConstraintExtractorError {}

// ============================================================================
// Data Structures
// ============================================================================

/// Represents a variable in the constraint problem
///
/// Variables can be:
/// - Known: have an initializer value (e.g., `let y = 10;`)
/// - Unknown: declared but not initialized (e.g., `let x;`)
#[derive(Debug, Clone, PartialEq)]
pub struct Variable<'src, 'arena> {
    /// Variable name
    pub name: &'src str,

    /// Variable type (required for constraint solving)
    pub var_type: &'arena ResolvedType<'src, 'arena>,

    /// Optional initial value (known variables)
    pub init: Option<&'arena ResolvedExpr<'src, 'arena>>,

    /// Source span for error reporting
    pub span: Span,
}

impl<'src, 'arena> Variable<'src, 'arena> {
    /// Create a new variable
    pub fn new(
        name: &'src str,
        var_type: &'arena ResolvedType<'src, 'arena>,
        init: Option<&'arena ResolvedExpr<'src, 'arena>>,
        span: Span,
    ) -> Self {
        Self {
            name,
            var_type,
            init,
            span,
        }
    }

    /// Check if this variable is known (has an initializer)
    pub fn is_known(&self) -> bool {
        self.init.is_some()
    }

    /// Check if this variable is unknown (no initializer)
    pub fn is_unknown(&self) -> bool {
        self.init.is_none()
    }
}

/// Represents a constraint expression
///
/// Constraints are comparison expressions that must be satisfied:
/// - Equality: `x + y == 20`
/// - Inequality: `x != 5`
/// - Relational: `x < 10`, `y >= 0`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint<'src, 'arena> {
    /// The constraint expression (must be a comparison)
    pub expr: &'arena ResolvedExpr<'src, 'arena>,

    /// Source span for error reporting
    pub span: Span,
}

impl<'src, 'arena> Constraint<'src, 'arena> {
    /// Create a new constraint
    pub fn new(expr: &'arena ResolvedExpr<'src, 'arena>, span: Span) -> Self {
        Self { expr, span }
    }
}

/// A complete constraint problem ready for solving
///
/// Contains all variables (known and unknown) and all constraints
/// extracted from the HIR.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintProblem<'src, 'arena> {
    /// All variables in the problem
    pub variables: Vec<Variable<'src, 'arena>>,

    /// All constraints in the problem
    pub constraints: Vec<Constraint<'src, 'arena>>,
}

impl<'src, 'arena> ConstraintProblem<'src, 'arena> {
    /// Create a new empty constraint problem
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            constraints: Vec::new(),
        }
    }

    /// Add a variable to the problem
    pub fn add_variable(&mut self, variable: Variable<'src, 'arena>) {
        self.variables.push(variable);
    }

    /// Add a constraint to the problem
    pub fn add_constraint(&mut self, constraint: Constraint<'src, 'arena>) {
        self.constraints.push(constraint);
    }

    /// Get the number of variables
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// Get the number of constraints
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    /// Get all unknown variables (no initializer)
    pub fn unknown_variables(&self) -> Vec<&Variable<'src, 'arena>> {
        self.variables.iter().filter(|v| v.is_unknown()).collect()
    }

    /// Get all known variables (have initializer)
    pub fn known_variables(&self) -> Vec<&Variable<'src, 'arena>> {
        self.variables.iter().filter(|v| v.is_known()).collect()
    }
}

impl<'src, 'arena> Default for ConstraintProblem<'src, 'arena> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Constraint Extractor
// ============================================================================

/// Extract constraints from HIR statements
///
/// Walks through the HIR and identifies:
/// - Variable declarations (let statements)
/// - Constraint expressions (comparison operators)
///
/// Returns a `ConstraintProblem` or a list of errors if unsupported
/// constructs are encountered.
pub fn extract_constraints<'src, 'arena>(
    statements: &[&'arena ResolvedStmt<'src, 'arena>],
) -> Result<ConstraintProblem<'src, 'arena>, Vec<ConstraintExtractorError>> {
    let mut problem = ConstraintProblem::new();
    let mut errors = Vec::new();

    for stmt in statements {
        if let Err(err) = process_statement(stmt, &mut problem) {
            errors.push(err);
        }
    }

    if errors.is_empty() {
        Ok(problem)
    } else {
        Err(errors)
    }
}

/// Process a single statement
fn process_statement<'src, 'arena>(
    stmt: &'arena ResolvedStmt<'src, 'arena>,
    problem: &mut ConstraintProblem<'src, 'arena>,
) -> Result<(), ConstraintExtractorError> {
    match &stmt.kind {
        // Handle let statements - extract variable information
        ResolvedStmtKind::Let {
            var_def,
            init,
            span,
            ..
        } => {
            // Get the variable type
            let var_type = var_def.var_type.as_ref().ok_or_else(|| {
                ConstraintExtractorError::MissingTypeInfo {
                    var_name: var_def.name.to_string(),
                    span: *span,
                }
            })?;

            // Create a variable and add it to the problem
            let variable = Variable::new(var_def.name, var_type, *init, *span);
            problem.add_variable(variable);
            Ok(())
        }

        // Handle expression statements - extract constraints
        ResolvedStmtKind::Expression { expr, span } => {
            // Check if this is a comparison expression (a constraint)
            if is_comparison_expr(expr) {
                let constraint = Constraint::new(expr, *span);
                problem.add_constraint(constraint);
                Ok(())
            } else {
                Err(ConstraintExtractorError::NotAConstraint { span: *span })
            }
        }

        // Unsupported: control flow
        ResolvedStmtKind::If { span, .. } => Err(ConstraintExtractorError::UnsupportedStatement {
            statement_type: "if".to_string(),
            span: *span,
            message: "Control flow is not supported in constraint problems".to_string(),
        }),

        ResolvedStmtKind::For { span, .. } => Err(ConstraintExtractorError::UnsupportedStatement {
            statement_type: "for".to_string(),
            span: *span,
            message: "Loops are not supported in constraint problems".to_string(),
        }),

        ResolvedStmtKind::Return { span, .. } => {
            Err(ConstraintExtractorError::UnsupportedStatement {
                statement_type: "return".to_string(),
                span: *span,
                message: "Return statements are not supported in constraint problems".to_string(),
            })
        }

        // Unsupported: definitions
        ResolvedStmtKind::FunctionDef { span, .. } => {
            Err(ConstraintExtractorError::UnsupportedStatement {
                statement_type: "function definition".to_string(),
                span: *span,
                message: "Function definitions are not supported in constraint problems"
                    .to_string(),
            })
        }

        ResolvedStmtKind::StructDef { span, .. } => {
            Err(ConstraintExtractorError::UnsupportedStatement {
                statement_type: "struct definition".to_string(),
                span: *span,
                message: "Struct definitions are not supported in constraint problems".to_string(),
            })
        }

        // Unsupported: advanced features
        ResolvedStmtKind::Assignment { span, .. } => {
            Err(ConstraintExtractorError::UnsupportedStatement {
                statement_type: "assignment".to_string(),
                span: *span,
                message: "Variable reassignment is not supported in constraint problems"
                    .to_string(),
            })
        }

        ResolvedStmtKind::FieldAssignment { span, .. } => {
            Err(ConstraintExtractorError::UnsupportedStatement {
                statement_type: "field assignment".to_string(),
                span: *span,
                message: "Field assignments are not supported in constraint problems".to_string(),
            })
        }

        ResolvedStmtKind::With { span, .. } => {
            Err(ConstraintExtractorError::UnsupportedStatement {
                statement_type: "with".to_string(),
                span: *span,
                message: "With blocks are not supported in constraint problems".to_string(),
            })
        }

        // Block: recursively process statements
        ResolvedStmtKind::Block { statements, .. } => {
            for inner_stmt in statements {
                process_statement(inner_stmt, problem)?;
            }
            Ok(())
        }
    }
}

/// Check if an expression is a comparison (constraint)
fn is_comparison_expr<'src, 'arena>(expr: &ResolvedExpr<'src, 'arena>) -> bool {
    matches!(
        expr.kind,
        ResolvedExprKind::Eq { .. }
            | ResolvedExprKind::NotEq { .. }
            | ResolvedExprKind::Lt { .. }
            | ResolvedExprKind::Gt { .. }
            | ResolvedExprKind::LtEq { .. }
            | ResolvedExprKind::GtEq { .. }
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir_definitions::VarDefinition;
    use crate::hir_types::ResolvedType;
    use crate::lexer::LineColumn;
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
    fn test_variable_is_known() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let init = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, ty);

        let var = Variable::new("x", ty, Some(init), test_span());
        assert!(var.is_known());
        assert!(!var.is_unknown());
    }

    #[test]
    fn test_variable_is_unknown() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let var = Variable::new("x", ty, None, test_span());
        assert!(var.is_unknown());
        assert!(!var.is_known());
    }

    #[test]
    fn test_constraint_problem_counts() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let mut problem = ConstraintProblem::new();
        assert_eq!(problem.variable_count(), 0);
        assert_eq!(problem.constraint_count(), 0);

        let var = Variable::new("x", ty, None, test_span());
        problem.add_variable(var);
        assert_eq!(problem.variable_count(), 1);

        let expr = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, ty),
                rhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, ty),
            },
            ty,
        );
        let constraint = Constraint::new(expr, test_span());
        problem.add_constraint(constraint);
        assert_eq!(problem.constraint_count(), 1);
    }

    #[test]
    fn test_constraint_problem_filters() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let init = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, ty);

        let mut problem = ConstraintProblem::new();

        // Add known variable
        let var1 = Variable::new("x", ty, Some(init), test_span());
        problem.add_variable(var1);

        // Add unknown variable
        let var2 = Variable::new("y", ty, None, test_span());
        problem.add_variable(var2);

        assert_eq!(problem.known_variables().len(), 1);
        assert_eq!(problem.unknown_variables().len(), 1);
    }

    #[test]
    fn test_extract_let_statement_with_init() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let init = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, ty);

        let var_def = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*ty),
            init: Some(init),
            scope_level: 0,
            span: test_span(),
        });

        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def,
                init: Some(init),
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.variable_count(), 1);
        assert_eq!(problem.constraint_count(), 0);
        assert_eq!(problem.known_variables().len(), 1);
        assert_eq!(problem.variables[0].name, "x");
    }

    #[test]
    fn test_extract_let_statement_without_init() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let var_def = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*ty),
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

        let result = extract_constraints(&[stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.variable_count(), 1);
        assert_eq!(problem.constraint_count(), 0);
        assert_eq!(problem.unknown_variables().len(), 1);
        assert_eq!(problem.variables[0].name, "x");
    }

    #[test]
    fn test_extract_constraint_expression() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty);
        let expr = make_expr(&arena, ResolvedExprKind::Eq { lhs, rhs }, ty);

        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.variable_count(), 0);
        assert_eq!(problem.constraint_count(), 1);
    }

    #[test]
    fn test_multiple_variables_and_constraints() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // let x = 10;
        let init_x = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: Some(init_x),
            scope_level: 0,
            span: test_span(),
        });
        let stmt_x = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def: var_def_x,
                init: Some(init_x),
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
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty);
        let expr = make_expr(&arena, ResolvedExprKind::Eq { lhs, rhs }, bool_ty);
        let stmt_constraint = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt_x, stmt_y, stmt_constraint]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.variable_count(), 2);
        assert_eq!(problem.constraint_count(), 1);
        assert_eq!(problem.known_variables().len(), 1);
        assert_eq!(problem.unknown_variables().len(), 1);
    }

    #[test]
    fn test_is_comparison_expr() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, ty);

        // Test all comparison operators
        assert!(is_comparison_expr(make_expr(
            &arena,
            ResolvedExprKind::Eq { lhs, rhs },
            ty
        )));
        assert!(is_comparison_expr(make_expr(
            &arena,
            ResolvedExprKind::NotEq { lhs, rhs },
            ty
        )));
        assert!(is_comparison_expr(make_expr(
            &arena,
            ResolvedExprKind::Lt { lhs, rhs },
            ty
        )));
        assert!(is_comparison_expr(make_expr(
            &arena,
            ResolvedExprKind::Gt { lhs, rhs },
            ty
        )));
        assert!(is_comparison_expr(make_expr(
            &arena,
            ResolvedExprKind::LtEq { lhs, rhs },
            ty
        )));
        assert!(is_comparison_expr(make_expr(
            &arena,
            ResolvedExprKind::GtEq { lhs, rhs },
            ty
        )));

        // Non-comparison expressions
        assert!(!is_comparison_expr(make_expr(
            &arena,
            ResolvedExprKind::IntLit { value: 42 },
            ty
        )));
        assert!(!is_comparison_expr(make_expr(
            &arena,
            ResolvedExprKind::Add { lhs, rhs },
            ty
        )));
    }

    #[test]
    fn test_error_not_a_constraint() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Expression that is not a comparison
        let expr = make_expr(&arena, ResolvedExprKind::IntLit { value: 42 }, ty);
        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt]);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(errors[0], ConstraintExtractorError::NotAConstraint { .. });
    }

    #[test]
    fn test_error_unsupported_if_statement() {
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

        let result = extract_constraints(&[stmt]);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            errors[0],
            ConstraintExtractorError::UnsupportedStatement { ref statement_type, .. }
            if statement_type == "if"
        );
    }

    #[test]
    fn test_error_unsupported_for_statement() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let iterator = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, ty);
        let loop_var_def = arena.alloc(VarDefinition {
            name: "i",
            name_span: test_span(),
            var_type: Some(*ty),
            init: None,
            scope_level: 1,
            span: test_span(),
        });

        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::For {
                loop_var_def,
                iterator,
                body: vec![],
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt]);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            errors[0],
            ConstraintExtractorError::UnsupportedStatement { ref statement_type, .. }
            if statement_type == "for"
        );
    }

    #[test]
    fn test_error_unsupported_struct_def() {
        let arena = Bump::new();
        let struct_def = arena.alloc(crate::hir_definitions::StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::StructDef {
                struct_def,
                methods: vec![],
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt]);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            errors[0],
            ConstraintExtractorError::UnsupportedStatement { ref statement_type, .. }
            if statement_type == "struct definition"
        );
    }

    #[test]
    fn test_error_unsupported_function_def() {
        let arena = Bump::new();
        let func_def = arena.alloc(crate::hir_definitions::FunctionDefinition {
            name: "foo",
            name_span: test_span(),
            params: vec![],
            return_type: ResolvedType::I32 { span: test_span() },
            body: vec![],
            parent_struct: None,
            span: test_span(),
        });

        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::FunctionDef {
                func_def,
                body: vec![],
                return_expr: None,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt]);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            errors[0],
            ConstraintExtractorError::UnsupportedStatement { ref statement_type, .. }
            if statement_type == "function definition"
        );
    }

    #[test]
    fn test_error_missing_type_info() {
        let arena = Bump::new();

        let var_def = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: None, // Missing type info
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

        let result = extract_constraints(&[stmt]);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            errors[0],
            ConstraintExtractorError::MissingTypeInfo { ref var_name, .. }
            if var_name == "x"
        );
    }

    #[test]
    fn test_block_statement_recursion() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create a variable inside a block
        let var_def = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*ty),
            init: None,
            scope_level: 1,
            span: test_span(),
        });
        let inner_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def,
                init: None,
                span: test_span(),
            },
        );

        let block_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Block {
                statements: vec![inner_stmt],
                span: test_span(),
            },
        );

        let result = extract_constraints(&[block_stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.variable_count(), 1);
        assert_eq!(problem.variables[0].name, "x");
    }

    #[test]
    fn test_error_display_unsupported_statement() {
        let error = ConstraintExtractorError::UnsupportedStatement {
            statement_type: "if".to_string(),
            span: test_span(),
            message: "Control flow is not supported".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("if"));
        assert!(display.contains("Control flow is not supported"));
    }

    #[test]
    fn test_error_display_not_a_constraint() {
        let error = ConstraintExtractorError::NotAConstraint { span: test_span() };
        let display = format!("{}", error);
        assert!(display.contains("not a constraint"));
        assert!(display.contains("comparison"));
    }

    #[test]
    fn test_error_display_missing_type_info() {
        let error = ConstraintExtractorError::MissingTypeInfo {
            var_name: "x".to_string(),
            span: test_span(),
        };
        let display = format!("{}", error);
        assert!(display.contains("x"));
        assert!(display.contains("no type information"));
    }
}
