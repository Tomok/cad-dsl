//! High-Level Intermediate Representation (HIR) - Resolved Expressions
//!
//! This module defines the resolved expression types for the HIR, which differ from
//! the AST in several key ways:

// Allow dead code for now since this module is not yet fully integrated
#![allow(dead_code)]
//!
//! # Differences from AST
//!
//! 1. **Type Resolution**: Every expression has an associated type (`Type<'src, 'arena>`),
//!    determined during semantic analysis.
//!
//! 2. **Name Resolution**: Variables, functions, methods, and fields are linked to their
//!    definitions via arena-allocated references:
//!    - `Var` contains `&'arena VarDefinition`
//!    - `FunctionCall` contains `&'arena FunctionDefinition`
//!    - `MethodCall` contains `&'arena FunctionDefinition`
//!    - `FieldAccess` contains `&'arena FieldDefinition`
//!
//! 3. **Context Resolution**: `ContainerFieldAccess` (dot-prefixed field access in `with`
//!    blocks) contains a reference to the `WithContext` it resolves to.
//!
//! 4. **Arena Allocation**: All cross-references use arena-allocated pointers (`&'arena`)
//!    instead of owned allocations (`Box`). This allows the entire HIR to be allocated
//!    in a single arena and deallocated together, avoiding fragmented heap allocations.
//!
//! 5. **No Precedence Hierarchy**: Unlike the AST which uses subenums to enforce operator
//!    precedence at the type level, the HIR uses a single `ResolvedExprKind` enum since
//!    precedence has already been handled by the parser.
//!
//! # Memory Management
//!
//! The HIR uses `bumpalo::Bump` as an arena allocator:
//! - All resolved expressions are allocated in the arena
//! - Cross-references between HIR nodes use `&'arena` pointers
//! - The entire HIR is deallocated when the arena is dropped
//! - This is faster than individual heap allocations and provides better cache locality
//!
//! # Lifetimes
//!
//! - `'src`: Lifetime of the source code string (for string slices)
//! - `'arena`: Lifetime of the arena allocator (for HIR node references)

use crate::ast::HasSpan;
use crate::hir_context::WithContext;
use crate::hir_definitions::{FieldDefinition, FunctionDefinition, VarDefinition};
use crate::hir_types::ResolvedType;
use crate::lexer::Span;

// ============================================================================
// Resolved Expression Types
// ============================================================================

/// A resolved expression in the HIR with type and span information
///
/// Every expression in the HIR has:
/// - A span for error reporting
/// - A kind (the actual expression variant)
/// - A type (resolved during semantic analysis)
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedExpr<'src, 'arena> {
    /// Source location for error reporting
    pub span: Span,

    /// The kind of expression
    pub kind: ResolvedExprKind<'src, 'arena>,

    /// The resolved type of this expression
    pub ty: &'arena ResolvedType<'src, 'arena>,
}

impl<'src, 'arena> HasSpan for ResolvedExpr<'src, 'arena> {
    fn span(&self) -> Span {
        self.span
    }
}

