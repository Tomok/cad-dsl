//! New Trait-Based Solver Architecture
//!
//! This module implements a trait-based constraint solver architecture for the CAD-DSL language.
//! Unlike the legacy imperative solver, this version uses the `Solvable` trait pattern where
//! HIR nodes implement their own constraint generation logic.
//!
//! # Architecture Overview
//!
//! The new solver follows these design principles:
//!
//! 1. **Trait-Based**: HIR nodes implement `Solvable` trait for constraint generation
//! 2. **Modular**: Functionality split into focused modules in `impls/` subdirectory
//! 3. **Tree-Based Variables**: Variables organized in a tree structure for scoping
//! 4. **RAII Guards**: Scope management using guard types
//!
//! # Migration Status
//!
//! **Phase 1** ✓ - Extracted reusable components from legacy solver:
//! - `struct_flattener.rs` - Flattens struct/array types to primitive fields
//! - `recursive_struct_detector.rs` - Detects cycles in struct definitions
//! - `solution_formatter.rs` - Formats Z3 solutions for display
//!
//! **Phase 2** (In Progress) - Core infrastructure:
//! - `PathComponent` and `VariablePath` types for tree navigation
//! - `Solvable` trait for HIR nodes
//! - `SolverContext` with tree-based variable management
//! - RAII guards for scope management
//!
//! **Phase 3+** (Planned) - Trait implementations for expressions and statements
//!
//! See `docs/SOLVER_ARCHITECTURE.md` and `docs/MIGRATION_STRATEGY.md` for details.

#![allow(dead_code)] // Module under development

use std::fmt::{self, Write as _};

// ============================================================================
// Reusable Components (Phase 1)
// ============================================================================

/// Struct and array field flattening for Z3 variable mapping
pub mod struct_flattener;

/// Recursive struct cycle detection
pub mod recursive_struct_detector;

/// Solution formatting for Z3 models
pub mod solution_formatter;

// ============================================================================
// Public Re-exports (Phase 1)
// ============================================================================

// NOTE: These are part of the public API and will be used in Phase 3+
#[allow(unused_imports)]
pub use recursive_struct_detector::detect_cycles;
#[allow(unused_imports)]
pub use solution_formatter::{SolutionFormatter, SolutionFormatterError};
#[allow(unused_imports)]
pub use struct_flattener::flatten_type;

// ============================================================================
// Phase 2: Core Types and Trait
// ============================================================================

/// Component of a variable path for tree navigation
///
/// Represents a single step in navigating the variable tree, either
/// accessing a struct field or indexing into an array.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathComponent<'src> {
    /// Struct field access: `.field`
    Field(&'src str),

    /// Array index access: `[0]`
    Index(usize),
}

/// Complete path to a variable or sub-variable in the tree
///
/// A path is a sequence of components that describes how to navigate
/// from the root of the variable tree to a specific node.
///
/// # Examples
///
/// - `p.x` → `[Field("p"), Field("x")]`
/// - `points[0].y` → `[Field("points"), Index(0), Field("y")]`
/// - `sketch.entities.line` → `[Field("sketch"), Field("entities"), Field("line")]`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariablePath<'src> {
    components: Vec<PathComponent<'src>>,
}

impl<'src> VariablePath<'src> {
    /// Create path from root variable name
    pub fn from_name(name: &'src str) -> Self {
        Self {
            components: vec![PathComponent::Field(name)],
        }
    }

    /// Create empty path (used internally)
    pub fn empty() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    /// Extend path with field access
    pub fn with_field(&self, field: &'src str) -> Self {
        let mut new_path = self.clone();
        new_path.components.push(PathComponent::Field(field));
        new_path
    }

    /// Extend path with array index
    pub fn with_index(&self, idx: usize) -> Self {
        let mut new_path = self.clone();
        new_path.components.push(PathComponent::Index(idx));
        new_path
    }

    /// Access the components slice
    pub fn components(&self) -> &[PathComponent<'src>] {
        &self.components
    }

    /// Check if path is empty
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// Get the length of the path
    pub fn len(&self) -> usize {
        self.components.len()
    }

    /// Generate Z3 variable name
    ///
    /// This is the ONLY place where String allocation happens!
    /// All navigation uses zero-copy `&'src str` references.
    pub fn to_z3_name(&self) -> String {
        let mut result = String::new();
        for (i, comp) in self.components.iter().enumerate() {
            match comp {
                PathComponent::Field(name) => {
                    if i > 0 {
                        result.push('.');
                    }
                    result.push_str(name);
                }
                PathComponent::Index(idx) => {
                    write!(&mut result, "[{}]", idx).unwrap();
                }
            }
        }
        result
    }
}

