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
//! - Flatten struct variables into their primitive fields
//! - Extract constraint expressions (comparisons): `x + y == 20`
//! - Report errors for unsupported constructs (recursive structs, functions)
//!
//! # Workflow
//!
//! 1. Walk the HIR statements
//! 2. Collect variable declarations with their types and initial values
//! 3. For struct types, flatten into primitive fields with qualified names
//! 4. Detect and reject recursive struct types
//! 5. Collect constraint expressions from expression statements
//! 6. Build a `ConstraintProblem` that can be passed to Z3
//!
//! # Supported Constructs
//!
//! - `let` statements (both initialized and uninitialized)
//! - Struct types (flattened into primitive fields)
//! - Expression statements with comparison operators (==, !=, <, >, <=, >=)
//! - Conditional constraints (if-statements)
//!
//! # Unsupported Constructs
//!
//! - Recursive struct types
//! - Control flow: for, return
//! - Definitions: function definitions
//! - Advanced features: with blocks
//!
//! These will generate errors as they're out of scope for constraint solving.

#![allow(dead_code)] // Public API for future constraint solving implementation

use crate::hir::expr::{ResolvedExpr, ResolvedExprKind, ResolvedStmt, ResolvedStmtKind};
use crate::hir::types::ResolvedType;
use crate::lexer::Span;
use crate::solver::recursive_struct_detector::detect_cycles;
use crate::solver::struct_flattener::flatten_type;
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

    /// Recursive struct type detected
    RecursiveStruct { cycle_path: Vec<String>, span: Span },
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
            ConstraintExtractorError::RecursiveStruct { cycle_path, span } => {
                write!(
                    f,
                    "Recursive struct detected at line {}, column {}: {}. Recursive structs cannot be solved.",
                    span.start.line,
                    span.start.column,
                    cycle_path.join(" → ")
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
///
/// For struct types, variables are flattened into their primitive fields
/// with qualified names (e.g., "line.start.x" for nested structs).
#[derive(Debug, Clone, PartialEq)]
pub struct Variable<'src, 'arena> {
    /// Variable name (owned to support generated names for flattened struct fields)
    pub name: String,

    /// Variable type (required for constraint solving)
    /// Copy type, so we can store it by value
    pub var_type: ResolvedType<'src, 'arena>,

    /// Optional initial value (known variables)
    pub init: Option<&'arena ResolvedExpr<'src, 'arena>>,

    /// Source span for error reporting
    pub span: Span,
}

impl<'src, 'arena> Variable<'src, 'arena> {
    /// Create a new variable
    pub fn new(
        name: &str,
        var_type: ResolvedType<'src, 'arena>,
        init: Option<&'arena ResolvedExpr<'src, 'arena>>,
        span: Span,
    ) -> Self {
        Self {
            name: name.to_string(),
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

/// Represents a conditional constraint (if-statement)
///
/// Conditional constraints express that certain constraints apply only when
/// a condition is true, and optionally different constraints apply when the
/// condition is false.
///
/// Example:
/// ```cad
/// if x > 0 {
///     y == x * 2;
/// } else {
///     y == 0;
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalConstraint<'src, 'arena> {
    /// The condition expression (must be boolean)
    pub condition: &'arena ResolvedExpr<'src, 'arena>,

    /// Constraints that apply when the condition is true
    pub then_constraints: Vec<Constraint<'src, 'arena>>,

    /// Constraints that apply when the condition is false
    pub else_constraints: Vec<Constraint<'src, 'arena>>,

    /// Source span for error reporting
    pub span: Span,
}

impl<'src, 'arena> ConditionalConstraint<'src, 'arena> {
    /// Create a new conditional constraint
    pub fn new(
        condition: &'arena ResolvedExpr<'src, 'arena>,
        then_constraints: Vec<Constraint<'src, 'arena>>,
        else_constraints: Vec<Constraint<'src, 'arena>>,
        span: Span,
    ) -> Self {
        Self {
            condition,
            then_constraints,
            else_constraints,
            span,
        }
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

    /// All unconditional constraints in the problem
    pub constraints: Vec<Constraint<'src, 'arena>>,

    /// All conditional constraints (if-statements) in the problem
    pub conditional_constraints: Vec<ConditionalConstraint<'src, 'arena>>,
}

impl<'src, 'arena> ConstraintProblem<'src, 'arena> {
    /// Create a new empty constraint problem
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
            constraints: Vec::new(),
            conditional_constraints: Vec::new(),
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

    /// Add a conditional constraint to the problem
    pub fn add_conditional_constraint(
        &mut self,
        conditional_constraint: ConditionalConstraint<'src, 'arena>,
    ) {
        self.conditional_constraints.push(conditional_constraint);
    }

    /// Get the number of variables
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// Get the number of constraints
    pub fn constraint_count(&self) -> usize {
        self.constraints.len()
    }

    /// Get the number of conditional constraints
    pub fn conditional_constraint_count(&self) -> usize {
        self.conditional_constraints.len()
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

            // Check if this is a struct type - if so, flatten it
            match var_type {
                ResolvedType::UserDefined { definition, .. } => {
                    // First, check for recursive structs
                    if let Err(cycle_err) = detect_cycles(definition) {
                        return Err(ConstraintExtractorError::RecursiveStruct {
                            cycle_path: cycle_err
                                .cycle_path
                                .into_iter()
                                .map(|s| s.to_string())
                                .collect(),
                            span: *span,
                        });
                    }

                    // Flatten the struct into primitive fields
                    let flattened_fields = flatten_type(var_def.name, *var_type);

                    // Create a variable for each flattened field
                    for field in flattened_fields {
                        let variable = Variable::new(
                            &field.full_name,
                            field.primitive_type,
                            None, // Struct fields are initialized via constraints from struct literal
                            field.span,
                        );
                        problem.add_variable(variable);
                    }

                    // If there's a struct literal initializer, extract constraints from it
                    if let Some(init_expr) = init {
                        process_struct_literal_init(var_def.name, init_expr, problem)?;
                    }

                    Ok(())
                }

                // Array types - flatten into indexed elements
                ResolvedType::Array { .. } => {
                    // Flatten the array into primitive fields
                    let flattened_fields = flatten_type(var_def.name, *var_type);

                    // Create a variable for each flattened field
                    for field in flattened_fields {
                        let variable = Variable::new(
                            &field.full_name,
                            field.primitive_type,
                            None, // Array elements are initialized via constraints
                            field.span,
                        );
                        problem.add_variable(variable);
                    }

                    // TODO: Handle array literal initializers when implemented
                    // For now, arrays must be constrained element-by-element

                    Ok(())
                }

                // For reference types, unwrap and check the inner type
                ResolvedType::Reference { inner, .. } => {
                    if let ResolvedType::UserDefined { definition, .. } = **inner {
                        // Check for recursive structs
                        if let Err(cycle_err) = detect_cycles(definition) {
                            return Err(ConstraintExtractorError::RecursiveStruct {
                                cycle_path: cycle_err
                                    .cycle_path
                                    .into_iter()
                                    .map(|s| s.to_string())
                                    .collect(),
                                span: *span,
                            });
                        }

                        // Flatten the referenced struct
                        let flattened_fields = flatten_type(var_def.name, **inner);

                        for field in flattened_fields {
                            let variable = Variable::new(
                                &field.full_name,
                                field.primitive_type,
                                None,
                                field.span,
                            );
                            problem.add_variable(variable);
                        }

                        // If there's a struct literal initializer, extract constraints from it
                        if let Some(init_expr) = init {
                            process_struct_literal_init(var_def.name, init_expr, problem)?;
                        }

                        Ok(())
                    } else {
                        // Reference to primitive type - create single variable
                        let variable = Variable::new(var_def.name, *var_type, *init, *span);
                        problem.add_variable(variable);
                        Ok(())
                    }
                }

                // Primitive types - create single variable
                _ => {
                    let variable = Variable::new(var_def.name, *var_type, *init, *span);
                    problem.add_variable(variable);
                    Ok(())
                }
            }
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

        // Handle if-statements - extract conditional constraints
        ResolvedStmtKind::If {
            condition,
            then_branch,
            else_branch,
            span,
        } => {
            // Process then branch to extract constraints
            let then_constraints = process_branch(then_branch)?;

            // Process else branch (if present) to extract constraints
            let else_constraints = if let Some(else_stmts) = else_branch {
                process_branch(else_stmts)?
            } else {
                Vec::new()
            };

            // Create a conditional constraint and add it to the problem
            let conditional_constraint =
                ConditionalConstraint::new(condition, then_constraints, else_constraints, *span);
            problem.add_conditional_constraint(conditional_constraint);
            Ok(())
        }

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
        ResolvedStmtKind::FunctionDef { .. } => {
            // Function definitions are needed for inlining but don't contribute constraints
            // They are inlined before constraint extraction, so skip them silently
            Ok(())
        }

        ResolvedStmtKind::StructDef { .. } => {
            // Struct definitions are needed for type checking but don't contribute constraints
            // Skip them silently
            Ok(())
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

        ResolvedStmtKind::FieldAssignment {
            target,
            value,
            span,
        } => {
            // Build the qualified field name from the target expression
            let qualified_name = build_qualified_field_name(target)?;

            // Find the corresponding variable and update its init value
            // This marks the variable as "known" with the assigned value
            if let Some(var) = problem
                .variables
                .iter_mut()
                .find(|v| v.name == qualified_name)
            {
                var.init = Some(*value);
                Ok(())
            } else {
                // Variable not found - this shouldn't happen if semantic analysis passed
                Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "field assignment to unknown variable".to_string(),
                    span: *span,
                    message: format!(
                        "Field assignment to '{}' but variable not found in problem",
                        qualified_name
                    ),
                })
            }
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

/// Process if-statement branches and extract constraints
///
/// This function processes the statements in an if-branch or else-branch and
/// extracts only constraint expressions. Variable declarations are not allowed
/// in conditional branches because they would create scope and initialization issues.
fn process_branch<'src, 'arena>(
    statements: &[&'arena ResolvedStmt<'src, 'arena>],
) -> Result<Vec<Constraint<'src, 'arena>>, ConstraintExtractorError> {
    let mut constraints = Vec::new();

    for stmt in statements {
        match &stmt.kind {
            // Expression statements - extract constraints
            ResolvedStmtKind::Expression { expr, span } => {
                if is_comparison_expr(expr) {
                    constraints.push(Constraint::new(expr, *span));
                } else {
                    return Err(ConstraintExtractorError::NotAConstraint { span: *span });
                }
            }

            // Block: recursively process statements
            ResolvedStmtKind::Block { statements, .. } => {
                let inner_constraints = process_branch(statements)?;
                constraints.extend(inner_constraints);
            }

            // Variable declarations are not allowed in conditional branches
            ResolvedStmtKind::Let { span, .. } => {
                return Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "let in conditional branch".to_string(),
                    span: *span,
                    message: "Variable declarations are not allowed in conditional branches"
                        .to_string(),
                });
            }

            // All other statement types are unsupported
            ResolvedStmtKind::If { span, .. } => {
                return Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "nested if".to_string(),
                    span: *span,
                    message: "Nested if-statements are not supported in constraint problems"
                        .to_string(),
                });
            }

            ResolvedStmtKind::For { span, .. } => {
                return Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "for in conditional branch".to_string(),
                    span: *span,
                    message: "Loops are not allowed in conditional branches".to_string(),
                });
            }

            ResolvedStmtKind::Return { span, .. } => {
                return Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "return in conditional branch".to_string(),
                    span: *span,
                    message: "Return statements are not allowed in conditional branches"
                        .to_string(),
                });
            }

            ResolvedStmtKind::FunctionDef { span, .. } => {
                return Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "function definition in conditional branch".to_string(),
                    span: *span,
                    message: "Function definitions are not allowed in conditional branches"
                        .to_string(),
                });
            }

            ResolvedStmtKind::StructDef { span, .. } => {
                return Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "struct definition in conditional branch".to_string(),
                    span: *span,
                    message: "Struct definitions are not allowed in conditional branches"
                        .to_string(),
                });
            }

            ResolvedStmtKind::Assignment { span, .. } => {
                return Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "assignment in conditional branch".to_string(),
                    span: *span,
                    message: "Variable reassignment is not allowed in conditional branches"
                        .to_string(),
                });
            }

            ResolvedStmtKind::FieldAssignment { span, .. } => {
                return Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "field assignment in conditional branch".to_string(),
                    span: *span,
                    message: "Field assignments are not allowed in conditional branches"
                        .to_string(),
                });
            }

            ResolvedStmtKind::With { span, .. } => {
                return Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "with in conditional branch".to_string(),
                    span: *span,
                    message: "With blocks are not allowed in conditional branches".to_string(),
                });
            }
        }
    }

    Ok(constraints)
}

