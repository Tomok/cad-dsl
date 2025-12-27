use subenum::subenum;

use crate::lexer::Span;

// ============================================================================
// Span Access Trait
// ============================================================================

/// Trait for AST nodes that have span information
pub trait HasSpan {
    /// Returns the span of this AST node
    fn span(&self) -> Span;
}

// ============================================================================
// Type Annotations
// ============================================================================

/// Type annotations for variable declarations
/// Currently includes only types without units
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
}

// ============================================================================
// Expression AST with Type-Safe Operator Precedence
// ============================================================================

/// Top-level expression type with operator precedence hierarchy
///
/// Hierarchy with separate Left/Right-hand side types:
/// - Expr: All variants (top-level)
/// - LogLhs: And, Or, Eq, NotEq, Add, Sub, Paren, Mul, Div, Pow, Var, IntLit, FloatLit, BoolLit (left side of logical ops)
/// - LogRhs: Paren, Eq, NotEq, Add, Sub, Mul, Div, Pow, Var, IntLit, FloatLit, BoolLit (right side of logical ops, NO And/Or)
/// - CmpLhs: Eq, Add, Sub, Paren, Mul, Div, Pow, Var, IntLit, FloatLit, BoolLit (left side of ==)
/// - CmpRhs: Paren, Add, Sub, Mul, Div, Pow, Var, IntLit, FloatLit, BoolLit (right side of ==, NO Eq)
/// - AddLhs: Add, Sub, Paren, Mul, Div, Pow, Var, IntLit, FloatLit, BoolLit (left side of +/-)
/// - AddRhs: Paren, Mul, Div, Pow, Var, IntLit, FloatLit, BoolLit (right side of +/-, NO Add/Sub)
/// - MulLhs: Paren, Mul, Div, Pow, Var, IntLit, FloatLit, BoolLit (left side of *//, NO Add/Sub)
/// - MulRhs: Paren, Pow, Var, IntLit, FloatLit, BoolLit (right side of *//, NO Mul/Div)
/// - PowLhs: Paren, Var, IntLit, FloatLit, BoolLit (left side of ^, NO operators)
/// - PowRhs: Paren, Pow, Var, IntLit, FloatLit, BoolLit (right side of ^, allows Pow for right-associativity)
/// - Atom: Var, IntLit, FloatLit, BoolLit (only literals and variables)
///
/// This ensures:
/// - Logical RHS cannot contain logical operators (enforces precedence)
/// - Comparison RHS cannot contain comparison operators (enforces precedence)
/// - Addition RHS cannot contain addition/subtraction (enforces precedence)
/// - Multiplication RHS cannot contain multiplication/division (enforces precedence)
/// - Power is right-associative (PowRhs can contain Pow, PowLhs cannot)
/// - Left-hand sides allow recursion at the same precedence level (left-associativity for logical, +, -, *, /)
/// - Right-hand sides enforce higher precedence
#[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    // Logical AND - in CmpLhs (same level as equality operators)
    // lhs can be And/Or, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    And {
        lhs: Box<CmpLhs<'src>>,
        rhs: Box<CmpRhs<'src>>,
        span: Span,
    },

    // Logical OR - in CmpLhs (same level as equality operators)
    // lhs can be And/Or, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    Or {
        lhs: Box<CmpLhs<'src>>,
        rhs: Box<CmpRhs<'src>>,
        span: Span,
    },

    // Equality - in CmpLhs only
    // lhs can be Eq, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    Eq {
        lhs: Box<CmpLhs<'src>>,
        rhs: Box<CmpRhs<'src>>,
        span: Span,
    },

    // Not Equal - in CmpLhs only
    // lhs can be NotEq, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    NotEq {
        lhs: Box<CmpLhs<'src>>,
        rhs: Box<CmpRhs<'src>>,
        span: Span,
    },

    // Addition - in CmpLhs, CmpRhs, AddLhs
    // lhs can be Add/Sub, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs, CmpRhs, AddLhs)]
    Add {
        lhs: Box<AddLhs<'src>>,
        rhs: Box<AddRhs<'src>>,
        span: Span,
    },

    // Subtraction - in CmpLhs, CmpRhs, AddLhs
    #[subenum(CmpLhs, CmpRhs, AddLhs)]
    Sub {
        lhs: Box<AddLhs<'src>>,
        rhs: Box<AddRhs<'src>>,
        span: Span,
    },

    // Parentheses - in all contexts except Atom (resets precedence)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs)]
    Paren { inner: Box<Expr<'src>>, span: Span },

    // Multiplication - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs
    // lhs can be Mul/Div, rhs cannot (enforces left-associativity)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs)]
    Mul {
        lhs: Box<MulLhs<'src>>,
        rhs: Box<MulRhs<'src>>,
        span: Span,
    },

    // Division - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs)]
    Div {
        lhs: Box<MulLhs<'src>>,
        rhs: Box<MulRhs<'src>>,
        span: Span,
    },

    // Modulo - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs)]
    Mod {
        lhs: Box<MulLhs<'src>>,
        rhs: Box<MulRhs<'src>>,
        span: Span,
    },

    // Power - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowRhs
    // lhs cannot be Pow (enforces right-associativity), rhs can be Pow
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowRhs)]
    Pow {
        lhs: Box<PowLhs<'src>>,
        rhs: Box<PowRhs<'src>>,
        span: Span,
    },

    // Unary negation - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs
    // Higher precedence than power (binds tighter)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs)]
    Neg {
        inner: Box<PowLhs<'src>>,
        span: Span,
    },

    // Unary reference - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs
    // Higher precedence than power (binds tighter)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs)]
    Ref {
        inner: Box<PowLhs<'src>>,
        span: Span,
    },

    // Variable reference - in all levels
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    Var { name: &'src str, span: Span },

    // Integer literal - in all levels
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    IntLit { value: i32, span: Span },

    // Float literal - in all levels
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    FloatLit { value: f64, span: Span },

    // Boolean literal - in all levels
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    BoolLit { value: bool, span: Span },

    // Function call - in all levels (high precedence like atoms)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    Call {
        name: &'src str,
        args: Vec<Expr<'src>>,
        span: Span,
    },
}

