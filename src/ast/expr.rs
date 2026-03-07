use crate::ast::span::HasSpan;
use crate::lexer::Span;
use subenum::subenum;

// ============================================================================
// Struct Literal Field
// ============================================================================

/// Represents a field in a struct literal
#[derive(Debug, Clone, PartialEq)]
pub enum StructLitField<'src> {
    /// Regular field assignment: `field: value`
    Field {
        name: &'src str,
        value: Expr<'src>,
        span: Span,
    },
    /// Computed property constraint: `method() = value`
    ComputedProperty {
        name: &'src str,
        value: Expr<'src>,
        span: Span,
    },
}

impl<'src> HasSpan for StructLitField<'src> {
    fn span(&self) -> Span {
        match self {
            StructLitField::Field { span, .. } => *span,
            StructLitField::ComputedProperty { span, .. } => *span,
        }
    }
}

// ============================================================================
// Rune Block Structures
// ============================================================================

/// Represents a parameter in a rune block
#[derive(Debug, Clone, PartialEq)]
pub struct RuneParam<'src> {
    /// Parameter name in rune code
    pub name: &'src str,
    /// Optional expression to bind (None for direct parameters like `x`, Some for `x=expr`)
    pub value: Option<Expr<'src>>,
    pub span: Span,
}

impl<'src> HasSpan for RuneParam<'src> {
    fn span(&self) -> Span {
        self.span
    }
}

/// Represents a rune block expression
#[derive(Debug, Clone, PartialEq)]
pub struct RuneBlock<'src> {
    /// Parameters to the rune block
    pub params: Vec<RuneParam<'src>>,
    /// Raw Rune code (captured as string from source)
    pub body: &'src str,
    pub span: Span,
}

impl<'src> HasSpan for RuneBlock<'src> {
    fn span(&self) -> Span {
        self.span
    }
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

