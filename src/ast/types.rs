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
    Let {
        name: &'src str,
        name_span: Span,
        type_annotation: Option<Type>,
        init: Option<Expr<'src>>,
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
}

impl<'src> HasSpan for Stmt<'src> {
    fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::FunctionDef { span, .. } => *span,
        }
    }
}