// ============================================================================
// HasSpan Implementations
// ============================================================================

impl HasSpan for Type {
    fn span(&self) -> Span {
        match self {
            Type::Bool { span } => *span,
            Type::I32 { span } => *span,
            Type::F64 { span } => *span,
            Type::Real { span } => *span,
            Type::Algebraic { span } => *span,
        }
    }
}

impl<'src> HasSpan for Stmt<'src> {
    fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
        }
    }
}

impl<'src> HasSpan for Expr<'src> {
    fn span(&self) -> Span {
        match self {
            Expr::And { span, .. } => *span,
            Expr::Or { span, .. } => *span,
            Expr::Eq { span, .. } => *span,
            Expr::NotEq { span, .. } => *span,
            Expr::Add { span, .. } => *span,
            Expr::Sub { span, .. } => *span,
            Expr::Paren { span, .. } => *span,
            Expr::Mul { span, .. } => *span,
            Expr::Div { span, .. } => *span,
            Expr::Mod { span, .. } => *span,
            Expr::Pow { span, .. } => *span,
            Expr::Neg { span, .. } => *span,
            Expr::Ref { span, .. } => *span,
            Expr::Var { span, .. } => *span,
            Expr::IntLit { span, .. } => *span,
            Expr::FloatLit { span, .. } => *span,
            Expr::BoolLit { span, .. } => *span,
            Expr::Call { span, .. } => *span,
        }
    }
}

impl<'src> HasSpan for CmpLhs<'src> {
    fn span(&self) -> Span {
        match self {
            CmpLhs::And { span, .. } => *span,
            CmpLhs::Or { span, .. } => *span,
            CmpLhs::Eq { span, .. } => *span,
            CmpLhs::NotEq { span, .. } => *span,
            CmpLhs::Add { span, .. } => *span,
            CmpLhs::Sub { span, .. } => *span,
            CmpLhs::Paren { span, .. } => *span,
            CmpLhs::Mul { span, .. } => *span,
            CmpLhs::Div { span, .. } => *span,
            CmpLhs::Mod { span, .. } => *span,
            CmpLhs::Pow { span, .. } => *span,
            CmpLhs::Neg { span, .. } => *span,
            CmpLhs::Ref { span, .. } => *span,
            CmpLhs::Var { span, .. } => *span,
            CmpLhs::IntLit { span, .. } => *span,
            CmpLhs::FloatLit { span, .. } => *span,
            CmpLhs::BoolLit { span, .. } => *span,
            CmpLhs::Call { span, .. } => *span,
        }
    }
}

impl<'src> HasSpan for CmpRhs<'src> {
    fn span(&self) -> Span {
        match self {
            CmpRhs::Add { span, .. } => *span,
            CmpRhs::Sub { span, .. } => *span,
            CmpRhs::Paren { span, .. } => *span,
            CmpRhs::Mul { span, .. } => *span,
            CmpRhs::Div { span, .. } => *span,
            CmpRhs::Mod { span, .. } => *span,
            CmpRhs::Pow { span, .. } => *span,
            CmpRhs::Neg { span, .. } => *span,
            CmpRhs::Ref { span, .. } => *span,
            CmpRhs::Var { span, .. } => *span,
            CmpRhs::IntLit { span, .. } => *span,
            CmpRhs::FloatLit { span, .. } => *span,
            CmpRhs::BoolLit { span, .. } => *span,
            CmpRhs::Call { span, .. } => *span,
        }
    }
}

