use crate::ast::expr::Expr;
use crate::ast::span::HasSpan;
use crate::lexer::Span;

// ============================================================================
// Type Annotations
// ============================================================================

/// Type annotations for variable declarations and function parameters
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Boolean type
    Bool { span: Span },
    /// 32-bit integer type
    I32 { span: Span },
    /// 64-bit floating point type
    F64 { span: Span },
    /// Mathematical real number with exact precision
    Real { span: Span },
    /// Algebraic number (roots of polynomials with integer coefficients)
    Algebraic { span: Span },
    /// Reference type (e.g., &Point)
    Reference { inner: Box<Type>, span: Span },
    /// User-defined type (e.g., Point, Circle)
    UserDefined { name: String, span: Span },
}

impl HasSpan for Type {
    fn span(&self) -> Span {
        match self {
            Type::Bool { span } => *span,
            Type::I32 { span } => *span,
            Type::F64 { span } => *span,
            Type::Real { span } => *span,
            Type::Algebraic { span } => *span,
            Type::Reference { span, .. } => *span,
            Type::UserDefined { span, .. } => *span,
        }
    }
}

// ============================================================================
// Function Parameters
// ============================================================================

/// Function parameter with name and type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionParam {
    pub name: String,
    pub name_span: Span,
    pub type_annotation: Type,
    pub span: Span,
}

impl HasSpan for FunctionParam {
    fn span(&self) -> Span {
        self.span
    }
}

// ============================================================================
// Struct Field
// ============================================================================

/// Struct field with name and type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructField {
    pub name: String,
    pub name_span: Span,
    pub type_annotation: Type,
    pub span: Span,
}

impl HasSpan for StructField {
    fn span(&self) -> Span {
        self.span
    }
}

// ============================================================================
// Statements
// ============================================================================

/// Statements perform declarations and actions (not expressions)
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt<'src> {
    /// Variable declaration with optional type annotation and initialization
    /// Examples:
    ///   let x: i32 = 42;
    ///   let y: bool;
    ///   let z = 3.14;
    ///   let container.field: Point = point(0mm, 0mm);
    ///   let sketch.entities.p1: Point = point(10mm, 10mm);
    Let {
        /// Path segments for the variable name
        /// - Simple let: `let x` -> vec![("x", span)]
        /// - Container field: `let container.field` -> vec![("container", span1), ("field", span2)]
        /// - Nested: `let a.b.c` -> vec![("a", span1), ("b", span2), ("c", span3)]
        name_path: Vec<(&'src str, Span)>,
        type_annotation: Option<Type>,
        init: Option<Expr<'src>>,
        span: Span,
    },

    /// Assignment statement (creates a constraint)
    /// Examples:
    ///   x = 42;
    ///   width = 100;
    ///   result = a + b;
    /// Note: This is for simple variable assignment only.
    /// Field assignment uses Stmt::FieldAssignment.
    Assignment {
        /// Variable name being assigned to
        name: &'src str,
        /// Span of the variable name
        name_span: Span,
        /// Value expression
        value: Expr<'src>,
        /// Overall span of the statement
        span: Span,
    },

    /// Field assignment statement (assigns to object fields)
    /// Examples:
    ///   obj.field = 42;
    ///   sketch.origin.x = 10mm;
    ///   container.entities.p1.x = 5;
    /// Note: The path must have at least 2 segments (object.field).
    FieldAssignment {
        /// Path to the field being assigned
        /// - obj.field -> vec![("obj", span1), ("field", span2)]
        /// - obj.nested.field -> vec![("obj", span1), ("nested", span2), ("field", span3)]
        field_path: Vec<(&'src str, Span)>,
        /// Value expression
        value: Expr<'src>,
        /// Overall span of the statement
        span: Span,
    },

    /// For loop over ranges or arrays
    /// Examples:
    ///   for i in 0..10 { ... }
    ///   for elem in array { ... }
    For {
        loop_var: &'src str,
        loop_var_span: Span,
        iterator: Expr<'src>,
        body: Vec<Stmt<'src>>,
        span: Span,
    },

    /// Function definition with parameters, return type, and body
    /// Examples:
    ///   fn distance(p1: &Point, p2: &Point) -> Length { ... }
    ///   fn area() -> f64 { self.width * self.height }
    FunctionDef {
        name: String,
        name_span: Span,
        params: Vec<FunctionParam>,
        return_type: Type,
        body: Vec<Stmt<'src>>,
        return_expr: Option<Expr<'src>>,
        span: Span,
    },

    /// Struct definition with fields and methods
    /// Examples:
    ///   struct Point { x: f64, y: f64 }
    ///   struct Circle { center: Point, radius: f64, fn area() -> f64 { ... } }
    ///   struct Sketch { container entities, origin: Point }
    StructDef {
        name: String,
        name_span: Span,
        /// Optional container field name
        container: Option<(String, Span)>,
        fields: Vec<StructField>,
        methods: Vec<Stmt<'src>>,
        span: Span,
    },
}

impl<'src> HasSpan for Stmt<'src> {
    fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Assignment { span, .. } => *span,
            Stmt::FieldAssignment { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::FunctionDef { span, .. } => *span,
            Stmt::StructDef { span, .. } => *span,
        }
    }
}
