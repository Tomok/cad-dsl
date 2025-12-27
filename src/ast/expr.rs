use crate::ast::span::HasSpan;
use crate::lexer::Span;
use subenum::subenum;

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

    // Method call - in all levels (high precedence like atoms)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    MethodCall {
        receiver: Box<Expr<'src>>,
        method: &'src str,
        args: Vec<Expr<'src>>,
        span: Span,
    },

    // Field access - in all levels (high precedence like atoms)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    FieldAccess {
        receiver: Box<Expr<'src>>,
        field: &'src str,
        span: Span,
    },

    // Array literal - in all levels (high precedence like atoms)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    ArrayLit {
        elements: Vec<Expr<'src>>,
        span: Span,
    },

    // Struct literal - in all levels (high precedence like atoms)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    StructLit {
        name: &'src str,
        fields: Vec<(&'src str, Expr<'src>)>,
        span: Span,
    },
}

// ============================================================================
// HasSpan Implementations
// ============================================================================

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
            Expr::MethodCall { span, .. } => *span,
            Expr::FieldAccess { span, .. } => *span,
            Expr::ArrayLit { span, .. } => *span,
            Expr::StructLit { span, .. } => *span,
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
            CmpLhs::MethodCall { span, .. } => *span,
            CmpLhs::FieldAccess { span, .. } => *span,
            CmpLhs::ArrayLit { span, .. } => *span,
            CmpLhs::StructLit { span, .. } => *span,
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
            CmpRhs::MethodCall { span, .. } => *span,
            CmpRhs::FieldAccess { span, .. } => *span,
            CmpRhs::ArrayLit { span, .. } => *span,
            CmpRhs::StructLit { span, .. } => *span,
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
            AddLhs::MethodCall { span, .. } => *span,
            AddLhs::FieldAccess { span, .. } => *span,
            AddLhs::ArrayLit { span, .. } => *span,
            AddLhs::StructLit { span, .. } => *span,
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
            AddRhs::MethodCall { span, .. } => *span,
            AddRhs::FieldAccess { span, .. } => *span,
            AddRhs::ArrayLit { span, .. } => *span,
            AddRhs::StructLit { span, .. } => *span,
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
            MulLhs::MethodCall { span, .. } => *span,
            MulLhs::FieldAccess { span, .. } => *span,
            MulLhs::ArrayLit { span, .. } => *span,
            MulLhs::StructLit { span, .. } => *span,
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
            MulRhs::MethodCall { span, .. } => *span,
            MulRhs::FieldAccess { span, .. } => *span,
            MulRhs::ArrayLit { span, .. } => *span,
            MulRhs::StructLit { span, .. } => *span,
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
            PowLhs::MethodCall { span, .. } => *span,
            PowLhs::FieldAccess { span, .. } => *span,
            PowLhs::ArrayLit { span, .. } => *span,
            PowLhs::StructLit { span, .. } => *span,
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
            PowRhs::MethodCall { span, .. } => *span,
            PowRhs::FieldAccess { span, .. } => *span,
            PowRhs::ArrayLit { span, .. } => *span,
            PowRhs::StructLit { span, .. } => *span,
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
            Atom::MethodCall { span, .. } => *span,
            Atom::FieldAccess { span, .. } => *span,
            Atom::ArrayLit { span, .. } => *span,
            Atom::StructLit { span, .. } => *span,
        }
    }
}