impl<'src> HasSpan for AddLhs<'src> {
    fn span(&self) -> Span {
        match self {
            AddLhs::Add { span, .. } => *span,
            AddLhs::Sub { span, .. } => *span,
            AddLhs::Paren { span, .. } => *span,
            AddLhs::Mul { span, .. } => *span,
            AddLhs::Div { span, .. } => *span,
            AddLhs::Mod { span, .. } => *span,
            AddLhs::Pow { span, .. } => *span,
            AddLhs::Neg { span, .. } => *span,
            AddLhs::Ref { span, .. } => *span,
            AddLhs::Var { span, .. } => *span,
            AddLhs::IntLit { span, .. } => *span,
            AddLhs::FloatLit { span, .. } => *span,
            AddLhs::BoolLit { span, .. } => *span,
            AddLhs::Call { span, .. } => *span,
        }
    }
}

impl<'src> HasSpan for AddRhs<'src> {
    fn span(&self) -> Span {
        match self {
            AddRhs::Paren { span, .. } => *span,
            AddRhs::Mul { span, .. } => *span,
            AddRhs::Div { span, .. } => *span,
            AddRhs::Mod { span, .. } => *span,
            AddRhs::Pow { span, .. } => *span,
            AddRhs::Neg { span, .. } => *span,
            AddRhs::Ref { span, .. } => *span,
            AddRhs::Var { span, .. } => *span,
            AddRhs::IntLit { span, .. } => *span,
            AddRhs::FloatLit { span, .. } => *span,
            AddRhs::BoolLit { span, .. } => *span,
            AddRhs::Call { span, .. } => *span,
        }
    }
}

impl<'src> HasSpan for MulLhs<'src> {
    fn span(&self) -> Span {
        match self {
            MulLhs::Paren { span, .. } => *span,
            MulLhs::Mul { span, .. } => *span,
            MulLhs::Div { span, .. } => *span,
            MulLhs::Mod { span, .. } => *span,
            MulLhs::Pow { span, .. } => *span,
            MulLhs::Neg { span, .. } => *span,
            MulLhs::Ref { span, .. } => *span,
            MulLhs::Var { span, .. } => *span,
            MulLhs::IntLit { span, .. } => *span,
            MulLhs::FloatLit { span, .. } => *span,
            MulLhs::BoolLit { span, .. } => *span,
            MulLhs::Call { span, .. } => *span,
        }
    }
}

impl<'src> HasSpan for MulRhs<'src> {
    fn span(&self) -> Span {
        match self {
            MulRhs::Paren { span, .. } => *span,
            MulRhs::Pow { span, .. } => *span,
            MulRhs::Neg { span, .. } => *span,
            MulRhs::Ref { span, .. } => *span,
            MulRhs::Var { span, .. } => *span,
            MulRhs::IntLit { span, .. } => *span,
            MulRhs::FloatLit { span, .. } => *span,
            MulRhs::BoolLit { span, .. } => *span,
            MulRhs::Call { span, .. } => *span,
        }
    }
}

impl<'src> HasSpan for PowLhs<'src> {
    fn span(&self) -> Span {
        match self {
            PowLhs::Paren { span, .. } => *span,
            PowLhs::Neg { span, .. } => *span,
            PowLhs::Ref { span, .. } => *span,
            PowLhs::Var { span, .. } => *span,
            PowLhs::IntLit { span, .. } => *span,
            PowLhs::FloatLit { span, .. } => *span,
            PowLhs::BoolLit { span, .. } => *span,
            PowLhs::Call { span, .. } => *span,
        }
    }
}

impl<'src> HasSpan for PowRhs<'src> {
    fn span(&self) -> Span {
        match self {
            PowRhs::Paren { span, .. } => *span,
            PowRhs::Pow { span, .. } => *span,
            PowRhs::Neg { span, .. } => *span,
            PowRhs::Ref { span, .. } => *span,
            PowRhs::Var { span, .. } => *span,
            PowRhs::IntLit { span, .. } => *span,
            PowRhs::FloatLit { span, .. } => *span,
            PowRhs::BoolLit { span, .. } => *span,
            PowRhs::Call { span, .. } => *span,
        }
    }
}