/// Process a struct literal initializer and generate field constraints
///
/// This function handles struct literal initialization by generating equality
/// constraints for each field. For example:
///
/// ```cad
/// let p: Point = Point { x: 5, y: 10 };
/// ```
///
/// This generates constraints:
/// - `p.x == 5`
/// - `p.y == 10`
///
/// For nested structs like `Line { start: Point { x: 0, y: 0 }, ... }`, it
/// recursively processes nested struct literals and generates constraints with
/// fully qualified names like `line.start.x == 0`.
fn process_struct_literal_init<'src, 'arena>(
    base_name: &str,
    init_expr: &'arena ResolvedExpr<'src, 'arena>,
    problem: &mut ConstraintProblem<'src, 'arena>,
) -> Result<(), ConstraintExtractorError> {
    match &init_expr.kind {
        ResolvedExprKind::StructLit { fields, .. } => {
            // Process each field in the struct literal
            for field in fields {
                match field {
                    crate::hir::expr::ResolvedStructLitField::Field {
                        name: field_name,
                        value: field_value,
                        ..
                    } => {
                        // Build the qualified field name (e.g., "p.x", "line.start.x")
                        let qualified_name = format!("{}.{}", base_name, field_name);

                        // Check if the field value is itself a struct literal
                        if matches!(field_value.kind, ResolvedExprKind::StructLit { .. }) {
                            // Recursively process nested struct literal
                            process_struct_literal_init(&qualified_name, field_value, problem)?;
                        } else {
                            // For primitive field values, update the corresponding variable's
                            // init value. This marks the variable as "known" with its initializer.
                            // The Z3 bridge will handle this by creating an equality constraint.
                            if let Some(var) = problem
                                .variables
                                .iter_mut()
                                .find(|v| v.name == qualified_name)
                            {
                                var.init = Some(*field_value);
                            }
                        }
                    }
                    crate::hir::expr::ResolvedStructLitField::ComputedProperty { span, .. } => {
                        // Computed properties are not supported in struct literals for constraint solving
                        return Err(ConstraintExtractorError::UnsupportedStatement {
                            statement_type: "computed property in struct literal".to_string(),
                            span: *span,
                            message: "Computed properties are not supported in struct literal initialization".to_string(),
                        });
                    }
                }
            }
            Ok(())
        }
        _ => {
            // If the initializer is not a struct literal, we can't process it
            // This is not necessarily an error - the user might be initializing
            // with a variable or function call, which we don't support yet.
            Ok(())
        }
    }
}