/// The kind of a resolved expression
///
/// Unlike the AST, this is a single flat enum without precedence hierarchies,
/// since operator precedence has already been handled during parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedExprKind<'src, 'arena> {
    // ========================================================================
    // Variables and References
    // ========================================================================
    /// Variable reference - resolved to its definition
    Var {
        /// The variable name (from source)
        name: &'src str,
        /// Reference to the variable's definition
        definition: &'arena VarDefinition<'src, 'arena>,
    },

    // ========================================================================
    // Function and Method Calls
    // ========================================================================
    /// Function call - resolved to function definition
    FunctionCall {
        /// The function name (from source)
        name: &'src str,
        /// Reference to the function's definition
        function: &'arena FunctionDefinition<'src, 'arena>,
        /// Resolved argument expressions
        args: Vec<&'arena ResolvedExpr<'src, 'arena>>,
    },

    /// Method call - resolved to method definition
    MethodCall {
        /// The receiver expression (what the method is called on)
        receiver: &'arena ResolvedExpr<'src, 'arena>,
        /// The method name (from source)
        method_name: &'src str,
        /// Reference to the method's definition
        method: &'arena FunctionDefinition<'src, 'arena>,
        /// Resolved argument expressions
        args: Vec<&'arena ResolvedExpr<'src, 'arena>>,
    },

    // ========================================================================
    // Field Access
    // ========================================================================
    /// Field access - resolved to field definition
    FieldAccess {
        /// The receiver expression (struct/object being accessed)
        receiver: &'arena ResolvedExpr<'src, 'arena>,
        /// The field name (from source)
        field_name: &'src str,
        /// Reference to the field's definition
        field: &'arena FieldDefinition<'src, 'arena>,
    },

    /// Container field access (dot-prefixed in `with` blocks)
    ///
    /// Example: `.field` or `.field.x` within a `with` block
    /// This resolves to fields on the container being constrained.
    ContainerFieldAccess {
        /// Resolved path to the field
        /// For `.field` -> vec!["field"]
        /// For `.field.x` -> vec!["field", "x"]
        resolved_path: Vec<&'src str>,
        /// Reference to the `with` context this access resolves to
        with_context: &'arena WithContext<'src, 'arena>,
        /// Optional transform applied to the container
        /// (e.g., for accessing transformed geometry)
        transform: Option<&'arena ResolvedExpr<'src, 'arena>>,
    },

    // ========================================================================
    // Binary Operations
    // ========================================================================
    /// Logical AND: `lhs && rhs`
    And {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Logical OR: `lhs || rhs`
    Or {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Equality: `lhs == rhs`
    Eq {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Not equal: `lhs != rhs`
    NotEq {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Less than: `lhs < rhs`
    Lt {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Greater than: `lhs > rhs`
    Gt {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Less than or equal: `lhs <= rhs`
    LtEq {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Greater than or equal: `lhs >= rhs`
    GtEq {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Addition: `lhs + rhs`
    Add {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Subtraction: `lhs - rhs`
    Sub {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Multiplication: `lhs * rhs`
    Mul {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Division: `lhs / rhs`
    Div {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Modulo: `lhs % rhs`
    Mod {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Power: `lhs ^ rhs`
    Pow {
        lhs: &'arena ResolvedExpr<'src, 'arena>,
        rhs: &'arena ResolvedExpr<'src, 'arena>,
    },

    // ========================================================================
    // Unary Operations
    // ========================================================================
    /// Unary negation: `-expr`
    Neg {
        inner: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Unary reference: `&expr`
    /// Used for creating references in the constraint system
    Ref {
        inner: &'arena ResolvedExpr<'src, 'arena>,
    },

    // ========================================================================
    // Literals
    // ========================================================================
    /// Integer literal
    IntLit { value: i32 },

    /// Floating-point literal
    FloatLit { value: f64 },

    /// Boolean literal
    BoolLit { value: bool },

    // ========================================================================
    // Composite Expressions
    // ========================================================================
    /// Struct literal with resolved field assignments
    StructLit {
        /// The struct type name
        name: &'src str,
        /// Resolved field assignments
        fields: Vec<ResolvedStructLitField<'src, 'arena>>,
    },

    /// Array literal
    ArrayLit {
        /// Resolved element expressions
        elements: Vec<&'arena ResolvedExpr<'src, 'arena>>,
    },

    /// Array indexing: `array[index]`
    Index {
        /// The array being indexed
        array: &'arena ResolvedExpr<'src, 'arena>,
        /// The index expression
        index: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Range expression: `start..end`
    Range {
        /// Start of the range
        start: &'arena ResolvedExpr<'src, 'arena>,
        /// End of the range
        end: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Closure expression: `|params| body`
    Closure {
        /// Parameter names
        params: Vec<&'src str>,
        /// Resolved body expression
        body: &'arena ResolvedExpr<'src, 'arena>,
    },

    /// Parenthesized expression: `(expr)`
    /// Kept in HIR for source fidelity and error reporting
    Paren {
        inner: &'arena ResolvedExpr<'src, 'arena>,
    },
}

// ============================================================================
// Struct Literal Field
// ============================================================================

/// A resolved field in a struct literal
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedStructLitField<'src, 'arena> {
    /// Regular field assignment: `field: value`
    Field {
        /// Field name
        name: &'src str,
        /// Resolved value expression
        value: &'arena ResolvedExpr<'src, 'arena>,
        /// Reference to the field definition
        field_def: &'arena FieldDefinition<'src, 'arena>,
        /// Span for error reporting
        span: Span,
    },

    /// Computed property constraint: `method() = value`
    /// Used for constraint-based property assignment
    ComputedProperty {
        /// Method name
        name: &'src str,
        /// Resolved value expression
        value: &'arena ResolvedExpr<'src, 'arena>,
        /// Reference to the method definition
        method_def: &'arena FunctionDefinition<'src, 'arena>,
        /// Span for error reporting
        span: Span,
    },
}

impl<'src, 'arena> HasSpan for ResolvedStructLitField<'src, 'arena> {
    fn span(&self) -> Span {
        match self {
            ResolvedStructLitField::Field { span, .. } => *span,
            ResolvedStructLitField::ComputedProperty { span, .. } => *span,
        }
    }
}

// ============================================================================
// Helper Methods
// ============================================================================

impl<'src, 'arena> ResolvedExpr<'src, 'arena> {
    /// Create a new resolved expression
    pub fn new(
        span: Span,
        kind: ResolvedExprKind<'src, 'arena>,
        ty: &'arena ResolvedType<'src, 'arena>,
    ) -> Self {
        Self { span, kind, ty }
    }

    /// Get the type of this expression
    pub fn ty(&self) -> &'arena ResolvedType<'src, 'arena> {
        self.ty
    }

    /// Get the kind of this expression
    pub fn kind(&self) -> &ResolvedExprKind<'src, 'arena> {
        &self.kind
    }
}