impl<'src> HasSpan for Atom<'src> {
    fn span(&self) -> Span {
        match self {
            Atom::Var { span, .. } => *span,
            Atom::IntLit { span, .. } => *span,
            Atom::FloatLit { span, .. } => *span,
            Atom::BoolLit { span, .. } => *span,
            Atom::Call { span, .. } => *span,
        }
    }
}

// ============================================================================
// Display Implementations
// ============================================================================

impl<'src> std::fmt::Display for Expr<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::And { lhs, rhs, .. } => write!(f, "({} and {})", lhs, rhs),
            Expr::Or { lhs, rhs, .. } => write!(f, "({} or {})", lhs, rhs),
            Expr::Eq { lhs, rhs, .. } => write!(f, "({} == {})", lhs, rhs),
            Expr::NotEq { lhs, rhs, .. } => write!(f, "({} != {})", lhs, rhs),
            Expr::Add { lhs, rhs, .. } => write!(f, "({} + {})", lhs, rhs),
            Expr::Sub { lhs, rhs, .. } => write!(f, "({} - {})", lhs, rhs),
            Expr::Paren { inner, .. } => write!(f, "({})", inner),
            Expr::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            Expr::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            Expr::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            Expr::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            Expr::Neg { inner, .. } => write!(f, "(-{})", inner),
            Expr::Ref { inner, .. } => write!(f, "(&{})", inner),
            Expr::Var { name, .. } => write!(f, "{}", name),
            Expr::IntLit { value, .. } => write!(f, "{}", value),
            Expr::FloatLit { value, .. } => write!(f, "{}", value),
            Expr::BoolLit { value, .. } => write!(f, "{}", value),
            Expr::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl<'src> std::fmt::Display for CmpLhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmpLhs::And { lhs, rhs, .. } => write!(f, "({} and {})", lhs, rhs),
            CmpLhs::Or { lhs, rhs, .. } => write!(f, "({} or {})", lhs, rhs),
            CmpLhs::Eq { lhs, rhs, .. } => write!(f, "({} == {})", lhs, rhs),
            CmpLhs::NotEq { lhs, rhs, .. } => write!(f, "({} != {})", lhs, rhs),
            CmpLhs::Add { lhs, rhs, .. } => write!(f, "({} + {})", lhs, rhs),
            CmpLhs::Sub { lhs, rhs, .. } => write!(f, "({} - {})", lhs, rhs),
            CmpLhs::Paren { inner, .. } => write!(f, "({})", inner),
            CmpLhs::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            CmpLhs::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            CmpLhs::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            CmpLhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            CmpLhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            CmpLhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            CmpLhs::Var { name, .. } => write!(f, "{}", name),
            CmpLhs::IntLit { value, .. } => write!(f, "{}", value),
            CmpLhs::FloatLit { value, .. } => write!(f, "{}", value),
            CmpLhs::BoolLit { value, .. } => write!(f, "{}", value),
            CmpLhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl<'src> std::fmt::Display for CmpRhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmpRhs::Add { lhs, rhs, .. } => write!(f, "({} + {})", lhs, rhs),
            CmpRhs::Sub { lhs, rhs, .. } => write!(f, "({} - {})", lhs, rhs),
            CmpRhs::Paren { inner, .. } => write!(f, "({})", inner),
            CmpRhs::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            CmpRhs::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            CmpRhs::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            CmpRhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            CmpRhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            CmpRhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            CmpRhs::Var { name, .. } => write!(f, "{}", name),
            CmpRhs::IntLit { value, .. } => write!(f, "{}", value),
            CmpRhs::FloatLit { value, .. } => write!(f, "{}", value),
            CmpRhs::BoolLit { value, .. } => write!(f, "{}", value),
            CmpRhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl<'src> std::fmt::Display for AddLhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddLhs::Add { lhs, rhs, .. } => write!(f, "({} + {})", lhs, rhs),
            AddLhs::Sub { lhs, rhs, .. } => write!(f, "({} - {})", lhs, rhs),
            AddLhs::Paren { inner, .. } => write!(f, "({})", inner),
            AddLhs::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            AddLhs::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            AddLhs::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            AddLhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            AddLhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            AddLhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            AddLhs::Var { name, .. } => write!(f, "{}", name),
            AddLhs::IntLit { value, .. } => write!(f, "{}", value),
            AddLhs::FloatLit { value, .. } => write!(f, "{}", value),
            AddLhs::BoolLit { value, .. } => write!(f, "{}", value),
            AddLhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl<'src> std::fmt::Display for AddRhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddRhs::Paren { inner, .. } => write!(f, "({})", inner),
            AddRhs::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            AddRhs::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            AddRhs::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            AddRhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            AddRhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            AddRhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            AddRhs::Var { name, .. } => write!(f, "{}", name),
            AddRhs::IntLit { value, .. } => write!(f, "{}", value),
            AddRhs::FloatLit { value, .. } => write!(f, "{}", value),
            AddRhs::BoolLit { value, .. } => write!(f, "{}", value),
            AddRhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl<'src> std::fmt::Display for MulLhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MulLhs::Paren { inner, .. } => write!(f, "({})", inner),
            MulLhs::Mul { lhs, rhs, .. } => write!(f, "({} * {})", lhs, rhs),
            MulLhs::Div { lhs, rhs, .. } => write!(f, "({} / {})", lhs, rhs),
            MulLhs::Mod { lhs, rhs, .. } => write!(f, "({} % {})", lhs, rhs),
            MulLhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            MulLhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            MulLhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            MulLhs::Var { name, .. } => write!(f, "{}", name),
            MulLhs::IntLit { value, .. } => write!(f, "{}", value),
            MulLhs::FloatLit { value, .. } => write!(f, "{}", value),
            MulLhs::BoolLit { value, .. } => write!(f, "{}", value),
            MulLhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl<'src> std::fmt::Display for MulRhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MulRhs::Paren { inner, .. } => write!(f, "({})", inner),
            MulRhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            MulRhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            MulRhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            MulRhs::Var { name, .. } => write!(f, "{}", name),
            MulRhs::IntLit { value, .. } => write!(f, "{}", value),
            MulRhs::FloatLit { value, .. } => write!(f, "{}", value),
            MulRhs::BoolLit { value, .. } => write!(f, "{}", value),
            MulRhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl<'src> std::fmt::Display for PowLhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowLhs::Paren { inner, .. } => write!(f, "({})", inner),
            PowLhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            PowLhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            PowLhs::Var { name, .. } => write!(f, "{}", name),
            PowLhs::IntLit { value, .. } => write!(f, "{}", value),
            PowLhs::FloatLit { value, .. } => write!(f, "{}", value),
            PowLhs::BoolLit { value, .. } => write!(f, "{}", value),
            PowLhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl<'src> std::fmt::Display for PowRhs<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowRhs::Paren { inner, .. } => write!(f, "({})", inner),
            PowRhs::Pow { lhs, rhs, .. } => write!(f, "({} ^ {})", lhs, rhs),
            PowRhs::Neg { inner, .. } => write!(f, "(-{})", inner),
            PowRhs::Ref { inner, .. } => write!(f, "(&{})", inner),
            PowRhs::Var { name, .. } => write!(f, "{}", name),
            PowRhs::IntLit { value, .. } => write!(f, "{}", value),
            PowRhs::FloatLit { value, .. } => write!(f, "{}", value),
            PowRhs::BoolLit { value, .. } => write!(f, "{}", value),
            PowRhs::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl<'src> std::fmt::Display for Atom<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Var { name, .. } => write!(f, "{}", name),
            Atom::IntLit { value, .. } => write!(f, "{}", value),
            Atom::FloatLit { value, .. } => write!(f, "{}", value),
            Atom::BoolLit { value, .. } => write!(f, "{}", value),
            Atom::Call { name, args, .. } => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
        }
    }
}

