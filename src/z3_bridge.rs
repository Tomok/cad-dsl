//! Z3 Bridge for Constraint Solving
//!
//! This module translates HIR expressions to Z3 assertions for constraint solving.
//! It provides a bridge between the CAD-DSL's type-checked HIR and the Z3 SMT solver.
//!
//! # Purpose
//!
//! The Z3 bridge performs the following tasks:
//! - Translates `ResolvedType` to Z3 sorts (Int, Real, Bool)
//! - Translates `ResolvedExpr` to Z3 AST nodes
//! - Creates Z3 constants for variables
//! - Builds Z3 assertions from constraints
//! - Provides a configured Z3 solver ready for solving
//!
//! # Workflow
//!
//! 1. Create a `Z3Bridge` instance
//! 2. Add variables from `ConstraintProblem`
//! 3. Add constraint expressions
//! 4. Get a Z3 solver with all assertions added
//! 5. Call `solver.check()` to solve
//! 6. Extract solution from the model
//!
//! # Type Translation
//!
//! - `ResolvedType::I32` → Z3 Int sort
//! - `ResolvedType::F64` → Z3 Real sort
//! - `ResolvedType::Bool` → Z3 Bool sort
//!
//! # Expression Translation
//!
//! - Literals: `IntLit`, `FloatLit`, `BoolLit` → Z3 constants
//! - Variables: `Var` → Z3 symbolic constants
//! - Arithmetic: `Add`, `Sub`, `Mul`, `Div` → Z3 arithmetic operations
//! - Comparisons: `Eq`, `NotEq`, `Lt`, `Gt`, `LtEq`, `GtEq` → Z3 comparison operations
//!
//! # Example
//!
//! ```ignore
//! let problem = extract_constraints(&statements)?;
//! let mut bridge = Z3Bridge::new();
//! bridge.add_problem(&problem)?;
//! let solver = bridge.build_solver();
//!
//! if solver.check() == z3::SatResult::Sat {
//!     let model = solver.get_model().unwrap();
//!     // Extract variable values from model
//! }
//! ```

#![allow(dead_code)] // Public API for future constraint solving implementation

use crate::constraint_extractor::{ConstraintProblem, Variable};
use crate::hir::expr::{ResolvedExpr, ResolvedExprKind};
use crate::hir::types::ResolvedType;
use crate::lexer::Span;
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during Z3 translation
#[derive(Debug, Clone, PartialEq)]
pub enum Z3BridgeError {
    /// Unsupported expression type for Z3 translation
    UnsupportedExpression {
        expr_type: String,
        span: Span,
        message: String,
    },

    /// Unsupported type for Z3 translation
    UnsupportedType {
        type_name: String,
        span: Span,
        message: String,
    },

    /// Variable not found in the Z3 context
    VariableNotFound { var_name: String, span: Span },

    /// Type mismatch during translation
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },
}

impl fmt::Display for Z3BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Z3BridgeError::UnsupportedExpression {
                expr_type,
                span,
                message,
            } => {
                write!(
                    f,
                    "Unsupported expression '{}' at line {}, column {}: {}",
                    expr_type, span.start.line, span.start.column, message
                )
            }
            Z3BridgeError::UnsupportedType {
                type_name,
                span,
                message,
            } => {
                write!(
                    f,
                    "Unsupported type '{}' at line {}, column {}: {}",
                    type_name, span.start.line, span.start.column, message
                )
            }
            Z3BridgeError::VariableNotFound { var_name, span } => {
                write!(
                    f,
                    "Variable '{}' not found at line {}, column {}",
                    var_name, span.start.line, span.start.column
                )
            }
            Z3BridgeError::TypeMismatch {
                expected,
                found,
                span,
            } => {
                write!(
                    f,
                    "Type mismatch at line {}, column {}: expected {}, found {}",
                    span.start.line, span.start.column, expected, found
                )
            }
        }
    }
}

impl std::error::Error for Z3BridgeError {}