impl fmt::Display for VariablePath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_z3_name())
    }
}

// ============================================================================
// Phase 3a: Solution and Result Types
// ============================================================================

/// Concrete value extracted from Z3 model
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Integer value
    Int(i64),
    /// Real (floating-point) value
    Real(f64),
    /// Boolean value
    Bool(bool),
    /// Under-constrained variable (not uniquely determined by constraints)
    UnderConstrained,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{}", v),
            Value::Real(v) => write!(f, "{}", v),
            Value::Bool(v) => write!(f, "{}", v),
            Value::UnderConstrained => write!(f, "<under-constrained>"),
        }
    }
}

/// Solution containing variable assignments from Z3
#[derive(Debug, Clone)]
pub struct Solution<'src> {
    /// Map from variable path to concrete value
    pub assignments: std::collections::HashMap<VariablePath<'src>, Value>,
}

impl<'src> Solution<'src> {
    /// Create an empty solution
    pub fn new() -> Self {
        Self {
            assignments: std::collections::HashMap::new(),
        }
    }

    /// Get the value of a variable
    pub fn get(&self, path: &VariablePath<'src>) -> Option<&Value> {
        self.assignments.get(path)
    }

    /// Number of resolved variables
    pub fn resolved_count(&self) -> usize {
        self.assignments.len()
    }
}

impl<'src> Default for Solution<'src> {
    fn default() -> Self {
        Self::new()
    }
}

/// A constraint that has been deferred for later resolution
#[derive(Debug, Clone)]
pub struct DeferredConstraint<'src> {
    /// Variables that must have known values to process this constraint
    pub dependencies: Vec<&'src str>,

    /// Human-readable description of what's being deferred
    pub description: String,
}

/// Reason why solving was only partial
#[derive(Debug, Clone, PartialEq)]
pub enum PartialReason {
    /// For-loop with unresolved range variable
    UnknownLoopRange { range_var: String },

    /// Function call with unresolved dependencies
    UnresolvedFunctionCall {
        function_name: String,
        missing_deps: Vec<String>,
    },

    /// No progress made - deferred constraints still have unknown dependencies
    ///
    /// Solving stops when no new variables are resolved between iterations,
    /// indicating that the remaining deferred constraints cannot be satisfied
    /// with the current information.
    NoProgress { stuck_constraints: Vec<String> },
}

impl fmt::Display for PartialReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PartialReason::UnknownLoopRange { range_var } => {
                write!(
                    f,
                    "for-loop range depends on unknown variable '{}'",
                    range_var
                )
            }
            PartialReason::UnresolvedFunctionCall {
                function_name,
                missing_deps,
            } => {
                write!(
                    f,
                    "function '{}' has unresolved dependencies: {:?}",
                    function_name, missing_deps
                )
            }
            PartialReason::NoProgress { stuck_constraints } => {
                write!(
                    f,
                    "no progress - {} constraint(s) still blocked",
                    stuck_constraints.len()
                )
            }
        }
    }
}

/// Result of a solve operation
///
/// Both Complete and Partial are valid outcomes (not errors).
/// Partial means some constraints couldn't be resolved due to
/// missing dependencies, but a valid partial solution exists.
#[derive(Debug, Clone)]
pub enum SolveResult<'src> {
    /// All constraints were fully resolved
    Complete {
        solution: Solution<'src>,
        iterations: usize,
    },

    /// Partial solution - some constraints could not be resolved
    ///
    /// This is NOT an error - it's a valid result indicating
    /// that solving progressed as far as possible with the
    /// given constraints.
    Partial {
        solution: Solution<'src>,
        deferred: Vec<DeferredConstraint<'src>>,
        reason: PartialReason,
        iterations: usize,
    },
}

impl<'src> SolveResult<'src> {
    /// Check if the solve was complete (all constraints resolved)
    pub fn is_complete(&self) -> bool {
        matches!(self, SolveResult::Complete { .. })
    }