// ============================================================================
// Additional From Implementations for Parser Convenience
// ============================================================================

/// Convert AddLhs to CmpRhs (AddLhs is a subset of CmpRhs)
impl<'src> From<AddLhs<'src>> for CmpRhs<'src> {
    fn from(add: AddLhs<'src>) -> Self {
        match add {
            AddLhs::Add { lhs, rhs, span } => CmpRhs::Add { lhs, rhs, span },
            AddLhs::Sub { lhs, rhs, span } => CmpRhs::Sub { lhs, rhs, span },
            AddLhs::Paren { inner, span } => CmpRhs::Paren { inner, span },
            AddLhs::Mul { lhs, rhs, span } => CmpRhs::Mul { lhs, rhs, span },
            AddLhs::Div { lhs, rhs, span } => CmpRhs::Div { lhs, rhs, span },
            AddLhs::Mod { lhs, rhs, span } => CmpRhs::Mod { lhs, rhs, span },
            AddLhs::Pow { lhs, rhs, span } => CmpRhs::Pow { lhs, rhs, span },
            AddLhs::Neg { inner, span } => CmpRhs::Neg { inner, span },
            AddLhs::Ref { inner, span } => CmpRhs::Ref { inner, span },
            AddLhs::Var { name, span } => CmpRhs::Var { name, span },
            AddLhs::IntLit { value, span } => CmpRhs::IntLit { value, span },
            AddLhs::FloatLit { value, span } => CmpRhs::FloatLit { value, span },
            AddLhs::BoolLit { value, span } => CmpRhs::BoolLit { value, span },
            AddLhs::Call { name, args, span } => CmpRhs::Call { name, args, span },
        }
    }
}

