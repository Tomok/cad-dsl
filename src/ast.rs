//! Abstract Syntax Tree (AST) definitions for the CAD-DSL language
//!
//! This module contains the type-safe AST representation with operator precedence
//! enforced at the type level using subenums.
//!
//! # Module Structure
//!
//! - `span`: Span access trait for AST nodes
//! - `types`: Type annotations and statement definitions
//! - `expr`: Expression AST with type-safe operator precedence
//! - `display`: Display trait implementations for pretty-printing
//! - `conversions`: Type conversions for parser convenience
//! - `tests`: Unit tests for AST functionality

// ============================================================================
// Submodules
// ============================================================================

mod conversions;
mod display;
pub mod expr;
pub mod span;
pub mod types;

#[cfg(test)]
mod tests;

// ============================================================================
// Re-exports
// ============================================================================

// Re-export span trait
pub use span::HasSpan;

// Re-export types
pub use types::{Stmt, Type};

// Re-export all expression types
pub use expr::{AddLhs, AddRhs, Atom, CmpLhs, CmpRhs, Expr, MulLhs, MulRhs, PowLhs, PowRhs};