    // Less Than - in CmpLhs only
    // lhs can be Lt, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    Lt {
        lhs: Box<CmpLhs<'src>>,
        rhs: Box<CmpRhs<'src>>,
        span: Span,
    },

    // Greater Than - in CmpLhs only
    // lhs can be Gt, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    Gt {
        lhs: Box<CmpLhs<'src>>,
        rhs: Box<CmpRhs<'src>>,
        span: Span,
    },

    // Less Than or Equal - in CmpLhs only
    // lhs can be LtEq, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    LtEq {
        lhs: Box<CmpLhs<'src>>,
        rhs: Box<CmpRhs<'src>>,
        span: Span,
    },

    // Greater Than or Equal - in CmpLhs only
    // lhs can be GtEq, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    GtEq {
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

    // Unary dereference - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs
    // Higher precedence than power (binds tighter)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs)]
    Deref {
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

    // Unit literal: a numeric value with a unit suffix (e.g., `10mm`, `45deg`, `1.5m`)
    // The unit_suffix is the raw identifier string; it is resolved during semantic analysis.
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    UnitLit {
        value: f64,
        unit_suffix: &'src str,
        span: Span,
    },

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

    // Container field access (dot prefix in with blocks) - in all levels (high precedence like atoms)
    // Example: .field or .field.x (starts with a dot, refers to container context)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    ContainerFieldAccess {
        /// Path segments after the leading dot
        /// For `.field` -> vec!["field"]
        /// For `.field.x` -> vec!["field", "x"]
        field_path: Vec<&'src str>,
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
        fields: Vec<StructLitField<'src>>,
        span: Span,
    },

    // Array indexing - in all levels (high precedence like atoms)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    Index {
        array: Box<Expr<'src>>,
        index: Box<Expr<'src>>,
        span: Span,
    },

    // Range expression - in all levels (high precedence like atoms)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    Range {
        start: Box<Expr<'src>>,
        end: Box<Expr<'src>>,
        span: Span,
    },

    // Closure expression - in all levels (high precedence like atoms)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    Closure {
        params: Vec<&'src str>,
        body: Box<Expr<'src>>,
        span: Span,
    },

    // Rune block - in all levels (high precedence like atoms)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    RuneBlock(Box<RuneBlock<'src>>),
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
            Expr::Lt { span, .. } => *span,
            Expr::Gt { span, .. } => *span,
            Expr::LtEq { span, .. } => *span,
            Expr::GtEq { span, .. } => *span,
            Expr::Add { span, .. } => *span,
            Expr::Sub { span, .. } => *span,
            Expr::Paren { span, .. } => *span,
            Expr::Mul { span, .. } => *span,
            Expr::Div { span, .. } => *span,
            Expr::Mod { span, .. } => *span,
            Expr::Pow { span, .. } => *span,
            Expr::Neg { span, .. } => *span,
            Expr::Ref { span, .. } => *span,
            Expr::Deref { span, .. } => *span,
            Expr::Var { span, .. } => *span,
            Expr::IntLit { span, .. } => *span,
            Expr::FloatLit { span, .. } => *span,
            Expr::BoolLit { span, .. } => *span,
            Expr::UnitLit { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::MethodCall { span, .. } => *span,
            Expr::FieldAccess { span, .. } => *span,
            Expr::ContainerFieldAccess { span, .. } => *span,
            Expr::ArrayLit { span, .. } => *span,
            Expr::StructLit { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Range { span, .. } => *span,

            Expr::Closure { span, .. } => *span,
            Expr::RuneBlock(block) => block.span,
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
            CmpLhs::Lt { span, .. } => *span,
            CmpLhs::Gt { span, .. } => *span,
            CmpLhs::LtEq { span, .. } => *span,
            CmpLhs::GtEq { span, .. } => *span,
            CmpLhs::Add { span, .. } => *span,
            CmpLhs::Sub { span, .. } => *span,
            CmpLhs::Paren { span, .. } => *span,
            CmpLhs::Mul { span, .. } => *span,
            CmpLhs::Div { span, .. } => *span,
            CmpLhs::Mod { span, .. } => *span,
            CmpLhs::Pow { span, .. } => *span,
            CmpLhs::Neg { span, .. } => *span,
            CmpLhs::Ref { span, .. } => *span,
            CmpLhs::Deref { span, .. } => *span,
            CmpLhs::Var { span, .. } => *span,
            CmpLhs::IntLit { span, .. } => *span,
            CmpLhs::FloatLit { span, .. } => *span,
            CmpLhs::BoolLit { span, .. } => *span,
            CmpLhs::UnitLit { span, .. } => *span,
            CmpLhs::Call { span, .. } => *span,
            CmpLhs::MethodCall { span, .. } => *span,
            CmpLhs::FieldAccess { span, .. } => *span,
            CmpLhs::ContainerFieldAccess { span, .. } => *span,
            CmpLhs::ArrayLit { span, .. } => *span,
            CmpLhs::StructLit { span, .. } => *span,
            CmpLhs::Index { span, .. } => *span,
            CmpLhs::Range { span, .. } => *span,

            CmpLhs::Closure { span, .. } => *span,
            CmpLhs::RuneBlock(block) => block.span,
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
            CmpRhs::Deref { span, .. } => *span,
            CmpRhs::Var { span, .. } => *span,
            CmpRhs::IntLit { span, .. } => *span,
            CmpRhs::FloatLit { span, .. } => *span,
            CmpRhs::BoolLit { span, .. } => *span,
            CmpRhs::UnitLit { span, .. } => *span,
            CmpRhs::Call { span, .. } => *span,
            CmpRhs::MethodCall { span, .. } => *span,
            CmpRhs::FieldAccess { span, .. } => *span,
            CmpRhs::ContainerFieldAccess { span, .. } => *span,
            CmpRhs::ArrayLit { span, .. } => *span,
            CmpRhs::StructLit { span, .. } => *span,
            CmpRhs::Index { span, .. } => *span,
            CmpRhs::Range { span, .. } => *span,

            CmpRhs::Closure { span, .. } => *span,
            CmpRhs::RuneBlock(block) => block.span,
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
            AddLhs::Deref { span, .. } => *span,
            AddLhs::Var { span, .. } => *span,
            AddLhs::IntLit { span, .. } => *span,
            AddLhs::FloatLit { span, .. } => *span,
            AddLhs::BoolLit { span, .. } => *span,
            AddLhs::UnitLit { span, .. } => *span,
            AddLhs::Call { span, .. } => *span,
            AddLhs::MethodCall { span, .. } => *span,
            AddLhs::FieldAccess { span, .. } => *span,
            AddLhs::ContainerFieldAccess { span, .. } => *span,
            AddLhs::ArrayLit { span, .. } => *span,
            AddLhs::StructLit { span, .. } => *span,
            AddLhs::Index { span, .. } => *span,
            AddLhs::Range { span, .. } => *span,

            AddLhs::Closure { span, .. } => *span,
            AddLhs::RuneBlock(block) => block.span,
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
            AddRhs::Deref { span, .. } => *span,
            AddRhs::Var { span, .. } => *span,
            AddRhs::IntLit { span, .. } => *span,
            AddRhs::FloatLit { span, .. } => *span,
            AddRhs::BoolLit { span, .. } => *span,
            AddRhs::UnitLit { span, .. } => *span,
            AddRhs::Call { span, .. } => *span,
            AddRhs::MethodCall { span, .. } => *span,
            AddRhs::FieldAccess { span, .. } => *span,
            AddRhs::ContainerFieldAccess { span, .. } => *span,
            AddRhs::ArrayLit { span, .. } => *span,
            AddRhs::StructLit { span, .. } => *span,
            AddRhs::Index { span, .. } => *span,
            AddRhs::Range { span, .. } => *span,

            AddRhs::Closure { span, .. } => *span,
            AddRhs::RuneBlock(block) => block.span,
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
            MulLhs::Deref { span, .. } => *span,
            MulLhs::Var { span, .. } => *span,
            MulLhs::IntLit { span, .. } => *span,
            MulLhs::FloatLit { span, .. } => *span,
            MulLhs::BoolLit { span, .. } => *span,
            MulLhs::UnitLit { span, .. } => *span,
            MulLhs::Call { span, .. } => *span,
            MulLhs::MethodCall { span, .. } => *span,
            MulLhs::FieldAccess { span, .. } => *span,
            MulLhs::ContainerFieldAccess { span, .. } => *span,
            MulLhs::ArrayLit { span, .. } => *span,
            MulLhs::StructLit { span, .. } => *span,
            MulLhs::Index { span, .. } => *span,
            MulLhs::Range { span, .. } => *span,

            MulLhs::Closure { span, .. } => *span,
            MulLhs::RuneBlock(block) => block.span,
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
            MulRhs::Deref { span, .. } => *span,
            MulRhs::Var { span, .. } => *span,
            MulRhs::IntLit { span, .. } => *span,
            MulRhs::FloatLit { span, .. } => *span,
            MulRhs::BoolLit { span, .. } => *span,
            MulRhs::UnitLit { span, .. } => *span,
            MulRhs::Call { span, .. } => *span,
            MulRhs::MethodCall { span, .. } => *span,
            MulRhs::FieldAccess { span, .. } => *span,
            MulRhs::ContainerFieldAccess { span, .. } => *span,
            MulRhs::ArrayLit { span, .. } => *span,
            MulRhs::StructLit { span, .. } => *span,
            MulRhs::Index { span, .. } => *span,
            MulRhs::Range { span, .. } => *span,

            MulRhs::Closure { span, .. } => *span,
            MulRhs::RuneBlock(block) => block.span,
        }
    }
}