/// Convert AddLhs to CmpLhs (AddLhs is a subset of CmpLhs)
impl<'src> From<AddLhs<'src>> for CmpLhs<'src> {
    fn from(add: AddLhs<'src>) -> Self {
        match add {
            AddLhs::Add { lhs, rhs, span } => CmpLhs::Add { lhs, rhs, span },
            AddLhs::Sub { lhs, rhs, span } => CmpLhs::Sub { lhs, rhs, span },
            AddLhs::Paren { inner, span } => CmpLhs::Paren { inner, span },
            AddLhs::Mul { lhs, rhs, span } => CmpLhs::Mul { lhs, rhs, span },
            AddLhs::Div { lhs, rhs, span } => CmpLhs::Div { lhs, rhs, span },
            AddLhs::Mod { lhs, rhs, span } => CmpLhs::Mod { lhs, rhs, span },
            AddLhs::Pow { lhs, rhs, span } => CmpLhs::Pow { lhs, rhs, span },
            AddLhs::Neg { inner, span } => CmpLhs::Neg { inner, span },
            AddLhs::Ref { inner, span } => CmpLhs::Ref { inner, span },
            AddLhs::Var { name, span } => CmpLhs::Var { name, span },
            AddLhs::IntLit { value, span } => CmpLhs::IntLit { value, span },
            AddLhs::FloatLit { value, span } => CmpLhs::FloatLit { value, span },
            AddLhs::BoolLit { value, span } => CmpLhs::BoolLit { value, span },
            AddLhs::Call { name, args, span } => CmpLhs::Call { name, args, span },
        }
    }
}

/// Convert Atom to MulRhs (Atom is a subset of MulRhs)
impl<'src> From<Atom<'src>> for MulRhs<'src> {
    fn from(atom: Atom<'src>) -> Self {
        match atom {
            Atom::Var { name, span } => MulRhs::Var { name, span },
            Atom::IntLit { value, span } => MulRhs::IntLit { value, span },
            Atom::FloatLit { value, span } => MulRhs::FloatLit { value, span },
            Atom::BoolLit { value, span } => MulRhs::BoolLit { value, span },
            Atom::Call { name, args, span } => MulRhs::Call { name, args, span },
        }
    }
}

/// Convert Atom to MulLhs (Atom is a subset of MulLhs)
impl<'src> From<Atom<'src>> for MulLhs<'src> {
    fn from(atom: Atom<'src>) -> Self {
        match atom {
            Atom::Var { name, span } => MulLhs::Var { name, span },
            Atom::IntLit { value, span } => MulLhs::IntLit { value, span },
            Atom::FloatLit { value, span } => MulLhs::FloatLit { value, span },
            Atom::BoolLit { value, span } => MulLhs::BoolLit { value, span },
            Atom::Call { name, args, span } => MulLhs::Call { name, args, span },
        }
    }
}

/// Convert MulLhs to AddRhs (MulLhs is a subset of AddRhs)
impl<'src> From<MulLhs<'src>> for AddRhs<'src> {
    fn from(mul: MulLhs<'src>) -> Self {
        match mul {
            MulLhs::Paren { inner, span } => AddRhs::Paren { inner, span },
            MulLhs::Mul { lhs, rhs, span } => AddRhs::Mul { lhs, rhs, span },
            MulLhs::Div { lhs, rhs, span } => AddRhs::Div { lhs, rhs, span },
            MulLhs::Mod { lhs, rhs, span } => AddRhs::Mod { lhs, rhs, span },
            MulLhs::Pow { lhs, rhs, span } => AddRhs::Pow { lhs, rhs, span },
            MulLhs::Neg { inner, span } => AddRhs::Neg { inner, span },
            MulLhs::Ref { inner, span } => AddRhs::Ref { inner, span },
            MulLhs::Var { name, span } => AddRhs::Var { name, span },
            MulLhs::IntLit { value, span } => AddRhs::IntLit { value, span },
            MulLhs::FloatLit { value, span } => AddRhs::FloatLit { value, span },
            MulLhs::BoolLit { value, span } => AddRhs::BoolLit { value, span },
            MulLhs::Call { name, args, span } => AddRhs::Call { name, args, span },
        }
    }
}