// ============================================================================
// Z3 AST Wrapper
// ============================================================================

/// Wrapper for Z3 AST nodes to handle different types uniformly
#[derive(Debug, Clone)]
pub enum Z3Ast {
    /// Integer AST
    Int(z3::ast::Int),
    /// Real number AST
    Real(z3::ast::Real),
    /// Boolean AST
    Bool(z3::ast::Bool),
}

impl Z3Ast {
    /// Convert to Int, returning an error if not an Int
    fn as_int(&self, span: Span) -> Result<&z3::ast::Int, Z3BridgeError> {
        match self {
            Z3Ast::Int(i) => Ok(i),
            Z3Ast::Real(_) => Err(Z3BridgeError::TypeMismatch {
                expected: "Int".to_string(),
                found: "Real".to_string(),
                span,
            }),
            Z3Ast::Bool(_) => Err(Z3BridgeError::TypeMismatch {
                expected: "Int".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    /// Convert to Real, returning an error if not a Real
    fn as_real(&self, span: Span) -> Result<&z3::ast::Real, Z3BridgeError> {
        match self {
            Z3Ast::Real(r) => Ok(r),
            Z3Ast::Int(_) => Err(Z3BridgeError::TypeMismatch {
                expected: "Real".to_string(),
                found: "Int".to_string(),
                span,
            }),
            Z3Ast::Bool(_) => Err(Z3BridgeError::TypeMismatch {
                expected: "Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    /// Convert to Bool, returning an error if not a Bool
    fn as_bool(&self, span: Span) -> Result<&z3::ast::Bool, Z3BridgeError> {
        match self {
            Z3Ast::Bool(b) => Ok(b),
            Z3Ast::Int(_) => Err(Z3BridgeError::TypeMismatch {
                expected: "Bool".to_string(),
                found: "Int".to_string(),
                span,
            }),
            Z3Ast::Real(_) => Err(Z3BridgeError::TypeMismatch {
                expected: "Bool".to_string(),
                found: "Real".to_string(),
                span,
            }),
        }
    }

    /// Try to convert to Int, coercing Real to Int if possible
    fn to_int(&self, span: Span) -> Result<z3::ast::Int, Z3BridgeError> {
        match self {
            Z3Ast::Int(i) => Ok(i.clone()),
            Z3Ast::Real(r) => Ok(r.to_int()),
            Z3Ast::Bool(_) => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    /// Try to convert to Real, coercing Int to Real if needed
    fn to_real(&self, span: Span) -> Result<z3::ast::Real, Z3BridgeError> {
        match self {
            Z3Ast::Real(r) => Ok(r.clone()),
            Z3Ast::Int(i) => Ok(i.to_real()),
            Z3Ast::Bool(_) => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }
}

// ============================================================================
// Z3 Bridge
// ============================================================================

/// Bridge between HIR and Z3 constraint solver
///
/// Maintains a mapping of variable names to Z3 AST nodes and provides
/// methods to translate HIR expressions to Z3 assertions.
pub struct Z3Bridge<'src, 'arena> {
    /// Map of variable names to their Z3 AST representations
    variables: HashMap<&'src str, Z3Ast>,
    /// The Z3 solver
    solver: z3::Solver,
    /// Phantom data to maintain the 'arena lifetime
    _phantom: PhantomData<&'arena ()>,
}

impl<'src, 'arena> Z3Bridge<'src, 'arena> {
    /// Create a new Z3 bridge
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            solver: z3::Solver::new(),
            _phantom: PhantomData,
        }
    }

    /// Add a variable to the Z3 context
    ///
    /// Creates a Z3 constant for the variable based on its type.
    /// If the variable has an initializer, adds an assertion that the
    /// variable equals its initial value.
    pub fn add_variable(&mut self, variable: &Variable<'src, 'arena>) -> Result<(), Z3BridgeError> {
        // Create a Z3 constant based on the variable's type
        let z3_var = self.create_z3_constant(variable.name, variable.var_type)?;

        // Store the variable
        self.variables.insert(variable.name, z3_var.clone());

        // If the variable has an initializer, add an assertion
        if let Some(init_expr) = variable.init {
            let init_z3 = self.translate_expr(init_expr)?;
            let assertion = self.create_equality(&z3_var, &init_z3, variable.span)?;
            self.solver.assert(&assertion);
        }

        Ok(())
    }

    /// Add a constraint to the Z3 solver
    ///
    /// Translates the constraint expression to a Z3 assertion and adds it
    /// to the solver.
    pub fn add_constraint(
        &mut self,
        expr: &'arena ResolvedExpr<'src, 'arena>,
    ) -> Result<(), Z3BridgeError> {
        let z3_expr = self.translate_expr(expr)?;
        let bool_expr = z3_expr.as_bool(expr.span)?;
        self.solver.assert(bool_expr);
        Ok(())
    }

    /// Add all variables and constraints from a ConstraintProblem
    pub fn add_problem(
        &mut self,
        problem: &ConstraintProblem<'src, 'arena>,
    ) -> Result<(), Z3BridgeError> {
        // Add all variables
        for var in &problem.variables {
            self.add_variable(var)?;
        }

        // Add all constraints
        for constraint in &problem.constraints {
            self.add_constraint(constraint.expr)?;
        }

        Ok(())
    }

    /// Get the Z3 solver with all assertions added
    pub fn solver(&self) -> &z3::Solver {
        &self.solver
    }

    /// Get the variables map for solution formatting
    pub fn variables(&self) -> &HashMap<&'src str, Z3Ast> {
        &self.variables
    }

    /// Create a Z3 constant based on a ResolvedType
    fn create_z3_constant(
        &self,
        name: &'src str,
        ty: &'arena ResolvedType<'src, 'arena>,
    ) -> Result<Z3Ast, Z3BridgeError> {
        match ty {
            ResolvedType::I32 { .. } => Ok(Z3Ast::Int(z3::ast::Int::new_const(name))),
            ResolvedType::F64 { .. } => Ok(Z3Ast::Real(z3::ast::Real::new_const(name))),
            ResolvedType::Bool { .. } => Ok(Z3Ast::Bool(z3::ast::Bool::new_const(name))),
            ResolvedType::Real { span: _ } => Ok(Z3Ast::Real(z3::ast::Real::new_const(name))),
            ResolvedType::Algebraic { span: _ } => {
                // Treat algebraic as Real for now
                Ok(Z3Ast::Real(z3::ast::Real::new_const(name)))
            }
            ResolvedType::Reference { span, .. } => Err(Z3BridgeError::UnsupportedType {
                type_name: "Reference".to_string(),
                span: *span,
                message: "Reference types are not supported in constraint solving".to_string(),
            }),
            ResolvedType::UserDefined { name, span, .. } => Err(Z3BridgeError::UnsupportedType {
                type_name: name.to_string(),
                span: *span,
                message: "User-defined types are not supported in constraint solving".to_string(),
            }),
        }
    }

    /// Translate a HIR expression to a Z3 AST
    fn translate_expr(
        &self,
        expr: &'arena ResolvedExpr<'src, 'arena>,
    ) -> Result<Z3Ast, Z3BridgeError> {
        match &expr.kind {
            // Literals
            ResolvedExprKind::IntLit { value } => {
                Ok(Z3Ast::Int(z3::ast::Int::from_i64(*value as i64)))
            }
            ResolvedExprKind::FloatLit { value } => {
                // Convert f64 to a rational representation for Z3 Real
                // We approximate the float as a fraction: value * 1000000 / 1000000
                // This gives us 6 decimal places of precision
                let numerator = (*value * 1_000_000.0).round() as i64;
                let denominator = 1_000_000i64;
                Ok(Z3Ast::Real(
                    z3::ast::Real::from_rational_str(
                        &numerator.to_string(),
                        &denominator.to_string(),
                    )
                    .expect("Failed to create Z3 Real from f64"),
                ))
            }
            ResolvedExprKind::BoolLit { value } => {
                Ok(Z3Ast::Bool(z3::ast::Bool::from_bool(*value)))
            }

            // Variables
            ResolvedExprKind::Var { name, .. } => {
                self.variables
                    .get(name)
                    .cloned()
                    .ok_or_else(|| Z3BridgeError::VariableNotFound {
                        var_name: name.to_string(),
                        span: expr.span,
                    })
            }

            // Arithmetic operations
            ResolvedExprKind::Add { lhs, rhs } => {
                let lhs_z3 = self.translate_expr(lhs)?;
                let rhs_z3 = self.translate_expr(rhs)?;
                self.add_operation(&lhs_z3, &rhs_z3, expr.span)
            }
            ResolvedExprKind::Sub { lhs, rhs } => {
                let lhs_z3 = self.translate_expr(lhs)?;
                let rhs_z3 = self.translate_expr(rhs)?;
                self.sub_operation(&lhs_z3, &rhs_z3, expr.span)
            }
            ResolvedExprKind::Mul { lhs, rhs } => {
                let lhs_z3 = self.translate_expr(lhs)?;
                let rhs_z3 = self.translate_expr(rhs)?;
                self.mul_operation(&lhs_z3, &rhs_z3, expr.span)
            }
            ResolvedExprKind::Div { lhs, rhs } => {
                let lhs_z3 = self.translate_expr(lhs)?;
                let rhs_z3 = self.translate_expr(rhs)?;
                self.div_operation(&lhs_z3, &rhs_z3, expr.span)
            }

            // Comparison operations
            ResolvedExprKind::Eq { lhs, rhs } => {
                let lhs_z3 = self.translate_expr(lhs)?;
                let rhs_z3 = self.translate_expr(rhs)?;
                self.eq_operation(&lhs_z3, &rhs_z3, expr.span)
            }
            ResolvedExprKind::NotEq { lhs, rhs } => {
                let lhs_z3 = self.translate_expr(lhs)?;
                let rhs_z3 = self.translate_expr(rhs)?;
                let eq = self.eq_operation(&lhs_z3, &rhs_z3, expr.span)?;
                Ok(Z3Ast::Bool(eq.as_bool(expr.span)?.not()))
            }
            ResolvedExprKind::Lt { lhs, rhs } => {
                let lhs_z3 = self.translate_expr(lhs)?;
                let rhs_z3 = self.translate_expr(rhs)?;
                self.lt_operation(&lhs_z3, &rhs_z3, expr.span)
            }
            ResolvedExprKind::Gt { lhs, rhs } => {
                let lhs_z3 = self.translate_expr(lhs)?;
                let rhs_z3 = self.translate_expr(rhs)?;
                self.gt_operation(&lhs_z3, &rhs_z3, expr.span)
            }
            ResolvedExprKind::LtEq { lhs, rhs } => {
                let lhs_z3 = self.translate_expr(lhs)?;
                let rhs_z3 = self.translate_expr(rhs)?;
                self.le_operation(&lhs_z3, &rhs_z3, expr.span)
            }
            ResolvedExprKind::GtEq { lhs, rhs } => {
                let lhs_z3 = self.translate_expr(lhs)?;
                let rhs_z3 = self.translate_expr(rhs)?;
                self.ge_operation(&lhs_z3, &rhs_z3, expr.span)
            }

            // Unary operations
            ResolvedExprKind::Neg { inner } => {
                let inner_z3 = self.translate_expr(inner)?;
                self.neg_operation(&inner_z3, expr.span)
            }

            // Unsupported operations
            _ => Err(Z3BridgeError::UnsupportedExpression {
                expr_type: format!("{:?}", expr.kind),
                span: expr.span,
                message: "This expression type is not supported in constraint solving".to_string(),
            }),
        }
    }

    // ========================================================================
    // Arithmetic Operations
    // ========================================================================

    fn add_operation(&self, lhs: &Z3Ast, rhs: &Z3Ast, span: Span) -> Result<Z3Ast, Z3BridgeError> {
        match (lhs, rhs) {
            (Z3Ast::Int(l), Z3Ast::Int(r)) => Ok(Z3Ast::Int(z3::ast::Int::add(&[l, r]))),
            (Z3Ast::Real(l), Z3Ast::Real(r)) => Ok(Z3Ast::Real(z3::ast::Real::add(&[l, r]))),
            // Mixed Int/Real: promote to Real
            (Z3Ast::Int(l), Z3Ast::Real(r)) => {
                Ok(Z3Ast::Real(z3::ast::Real::add(&[&l.to_real(), r])))
            }
            (Z3Ast::Real(l), Z3Ast::Int(r)) => {
                Ok(Z3Ast::Real(z3::ast::Real::add(&[l, &r.to_real()])))
            }
            _ => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    fn sub_operation(&self, lhs: &Z3Ast, rhs: &Z3Ast, span: Span) -> Result<Z3Ast, Z3BridgeError> {
        match (lhs, rhs) {
            (Z3Ast::Int(l), Z3Ast::Int(r)) => Ok(Z3Ast::Int(z3::ast::Int::sub(&[l, r]))),
            (Z3Ast::Real(l), Z3Ast::Real(r)) => Ok(Z3Ast::Real(z3::ast::Real::sub(&[l, r]))),
            // Mixed Int/Real: promote to Real
            (Z3Ast::Int(l), Z3Ast::Real(r)) => {
                Ok(Z3Ast::Real(z3::ast::Real::sub(&[&l.to_real(), r])))
            }
            (Z3Ast::Real(l), Z3Ast::Int(r)) => {
                Ok(Z3Ast::Real(z3::ast::Real::sub(&[l, &r.to_real()])))
            }
            _ => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    fn mul_operation(&self, lhs: &Z3Ast, rhs: &Z3Ast, span: Span) -> Result<Z3Ast, Z3BridgeError> {
        match (lhs, rhs) {
            (Z3Ast::Int(l), Z3Ast::Int(r)) => Ok(Z3Ast::Int(z3::ast::Int::mul(&[l, r]))),
            (Z3Ast::Real(l), Z3Ast::Real(r)) => Ok(Z3Ast::Real(z3::ast::Real::mul(&[l, r]))),
            // Mixed Int/Real: promote to Real
            (Z3Ast::Int(l), Z3Ast::Real(r)) => {
                Ok(Z3Ast::Real(z3::ast::Real::mul(&[&l.to_real(), r])))
            }
            (Z3Ast::Real(l), Z3Ast::Int(r)) => {
                Ok(Z3Ast::Real(z3::ast::Real::mul(&[l, &r.to_real()])))
            }
            _ => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    fn div_operation(&self, lhs: &Z3Ast, rhs: &Z3Ast, span: Span) -> Result<Z3Ast, Z3BridgeError> {
        match (lhs, rhs) {
            (Z3Ast::Int(l), Z3Ast::Int(r)) => Ok(Z3Ast::Int(l.div(r))),
            (Z3Ast::Real(l), Z3Ast::Real(r)) => Ok(Z3Ast::Real(l.div(r))),
            // Mixed Int/Real: promote to Real
            (Z3Ast::Int(l), Z3Ast::Real(r)) => Ok(Z3Ast::Real(l.to_real().div(r))),
            (Z3Ast::Real(l), Z3Ast::Int(r)) => Ok(Z3Ast::Real(l.div(r.to_real()))),
            _ => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    fn neg_operation(&self, inner: &Z3Ast, span: Span) -> Result<Z3Ast, Z3BridgeError> {
        match inner {
            Z3Ast::Int(i) => Ok(Z3Ast::Int(-i)),
            Z3Ast::Real(r) => Ok(Z3Ast::Real(-r)),
            Z3Ast::Bool(_) => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    // ========================================================================
    // Comparison Operations
    // ========================================================================

    fn create_equality(
        &self,
        lhs: &Z3Ast,
        rhs: &Z3Ast,
        span: Span,
    ) -> Result<z3::ast::Bool, Z3BridgeError> {
        self.eq_operation(lhs, rhs, span)?.as_bool(span).cloned()
    }

    fn eq_operation(&self, lhs: &Z3Ast, rhs: &Z3Ast, span: Span) -> Result<Z3Ast, Z3BridgeError> {
        match (lhs, rhs) {
            (Z3Ast::Int(l), Z3Ast::Int(r)) => Ok(Z3Ast::Bool(l.eq(r))),
            (Z3Ast::Real(l), Z3Ast::Real(r)) => Ok(Z3Ast::Bool(l.eq(r))),
            (Z3Ast::Bool(l), Z3Ast::Bool(r)) => Ok(Z3Ast::Bool(l.eq(r))),
            // Mixed Int/Real: promote to Real
            (Z3Ast::Int(l), Z3Ast::Real(r)) => Ok(Z3Ast::Bool(l.to_real().eq(r))),
            (Z3Ast::Real(l), Z3Ast::Int(r)) => Ok(Z3Ast::Bool(l.eq(r.to_real()))),
            _ => Err(Z3BridgeError::TypeMismatch {
                expected: "matching types".to_string(),
                found: "mismatched types".to_string(),
                span,
            }),
        }
    }

    fn lt_operation(&self, lhs: &Z3Ast, rhs: &Z3Ast, span: Span) -> Result<Z3Ast, Z3BridgeError> {
        match (lhs, rhs) {
            (Z3Ast::Int(l), Z3Ast::Int(r)) => Ok(Z3Ast::Bool(l.lt(r))),
            (Z3Ast::Real(l), Z3Ast::Real(r)) => Ok(Z3Ast::Bool(l.lt(r))),
            // Mixed Int/Real: promote to Real
            (Z3Ast::Int(l), Z3Ast::Real(r)) => Ok(Z3Ast::Bool(l.to_real().lt(r))),
            (Z3Ast::Real(l), Z3Ast::Int(r)) => Ok(Z3Ast::Bool(l.lt(r.to_real()))),
            _ => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    fn gt_operation(&self, lhs: &Z3Ast, rhs: &Z3Ast, span: Span) -> Result<Z3Ast, Z3BridgeError> {
        match (lhs, rhs) {
            (Z3Ast::Int(l), Z3Ast::Int(r)) => Ok(Z3Ast::Bool(l.gt(r))),
            (Z3Ast::Real(l), Z3Ast::Real(r)) => Ok(Z3Ast::Bool(l.gt(r))),
            // Mixed Int/Real: promote to Real
            (Z3Ast::Int(l), Z3Ast::Real(r)) => Ok(Z3Ast::Bool(l.to_real().gt(r))),
            (Z3Ast::Real(l), Z3Ast::Int(r)) => Ok(Z3Ast::Bool(l.gt(r.to_real()))),
            _ => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    fn le_operation(&self, lhs: &Z3Ast, rhs: &Z3Ast, span: Span) -> Result<Z3Ast, Z3BridgeError> {
        match (lhs, rhs) {
            (Z3Ast::Int(l), Z3Ast::Int(r)) => Ok(Z3Ast::Bool(l.le(r))),
            (Z3Ast::Real(l), Z3Ast::Real(r)) => Ok(Z3Ast::Bool(l.le(r))),
            // Mixed Int/Real: promote to Real
            (Z3Ast::Int(l), Z3Ast::Real(r)) => Ok(Z3Ast::Bool(l.to_real().le(r))),
            (Z3Ast::Real(l), Z3Ast::Int(r)) => Ok(Z3Ast::Bool(l.le(r.to_real()))),
            _ => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }

    fn ge_operation(&self, lhs: &Z3Ast, rhs: &Z3Ast, span: Span) -> Result<Z3Ast, Z3BridgeError> {
        match (lhs, rhs) {
            (Z3Ast::Int(l), Z3Ast::Int(r)) => Ok(Z3Ast::Bool(l.ge(r))),
            (Z3Ast::Real(l), Z3Ast::Real(r)) => Ok(Z3Ast::Bool(l.ge(r))),
            // Mixed Int/Real: promote to Real
            (Z3Ast::Int(l), Z3Ast::Real(r)) => Ok(Z3Ast::Bool(l.to_real().ge(r))),
            (Z3Ast::Real(l), Z3Ast::Int(r)) => Ok(Z3Ast::Bool(l.ge(r.to_real()))),
            _ => Err(Z3BridgeError::TypeMismatch {
                expected: "Int or Real".to_string(),
                found: "Bool".to_string(),
                span,
            }),
        }
    }
}

impl<'src, 'arena> Default for Z3Bridge<'src, 'arena> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint_extractor::{Constraint, Variable};
    use crate::hir::definitions::VarDefinition;
    use crate::hir::expr::{ResolvedExpr, ResolvedExprKind};
    use crate::hir::types::ResolvedType;
    use crate::lexer::{LineColumn, Span};
    use assert_matches::assert_matches;
    use bumpalo::Bump;

    fn test_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 10,
        }
    }

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

    #[test]
    fn test_create_z3_constant_i32() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bridge = Z3Bridge::<'static, 'static>::new();

        let result = bridge.create_z3_constant("x", ty);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_create_z3_constant_f64() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::F64 { span: test_span() });
        let bridge = Z3Bridge::<'static, 'static>::new();

        let result = bridge.create_z3_constant("x", ty);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Real(_));
    }

    #[test]
    fn test_create_z3_constant_bool() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let bridge = Z3Bridge::<'static, 'static>::new();

        let result = bridge.create_z3_constant("x", ty);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Bool(_));
    }

    #[test]
    fn test_translate_int_literal() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let expr = make_expr(&arena, ResolvedExprKind::IntLit { value: 42 }, ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_translate_float_literal() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::F64 { span: test_span() });
        let expr = make_expr(&arena, ResolvedExprKind::FloatLit { value: 3.14 }, ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Real(_));
    }

    #[test]
    fn test_translate_bool_literal() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let expr = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Bool(_));
    }

    #[test]
    fn test_add_variable_without_init() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let var = Variable::new("x", ty, None, test_span());

        let mut bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.add_variable(&var);
        assert!(result.is_ok());
        assert!(bridge.variables.contains_key("x"));
    }

    #[test]
    fn test_add_variable_with_init() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let init = make_expr(&arena, ResolvedExprKind::IntLit { value: 42 }, ty);
        let var = Variable::new("x", ty, Some(init), test_span());

        let mut bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.add_variable(&var);
        assert!(result.is_ok());
        assert!(bridge.variables.contains_key("x"));
    }

    #[test]
    fn test_translate_variable_reference() {
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

        let var = Variable::new("x", ty, None, test_span());
        let mut bridge = Z3Bridge::<'static, 'static>::new();
        bridge.add_variable(&var).unwrap();

        let expr = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def,
            },
            ty,
        );

        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_translate_add_operation() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, ty);
        let expr = make_expr(&arena, ResolvedExprKind::Add { lhs, rhs }, ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_translate_sub_operation() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 5 }, ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 3 }, ty);
        let expr = make_expr(&arena, ResolvedExprKind::Sub { lhs, rhs }, ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_translate_mul_operation() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 3 }, ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 4 }, ty);
        let expr = make_expr(&arena, ResolvedExprKind::Mul { lhs, rhs }, ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_translate_div_operation() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, ty);
        let expr = make_expr(&arena, ResolvedExprKind::Div { lhs, rhs }, ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_translate_eq_operation() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty);
        let expr = make_expr(&arena, ResolvedExprKind::Eq { lhs, rhs }, bool_ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Bool(_));
    }

    #[test]
    fn test_translate_lt_operation() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty);
        let expr = make_expr(&arena, ResolvedExprKind::Lt { lhs, rhs }, bool_ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Bool(_));
    }

    #[test]
    fn test_translate_gt_operation() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 2 }, int_ty);
        let rhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty);
        let expr = make_expr(&arena, ResolvedExprKind::Gt { lhs, rhs }, bool_ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Bool(_));
    }

    #[test]
    fn test_translate_neg_operation() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let inner = make_expr(&arena, ResolvedExprKind::IntLit { value: 42 }, ty);
        let expr = make_expr(&arena, ResolvedExprKind::Neg { inner }, ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_mixed_int_real_add() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let real_ty = arena.alloc(ResolvedType::F64 { span: test_span() });
        let lhs = make_expr(&arena, ResolvedExprKind::IntLit { value: 1 }, int_ty);
        let rhs = make_expr(&arena, ResolvedExprKind::FloatLit { value: 2.5 }, real_ty);
        let expr = make_expr(&arena, ResolvedExprKind::Add { lhs, rhs }, real_ty);

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Real(_));
    }

    #[test]
    fn test_end_to_end_simple_constraint() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // Create: let x = 10;
        let init_x = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let var_x = Variable::new("x", int_ty, Some(init_x), test_span());

        // Create: let y;
        let var_y = Variable::new("y", int_ty, None, test_span());

        // Create: x + y == 20
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: Some(init_x),
            scope_level: 0,
            span: test_span(),
        });
        let var_def_y = arena.alloc(VarDefinition {
            name: "y",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

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

        // Build the constraint problem
        let mut problem = ConstraintProblem::new();
        problem.add_variable(var_x);
        problem.add_variable(var_y);
        problem.add_constraint(Constraint::new(constraint, test_span()));

        // Translate to Z3
        let mut bridge = Z3Bridge::new();
        let result = bridge.add_problem(&problem);
        assert!(result.is_ok());

        // Solve
        let solver = bridge.solver();
        assert_eq!(solver.check(), z3::SatResult::Sat);

        // Get model and verify
        let model = solver.get_model().expect("Failed to get model");
        let y_z3 = bridge.variables.get("y").unwrap();
        let y_value = model
            .eval(y_z3.as_int(test_span()).unwrap(), true)
            .expect("Failed to evaluate y")
            .as_i64()
            .expect("y should be an integer");

        assert_eq!(y_value, 10); // x + y == 20, x == 10, so y == 10
    }

    #[test]
    fn test_error_unsupported_type() {
        let arena = Bump::new();
        let inner_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let ty = arena.alloc(ResolvedType::Reference {
            inner: inner_ty,
            span: test_span(),
        });
        let bridge = Z3Bridge::<'static, 'static>::new();

        let result = bridge.create_z3_constant("x", ty);
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), Z3BridgeError::UnsupportedType { .. });
    }

    #[test]
    fn test_error_variable_not_found() {
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

        let expr = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def,
            },
            ty,
        );

        let bridge = Z3Bridge::<'static, 'static>::new();
        let result = bridge.translate_expr(expr);
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), Z3BridgeError::VariableNotFound { .. });
    }

    #[test]
    fn test_error_display_unsupported_expression() {
        let error = Z3BridgeError::UnsupportedExpression {
            expr_type: "StructLit".to_string(),
            span: test_span(),
            message: "Not supported".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("StructLit"));
        assert!(display.contains("Not supported"));
    }

    #[test]
    fn test_error_display_variable_not_found() {
        let error = Z3BridgeError::VariableNotFound {
            var_name: "x".to_string(),
            span: test_span(),
        };
        let display = format!("{}", error);
        assert!(display.contains("x"));
        assert!(display.contains("not found"));
    }
}
