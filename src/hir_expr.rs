//! High-Level Intermediate Representation (HIR) - Resolved Expressions
//!
//! This module defines the resolved expression types for the HIR, which differ from
//! the AST in several key ways:
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
use crate::hir_definitions::{
    FieldDefinition, FunctionDefinition, StructDefinition, VarDefinition,
};
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
    #[allow(dead_code)] // Planned for method resolution in future phases
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
    #[allow(dead_code)] // Planned for field access resolution in future phases
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
    #[allow(dead_code)] // Public API for future compiler phases
    pub fn ty(&self) -> &'arena ResolvedType<'src, 'arena> {
        self.ty
    }

    /// Get the kind of this expression
    #[allow(dead_code)] // Public API for future compiler phases
    pub fn kind(&self) -> &ResolvedExprKind<'src, 'arena> {
        &self.kind
    }
}

// ============================================================================
// Resolved Statement Types
// ============================================================================

/// A resolved statement in the HIR
///
/// Statements in the HIR are similar to expressions but represent actions
/// rather than values. Each statement has:
/// - A span for error reporting
/// - A kind (the actual statement variant)
///
/// Unlike expressions, statements don't have an associated type.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedStmt<'src, 'arena> {
    /// Source location for error reporting
    pub span: Span,

    /// The kind of statement
    pub kind: ResolvedStmtKind<'src, 'arena>,
}

impl<'src, 'arena> HasSpan for ResolvedStmt<'src, 'arena> {
    fn span(&self) -> Span {
        self.span
    }
}

/// The kind of a resolved statement
///
/// This enum represents all statement types in the HIR, with all names
/// and types resolved to their definitions.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedStmtKind<'src, 'arena> {
    // ========================================================================
    // Variable Declarations and Assignments
    // ========================================================================
    /// Variable declaration with optional initializer
    ///
    /// Example: `let x = 5;` or `let Point p;`
    Let {
        /// Whether the variable name starts with a dot (for container fields)
        dot_prefix: bool,
        /// The path to the variable name (for nested declarations)
        name_path: Vec<(&'src str, Span)>,
        /// Reference to the variable's definition
        var_def: &'arena VarDefinition<'src, 'arena>,
        /// Optional initializer expression
        init: Option<&'arena ResolvedExpr<'src, 'arena>>,
        /// Span for the entire statement
        span: Span,
    },

    /// Assignment to an existing variable
    ///
    /// Example: `x = 10;`
    Assignment {
        /// Reference to the variable being assigned
        var_def: &'arena VarDefinition<'src, 'arena>,
        /// The value expression
        value: &'arena ResolvedExpr<'src, 'arena>,
        /// Span for the entire statement
        span: Span,
    },

    /// Assignment to a field
    ///
    /// Example: `point.x = 5;`
    FieldAssignment {
        /// The target field expression (resolved)
        target: &'arena ResolvedExpr<'src, 'arena>,
        /// The value expression
        value: &'arena ResolvedExpr<'src, 'arena>,
        /// Span for the entire statement
        span: Span,
    },

    // ========================================================================
    // Control Flow
    // ========================================================================
    /// Conditional statement
    ///
    /// Example: `if x > 0 { ... } else { ... }`
    If {
        /// The condition expression
        condition: &'arena ResolvedExpr<'src, 'arena>,
        /// The statements in the then branch
        then_branch: Vec<&'arena ResolvedStmt<'src, 'arena>>,
        /// Optional else branch statements
        else_branch: Option<Vec<&'arena ResolvedStmt<'src, 'arena>>>,
        /// Span for the entire statement
        span: Span,
    },

    /// Loop statement
    ///
    /// Example: `for i in 0..10 { ... }`
    For {
        /// The loop variable definition
        loop_var_def: &'arena VarDefinition<'src, 'arena>,
        /// The iterator expression
        iterator: &'arena ResolvedExpr<'src, 'arena>,
        /// The loop body statements
        body: Vec<&'arena ResolvedStmt<'src, 'arena>>,
        /// Span for the entire statement
        span: Span,
    },

    /// Return statement
    ///
    /// Example: `return x;` or `return;`
    Return {
        /// Optional return value expression
        value: Option<&'arena ResolvedExpr<'src, 'arena>>,
        /// Span for the entire statement
        span: Span,
    },

    // ========================================================================
    // Definitions
    // ========================================================================
    /// Function definition
    ///
    /// Example: `fn add(x: i32, y: i32) -> i32 { return x + y; }`
    FunctionDef {
        /// Reference to the function's definition
        func_def: &'arena FunctionDefinition<'src, 'arena>,
        /// The function body statements
        body: Vec<&'arena ResolvedStmt<'src, 'arena>>,
        /// Optional return expression (implicit return)
        return_expr: Option<&'arena ResolvedExpr<'src, 'arena>>,
        /// Span for the entire statement
        span: Span,
    },

    /// Struct definition
    ///
    /// Example: `struct Point { x: f64, y: f64 }`
    StructDef {
        /// Reference to the struct's definition
        struct_def: &'arena StructDefinition<'src, 'arena>,
        /// The method definitions (functions defined within the struct)
        methods: Vec<&'arena ResolvedStmt<'src, 'arena>>,
        /// Span for the entire statement
        span: Span,
    },

    // ========================================================================
    // Other Statements
    // ========================================================================
    /// Expression statement
    ///
    /// Example: `foo();` (a function call as a statement)
    Expression {
        /// The expression being evaluated
        expr: &'arena ResolvedExpr<'src, 'arena>,
        /// Span for the entire statement
        span: Span,
    },

    /// Block of statements
    ///
    /// Example: `{ let x = 5; foo(x); }`
    Block {
        /// The statements in the block
        statements: Vec<&'arena ResolvedStmt<'src, 'arena>>,
        /// Span for the entire statement
        span: Span,
    },

    /// With-context statement for constraint blocks
    ///
    /// Example: `with container { .field = value; }`
    With {
        /// Reference to the with-context
        with_context: &'arena WithContext<'src, 'arena>,
        /// The statements in the with block
        body: Vec<&'arena ResolvedStmt<'src, 'arena>>,
        /// Span for the entire statement
        span: Span,
    },
}