/// Convert MulLhs to AddLhs (MulLhs is a subset of AddLhs)
impl<'src> From<MulLhs<'src>> for AddLhs<'src> {
    fn from(mul: MulLhs<'src>) -> Self {
        match mul {
            MulLhs::Paren { inner, span } => AddLhs::Paren { inner, span },
            MulLhs::Mul { lhs, rhs, span } => AddLhs::Mul { lhs, rhs, span },
            MulLhs::Div { lhs, rhs, span } => AddLhs::Div { lhs, rhs, span },
            MulLhs::Mod { lhs, rhs, span } => AddLhs::Mod { lhs, rhs, span },
            MulLhs::Pow { lhs, rhs, span } => AddLhs::Pow { lhs, rhs, span },
            MulLhs::Neg { inner, span } => AddLhs::Neg { inner, span },
            MulLhs::Ref { inner, span } => AddLhs::Ref { inner, span },
            MulLhs::Var { name, span } => AddLhs::Var { name, span },
            MulLhs::IntLit { value, span } => AddLhs::IntLit { value, span },
            MulLhs::FloatLit { value, span } => AddLhs::FloatLit { value, span },
            MulLhs::BoolLit { value, span } => AddLhs::BoolLit { value, span },
            MulLhs::Call { name, args, span } => AddLhs::Call { name, args, span },
        }
    }
}

/// Convert Atom to PowLhs (Atom is a subset of PowLhs)
impl<'src> From<Atom<'src>> for PowLhs<'src> {
    fn from(atom: Atom<'src>) -> Self {
        match atom {
            Atom::Var { name, span } => PowLhs::Var { name, span },
            Atom::IntLit { value, span } => PowLhs::IntLit { value, span },
            Atom::FloatLit { value, span } => PowLhs::FloatLit { value, span },
            Atom::BoolLit { value, span } => PowLhs::BoolLit { value, span },
            Atom::Call { name, args, span } => PowLhs::Call { name, args, span },
        }
    }
}

/// Convert Atom to PowRhs (Atom is a subset of PowRhs)
impl<'src> From<Atom<'src>> for PowRhs<'src> {
    fn from(atom: Atom<'src>) -> Self {
        match atom {
            Atom::Var { name, span } => PowRhs::Var { name, span },
            Atom::IntLit { value, span } => PowRhs::IntLit { value, span },
            Atom::FloatLit { value, span } => PowRhs::FloatLit { value, span },
            Atom::BoolLit { value, span } => PowRhs::BoolLit { value, span },
            Atom::Call { name, args, span } => PowRhs::Call { name, args, span },
        }
    }
}

/// Convert PowLhs to PowRhs (PowLhs is a subset of PowRhs except for Pow)
impl<'src> From<PowLhs<'src>> for PowRhs<'src> {
    fn from(pow: PowLhs<'src>) -> Self {
        match pow {
            PowLhs::Paren { inner, span } => PowRhs::Paren { inner, span },
            PowLhs::Neg { inner, span } => PowRhs::Neg { inner, span },
            PowLhs::Ref { inner, span } => PowRhs::Ref { inner, span },
            PowLhs::Var { name, span } => PowRhs::Var { name, span },
            PowLhs::IntLit { value, span } => PowRhs::IntLit { value, span },
            PowLhs::FloatLit { value, span } => PowRhs::FloatLit { value, span },
            PowLhs::BoolLit { value, span } => PowRhs::BoolLit { value, span },
            PowLhs::Call { name, args, span } => PowRhs::Call { name, args, span },
        }
    }
}

/// Convert PowLhs to MulRhs (PowLhs is a subset of MulRhs)
impl<'src> From<PowLhs<'src>> for MulRhs<'src> {
    fn from(pow: PowLhs<'src>) -> Self {
        match pow {
            PowLhs::Paren { inner, span } => MulRhs::Paren { inner, span },
            PowLhs::Neg { inner, span } => MulRhs::Neg { inner, span },
            PowLhs::Ref { inner, span } => MulRhs::Ref { inner, span },
            PowLhs::Var { name, span } => MulRhs::Var { name, span },
            PowLhs::IntLit { value, span } => MulRhs::IntLit { value, span },
            PowLhs::FloatLit { value, span } => MulRhs::FloatLit { value, span },
            PowLhs::BoolLit { value, span } => MulRhs::BoolLit { value, span },
            PowLhs::Call { name, args, span } => MulRhs::Call { name, args, span },
        }
    }
}

