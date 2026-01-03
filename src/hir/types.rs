//! High-Level IR (HIR) Type Definitions
//!
//! This module defines the type system for the HIR, which represents types after
//! name resolution and type checking have been performed.
//!
#![allow(dead_code)] // Public API for type system in future phases
//! # Arena Allocation
//!
//! The HIR uses arena allocation via `bumpalo::Bump` for memory management. This
//! approach offers several benefits:
//!
//! 1. **Performance**: Arena allocation is extremely fast - allocations are just
//!    pointer bumps, and deallocation happens all at once when the arena is dropped.
//!
//! 2. **Lifetime Management**: By tying all HIR data to the arena's lifetime
//!    (`'arena`), we get compile-time guarantees that HIR data won't outlive the
//!    arena. This eliminates use-after-free bugs.
//!
//! 3. **Simplicity**: We can use direct references (`&'arena T`) instead of
//!    reference-counted pointers or indices. The borrow checker ensures safety.
//!
//! 4. **Cache Locality**: Related data allocated together tends to be stored
//!    nearby in memory, improving CPU cache performance.
//!
//! # Lifetime Parameters
//!
//! Types in this module have two lifetime parameters:
//!
//! - `'src`: Lifetime of the source code string. String slices (`&'src str`) point
//!   directly into the original source, avoiding allocations.
//!
//! - `'arena`: Lifetime of the arena allocator. All HIR data structures allocated
//!   in the arena share this lifetime.
//!
//! # Type Resolution
//!
//! `ResolvedType` represents types that have been fully resolved:
//! - Primitive types (bool, i32, f64, real, algebraic) are represented directly
//! - User-defined types include a reference to their struct definition
//! - Reference types point to their inner type in the arena
//!
//! All types retain their original source spans for error reporting and tooling.

use super::definitions::StructDefinition;
use crate::ast::span::HasSpan;
use crate::lexer::Span;

// ============================================================================
// Resolved Type System
// ============================================================================

/// A fully resolved type in the HIR
///
/// This represents a type after name resolution, where user-defined types
/// have been linked to their definitions. All type data is allocated in
/// the arena and referenced by `'arena` lifetime.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResolvedType<'src, 'arena> {
    /// Boolean type (`bool`)
    Bool { span: Span },

    /// 32-bit signed integer type (`i32`)
    I32 { span: Span },

    /// 64-bit floating point type (`f64`)
    F64 { span: Span },

    /// Mathematical real number with exact precision (`real`)
    ///
    /// Represents arbitrary-precision real numbers for geometric calculations
    /// where floating-point errors are unacceptable.
    Real { span: Span },

    /// Algebraic number type (`algebraic`)
    ///
    /// Represents roots of polynomials with integer coefficients. This is
    /// a subset of real numbers that can be represented exactly.
    Algebraic { span: Span },

    /// Reference type (e.g., `&Point`)
    ///
    /// The inner type is allocated in the arena, so this is a reference to
    /// arena-allocated data, not a box.
    Reference {
        inner: &'arena ResolvedType<'src, 'arena>,
        span: Span,
    },

    /// User-defined type (e.g., `Point`, `Circle`)
    ///
    /// Contains the type name and a reference to the struct definition in the
    /// arena. This allows type checking to access field information and other
    /// metadata.
    UserDefined {
        name: &'src str,
        definition: &'arena StructDefinition<'src, 'arena>,
        span: Span,
    },

    /// Fixed-size array type (e.g., `[i32; 5]`, `[Point; 3]`)
    ///
    /// Arrays have a compile-time-known size. The element type is allocated
    /// in the arena.
    Array {
        element_type: &'arena ResolvedType<'src, 'arena>,
        size: usize,
        span: Span,
    },
}

// ============================================================================
// HasSpan Implementation
// ============================================================================

impl<'src, 'arena> HasSpan for ResolvedType<'src, 'arena> {
    fn span(&self) -> Span {
        match self {
            ResolvedType::Bool { span } => *span,
            ResolvedType::I32 { span } => *span,
            ResolvedType::F64 { span } => *span,
            ResolvedType::Real { span } => *span,
            ResolvedType::Algebraic { span } => *span,
            ResolvedType::Reference { span, .. } => *span,
            ResolvedType::UserDefined { span, .. } => *span,
            ResolvedType::Array { span, .. } => *span,
        }
    }
}

// ============================================================================
// Helper Methods
// ============================================================================

impl<'src, 'arena> ResolvedType<'src, 'arena> {
    /// Returns true if this type is a primitive numeric type (i32, f64, real, algebraic)
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            ResolvedType::I32 { .. }
                | ResolvedType::F64 { .. }
                | ResolvedType::Real { .. }
                | ResolvedType::Algebraic { .. }
        )
    }

    /// Returns true if this type is a reference type
    pub fn is_reference(&self) -> bool {
        matches!(self, ResolvedType::Reference { .. })
    }

    /// Returns true if this type is a user-defined type
    pub fn is_user_defined(&self) -> bool {
        matches!(self, ResolvedType::UserDefined { .. })
    }

    /// If this is a reference type, returns the inner type
    pub fn as_reference(&self) -> Option<&'arena ResolvedType<'src, 'arena>> {
        match self {
            ResolvedType::Reference { inner, .. } => Some(inner),
            _ => None,
        }
    }

    /// If this is a user-defined type, returns the name and definition
    pub fn as_user_defined(&self) -> Option<(&'src str, &'arena StructDefinition<'src, 'arena>)> {
        match self {
            ResolvedType::UserDefined {
                name, definition, ..
            } => Some((name, definition)),
            _ => None,
        }
    }
}