/// Build a qualified field name from a field access expression
///
/// Recursively walks the target expression to build the full qualified name.
///
/// # Examples
/// - `p.x` → "p.x"
/// - `line.start.x` → "line.start.x"
///
/// # Errors
/// Returns an error if the target expression is not a valid field access chain.
fn build_qualified_field_name<'src, 'arena>(
    target: &'arena ResolvedExpr<'src, 'arena>,
) -> Result<String, ConstraintExtractorError> {
    match &target.kind {
        // Base case: field access directly on a variable
        ResolvedExprKind::FieldAccess {
            receiver,
            field_name,
            ..
        } => {
            // Check if receiver is a variable (base case) or another field access (recursive case)
            match &receiver.kind {
                ResolvedExprKind::Var { name, .. } => {
                    // Base case: variable.field
                    Ok(format!("{}.{}", name, field_name))
                }
                ResolvedExprKind::FieldAccess { .. } => {
                    // Recursive case: build the prefix recursively
                    let prefix = build_qualified_field_name(receiver)?;
                    Ok(format!("{}.{}", prefix, field_name))
                }
                _ => Err(ConstraintExtractorError::UnsupportedStatement {
                    statement_type: "field assignment with non-field-access receiver".to_string(),
                    span: target.span,
                    message: "Field assignment target must be a field access expression"
                        .to_string(),
                }),
            }
        }
        _ => Err(ConstraintExtractorError::UnsupportedStatement {
            statement_type: "field assignment with invalid target".to_string(),
            span: target.span,
            message: "Field assignment target must be a field access expression".to_string(),
        }),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::definitions::VarDefinition;
    use crate::hir::types::ResolvedType;
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

        let var = Variable::new("x", *ty, Some(init), test_span());
        assert!(var.is_known());
        assert!(!var.is_unknown());
    }

    #[test]
    fn test_variable_is_unknown() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let var = Variable::new("x", *ty, None, test_span());
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

        let var = Variable::new("x", *ty, None, test_span());
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
        let var1 = Variable::new("x", *ty, Some(init), test_span());
        problem.add_variable(var1);

        // Add unknown variable
        let var2 = Variable::new("y", *ty, None, test_span());
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
    fn test_if_statement_with_empty_branches() {
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
        assert!(result.is_ok());

        let problem = result.unwrap();
        // Empty branches should result in a conditional constraint with no then/else constraints
        assert_eq!(problem.conditional_constraint_count(), 1);
        assert_eq!(problem.conditional_constraints[0].then_constraints.len(), 0);
        assert_eq!(problem.conditional_constraints[0].else_constraints.len(), 0);
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
    fn test_struct_def_allowed() {
        // Struct definitions should be allowed (silently skipped) in constraint problems
        let arena = Bump::new();
        let struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
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

        // Struct definitions should be accepted (they define types but don't contribute constraints)
        let result = extract_constraints(&[stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.variables.len(), 0); // No variables from struct definition
        assert_eq!(problem.constraints.len(), 0); // No constraints from struct definition
    }

    #[test]
    fn test_function_def_skipped() {
        // Function definitions are skipped during constraint extraction
        // because they are inlined before this stage
        let arena = Bump::new();
        let func_def = arena.alloc(crate::hir::definitions::FunctionDefinition {
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
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.constraints.len(), 0); // No constraints from function definition
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

    #[test]
    fn test_if_statement_with_then_branch_only() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Condition: true
        let condition = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);

        // Then branch: x == 10
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let constraint_expr = make_expr(&arena, ResolvedExprKind::Eq { lhs, rhs }, bool_ty);
        let then_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint_expr,
                span: test_span(),
            },
        );

        let if_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::If {
                condition,
                then_branch: vec![then_stmt],
                else_branch: None,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[if_stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.conditional_constraint_count(), 1);
        assert_eq!(problem.conditional_constraints[0].then_constraints.len(), 1);
        assert_eq!(problem.conditional_constraints[0].else_constraints.len(), 0);
    }

    #[test]
    fn test_if_statement_with_else_branch() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Condition: x > 0
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let condition = make_expr(&arena, ResolvedExprKind::Gt { lhs, rhs }, bool_ty);

        // Then branch: y == 10
        let then_lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty);
        let then_rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let then_expr = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: then_lhs,
                rhs: then_rhs,
            },
            bool_ty,
        );
        let then_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: then_expr,
                span: test_span(),
            },
        );

        // Else branch: y == 0
        let else_lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty);
        let else_rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let else_expr = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: else_lhs,
                rhs: else_rhs,
            },
            bool_ty,
        );
        let else_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: else_expr,
                span: test_span(),
            },
        );

        let if_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::If {
                condition,
                then_branch: vec![then_stmt],
                else_branch: Some(vec![else_stmt]),
                span: test_span(),
            },
        );

        let result = extract_constraints(&[if_stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.conditional_constraint_count(), 1);
        assert_eq!(problem.conditional_constraints[0].then_constraints.len(), 1);
        assert_eq!(problem.conditional_constraints[0].else_constraints.len(), 1);
    }

    #[test]
    fn test_if_statement_with_multiple_constraints() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Condition
        let condition = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);

        // Then branch: x == 10 and y == 20
        let constraint1 = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty),
                rhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty),
            },
            bool_ty,
        );
        let constraint2 = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty),
                rhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty),
            },
            bool_ty,
        );

        let then_stmt1 = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint1,
                span: test_span(),
            },
        );
        let then_stmt2 = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint2,
                span: test_span(),
            },
        );

        let if_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::If {
                condition,
                then_branch: vec![then_stmt1, then_stmt2],
                else_branch: None,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[if_stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.conditional_constraint_count(), 1);
        assert_eq!(problem.conditional_constraints[0].then_constraints.len(), 2);
    }

    #[test]
    fn test_error_variable_declaration_in_then_branch() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Condition
        let condition = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);

        // Then branch with variable declaration (not allowed)
        let var_def = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 1,
            span: test_span(),
        });
        let then_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def,
                init: None,
                span: test_span(),
            },
        );

        let if_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::If {
                condition,
                then_branch: vec![then_stmt],
                else_branch: None,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[if_stmt]);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            errors[0],
            ConstraintExtractorError::UnsupportedStatement { ref statement_type, .. }
            if statement_type == "let in conditional branch"
        );
    }

    #[test]
    fn test_error_variable_declaration_in_else_branch() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Condition
        let condition = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);

        // Then branch with valid constraint
        let constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty),
                rhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty),
            },
            bool_ty,
        );
        let then_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint,
                span: test_span(),
            },
        );

        // Else branch with variable declaration (not allowed)
        let var_def = arena.alloc(VarDefinition {
            name: "y",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 1,
            span: test_span(),
        });
        let else_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("y", test_span())],
                var_def,
                init: None,
                span: test_span(),
            },
        );

        let if_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::If {
                condition,
                then_branch: vec![then_stmt],
                else_branch: Some(vec![else_stmt]),
                span: test_span(),
            },
        );

        let result = extract_constraints(&[if_stmt]);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            errors[0],
            ConstraintExtractorError::UnsupportedStatement { ref statement_type, .. }
            if statement_type == "let in conditional branch"
        );
    }

    #[test]
    fn test_error_nested_if_statement() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // Outer condition
        let outer_condition = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);

        // Inner if-statement (nested, not allowed)
        let inner_condition =
            make_expr(&arena, ResolvedExprKind::BoolLit { value: false }, bool_ty);
        let inner_if = make_stmt(
            &arena,
            ResolvedStmtKind::If {
                condition: inner_condition,
                then_branch: vec![],
                else_branch: None,
                span: test_span(),
            },
        );

        let outer_if = make_stmt(
            &arena,
            ResolvedStmtKind::If {
                condition: outer_condition,
                then_branch: vec![inner_if],
                else_branch: None,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[outer_if]);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            errors[0],
            ConstraintExtractorError::UnsupportedStatement { ref statement_type, .. }
            if statement_type == "nested if"
        );
    }

    #[test]
    fn test_if_statement_with_block_in_branch() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Condition
        let condition = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);

        // Then branch with block containing constraint
        let constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty),
                rhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty),
            },
            bool_ty,
        );
        let inner_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Expression {
                expr: constraint,
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

        let if_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::If {
                condition,
                then_branch: vec![block_stmt],
                else_branch: None,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[if_stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        assert_eq!(problem.conditional_constraint_count(), 1);
        assert_eq!(problem.conditional_constraints[0].then_constraints.len(), 1);
    }

    #[test]
    fn test_conditional_constraint_helper_methods() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let mut problem = ConstraintProblem::new();
        assert_eq!(problem.conditional_constraint_count(), 0);

        let condition = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);
        let constraint_expr = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty),
                rhs: make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty),
            },
            bool_ty,
        );

        let conditional = ConditionalConstraint::new(
            condition,
            vec![Constraint::new(constraint_expr, test_span())],
            vec![],
            test_span(),
        );

        problem.add_conditional_constraint(conditional);
        assert_eq!(problem.conditional_constraint_count(), 1);
    }

    // ============================================================================
    // Struct Literal Tests (Step 5)
    // ============================================================================

    #[test]
    fn test_struct_literal_simple() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create Point struct definition
        let x_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "x",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let y_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "y",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![x_field, y_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def,
            span: test_span(),
        });

        // Create struct literal: Point { x: 5, y: 10 }
        let x_value = make_expr(&arena, ResolvedExprKind::IntLit { value: 5 }, int_ty);
        let y_value = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);

        let struct_lit = make_expr(
            &arena,
            ResolvedExprKind::StructLit {
                name: "Point",
                fields: vec![
                    crate::hir::expr::ResolvedStructLitField::Field {
                        name: "x",
                        value: x_value,
                        field_def: x_field,
                        span: test_span(),
                    },
                    crate::hir::expr::ResolvedStructLitField::Field {
                        name: "y",
                        value: y_value,
                        field_def: y_field,
                        span: test_span(),
                    },
                ],
            },
            point_ty,
        );

        // Create let statement: let p: Point = Point { x: 5, y: 10 };
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: Some(struct_lit),
            scope_level: 0,
            span: test_span(),
        });

        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("p", test_span())],
                var_def,
                init: Some(struct_lit),
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        // Should have 2 variables: p.x and p.y
        assert_eq!(problem.variable_count(), 2);
        // Both should be known (have initializers)
        assert_eq!(problem.known_variables().len(), 2);

        // Check variable names and values
        let p_x = problem.variables.iter().find(|v| v.name == "p.x").unwrap();
        assert!(p_x.is_known());
        assert_matches!(
            p_x.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 5 }
        );

        let p_y = problem.variables.iter().find(|v| v.name == "p.y").unwrap();
        assert!(p_y.is_known());
        assert_matches!(
            p_y.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 10 }
        );
    }

    #[test]
    fn test_struct_literal_without_init() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create Point struct definition
        let x_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "x",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let y_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "y",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![x_field, y_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def,
            span: test_span(),
        });

        // Create let statement without init: let p: Point;
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("p", test_span())],
                var_def,
                init: None,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        // Should have 2 variables: p.x and p.y
        assert_eq!(problem.variable_count(), 2);
        // Both should be unknown (no initializers)
        assert_eq!(problem.unknown_variables().len(), 2);
    }

    #[test]
    fn test_struct_literal_nested() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create Point struct definition
        let x_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "x",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let y_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "y",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let point_struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![x_field, y_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: point_struct_def,
            span: test_span(),
        });

        // Create Line struct definition with two Point fields
        let start_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "start",
            test_span(),
            *point_ty,
            test_span(),
        ));
        let end_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "end",
            test_span(),
            *point_ty,
            test_span(),
        ));
        let line_struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Line",
            name_span: test_span(),
            fields: vec![start_field, end_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let line_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Line",
            definition: line_struct_def,
            span: test_span(),
        });

        // Create nested struct literal: Line { start: Point { x: 0, y: 0 }, end: Point { x: 10, y: 10 } }
        let start_x = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let start_y = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let start_point = make_expr(
            &arena,
            ResolvedExprKind::StructLit {
                name: "Point",
                fields: vec![
                    crate::hir::expr::ResolvedStructLitField::Field {
                        name: "x",
                        value: start_x,
                        field_def: x_field,
                        span: test_span(),
                    },
                    crate::hir::expr::ResolvedStructLitField::Field {
                        name: "y",
                        value: start_y,
                        field_def: y_field,
                        span: test_span(),
                    },
                ],
            },
            point_ty,
        );

        let end_x = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let end_y = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let end_point = make_expr(
            &arena,
            ResolvedExprKind::StructLit {
                name: "Point",
                fields: vec![
                    crate::hir::expr::ResolvedStructLitField::Field {
                        name: "x",
                        value: end_x,
                        field_def: x_field,
                        span: test_span(),
                    },
                    crate::hir::expr::ResolvedStructLitField::Field {
                        name: "y",
                        value: end_y,
                        field_def: y_field,
                        span: test_span(),
                    },
                ],
            },
            point_ty,
        );

        let line_lit = make_expr(
            &arena,
            ResolvedExprKind::StructLit {
                name: "Line",
                fields: vec![
                    crate::hir::expr::ResolvedStructLitField::Field {
                        name: "start",
                        value: start_point,
                        field_def: start_field,
                        span: test_span(),
                    },
                    crate::hir::expr::ResolvedStructLitField::Field {
                        name: "end",
                        value: end_point,
                        field_def: end_field,
                        span: test_span(),
                    },
                ],
            },
            line_ty,
        );

        // Create let statement: let line: Line = Line { ... };
        let var_def = arena.alloc(VarDefinition {
            name: "line",
            name_span: test_span(),
            var_type: Some(*line_ty),
            init: Some(line_lit),
            scope_level: 0,
            span: test_span(),
        });

        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("line", test_span())],
                var_def,
                init: Some(line_lit),
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        // Should have 4 variables: line.start.x, line.start.y, line.end.x, line.end.y
        assert_eq!(problem.variable_count(), 4);
        // All should be known
        assert_eq!(problem.known_variables().len(), 4);

        // Check variable names and values
        let line_start_x = problem
            .variables
            .iter()
            .find(|v| v.name == "line.start.x")
            .unwrap();
        assert_matches!(
            line_start_x.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 0 }
        );

        let line_end_x = problem
            .variables
            .iter()
            .find(|v| v.name == "line.end.x")
            .unwrap();
        assert_matches!(
            line_end_x.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 10 }
        );
    }

    #[test]
    fn test_struct_literal_partial_init() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create Point struct definition
        let x_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "x",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let y_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "y",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![x_field, y_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def,
            span: test_span(),
        });

        // Create struct literal with only x field: Point { x: 5 }
        let x_value = make_expr(&arena, ResolvedExprKind::IntLit { value: 5 }, int_ty);

        let struct_lit = make_expr(
            &arena,
            ResolvedExprKind::StructLit {
                name: "Point",
                fields: vec![crate::hir::expr::ResolvedStructLitField::Field {
                    name: "x",
                    value: x_value,
                    field_def: x_field,
                    span: test_span(),
                }],
            },
            point_ty,
        );

        // Create let statement: let p: Point = Point { x: 5 };
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: Some(struct_lit),
            scope_level: 0,
            span: test_span(),
        });

        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("p", test_span())],
                var_def,
                init: Some(struct_lit),
                span: test_span(),
            },
        );

        let result = extract_constraints(&[stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        // Should have 2 variables: p.x and p.y
        assert_eq!(problem.variable_count(), 2);
        // Only p.x should be known, p.y should be unknown
        assert_eq!(problem.known_variables().len(), 1);
        assert_eq!(problem.unknown_variables().len(), 1);

        let p_x = problem.variables.iter().find(|v| v.name == "p.x").unwrap();
        assert!(p_x.is_known());
        assert_matches!(
            p_x.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 5 }
        );

        let p_y = problem.variables.iter().find(|v| v.name == "p.y").unwrap();
        assert!(p_y.is_unknown());
    }

    #[test]
    fn test_struct_literal_computed_property_error() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create Point struct definition
        let x_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "x",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![x_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def,
            span: test_span(),
        });

        // Create a dummy function definition for computed property
        let method_def = arena.alloc(crate::hir::definitions::FunctionDefinition {
            name: "length",
            name_span: test_span(),
            params: vec![],
            return_type: *int_ty,
            body: vec![],
            parent_struct: None,
            span: test_span(),
        });

        // Create struct literal with computed property: Point { length() = 5 }
        let value = make_expr(&arena, ResolvedExprKind::IntLit { value: 5 }, int_ty);

        let struct_lit = make_expr(
            &arena,
            ResolvedExprKind::StructLit {
                name: "Point",
                fields: vec![crate::hir::expr::ResolvedStructLitField::ComputedProperty {
                    name: "length",
                    value,
                    method_def,
                    span: test_span(),
                }],
            },
            point_ty,
        );

        // Create let statement: let p: Point = Point { length() = 5 };
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: Some(struct_lit),
            scope_level: 0,
            span: test_span(),
        });

        let stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("p", test_span())],
                var_def,
                init: Some(struct_lit),
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
            if statement_type == "computed property in struct literal"
        );
    }

    // ============================================================================
    // Field Assignment Tests (Step 6)
    // ============================================================================

    #[test]
    fn test_field_assignment_simple() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create Point struct definition
        let x_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "x",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let y_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "y",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![x_field, y_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def,
            span: test_span(),
        });

        // Create let statement: let p: Point;
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let let_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("p", test_span())],
                var_def,
                init: None,
                span: test_span(),
            },
        );

        // Create field assignment: p.x = 5;
        let p_var = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "p",
                definition: var_def,
            },
            point_ty,
        );
        let p_x = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: p_var,
                field_name: "x",
                field: x_field,
            },
            int_ty,
        );
        let value = make_expr(&arena, ResolvedExprKind::IntLit { value: 5 }, int_ty);
        let assignment_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::FieldAssignment {
                target: p_x,
                value,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[let_stmt, assignment_stmt]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        // Should have 2 variables: p.x and p.y
        assert_eq!(problem.variable_count(), 2);
        // p.x should be known (assigned), p.y should be unknown
        assert_eq!(problem.known_variables().len(), 1);
        assert_eq!(problem.unknown_variables().len(), 1);

        let p_x_var = problem.variables.iter().find(|v| v.name == "p.x").unwrap();
        assert!(p_x_var.is_known());
        assert_matches!(
            p_x_var.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 5 }
        );

        let p_y_var = problem.variables.iter().find(|v| v.name == "p.y").unwrap();
        assert!(p_y_var.is_unknown());
    }

    #[test]
    fn test_field_assignment_multiple() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create Point struct definition
        let x_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "x",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let y_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "y",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![x_field, y_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def,
            span: test_span(),
        });

        // Create let statement: let p: Point;
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let let_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("p", test_span())],
                var_def,
                init: None,
                span: test_span(),
            },
        );

        // Create field assignment: p.x = 10;
        let p_var1 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "p",
                definition: var_def,
            },
            point_ty,
        );
        let p_x = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: p_var1,
                field_name: "x",
                field: x_field,
            },
            int_ty,
        );
        let value_x = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let assignment_x = make_stmt(
            &arena,
            ResolvedStmtKind::FieldAssignment {
                target: p_x,
                value: value_x,
                span: test_span(),
            },
        );

        // Create field assignment: p.y = 20;
        let p_var2 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "p",
                definition: var_def,
            },
            point_ty,
        );
        let p_y = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: p_var2,
                field_name: "y",
                field: y_field,
            },
            int_ty,
        );
        let value_y = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let assignment_y = make_stmt(
            &arena,
            ResolvedStmtKind::FieldAssignment {
                target: p_y,
                value: value_y,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[let_stmt, assignment_x, assignment_y]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        // Both fields should be known now
        assert_eq!(problem.variable_count(), 2);
        assert_eq!(problem.known_variables().len(), 2);

        let p_x_var = problem.variables.iter().find(|v| v.name == "p.x").unwrap();
        assert_matches!(
            p_x_var.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 10 }
        );

        let p_y_var = problem.variables.iter().find(|v| v.name == "p.y").unwrap();
        assert_matches!(
            p_y_var.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 20 }
        );
    }

    #[test]
    fn test_field_assignment_nested() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create Point struct definition
        let x_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "x",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let y_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "y",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let point_struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![x_field, y_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: point_struct_def,
            span: test_span(),
        });

        // Create Line struct definition with two Point fields
        let start_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "start",
            test_span(),
            *point_ty,
            test_span(),
        ));
        let end_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "end",
            test_span(),
            *point_ty,
            test_span(),
        ));
        let line_struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Line",
            name_span: test_span(),
            fields: vec![start_field, end_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let line_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Line",
            definition: line_struct_def,
            span: test_span(),
        });

        // Create let statement: let line: Line;
        let var_def = arena.alloc(VarDefinition {
            name: "line",
            name_span: test_span(),
            var_type: Some(*line_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let let_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("line", test_span())],
                var_def,
                init: None,
                span: test_span(),
            },
        );

        // Create nested field assignment: line.start.x = 42;
        let line_var = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "line",
                definition: var_def,
            },
            line_ty,
        );
        let line_start = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: line_var,
                field_name: "start",
                field: start_field,
            },
            point_ty,
        );
        let line_start_x = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: line_start,
                field_name: "x",
                field: x_field,
            },
            int_ty,
        );
        let value = make_expr(&arena, ResolvedExprKind::IntLit { value: 42 }, int_ty);
        let assignment = make_stmt(
            &arena,
            ResolvedStmtKind::FieldAssignment {
                target: line_start_x,
                value,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[let_stmt, assignment]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        // Should have 4 variables: line.start.x, line.start.y, line.end.x, line.end.y
        assert_eq!(problem.variable_count(), 4);
        // Only line.start.x should be known
        assert_eq!(problem.known_variables().len(), 1);

        let line_start_x_var = problem
            .variables
            .iter()
            .find(|v| v.name == "line.start.x")
            .unwrap();
        assert_matches!(
            line_start_x_var.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 42 }
        );
    }

    #[test]
    fn test_field_assignment_mixed_with_struct_literal() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create Point struct definition
        let x_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "x",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let y_field = arena.alloc(crate::hir::definitions::FieldDefinition::new(
            "y",
            test_span(),
            *int_ty,
            test_span(),
        ));
        let struct_def = arena.alloc(crate::hir::definitions::StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![x_field, y_field],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: struct_def,
            span: test_span(),
        });

        // Create struct literal with only x field: Point { x: 5 }
        let x_value = make_expr(&arena, ResolvedExprKind::IntLit { value: 5 }, int_ty);

        let struct_lit = make_expr(
            &arena,
            ResolvedExprKind::StructLit {
                name: "Point",
                fields: vec![crate::hir::expr::ResolvedStructLitField::Field {
                    name: "x",
                    value: x_value,
                    field_def: x_field,
                    span: test_span(),
                }],
            },
            point_ty,
        );

        // Create let statement: let p: Point = Point { x: 5 };
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: Some(struct_lit),
            scope_level: 0,
            span: test_span(),
        });

        let let_stmt = make_stmt(
            &arena,
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("p", test_span())],
                var_def,
                init: Some(struct_lit),
                span: test_span(),
            },
        );

        // Create field assignment to the missing field: p.y = 10;
        let p_var = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "p",
                definition: var_def,
            },
            point_ty,
        );
        let p_y = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: p_var,
                field_name: "y",
                field: y_field,
            },
            int_ty,
        );
        let value_y = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let assignment = make_stmt(
            &arena,
            ResolvedStmtKind::FieldAssignment {
                target: p_y,
                value: value_y,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[let_stmt, assignment]);
        assert!(result.is_ok());

        let problem = result.unwrap();
        // Both fields should be known now
        assert_eq!(problem.variable_count(), 2);
        assert_eq!(problem.known_variables().len(), 2);

        let p_x_var = problem.variables.iter().find(|v| v.name == "p.x").unwrap();
        assert_matches!(
            p_x_var.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 5 }
        );

        let p_y_var = problem.variables.iter().find(|v| v.name == "p.y").unwrap();
        assert_matches!(
            p_y_var.init.unwrap().kind,
            ResolvedExprKind::IntLit { value: 10 }
        );
    }

    #[test]
    fn test_field_assignment_to_unknown_variable_error() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: arena.alloc(crate::hir::definitions::StructDefinition {
                name: "Point",
                name_span: test_span(),
                fields: vec![],
                methods: vec![],
                container_field: None,
                span: test_span(),
            }),
            span: test_span(),
        });

        // Create field assignment without declaring the variable first: p.x = 5;
        let dummy_var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let p_var = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "p",
                definition: dummy_var_def,
            },
            point_ty,
        );
        let p_x = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: p_var,
                field_name: "x",
                field: arena.alloc(crate::hir::definitions::FieldDefinition::new(
                    "x",
                    test_span(),
                    *int_ty,
                    test_span(),
                )),
            },
            int_ty,
        );
        let value = make_expr(&arena, ResolvedExprKind::IntLit { value: 5 }, int_ty);
        let assignment = make_stmt(
            &arena,
            ResolvedStmtKind::FieldAssignment {
                target: p_x,
                value,
                span: test_span(),
            },
        );

        let result = extract_constraints(&[assignment]);
        assert!(result.is_err());

        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_matches!(
            errors[0],
            ConstraintExtractorError::UnsupportedStatement { ref statement_type, .. }
            if statement_type == "field assignment to unknown variable"
        );
    }
}