    /// Get the solution (works for both complete and partial)
    pub fn solution(&self) -> &Solution<'src> {
        match self {
            SolveResult::Complete { solution, .. } => solution,
            SolveResult::Partial { solution, .. } => solution,
        }
    }

    /// Get number of iterations performed
    pub fn iterations(&self) -> usize {
        match self {
            SolveResult::Complete { iterations, .. } => *iterations,
            SolveResult::Partial { iterations, .. } => *iterations,
        }
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Error types for the solver
#[derive(Debug, Clone, PartialEq)]
pub enum SolverError {
    /// Unsupported type in constraint solving
    UnsupportedType(String),

    /// Variable not found
    UndefinedVariable(String),

    /// Not a primitive type
    NotAPrimitiveType,

    /// Unsupported statement
    UnsupportedStatement(String),

    /// Unsupported expression
    UnsupportedExpression(String),

    /// Context error
    ContextError(String),

    /// Z3 solver returned UNSAT (no solution exists)
    Unsatisfiable,

    /// Z3 solver returned Unknown
    Unknown,

    /// Z3 model evaluation error
    ModelEvaluationError(String),
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolverError::UnsupportedType(ty) => write!(f, "Unsupported type: {}", ty),
            SolverError::UndefinedVariable(var) => write!(f, "Undefined variable: {}", var),
            SolverError::NotAPrimitiveType => write!(f, "Not a primitive type"),
            SolverError::UnsupportedStatement(stmt) => write!(f, "Unsupported statement: {}", stmt),
            SolverError::UnsupportedExpression(expr) => {
                write!(f, "Unsupported expression: {}", expr)
            }
            SolverError::ContextError(msg) => write!(f, "Context error: {}", msg),
            SolverError::Unsatisfiable => write!(f, "No solution exists (UNSAT)"),
            SolverError::Unknown => write!(f, "Z3 solver returned unknown result"),
            SolverError::ModelEvaluationError(msg) => {
                write!(f, "Failed to evaluate Z3 model: {}", msg)
            }
        }
    }
}

impl std::error::Error for SolverError {}

/// Trait for HIR nodes that can be solved as constraints
///
/// This trait allows HIR nodes to translate themselves into Z3 constraints
/// using the solver context. Different node types return different outputs:
/// - Statements return `()` (they add constraints to the solver)
/// - Expressions return Z3 AST nodes
pub trait Solvable<'src, 'arena> {
    /// The output type when solving this node
    type Output;

    /// Solve this node, adding constraints to the context
    fn solve(&self, ctx: &mut SolverContext<'src, 'arena>) -> Result<Self::Output, SolverError>;
}

// ============================================================================
// Phase 2: Modules
// ============================================================================

/// Solver context with tree-based variable management and RAII guards
pub mod context;

// ============================================================================
// Phase 3: Trait Implementations
// ============================================================================

/// Trait implementations for HIR nodes (expressions and statements)
pub mod impls;

// ============================================================================
// Public Re-exports (Phase 2)
// ============================================================================