// ============================================================================
// Helper Methods
// ============================================================================

impl<'src, 'arena> ResolvedStmt<'src, 'arena> {
    /// Create a new resolved statement
    pub fn new(span: Span, kind: ResolvedStmtKind<'src, 'arena>) -> Self {
        Self { span, kind }
    }

    /// Get the kind of this statement
    #[allow(dead_code)] // Public API for future compiler phases
    pub fn kind(&self) -> &ResolvedStmtKind<'src, 'arena> {
        &self.kind
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir_definitions::{
        FieldDefinition, FunctionDefinition, FunctionParam, StructDefinition, VarDefinition,
    };
    use crate::hir_types::ResolvedType;
    use crate::lexer::LineColumn;
    use bumpalo::Bump;

    /// Helper to create a test span
    fn test_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 0 },
            lines: 0,
            end_column: 10,
        }
    }

    /// Helper to create a test type
    fn test_type() -> ResolvedType<'static, 'static> {
        ResolvedType::I32 { span: test_span() }
    }

    /// Helper to create a test expression
    fn test_expr<'src, 'arena>(
        arena: &'arena Bump,
        ty: &'arena ResolvedType<'src, 'arena>,
    ) -> &'arena ResolvedExpr<'src, 'arena> {
        arena.alloc(ResolvedExpr {
            span: test_span(),
            kind: ResolvedExprKind::IntLit { value: 42 },
            ty,
        })
    }

    /// Helper to create a test statement
    fn test_stmt<'src, 'arena>(
        arena: &'arena Bump,
        kind: ResolvedStmtKind<'src, 'arena>,
    ) -> &'arena ResolvedStmt<'src, 'arena> {
        arena.alloc(ResolvedStmt {
            span: test_span(),
            kind,
        })
    }

    #[test]
    fn test_let_stmt_construction() {
        let arena = Bump::new();
        let var_type = arena.alloc(test_type());
        let init_expr = test_expr(&arena, var_type);
        let var_def = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(test_type()),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::Let {
                dot_prefix: false,
                name_path: vec![("x", test_span())],
                var_def,
                init: Some(init_expr),
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(stmt.kind, ResolvedStmtKind::Let { .. }));
    }

    #[test]
    fn test_assignment_stmt_construction() {
        let arena = Bump::new();
        let var_type = arena.alloc(test_type());
        let value_expr = test_expr(&arena, var_type);
        let var_def = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(test_type()),
            init: None,
            scope_level: 0,
            span: test_span(),
        });

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::Assignment {
                var_def,
                value: value_expr,
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(stmt.kind, ResolvedStmtKind::Assignment { .. }));
    }

    #[test]
    fn test_field_assignment_stmt_construction() {
        let arena = Bump::new();
        let var_type = arena.alloc(test_type());
        let target = test_expr(&arena, var_type);
        let value = test_expr(&arena, var_type);

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::FieldAssignment {
                target,
                value,
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(
            stmt.kind,
            ResolvedStmtKind::FieldAssignment { .. }
        ));
    }

    #[test]
    fn test_if_stmt_construction() {
        let arena = Bump::new();
        let bool_type = arena.alloc(ResolvedType::Bool { span: test_span() });
        let condition = test_expr(&arena, bool_type);
        let then_stmt = test_stmt(
            &arena,
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );
        let else_stmt = test_stmt(
            &arena,
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::If {
                condition,
                then_branch: vec![then_stmt],
                else_branch: Some(vec![else_stmt]),
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(stmt.kind, ResolvedStmtKind::If { .. }));
    }

    #[test]
    fn test_for_stmt_construction() {
        let arena = Bump::new();
        let loop_var_def = arena.alloc(VarDefinition {
            name: "i",
            name_span: test_span(),
            var_type: Some(test_type()),
            init: None,
            scope_level: 1,
            span: test_span(),
        });
        let iter_type = arena.alloc(test_type());
        let iterator = test_expr(&arena, iter_type);
        let body_stmt = test_stmt(
            &arena,
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::For {
                loop_var_def,
                iterator,
                body: vec![body_stmt],
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(stmt.kind, ResolvedStmtKind::For { .. }));
    }

    #[test]
    fn test_return_stmt_construction() {
        let arena = Bump::new();
        let var_type = arena.alloc(test_type());
        let value = test_expr(&arena, var_type);

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::Return {
                value: Some(value),
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(stmt.kind, ResolvedStmtKind::Return { .. }));
    }

    #[test]
    fn test_function_def_stmt_construction() {
        let arena = Bump::new();
        let func_def = arena.alloc(FunctionDefinition {
            name: "foo",
            name_span: test_span(),
            params: vec![FunctionParam::new(
                "x",
                test_span(),
                test_type(),
                test_span(),
            )],
            return_type: test_type(),
            body: vec![],
            parent_struct: None,
            span: test_span(),
        });
        let body_stmt = test_stmt(
            &arena,
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );
        let var_type = arena.alloc(test_type());
        let return_expr = test_expr(&arena, var_type);

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::FunctionDef {
                func_def,
                body: vec![body_stmt],
                return_expr: Some(return_expr),
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(stmt.kind, ResolvedStmtKind::FunctionDef { .. }));
    }

    #[test]
    fn test_struct_def_stmt_construction() {
        let arena = Bump::new();
        let field_def = arena.alloc(FieldDefinition::new(
            "x",
            test_span(),
            ResolvedType::F64 { span: test_span() },
            test_span(),
        ));
        let struct_def = arena.alloc(StructDefinition {
            name: "Point",
            name_span: test_span(),
            fields: vec![field_def],
            methods: vec![],
            container_field: None,
            span: test_span(),
        });
        let method_stmt = test_stmt(
            &arena,
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::StructDef {
                struct_def,
                methods: vec![method_stmt],
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(stmt.kind, ResolvedStmtKind::StructDef { .. }));
    }

    #[test]
    fn test_expression_stmt_construction() {
        let arena = Bump::new();
        let var_type = arena.alloc(test_type());
        let expr = test_expr(&arena, var_type);

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::Expression {
                expr,
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(stmt.kind, ResolvedStmtKind::Expression { .. }));
    }

    #[test]
    fn test_block_stmt_construction() {
        let arena = Bump::new();
        let inner_stmt = test_stmt(
            &arena,
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::Block {
                statements: vec![inner_stmt],
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(stmt.kind, ResolvedStmtKind::Block { .. }));
    }

    #[test]
    fn test_with_stmt_construction() {
        let arena = Bump::new();
        let var_type = arena.alloc(test_type());
        let context_expr = test_expr(&arena, var_type);
        let with_context = arena.alloc(WithContext {
            context_expr,
            container_field: None,
            transforms: vec![],
        });
        let body_stmt = test_stmt(
            &arena,
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );

        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::With {
                with_context,
                body: vec![body_stmt],
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), test_span());
        assert!(matches!(stmt.kind, ResolvedStmtKind::With { .. }));
    }

    #[test]
    fn test_has_span_implementation() {
        let custom_span = Span {
            start: LineColumn { line: 2, column: 5 },
            lines: 0,
            end_column: 15,
        };
        let stmt = ResolvedStmt::new(
            custom_span,
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );

        assert_eq!(stmt.span(), custom_span);
    }

    #[test]
    fn test_clone_implementation() {
        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );

        let cloned = stmt.clone();
        assert_eq!(stmt, cloned);
    }

    #[test]
    fn test_partial_eq_implementation() {
        let stmt1 = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );
        let stmt2 = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );

        assert_eq!(stmt1, stmt2);
    }

    #[test]
    fn test_stmt_helper_methods() {
        let stmt = ResolvedStmt::new(
            test_span(),
            ResolvedStmtKind::Return {
                value: None,
                span: test_span(),
            },
        );

        assert!(matches!(
            stmt.kind(),
            ResolvedStmtKind::Return { value: None, .. }
        ));
    }
}
