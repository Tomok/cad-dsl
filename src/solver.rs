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

pub use recursive_struct_detector::detect_cycles;
pub use solution_formatter::{SolutionFormatter, SolutionFormatterError};
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
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolverError::UnsupportedType(ty) => write!(f, "Unsupported type: {}", ty),
            SolverError::UndefinedVariable(var) => write!(f, "Undefined variable: {}", var),
            SolverError::NotAPrimitiveType => write!(f, "Not a primitive type"),
            SolverError::UnsupportedStatement(stmt) => write!(f, "Unsupported statement: {}", stmt),
            SolverError::UnsupportedExpression(expr) => write!(f, "Unsupported expression: {}", expr),
            SolverError::ContextError(msg) => write!(f, "Context error: {}", msg),
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
    fn solve(
        &self,
        ctx: &mut SolverContext<'src, 'arena>,
    ) -> Result<Self::Output, SolverError>;
}

// ============================================================================
// Phase 2: Modules
// ============================================================================

/// Solver context with tree-based variable management and RAII guards
pub mod context;

// ============================================================================
// Future Modules (Phase 3+)
// ============================================================================

// TODO: Phase 3 - Trait implementations
// pub mod impls;       // expr.rs, stmt.rs, etc.

// ============================================================================
// Public Re-exports (Phase 2)
// ============================================================================

pub use context::SolverContext;