/// Convert PowLhs to MulLhs (PowLhs is a subset of MulLhs)
impl<'src> From<PowLhs<'src>> for MulLhs<'src> {
    fn from(pow: PowLhs<'src>) -> Self {
        match pow {
            PowLhs::Paren { inner, span } => MulLhs::Paren { inner, span },
            PowLhs::Neg { inner, span } => MulLhs::Neg { inner, span },
            PowLhs::Ref { inner, span } => MulLhs::Ref { inner, span },
            PowLhs::Var { name, span } => MulLhs::Var { name, span },
            PowLhs::IntLit { value, span } => MulLhs::IntLit { value, span },
            PowLhs::FloatLit { value, span } => MulLhs::FloatLit { value, span },
            PowLhs::BoolLit { value, span } => MulLhs::BoolLit { value, span },
            PowLhs::Call { name, args, span } => MulLhs::Call { name, args, span },
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::LineColumn;

    // Helper function to create a dummy span for testing
    fn dummy_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 2,
        }
    }

    #[test]
    fn test_atom_to_expr_conversion() {
        let atom = Atom::Var {
            name: "x",
            span: dummy_span(),
        };
        let expected_span = dummy_span();
        let expr: Expr = atom.into();
        assert_eq!(
            expr,
            Expr::Var {
                name: "x",
                span: expected_span
            }
        );
    }

    #[test]
    fn test_atom_to_mulrhs_via_expr() {
        let atom = Atom::Var {
            name: "x",
            span: dummy_span(),
        };
        let expected_span = dummy_span();
        let expr: Expr = atom.into();
        let mulrhs: Result<MulRhs, _> = expr.try_into();
        assert!(mulrhs.is_ok());
        assert_eq!(
            mulrhs.unwrap(),
            MulRhs::Var {
                name: "x",
                span: expected_span
            }
        );
    }

    #[test]
    fn test_mulrhs_to_addrhs_via_expr() {
        let mulrhs = MulRhs::Var {
            name: "x",
            span: dummy_span(),
        };
        let expected_span = dummy_span();
        let expr: Expr = mulrhs.into();
        let addrhs: Result<AddRhs, _> = expr.try_into();
        assert!(addrhs.is_ok());
        assert_eq!(
            addrhs.unwrap(),
            AddRhs::Var {
                name: "x",
                span: expected_span
            }
        );
    }

    #[test]
    fn test_mul_structure() {
        let mul = Expr::Mul {
            lhs: Box::new(MulLhs::Var {
                name: "a",
                span: dummy_span(),
            }),
            rhs: Box::new(MulRhs::Var {
                name: "b",
                span: dummy_span(),
            }),
            span: dummy_span(),
        };
        match mul {
            Expr::Mul { .. } => {}
            _ => panic!("Expected Mul"),
        }
    }

    #[test]
    fn test_add_structure() {
        let add = Expr::Add {
            lhs: Box::new(AddLhs::Var {
                name: "a",
                span: dummy_span(),
            }),
            rhs: Box::new(AddRhs::Var {
                name: "b",
                span: dummy_span(),
            }),
            span: dummy_span(),
        };
        match add {
            Expr::Add { .. } => {}
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_precedence_in_types() {
        // Multiplication in AddRhs
        let mul_in_addrhs = AddRhs::Mul {
            lhs: Box::new(MulLhs::Var {
                name: "b",
                span: dummy_span(),
            }),
            rhs: Box::new(MulRhs::Var {
                name: "c",
                span: dummy_span(),
            }),
            span: dummy_span(),
        };

        // Addition with multiplication on right side
        let add = Expr::Add {
            lhs: Box::new(AddLhs::Var {
                name: "a",
                span: dummy_span(),
            }),
            rhs: Box::new(mul_in_addrhs),
            span: dummy_span(),
        };

        assert!(matches!(add, Expr::Add { .. }));
    }

    #[test]
    fn test_display_simple_var() {
        let expr = Expr::Var {
            name: "x",
            span: dummy_span(),
        };
        assert_eq!(format!("{}", expr), "x");
    }

    #[test]
    fn test_display_simple_add() {
        let expr = Expr::Add {
            lhs: Box::new(AddLhs::Var {
                name: "a",
                span: dummy_span(),
            }),
            rhs: Box::new(AddRhs::Var {
                name: "b",
                span: dummy_span(),
            }),
            span: dummy_span(),
        };
        assert_eq!(format!("{}", expr), "(a + b)");
    }

    #[test]
    fn test_display_precedence() {
        // a + b * c
        let expr = Expr::Add {
            lhs: Box::new(AddLhs::Var {
                name: "a",
                span: dummy_span(),
            }),
            rhs: Box::new(AddRhs::Mul {
                lhs: Box::new(MulLhs::Var {
                    name: "b",
                    span: dummy_span(),
                }),
                rhs: Box::new(MulRhs::Var {
                    name: "c",
                    span: dummy_span(),
                }),
                span: dummy_span(),
            }),
            span: dummy_span(),
        };
        assert_eq!(format!("{}", expr), "(a + (b * c))");
    }
}
