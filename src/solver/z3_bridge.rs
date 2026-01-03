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

use super::constraint_extractor::{ConditionalConstraint, ConstraintProblem, Variable};
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
    /// Uses String to support both source-borrowed and generated names (flattened structs)
    variables: HashMap<String, Z3Ast>,
    /// The Z3 solver
    solver: z3::Solver,
    /// Phantom data to maintain the lifetimes
    _phantom: PhantomData<(&'src (), &'arena ())>,
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
        let z3_var = self.create_z3_constant(&variable.name, variable.var_type)?;

        // Store the variable
        self.variables.insert(variable.name.clone(), z3_var.clone());

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

    /// Add a conditional constraint to the Z3 solver
    ///
    /// Translates if-statement constraints to Z3 assertions using three strategies:
    /// 1. Both branches have constraints: Use ITE (if-then-else)
    /// 2. Only then branch: Use implication (condition => then_constraint)
    /// 3. Only else branch: Use implication (!condition => else_constraint)
    ///
    /// When branches have different numbers of constraints, pairs are processed
    /// with ITE and remaining constraints use implication.
    pub fn add_conditional_constraints(
        &mut self,
        cond_constraint: &ConditionalConstraint<'src, 'arena>,
    ) -> Result<(), Z3BridgeError> {
        let condition_z3 = self.translate_expr(cond_constraint.condition)?;
        let condition_bool = condition_z3.as_bool(cond_constraint.condition.span)?;

        let then_count = cond_constraint.then_constraints.len();
        let else_count = cond_constraint.else_constraints.len();
        let min_count = then_count.min(else_count);

        // Process paired constraints with ITE
        for i in 0..min_count {
            let then_expr = self.translate_expr(cond_constraint.then_constraints[i].expr)?;
            let else_expr = self.translate_expr(cond_constraint.else_constraints[i].expr)?;

            let then_bool = then_expr.as_bool(cond_constraint.then_constraints[i].span)?;
            let else_bool = else_expr.as_bool(cond_constraint.else_constraints[i].span)?;

            // Create: ite(condition, then_constraint, else_constraint)
            let ite_expr = condition_bool.ite(then_bool, else_bool);
            self.solver.assert(&ite_expr);
        }

        // Process remaining then-only constraints with implication
        for i in min_count..then_count {
            let then_expr = self.translate_expr(cond_constraint.then_constraints[i].expr)?;
            let then_bool = then_expr.as_bool(cond_constraint.then_constraints[i].span)?;

            // Create: condition => then_constraint
            let implication = condition_bool.implies(then_bool);
            self.solver.assert(&implication);
        }

        // Process remaining else-only constraints with implication
        for i in min_count..else_count {
            let else_expr = self.translate_expr(cond_constraint.else_constraints[i].expr)?;
            let else_bool = else_expr.as_bool(cond_constraint.else_constraints[i].span)?;

            // Create: !condition => else_constraint
            let not_condition = condition_bool.not();
            let implication = not_condition.implies(else_bool);
            self.solver.assert(&implication);
        }

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

        // Add all conditional constraints
        for cond_constraint in &problem.conditional_constraints {
            self.add_conditional_constraints(cond_constraint)?;
        }

        Ok(())
    }

    /// Get the Z3 solver with all assertions added
    pub fn solver(&self) -> &z3::Solver {
        &self.solver
    }

    /// Get the variables map for solution formatting
    pub fn variables(&self) -> &HashMap<String, Z3Ast> {
        &self.variables
    }

    /// Create a Z3 constant based on a ResolvedType
    fn create_z3_constant(
        &self,
        name: &str,
        ty: ResolvedType<'src, 'arena>,
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
                span,
                message: "Reference types are not supported in constraint solving".to_string(),
            }),
            ResolvedType::UserDefined { name, span, .. } => Err(Z3BridgeError::UnsupportedType {
                type_name: name.to_string(),
                span,
                message: "User-defined types are not supported in constraint solving".to_string(),
            }),
        }
    }

    /// Build a qualified variable name from a field access chain
    ///
    /// Recursively walks the receiver expression to build the full qualified name.
    ///
    /// # Examples
    /// - `p.x` → "p.x"
    /// - `line.start.x` → "line.start.x"
    fn build_field_access_name<'src2, 'arena2>(
        receiver: &'arena2 ResolvedExpr<'src2, 'arena2>,
        field_name: &str,
    ) -> Result<String, Z3BridgeError> {
        match &receiver.kind {
            // Base case: variable reference
            ResolvedExprKind::Var { name, .. } => Ok(format!("{}.{}", name, field_name)),

            // Recursive case: nested field access
            ResolvedExprKind::FieldAccess {
                receiver: nested_receiver,
                field_name: nested_field,
                ..
            } => {
                let prefix = Self::build_field_access_name(nested_receiver, nested_field)?;
                Ok(format!("{}.{}", prefix, field_name))
            }

            // Unsupported receiver type
            _ => Err(Z3BridgeError::UnsupportedExpression {
                expr_type: format!("{:?}", receiver.kind),
                span: receiver.span,
                message: "Field access receiver must be a variable or nested field access"
                    .to_string(),
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
                self.variables.get(&name[..]).cloned().ok_or_else(|| {
                    Z3BridgeError::VariableNotFound {
                        var_name: name.to_string(),
                        span: expr.span,
                    }
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

            // Field access - translate to flattened variable name
            ResolvedExprKind::FieldAccess {
                receiver,
                field_name,
                ..
            } => {
                let qualified_name = Self::build_field_access_name(receiver, field_name)?;
                self.variables.get(&qualified_name[..]).cloned().ok_or(
                    Z3BridgeError::VariableNotFound {
                        var_name: qualified_name,
                        span: expr.span,
                    },
                )
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

    // ========================================================================
    // Conditional (If-Then-Else) Operations
    // ========================================================================

    /// Create an if-then-else expression
    ///
    /// Uses Z3's native `ite` operator to create conditional expressions.
    ///
    /// # Arguments
    /// * `condition` - Boolean condition
    /// * `then_expr` - Expression if condition is true
    /// * `else_expr` - Expression if condition is false
    ///
    /// # Returns
    /// Returns the same type as then_expr and else_expr (must match or be compatible)
    ///
    /// # Type Promotion
    /// If one branch is Int and the other is Real, both are promoted to Real
    fn create_ite(
        &self,
        condition: &Z3Ast,
        then_expr: &Z3Ast,
        else_expr: &Z3Ast,
        span: Span,
    ) -> Result<Z3Ast, Z3BridgeError> {
        let cond_bool = condition.as_bool(span)?;

        match (then_expr, else_expr) {
            (Z3Ast::Int(t), Z3Ast::Int(e)) => Ok(Z3Ast::Int(cond_bool.ite(t, e))),
            (Z3Ast::Real(t), Z3Ast::Real(e)) => Ok(Z3Ast::Real(cond_bool.ite(t, e))),
            (Z3Ast::Bool(t), Z3Ast::Bool(e)) => Ok(Z3Ast::Bool(cond_bool.ite(t, e))),
            // Handle mixed Int/Real by promoting to Real
            (Z3Ast::Int(t), Z3Ast::Real(e)) => Ok(Z3Ast::Real(cond_bool.ite(&t.to_real(), e))),
            (Z3Ast::Real(t), Z3Ast::Int(e)) => Ok(Z3Ast::Real(cond_bool.ite(t, &e.to_real()))),
            _ => Err(Z3BridgeError::TypeMismatch {
                expected: "matching types in then/else branches".to_string(),
                found: "mismatched types".to_string(),
                span,
            }),
        }
    }

    /// Translate an if-then-else expression to Z3
    ///
    /// This method is public to support future HIR if-expressions.
    /// Currently used for testing the ITE functionality.
    ///
    /// # Example
    /// ```ignore
    /// // Translates: if x > 0 { 10 } else { -10 }
    /// let result = bridge.translate_if_expr(condition, then_expr, else_expr)?;
    /// ```
    pub fn translate_if_expr(
        &self,
        condition: &'arena ResolvedExpr<'src, 'arena>,
        then_expr: &'arena ResolvedExpr<'src, 'arena>,
        else_expr: &'arena ResolvedExpr<'src, 'arena>,
    ) -> Result<Z3Ast, Z3BridgeError> {
        let cond_z3 = self.translate_expr(condition)?;
        let then_z3 = self.translate_expr(then_expr)?;
        let else_z3 = self.translate_expr(else_expr)?;

        self.create_ite(&cond_z3, &then_z3, &else_z3, condition.span)
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
    use crate::hir::definitions::VarDefinition;
    use crate::hir::expr::{ResolvedExpr, ResolvedExprKind};
    use crate::hir::types::ResolvedType;
    use crate::lexer::{LineColumn, Span};
    use crate::solver::constraint_extractor::{Constraint, Variable};
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

        let result = bridge.create_z3_constant("x", *ty);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_create_z3_constant_f64() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::F64 { span: test_span() });
        let bridge = Z3Bridge::<'static, 'static>::new();

        let result = bridge.create_z3_constant("x", *ty);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Real(_));
    }

    #[test]
    fn test_create_z3_constant_bool() {
        let arena = Bump::new();
        let ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let bridge = Z3Bridge::<'static, 'static>::new();

        let result = bridge.create_z3_constant("x", *ty);
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
        let var = Variable::new("x", *ty, None, test_span());

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
        let var = Variable::new("x", *ty, Some(init), test_span());

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

        let var = Variable::new("x", *ty, None, test_span());
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
        let var_x = Variable::new("x", *int_ty, Some(init_x), test_span());

        // Create: let y;
        let var_y = Variable::new("y", *int_ty, None, test_span());

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

        let result = bridge.create_z3_constant("x", *ty);
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

    // ========================================================================
    // ITE (If-Then-Else) Tests
    // ========================================================================

    #[test]
    fn test_create_ite_int() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let bridge = Z3Bridge::<'static, 'static>::new();

        // Create: if true { 10 } else { 20 }
        let condition = Z3Ast::Bool(z3::ast::Bool::from_bool(true));
        let then_val = Z3Ast::Int(z3::ast::Int::from_i64(10));
        let else_val = Z3Ast::Int(z3::ast::Int::from_i64(20));

        let result = bridge.create_ite(&condition, &then_val, &else_val, test_span());
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_create_ite_real() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let real_ty = arena.alloc(ResolvedType::F64 { span: test_span() });

        let bridge = Z3Bridge::<'static, 'static>::new();

        // Create: if false { 1.5 } else { 2.5 }
        let condition = Z3Ast::Bool(z3::ast::Bool::from_bool(false));
        let then_val = Z3Ast::Real(z3::ast::Real::from_rational_str("15", "10").unwrap());
        let else_val = Z3Ast::Real(z3::ast::Real::from_rational_str("25", "10").unwrap());

        let result = bridge.create_ite(&condition, &then_val, &else_val, test_span());
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Real(_));
    }

    #[test]
    fn test_create_ite_bool() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        let bridge = Z3Bridge::<'static, 'static>::new();

        // Create: if true { true } else { false }
        let condition = Z3Ast::Bool(z3::ast::Bool::from_bool(true));
        let then_val = Z3Ast::Bool(z3::ast::Bool::from_bool(true));
        let else_val = Z3Ast::Bool(z3::ast::Bool::from_bool(false));

        let result = bridge.create_ite(&condition, &then_val, &else_val, test_span());
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Bool(_));
    }

    #[test]
    fn test_create_ite_mixed_int_real() {
        let bridge = Z3Bridge::<'static, 'static>::new();

        // Create: if true { 10 (int) } else { 2.5 (real) }
        let condition = Z3Ast::Bool(z3::ast::Bool::from_bool(true));
        let then_val = Z3Ast::Int(z3::ast::Int::from_i64(10));
        let else_val = Z3Ast::Real(z3::ast::Real::from_rational_str("25", "10").unwrap());

        let result = bridge.create_ite(&condition, &then_val, &else_val, test_span());
        assert!(result.is_ok());
        // Should be promoted to Real
        assert_matches!(result.unwrap(), Z3Ast::Real(_));
    }

    #[test]
    fn test_create_ite_mixed_real_int() {
        let bridge = Z3Bridge::<'static, 'static>::new();

        // Create: if true { 1.5 (real) } else { 20 (int) }
        let condition = Z3Ast::Bool(z3::ast::Bool::from_bool(true));
        let then_val = Z3Ast::Real(z3::ast::Real::from_rational_str("15", "10").unwrap());
        let else_val = Z3Ast::Int(z3::ast::Int::from_i64(20));

        let result = bridge.create_ite(&condition, &then_val, &else_val, test_span());
        assert!(result.is_ok());
        // Should be promoted to Real
        assert_matches!(result.unwrap(), Z3Ast::Real(_));
    }

    #[test]
    fn test_create_ite_type_mismatch() {
        let bridge = Z3Bridge::<'static, 'static>::new();

        // Create: if true { 10 (int) } else { true (bool) } - should fail
        let condition = Z3Ast::Bool(z3::ast::Bool::from_bool(true));
        let then_val = Z3Ast::Int(z3::ast::Int::from_i64(10));
        let else_val = Z3Ast::Bool(z3::ast::Bool::from_bool(true));

        let result = bridge.create_ite(&condition, &then_val, &else_val, test_span());
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), Z3BridgeError::TypeMismatch { .. });
    }

    #[test]
    fn test_translate_if_expr_basic() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let bridge = Z3Bridge::<'static, 'static>::new();

        // Create: if true { 10 } else { 20 }
        let condition = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);
        let then_expr = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let else_expr = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);

        let result = bridge.translate_if_expr(condition, then_expr, else_expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_translate_if_expr_with_variables() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create variables
        let var_x = Variable::new("x", *int_ty, None, test_span());
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let mut bridge = Z3Bridge::<'static, 'static>::new();
        bridge.add_variable(&var_x).unwrap();

        // Create: if x > 0 { 10 } else { -10 }
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let zero = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let condition = make_expr(
            &arena,
            ResolvedExprKind::Gt {
                lhs: x_ref,
                rhs: zero,
            },
            bool_ty,
        );
        let then_expr = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let else_expr = make_expr(&arena, ResolvedExprKind::IntLit { value: -10 }, int_ty);

        let result = bridge.translate_if_expr(condition, then_expr, else_expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    // ========================================================================
    // Conditional Constraint Tests
    // ========================================================================

    #[test]
    fn test_add_conditional_constraint_both_branches() {
        use super::super::constraint_extractor::{ConditionalConstraint, Constraint};

        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create variables: x, y
        let var_x = Variable::new("x", *int_ty, None, test_span());
        let var_y = Variable::new("y", *int_ty, None, test_span());
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
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

        let mut bridge = Z3Bridge::<'static, 'static>::new();
        bridge.add_variable(&var_x).unwrap();
        bridge.add_variable(&var_y).unwrap();

        // Create: if x > 0 { y == 10; } else { y == 20; }
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let zero = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let condition = make_expr(
            &arena,
            ResolvedExprKind::Gt {
                lhs: x_ref,
                rhs: zero,
            },
            bool_ty,
        );

        let y_ref_then = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let ten = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let then_constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: y_ref_then,
                rhs: ten,
            },
            bool_ty,
        );

        let y_ref_else = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let twenty = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let else_constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: y_ref_else,
                rhs: twenty,
            },
            bool_ty,
        );

        let cond_constraint = ConditionalConstraint::new(
            condition,
            vec![Constraint::new(then_constraint, test_span())],
            vec![Constraint::new(else_constraint, test_span())],
            test_span(),
        );

        let result = bridge.add_conditional_constraints(&cond_constraint);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_conditional_constraint_then_only() {
        use super::super::constraint_extractor::{ConditionalConstraint, Constraint};

        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create variables: x, y
        let var_x = Variable::new("x", *int_ty, None, test_span());
        let var_y = Variable::new("y", *int_ty, None, test_span());
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
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

        let mut bridge = Z3Bridge::<'static, 'static>::new();
        bridge.add_variable(&var_x).unwrap();
        bridge.add_variable(&var_y).unwrap();

        // Create: if x > 0 { y == 10; }
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let zero = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let condition = make_expr(
            &arena,
            ResolvedExprKind::Gt {
                lhs: x_ref,
                rhs: zero,
            },
            bool_ty,
        );

        let y_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let ten = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let then_constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: y_ref,
                rhs: ten,
            },
            bool_ty,
        );

        let cond_constraint = ConditionalConstraint::new(
            condition,
            vec![Constraint::new(then_constraint, test_span())],
            vec![], // No else constraints
            test_span(),
        );

        let result = bridge.add_conditional_constraints(&cond_constraint);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_conditional_constraint_else_only() {
        use super::super::constraint_extractor::{ConditionalConstraint, Constraint};

        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create variables: x, y
        let var_x = Variable::new("x", *int_ty, None, test_span());
        let var_y = Variable::new("y", *int_ty, None, test_span());
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
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

        let mut bridge = Z3Bridge::<'static, 'static>::new();
        bridge.add_variable(&var_x).unwrap();
        bridge.add_variable(&var_y).unwrap();

        // Create: if x > 0 { } else { y == 20; }
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let zero = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let condition = make_expr(
            &arena,
            ResolvedExprKind::Gt {
                lhs: x_ref,
                rhs: zero,
            },
            bool_ty,
        );

        let y_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let twenty = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let else_constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: y_ref,
                rhs: twenty,
            },
            bool_ty,
        );

        let cond_constraint = ConditionalConstraint::new(
            condition,
            vec![], // No then constraints
            vec![Constraint::new(else_constraint, test_span())],
            test_span(),
        );

        let result = bridge.add_conditional_constraints(&cond_constraint);
        assert!(result.is_ok());
    }

    #[test]
    fn test_add_conditional_constraint_multiple() {
        use super::super::constraint_extractor::{ConditionalConstraint, Constraint};

        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create variables: x, y, z
        let var_x = Variable::new("x", *int_ty, None, test_span());
        let var_y = Variable::new("y", *int_ty, None, test_span());
        let var_z = Variable::new("z", *int_ty, None, test_span());
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
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
        let var_def_z = arena.alloc(VarDefinition {
            name: "z",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let mut bridge = Z3Bridge::<'static, 'static>::new();
        bridge.add_variable(&var_x).unwrap();
        bridge.add_variable(&var_y).unwrap();
        bridge.add_variable(&var_z).unwrap();

        // Create: if x > 0 { y == 10; z == 5; } else { y == 20; }
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let zero = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let condition = make_expr(
            &arena,
            ResolvedExprKind::Gt {
                lhs: x_ref,
                rhs: zero,
            },
            bool_ty,
        );

        // Then branch: y == 10; z == 5;
        let y_ref_then = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let ten = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let then_constraint1 = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: y_ref_then,
                rhs: ten,
            },
            bool_ty,
        );

        let z_ref_then = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "z",
                definition: var_def_z,
            },
            int_ty,
        );
        let five = make_expr(&arena, ResolvedExprKind::IntLit { value: 5 }, int_ty);
        let then_constraint2 = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: z_ref_then,
                rhs: five,
            },
            bool_ty,
        );

        // Else branch: y == 20;
        let y_ref_else = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let twenty = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let else_constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: y_ref_else,
                rhs: twenty,
            },
            bool_ty,
        );

        let cond_constraint = ConditionalConstraint::new(
            condition,
            vec![
                Constraint::new(then_constraint1, test_span()),
                Constraint::new(then_constraint2, test_span()),
            ],
            vec![Constraint::new(else_constraint, test_span())],
            test_span(),
        );

        let result = bridge.add_conditional_constraints(&cond_constraint);
        assert!(result.is_ok());
    }

    #[test]
    fn test_end_to_end_if_constraint() {
        use super::super::constraint_extractor::{ConditionalConstraint, Constraint};

        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create variables: x = 5, y
        let init_x = make_expr(&arena, ResolvedExprKind::IntLit { value: 5 }, int_ty);
        let var_x = Variable::new("x", *int_ty, Some(init_x), test_span());
        let var_y = Variable::new("y", *int_ty, None, test_span());

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

        // Create: if x > 0 { y == 10; } else { y == -10; }
        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let zero = make_expr(&arena, ResolvedExprKind::IntLit { value: 0 }, int_ty);
        let condition = make_expr(
            &arena,
            ResolvedExprKind::Gt {
                lhs: x_ref,
                rhs: zero,
            },
            bool_ty,
        );

        let y_ref_then = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let ten = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let then_constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: y_ref_then,
                rhs: ten,
            },
            bool_ty,
        );

        let y_ref_else = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let neg_ten = make_expr(&arena, ResolvedExprKind::IntLit { value: -10 }, int_ty);
        let else_constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: y_ref_else,
                rhs: neg_ten,
            },
            bool_ty,
        );

        let cond_constraint = ConditionalConstraint::new(
            condition,
            vec![Constraint::new(then_constraint, test_span())],
            vec![Constraint::new(else_constraint, test_span())],
            test_span(),
        );

        // Build the constraint problem
        let mut problem = ConstraintProblem::new();
        problem.add_variable(var_x);
        problem.add_variable(var_y);
        problem.conditional_constraints.push(cond_constraint);

        // Translate to Z3
        let mut bridge = Z3Bridge::new();
        let result = bridge.add_problem(&problem);
        assert!(result.is_ok());

        // Solve
        let solver = bridge.solver();
        assert_eq!(solver.check(), z3::SatResult::Sat);

        // Get model and verify
        // Since x = 5 > 0, the then branch applies: y == 10
        let model = solver.get_model().expect("Failed to get model");
        let y_z3 = bridge.variables.get("y").unwrap();
        let y_value = model
            .eval(y_z3.as_int(test_span()).unwrap(), true)
            .expect("Failed to evaluate y")
            .as_i64()
            .expect("y should be an integer");

        assert_eq!(y_value, 10); // x = 5 > 0, so y == 10
    }

    // ========================================================================
    // Field Access Tests
    // ========================================================================

    #[test]
    fn test_build_field_access_name_simple() {
        use crate::hir::definitions::VarDefinition;

        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let bridge = Z3Bridge::<'static, 'static>::new();

        // Create: p.x
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let p_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "p",
                definition: var_def,
            },
            int_ty,
        );

        let result = Z3Bridge::build_field_access_name(p_ref, "x");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "p.x");
    }

    #[test]
    fn test_build_field_access_name_nested() {
        use crate::hir::definitions::{FieldDefinition, StructDefinition, VarDefinition};

        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        let bridge = Z3Bridge::<'static, 'static>::new();

        // Create struct Point { x: i32, y: i32 }
        let point_struct = arena.alloc(StructDefinition::new(
            "Point",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new("x", test_span(), *int_ty, test_span())),
                arena.alloc(FieldDefinition::new("y", test_span(), *int_ty, test_span())),
            ],
            vec![],
            None,
            test_span(),
        ));

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: point_struct,
            span: test_span(),
        });

        // Create struct Line { start: Point, end: Point }
        let line_struct = arena.alloc(StructDefinition::new(
            "Line",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new(
                    "start",
                    test_span(),
                    *point_ty,
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "end",
                    test_span(),
                    *point_ty,
                    test_span(),
                )),
            ],
            vec![],
            None,
            test_span(),
        ));

        let line_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Line",
            definition: line_struct,
            span: test_span(),
        });

        // Create variable: line
        let var_def = arena.alloc(VarDefinition {
            name: "line",
            name_span: test_span(),
            var_type: Some(*line_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        // Build: line.start.x
        let line_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "line",
                definition: var_def,
            },
            line_ty,
        );

        let start_field = &line_struct.fields[0];
        let line_start = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: line_ref,
                field_name: "start",
                field: start_field,
            },
            point_ty,
        );

        let result = Z3Bridge::build_field_access_name(line_start, "x");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "line.start.x");
    }

    #[test]
    fn test_translate_field_access_simple() {
        use crate::hir::definitions::{FieldDefinition, StructDefinition, VarDefinition};

        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create struct Point { x: i32, y: i32 }
        let point_struct = arena.alloc(StructDefinition::new(
            "Point",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new("x", test_span(), *int_ty, test_span())),
                arena.alloc(FieldDefinition::new("y", test_span(), *int_ty, test_span())),
            ],
            vec![],
            None,
            test_span(),
        ));

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: point_struct,
            span: test_span(),
        });

        // Create variable p: Point (with flattened fields p.x and p.y)
        let var_p_x = Variable::new("p.x", *int_ty, None, test_span());
        let var_p_y = Variable::new("p.y", *int_ty, None, test_span());

        let mut bridge = Z3Bridge::<'static, 'static>::new();
        bridge.add_variable(&var_p_x).unwrap();
        bridge.add_variable(&var_p_y).unwrap();

        // Create field access: p.x
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let p_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "p",
                definition: var_def,
            },
            point_ty,
        );

        let x_field = &point_struct.fields[0];
        let p_x_expr = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: p_ref,
                field_name: "x",
                field: x_field,
            },
            int_ty,
        );

        let result = bridge.translate_expr(p_x_expr);
        assert!(result.is_ok());
        assert_matches!(result.unwrap(), Z3Ast::Int(_));
    }

    #[test]
    fn test_field_access_in_constraint() {
        use crate::hir::definitions::{FieldDefinition, StructDefinition, VarDefinition};

        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // Create struct Point { x: i32, y: i32 }
        let point_struct = arena.alloc(StructDefinition::new(
            "Point",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new("x", test_span(), *int_ty, test_span())),
                arena.alloc(FieldDefinition::new("y", test_span(), *int_ty, test_span())),
            ],
            vec![],
            None,
            test_span(),
        ));

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: point_struct,
            span: test_span(),
        });

        // Create flattened variables: p.x, p.y
        let var_p_x = Variable::new("p.x", *int_ty, None, test_span());
        let var_p_y = Variable::new("p.y", *int_ty, None, test_span());

        let mut bridge = Z3Bridge::<'static, 'static>::new();
        bridge.add_variable(&var_p_x).unwrap();
        bridge.add_variable(&var_p_y).unwrap();

        // Create constraint: p.x + p.y == 15
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let p_ref1 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "p",
                definition: var_def,
            },
            point_ty,
        );

        let p_ref2 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "p",
                definition: var_def,
            },
            point_ty,
        );

        let x_field = &point_struct.fields[0];
        let y_field = &point_struct.fields[1];

        let p_x_expr = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: p_ref1,
                field_name: "x",
                field: x_field,
            },
            int_ty,
        );

        let p_y_expr = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: p_ref2,
                field_name: "y",
                field: y_field,
            },
            int_ty,
        );

        let sum = make_expr(
            &arena,
            ResolvedExprKind::Add {
                lhs: p_x_expr,
                rhs: p_y_expr,
            },
            int_ty,
        );

        let fifteen = make_expr(&arena, ResolvedExprKind::IntLit { value: 15 }, int_ty);

        let constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: sum,
                rhs: fifteen,
            },
            bool_ty,
        );

        // Add constraint and solve
        let result = bridge.add_constraint(constraint);
        assert!(result.is_ok());

        let solver = bridge.solver();
        assert_eq!(solver.check(), z3::SatResult::Sat);
    }

    #[test]
    fn test_field_access_variable_not_found() {
        use crate::hir::definitions::{FieldDefinition, StructDefinition, VarDefinition};

        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create struct Point { x: i32, y: i32 }
        let point_struct = arena.alloc(StructDefinition::new(
            "Point",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new("x", test_span(), *int_ty, test_span())),
                arena.alloc(FieldDefinition::new("y", test_span(), *int_ty, test_span())),
            ],
            vec![],
            None,
            test_span(),
        ));

        let point_ty = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: point_struct,
            span: test_span(),
        });

        // Create bridge WITHOUT adding p.x variable
        let bridge = Z3Bridge::<'static, 'static>::new();

        // Try to access p.x (should fail - variable not found)
        let var_def = arena.alloc(VarDefinition {
            name: "p",
            name_span: test_span(),
            var_type: Some(*point_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let p_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "p",
                definition: var_def,
            },
            point_ty,
        );

        let x_field = &point_struct.fields[0];
        let p_x_expr = make_expr(
            &arena,
            ResolvedExprKind::FieldAccess {
                receiver: p_ref,
                field_name: "x",
                field: x_field,
            },
            int_ty,
        );

        let result = bridge.translate_expr(p_x_expr);
        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), Z3BridgeError::VariableNotFound { .. });
    }
}