impl<'src> HasSpan for PowLhs<'src> {
    fn span(&self) -> Span {
        match self {
            PowLhs::Paren { span, .. } => *span,
            PowLhs::Neg { span, .. } => *span,
            PowLhs::Ref { span, .. } => *span,
            PowLhs::Deref { span, .. } => *span,
            PowLhs::Var { span, .. } => *span,
            PowLhs::IntLit { span, .. } => *span,
            PowLhs::FloatLit { span, .. } => *span,
            PowLhs::BoolLit { span, .. } => *span,
            PowLhs::UnitLit { span, .. } => *span,
            PowLhs::Call { span, .. } => *span,
            PowLhs::MethodCall { span, .. } => *span,
            PowLhs::FieldAccess { span, .. } => *span,
            PowLhs::ContainerFieldAccess { span, .. } => *span,
            PowLhs::ArrayLit { span, .. } => *span,
            PowLhs::StructLit { span, .. } => *span,
            PowLhs::Index { span, .. } => *span,
            PowLhs::Range { span, .. } => *span,

            PowLhs::Closure { span, .. } => *span,
            PowLhs::RuneBlock(block) => block.span,
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
            PowRhs::Deref { span, .. } => *span,
            PowRhs::Var { span, .. } => *span,
            PowRhs::IntLit { span, .. } => *span,
            PowRhs::FloatLit { span, .. } => *span,
            PowRhs::BoolLit { span, .. } => *span,
            PowRhs::UnitLit { span, .. } => *span,
            PowRhs::Call { span, .. } => *span,
            PowRhs::MethodCall { span, .. } => *span,
            PowRhs::FieldAccess { span, .. } => *span,
            PowRhs::ContainerFieldAccess { span, .. } => *span,
            PowRhs::ArrayLit { span, .. } => *span,
            PowRhs::StructLit { span, .. } => *span,
            PowRhs::Index { span, .. } => *span,
            PowRhs::Range { span, .. } => *span,

            PowRhs::Closure { span, .. } => *span,
            PowRhs::RuneBlock(block) => block.span,
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
            Atom::UnitLit { span, .. } => *span,
            Atom::Call { span, .. } => *span,
            Atom::MethodCall { span, .. } => *span,
            Atom::FieldAccess { span, .. } => *span,
            Atom::ContainerFieldAccess { span, .. } => *span,
            Atom::ArrayLit { span, .. } => *span,
            Atom::StructLit { span, .. } => *span,
            Atom::Index { span, .. } => *span,
            Atom::Range { span, .. } => *span,

            Atom::Closure { span, .. } => *span,
            Atom::RuneBlock(block) => block.span,
        }
    }
}