pub use context::SolverContext;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // PathComponent Tests
    // ========================================================================

    #[test]
    fn test_path_component_field() {
        let field = PathComponent::Field("test_field");
        assert_eq!(field, PathComponent::Field("test_field"));

        // Test Debug
        let debug_str = format!("{:?}", field);
        assert!(debug_str.contains("Field"));
        assert!(debug_str.contains("test_field"));
    }

    #[test]
    fn test_path_component_index() {
        let index = PathComponent::Index(42);
        assert_eq!(index, PathComponent::Index(42));

        // Test Debug
        let debug_str = format!("{:?}", index);
        assert!(debug_str.contains("Index"));
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn test_path_component_equality() {
        assert_eq!(PathComponent::Field("x"), PathComponent::Field("x"));
        assert_ne!(PathComponent::Field("x"), PathComponent::Field("y"));
        assert_eq!(PathComponent::Index(0), PathComponent::Index(0));
        assert_ne!(PathComponent::Index(0), PathComponent::Index(1));
    }

    #[test]
    fn test_path_component_clone() {
        let field = PathComponent::Field("test");
        let cloned = field.clone();
        assert_eq!(field, cloned);

        let index = PathComponent::Index(5);
        let cloned = index.clone();
        assert_eq!(index, cloned);
    }

    // ========================================================================
    // VariablePath Tests
    // ========================================================================

    #[test]
    fn test_variable_path_from_name() {
        let path = VariablePath::from_name("variable");
        assert_eq!(path.to_z3_name(), "variable");
        assert_eq!(path.len(), 1);
        assert!(!path.is_empty());
    }

    #[test]
    fn test_variable_path_empty() {
        let path = VariablePath::empty();
        assert_eq!(path.to_z3_name(), "");
        assert_eq!(path.len(), 0);
        assert!(path.is_empty());
    }

    #[test]
    fn test_variable_path_with_field() {
        let path = VariablePath::from_name("base");
        let nested = path.with_field("field1");
        assert_eq!(nested.to_z3_name(), "base.field1");
        assert_eq!(nested.len(), 2);

        let double_nested = nested.with_field("field2");
        assert_eq!(double_nested.to_z3_name(), "base.field1.field2");
        assert_eq!(double_nested.len(), 3);
    }

    #[test]
    fn test_variable_path_with_index() {
        let path = VariablePath::from_name("array");
        let indexed = path.with_index(0);
        assert_eq!(indexed.to_z3_name(), "array[0]");
        assert_eq!(indexed.len(), 2);

        let double_indexed = indexed.with_index(1);
        assert_eq!(double_indexed.to_z3_name(), "array[0][1]");
        assert_eq!(double_indexed.len(), 3);
    }

    #[test]
    fn test_variable_path_mixed() {
        let path = VariablePath::from_name("points")
            .with_index(0)
            .with_field("x");
        assert_eq!(path.to_z3_name(), "points[0].x");
        assert_eq!(path.len(), 3);
    }

    #[test]
    fn test_variable_path_components() {
        let path = VariablePath::from_name("var")
            .with_field("field")
            .with_index(5);

        let components = path.components();
        assert_eq!(components.len(), 3);
        assert_eq!(components[0], PathComponent::Field("var"));
        assert_eq!(components[1], PathComponent::Field("field"));
        assert_eq!(components[2], PathComponent::Index(5));
    }

    #[test]
    fn test_variable_path_display() {
        let path = VariablePath::from_name("sketch")
            .with_field("entities")
            .with_field("line")
            .with_field("start")
            .with_field("x");

        let display_str = format!("{}", path);
        assert_eq!(display_str, "sketch.entities.line.start.x");
    }

    #[test]
    fn test_variable_path_display_with_indices() {
        let path = VariablePath::from_name("points")
            .with_index(0)
            .with_field("y")
            .with_index(2);

        let display_str = format!("{}", path);
        assert_eq!(display_str, "points[0].y[2]");
    }

    #[test]
    fn test_variable_path_equality() {
        let path1 = VariablePath::from_name("x").with_field("y");
        let path2 = VariablePath::from_name("x").with_field("y");
        let path3 = VariablePath::from_name("x").with_field("z");

        assert_eq!(path1, path2);
        assert_ne!(path1, path3);
    }

    #[test]
    fn test_variable_path_clone() {
        let path = VariablePath::from_name("test").with_index(1);
        let cloned = path.clone();
        assert_eq!(path, cloned);
        assert_eq!(path.to_z3_name(), cloned.to_z3_name());
    }

    // ========================================================================
    // SolverError Tests
    // ========================================================================

    #[test]
    fn test_solver_error_unsupported_type() {
        let err = SolverError::UnsupportedType("CustomType".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "Unsupported type: CustomType");
    }

    #[test]
    fn test_solver_error_undefined_variable() {
        let err = SolverError::UndefinedVariable("unknown_var".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "Undefined variable: unknown_var");
    }

    #[test]
    fn test_solver_error_not_a_primitive_type() {
        let err = SolverError::NotAPrimitiveType;
        let display = format!("{}", err);
        assert_eq!(display, "Not a primitive type");
    }

    #[test]
    fn test_solver_error_unsupported_statement() {
        let err = SolverError::UnsupportedStatement("WhileLoop".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "Unsupported statement: WhileLoop");
    }

    #[test]
    fn test_solver_error_unsupported_expression() {
        let err = SolverError::UnsupportedExpression("Lambda".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "Unsupported expression: Lambda");
    }

    #[test]
    fn test_solver_error_context_error() {
        let err = SolverError::ContextError("Scope mismatch".to_string());
        let display = format!("{}", err);
        assert_eq!(display, "Context error: Scope mismatch");
    }

    #[test]
    fn test_solver_error_debug() {
        let err = SolverError::UnsupportedType("TestType".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("UnsupportedType"));
        assert!(debug_str.contains("TestType"));
    }

    #[test]
    fn test_solver_error_equality() {
        let err1 = SolverError::NotAPrimitiveType;
        let err2 = SolverError::NotAPrimitiveType;
        let err3 = SolverError::UnsupportedType("X".to_string());

        assert_eq!(err1, err2);
        assert_ne!(err1, err3);
    }

    #[test]
    fn test_solver_error_clone() {
        let err = SolverError::ContextError("test".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_solver_error_is_std_error() {
        // Ensure SolverError implements std::error::Error
        let err: Box<dyn std::error::Error> =
            Box::new(SolverError::UnsupportedType("test".to_string()));
        let _ = format!("{}", err);
    }
}
