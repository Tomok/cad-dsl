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
pub enum Expr {
    // Logical AND - in CmpLhs (same level as equality operators)
    // lhs can be And/Or, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    And { lhs: Box<CmpLhs>, rhs: Box<CmpRhs> },

    // Logical OR - in CmpLhs (same level as equality operators)
    // lhs can be And/Or, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    Or { lhs: Box<CmpLhs>, rhs: Box<CmpRhs> },

    // Equality - in CmpLhs only
    // lhs can be Eq, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    Eq { lhs: Box<CmpLhs>, rhs: Box<CmpRhs> },

    // Not Equal - in CmpLhs only
    // lhs can be NotEq, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs)]
    NotEq { lhs: Box<CmpLhs>, rhs: Box<CmpRhs> },

    // Addition - in CmpLhs, CmpRhs, AddLhs
    // lhs can be Add/Sub, rhs cannot (enforces left-associativity and precedence)
    #[subenum(CmpLhs, CmpRhs, AddLhs)]
    Add { lhs: Box<AddLhs>, rhs: Box<AddRhs> },

    // Subtraction - in CmpLhs, CmpRhs, AddLhs
    #[subenum(CmpLhs, CmpRhs, AddLhs)]
    Sub { lhs: Box<AddLhs>, rhs: Box<AddRhs> },

    // Parentheses - in all contexts except Atom (resets precedence)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs)]
    Paren(Box<Expr>),

    // Multiplication - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs
    // lhs can be Mul/Div, rhs cannot (enforces left-associativity)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs)]
    Mul { lhs: Box<MulLhs>, rhs: Box<MulRhs> },

    // Division - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs)]
    Div { lhs: Box<MulLhs>, rhs: Box<MulRhs> },

    // Modulo - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs)]
    Mod { lhs: Box<MulLhs>, rhs: Box<MulRhs> },

    // Power - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowRhs
    // lhs cannot be Pow (enforces right-associativity), rhs can be Pow
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowRhs)]
    Pow { lhs: Box<PowLhs>, rhs: Box<PowRhs> },

    // Unary negation - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs
    // Higher precedence than power (binds tighter)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs)]
    Neg { inner: Box<PowLhs> },

    // Unary reference - in CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs
    // Higher precedence than power (binds tighter)
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs)]
    Ref { inner: Box<PowLhs> },

    // Variable reference - in all levels
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    Var(String),

    // Integer literal - in all levels
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    IntLit(i32),

    // Float literal - in all levels
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    FloatLit(f64),

    // Boolean literal - in all levels
    #[subenum(CmpLhs, CmpRhs, AddLhs, AddRhs, MulLhs, MulRhs, PowLhs, PowRhs, Atom)]
    BoolLit(bool),
}

// ============================================================================
// Display Implementations
// ============================================================================

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::And { lhs, rhs } => write!(f, "({} and {})", lhs, rhs),
            Expr::Or { lhs, rhs } => write!(f, "({} or {})", lhs, rhs),
            Expr::Eq { lhs, rhs } => write!(f, "({} == {})", lhs, rhs),
            Expr::NotEq { lhs, rhs } => write!(f, "({} != {})", lhs, rhs),
            Expr::Add { lhs, rhs } => write!(f, "({} + {})", lhs, rhs),
            Expr::Sub { lhs, rhs } => write!(f, "({} - {})", lhs, rhs),
            Expr::Paren(inner) => write!(f, "({})", inner),
            Expr::Mul { lhs, rhs } => write!(f, "({} * {})", lhs, rhs),
            Expr::Div { lhs, rhs } => write!(f, "({} / {})", lhs, rhs),
            Expr::Mod { lhs, rhs } => write!(f, "({} % {})", lhs, rhs),
            Expr::Pow { lhs, rhs } => write!(f, "({} ^ {})", lhs, rhs),
            Expr::Neg { inner } => write!(f, "(-{})", inner),
            Expr::Ref { inner } => write!(f, "(&{})", inner),
            Expr::Var(name) => write!(f, "{}", name),
            Expr::IntLit(value) => write!(f, "{}", value),
            Expr::FloatLit(value) => write!(f, "{}", value),
            Expr::BoolLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for CmpLhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmpLhs::And { lhs, rhs } => write!(f, "({} and {})", lhs, rhs),
            CmpLhs::Or { lhs, rhs } => write!(f, "({} or {})", lhs, rhs),
            CmpLhs::Eq { lhs, rhs } => write!(f, "({} == {})", lhs, rhs),
            CmpLhs::NotEq { lhs, rhs } => write!(f, "({} != {})", lhs, rhs),
            CmpLhs::Add { lhs, rhs } => write!(f, "({} + {})", lhs, rhs),
            CmpLhs::Sub { lhs, rhs } => write!(f, "({} - {})", lhs, rhs),
            CmpLhs::Paren(inner) => write!(f, "({})", inner),
            CmpLhs::Mul { lhs, rhs } => write!(f, "({} * {})", lhs, rhs),
            CmpLhs::Div { lhs, rhs } => write!(f, "({} / {})", lhs, rhs),
            CmpLhs::Mod { lhs, rhs } => write!(f, "({} % {})", lhs, rhs),
            CmpLhs::Pow { lhs, rhs } => write!(f, "({} ^ {})", lhs, rhs),
            CmpLhs::Neg { inner } => write!(f, "(-{})", inner),
            CmpLhs::Ref { inner } => write!(f, "(&{})", inner),
            CmpLhs::Var(name) => write!(f, "{}", name),
            CmpLhs::IntLit(value) => write!(f, "{}", value),
            CmpLhs::FloatLit(value) => write!(f, "{}", value),
            CmpLhs::BoolLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for CmpRhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CmpRhs::Add { lhs, rhs } => write!(f, "({} + {})", lhs, rhs),
            CmpRhs::Sub { lhs, rhs } => write!(f, "({} - {})", lhs, rhs),
            CmpRhs::Paren(inner) => write!(f, "({})", inner),
            CmpRhs::Mul { lhs, rhs } => write!(f, "({} * {})", lhs, rhs),
            CmpRhs::Div { lhs, rhs } => write!(f, "({} / {})", lhs, rhs),
            CmpRhs::Mod { lhs, rhs } => write!(f, "({} % {})", lhs, rhs),
            CmpRhs::Pow { lhs, rhs } => write!(f, "({} ^ {})", lhs, rhs),
            CmpRhs::Neg { inner } => write!(f, "(-{})", inner),
            CmpRhs::Ref { inner } => write!(f, "(&{})", inner),
            CmpRhs::Var(name) => write!(f, "{}", name),
            CmpRhs::IntLit(value) => write!(f, "{}", value),
            CmpRhs::FloatLit(value) => write!(f, "{}", value),
            CmpRhs::BoolLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for AddLhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddLhs::Add { lhs, rhs } => write!(f, "({} + {})", lhs, rhs),
            AddLhs::Sub { lhs, rhs } => write!(f, "({} - {})", lhs, rhs),
            AddLhs::Paren(inner) => write!(f, "({})", inner),
            AddLhs::Mul { lhs, rhs } => write!(f, "({} * {})", lhs, rhs),
            AddLhs::Div { lhs, rhs } => write!(f, "({} / {})", lhs, rhs),
            AddLhs::Mod { lhs, rhs } => write!(f, "({} % {})", lhs, rhs),
            AddLhs::Pow { lhs, rhs } => write!(f, "({} ^ {})", lhs, rhs),
            AddLhs::Neg { inner } => write!(f, "(-{})", inner),
            AddLhs::Ref { inner } => write!(f, "(&{})", inner),
            AddLhs::Var(name) => write!(f, "{}", name),
            AddLhs::IntLit(value) => write!(f, "{}", value),
            AddLhs::FloatLit(value) => write!(f, "{}", value),
            AddLhs::BoolLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for AddRhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddRhs::Paren(inner) => write!(f, "({})", inner),
            AddRhs::Mul { lhs, rhs } => write!(f, "({} * {})", lhs, rhs),
            AddRhs::Div { lhs, rhs } => write!(f, "({} / {})", lhs, rhs),
            AddRhs::Mod { lhs, rhs } => write!(f, "({} % {})", lhs, rhs),
            AddRhs::Pow { lhs, rhs } => write!(f, "({} ^ {})", lhs, rhs),
            AddRhs::Neg { inner } => write!(f, "(-{})", inner),
            AddRhs::Ref { inner } => write!(f, "(&{})", inner),
            AddRhs::Var(name) => write!(f, "{}", name),
            AddRhs::IntLit(value) => write!(f, "{}", value),
            AddRhs::FloatLit(value) => write!(f, "{}", value),
            AddRhs::BoolLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for MulLhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MulLhs::Paren(inner) => write!(f, "({})", inner),
            MulLhs::Mul { lhs, rhs } => write!(f, "({} * {})", lhs, rhs),
            MulLhs::Div { lhs, rhs } => write!(f, "({} / {})", lhs, rhs),
            MulLhs::Mod { lhs, rhs } => write!(f, "({} % {})", lhs, rhs),
            MulLhs::Pow { lhs, rhs } => write!(f, "({} ^ {})", lhs, rhs),
            MulLhs::Neg { inner } => write!(f, "(-{})", inner),
            MulLhs::Ref { inner } => write!(f, "(&{})", inner),
            MulLhs::Var(name) => write!(f, "{}", name),
            MulLhs::IntLit(value) => write!(f, "{}", value),
            MulLhs::FloatLit(value) => write!(f, "{}", value),
            MulLhs::BoolLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for MulRhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MulRhs::Paren(inner) => write!(f, "({})", inner),
            MulRhs::Pow { lhs, rhs } => write!(f, "({} ^ {})", lhs, rhs),
            MulRhs::Neg { inner } => write!(f, "(-{})", inner),
            MulRhs::Ref { inner } => write!(f, "(&{})", inner),
            MulRhs::Var(name) => write!(f, "{}", name),
            MulRhs::IntLit(value) => write!(f, "{}", value),
            MulRhs::FloatLit(value) => write!(f, "{}", value),
            MulRhs::BoolLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for PowLhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowLhs::Paren(inner) => write!(f, "({})", inner),
            PowLhs::Neg { inner } => write!(f, "(-{})", inner),
            PowLhs::Ref { inner } => write!(f, "(&{})", inner),
            PowLhs::Var(name) => write!(f, "{}", name),
            PowLhs::IntLit(value) => write!(f, "{}", value),
            PowLhs::FloatLit(value) => write!(f, "{}", value),
            PowLhs::BoolLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for PowRhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowRhs::Paren(inner) => write!(f, "({})", inner),
            PowRhs::Pow { lhs, rhs } => write!(f, "({} ^ {})", lhs, rhs),
            PowRhs::Neg { inner } => write!(f, "(-{})", inner),
            PowRhs::Ref { inner } => write!(f, "(&{})", inner),
            PowRhs::Var(name) => write!(f, "{}", name),
            PowRhs::IntLit(value) => write!(f, "{}", value),
            PowRhs::FloatLit(value) => write!(f, "{}", value),
            PowRhs::BoolLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Var(name) => write!(f, "{}", name),
            Atom::IntLit(value) => write!(f, "{}", value),
            Atom::FloatLit(value) => write!(f, "{}", value),
            Atom::BoolLit(value) => write!(f, "{}", value),
        }
    }
}

// ============================================================================
// Additional From Implementations for Parser Convenience
// ============================================================================

/// Convert AddLhs to CmpRhs (AddLhs is a subset of CmpRhs)
impl From<AddLhs> for CmpRhs {
    fn from(add: AddLhs) -> Self {
        match add {
            AddLhs::Add { lhs, rhs } => CmpRhs::Add { lhs, rhs },
            AddLhs::Sub { lhs, rhs } => CmpRhs::Sub { lhs, rhs },
            AddLhs::Paren(e) => CmpRhs::Paren(e),
            AddLhs::Mul { lhs, rhs } => CmpRhs::Mul { lhs, rhs },
            AddLhs::Div { lhs, rhs } => CmpRhs::Div { lhs, rhs },
            AddLhs::Mod { lhs, rhs } => CmpRhs::Mod { lhs, rhs },
            AddLhs::Pow { lhs, rhs } => CmpRhs::Pow { lhs, rhs },
            AddLhs::Neg { inner } => CmpRhs::Neg { inner },
            AddLhs::Ref { inner } => CmpRhs::Ref { inner },
            AddLhs::Var(s) => CmpRhs::Var(s),
            AddLhs::IntLit(i) => CmpRhs::IntLit(i),
            AddLhs::FloatLit(f) => CmpRhs::FloatLit(f),
            AddLhs::BoolLit(b) => CmpRhs::BoolLit(b),
        }
    }
}

/// Convert AddLhs to CmpLhs (AddLhs is a subset of CmpLhs)
impl From<AddLhs> for CmpLhs {
    fn from(add: AddLhs) -> Self {
        match add {
            AddLhs::Add { lhs, rhs } => CmpLhs::Add { lhs, rhs },
            AddLhs::Sub { lhs, rhs } => CmpLhs::Sub { lhs, rhs },
            AddLhs::Paren(e) => CmpLhs::Paren(e),
            AddLhs::Mul { lhs, rhs } => CmpLhs::Mul { lhs, rhs },
            AddLhs::Div { lhs, rhs } => CmpLhs::Div { lhs, rhs },
            AddLhs::Mod { lhs, rhs } => CmpLhs::Mod { lhs, rhs },
            AddLhs::Pow { lhs, rhs } => CmpLhs::Pow { lhs, rhs },
            AddLhs::Neg { inner } => CmpLhs::Neg { inner },
            AddLhs::Ref { inner } => CmpLhs::Ref { inner },
            AddLhs::Var(s) => CmpLhs::Var(s),
            AddLhs::IntLit(i) => CmpLhs::IntLit(i),
            AddLhs::FloatLit(f) => CmpLhs::FloatLit(f),
            AddLhs::BoolLit(b) => CmpLhs::BoolLit(b),
        }
    }
}

/// Convert Atom to MulRhs (Atom is a subset of MulRhs)
impl From<Atom> for MulRhs {
    fn from(atom: Atom) -> Self {
        match atom {
            Atom::Var(s) => MulRhs::Var(s),
            Atom::IntLit(i) => MulRhs::IntLit(i),
            Atom::FloatLit(f) => MulRhs::FloatLit(f),
            Atom::BoolLit(b) => MulRhs::BoolLit(b),
        }
    }
}

/// Convert Atom to MulLhs (Atom is a subset of MulLhs)
impl From<Atom> for MulLhs {
    fn from(atom: Atom) -> Self {
        match atom {
            Atom::Var(s) => MulLhs::Var(s),
            Atom::IntLit(i) => MulLhs::IntLit(i),
            Atom::FloatLit(f) => MulLhs::FloatLit(f),
            Atom::BoolLit(b) => MulLhs::BoolLit(b),
        }
    }
}

/// Convert MulLhs to AddRhs (MulLhs is a subset of AddRhs)
impl From<MulLhs> for AddRhs {
    fn from(mul: MulLhs) -> Self {
        match mul {
            MulLhs::Paren(e) => AddRhs::Paren(e),
            MulLhs::Mul { lhs, rhs } => AddRhs::Mul { lhs, rhs },
            MulLhs::Div { lhs, rhs } => AddRhs::Div { lhs, rhs },
            MulLhs::Mod { lhs, rhs } => AddRhs::Mod { lhs, rhs },
            MulLhs::Pow { lhs, rhs } => AddRhs::Pow { lhs, rhs },
            MulLhs::Neg { inner } => AddRhs::Neg { inner },
            MulLhs::Ref { inner } => AddRhs::Ref { inner },
            MulLhs::Var(s) => AddRhs::Var(s),
            MulLhs::IntLit(i) => AddRhs::IntLit(i),
            MulLhs::FloatLit(f) => AddRhs::FloatLit(f),
            MulLhs::BoolLit(b) => AddRhs::BoolLit(b),
        }
    }
}

/// Convert MulLhs to AddLhs (MulLhs is a subset of AddLhs)
impl From<MulLhs> for AddLhs {
    fn from(mul: MulLhs) -> Self {
        match mul {
            MulLhs::Paren(e) => AddLhs::Paren(e),
            MulLhs::Mul { lhs, rhs } => AddLhs::Mul { lhs, rhs },
            MulLhs::Div { lhs, rhs } => AddLhs::Div { lhs, rhs },
            MulLhs::Mod { lhs, rhs } => AddLhs::Mod { lhs, rhs },
            MulLhs::Pow { lhs, rhs } => AddLhs::Pow { lhs, rhs },
            MulLhs::Neg { inner } => AddLhs::Neg { inner },
            MulLhs::Ref { inner } => AddLhs::Ref { inner },
            MulLhs::Var(s) => AddLhs::Var(s),
            MulLhs::IntLit(i) => AddLhs::IntLit(i),
            MulLhs::FloatLit(f) => AddLhs::FloatLit(f),
            MulLhs::BoolLit(b) => AddLhs::BoolLit(b),
        }
    }
}

/// Convert Atom to PowLhs (Atom is a subset of PowLhs)
impl From<Atom> for PowLhs {
    fn from(atom: Atom) -> Self {
        match atom {
            Atom::Var(s) => PowLhs::Var(s),
            Atom::IntLit(i) => PowLhs::IntLit(i),
            Atom::FloatLit(f) => PowLhs::FloatLit(f),
            Atom::BoolLit(b) => PowLhs::BoolLit(b),
        }
    }
}

/// Convert Atom to PowRhs (Atom is a subset of PowRhs)
impl From<Atom> for PowRhs {
    fn from(atom: Atom) -> Self {
        match atom {
            Atom::Var(s) => PowRhs::Var(s),
            Atom::IntLit(i) => PowRhs::IntLit(i),
            Atom::FloatLit(f) => PowRhs::FloatLit(f),
            Atom::BoolLit(b) => PowRhs::BoolLit(b),
        }
    }
}

/// Convert PowLhs to PowRhs (PowLhs is a subset of PowRhs except for Pow)
impl From<PowLhs> for PowRhs {
    fn from(pow: PowLhs) -> Self {
        match pow {
            PowLhs::Paren(e) => PowRhs::Paren(e),
            PowLhs::Neg { inner } => PowRhs::Neg { inner },
            PowLhs::Ref { inner } => PowRhs::Ref { inner },
            PowLhs::Var(s) => PowRhs::Var(s),
            PowLhs::IntLit(i) => PowRhs::IntLit(i),
            PowLhs::FloatLit(f) => PowRhs::FloatLit(f),
            PowLhs::BoolLit(b) => PowRhs::BoolLit(b),
        }
    }
}

/// Convert PowLhs to MulRhs (PowLhs is a subset of MulRhs)
impl From<PowLhs> for MulRhs {
    fn from(pow: PowLhs) -> Self {
        match pow {
            PowLhs::Paren(e) => MulRhs::Paren(e),
            PowLhs::Neg { inner } => MulRhs::Neg { inner },
            PowLhs::Ref { inner } => MulRhs::Ref { inner },
            PowLhs::Var(s) => MulRhs::Var(s),
            PowLhs::IntLit(i) => MulRhs::IntLit(i),
            PowLhs::FloatLit(f) => MulRhs::FloatLit(f),
            PowLhs::BoolLit(b) => MulRhs::BoolLit(b),
        }
    }
}

/// Convert PowLhs to MulLhs (PowLhs is a subset of MulLhs)
impl From<PowLhs> for MulLhs {
    fn from(pow: PowLhs) -> Self {
        match pow {
            PowLhs::Paren(e) => MulLhs::Paren(e),
            PowLhs::Neg { inner } => MulLhs::Neg { inner },
            PowLhs::Ref { inner } => MulLhs::Ref { inner },
            PowLhs::Var(s) => MulLhs::Var(s),
            PowLhs::IntLit(i) => MulLhs::IntLit(i),
            PowLhs::FloatLit(f) => MulLhs::FloatLit(f),
            PowLhs::BoolLit(b) => MulLhs::BoolLit(b),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom_to_expr_conversion() {
        let atom = Atom::Var("x".to_string());
        let expr: Expr = atom.into();
        assert_eq!(expr, Expr::Var("x".to_string()));
    }

    #[test]
    fn test_atom_to_mulrhs_via_expr() {
        let atom = Atom::Var("x".to_string());
        let expr: Expr = atom.into();
        let mulrhs: Result<MulRhs, _> = expr.try_into();
        assert!(mulrhs.is_ok());
        assert_eq!(mulrhs.unwrap(), MulRhs::Var("x".to_string()));
    }

    #[test]
    fn test_mulrhs_to_addrhs_via_expr() {
        let mulrhs = MulRhs::Var("x".to_string());
        let expr: Expr = mulrhs.into();
        let addrhs: Result<AddRhs, _> = expr.try_into();
        assert!(addrhs.is_ok());
        assert_eq!(addrhs.unwrap(), AddRhs::Var("x".to_string()));
    }

    #[test]
    fn test_mul_structure() {
        let mul = Expr::Mul {
            lhs: Box::new(MulLhs::Var("a".to_string())),
            rhs: Box::new(MulRhs::Var("b".to_string())),
        };
        match mul {
            Expr::Mul { .. } => {}
            _ => panic!("Expected Mul"),
        }
    }

    #[test]
    fn test_add_structure() {
        let add = Expr::Add {
            lhs: Box::new(AddLhs::Var("a".to_string())),
            rhs: Box::new(AddRhs::Var("b".to_string())),
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
            lhs: Box::new(MulLhs::Var("b".to_string())),
            rhs: Box::new(MulRhs::Var("c".to_string())),
        };

        // Addition with multiplication on right side
        let add = Expr::Add {
            lhs: Box::new(AddLhs::Var("a".to_string())),
            rhs: Box::new(mul_in_addrhs),
        };

        assert!(matches!(add, Expr::Add { .. }));
    }

    #[test]
    fn test_display_simple_var() {
        let expr = Expr::Var("x".to_string());
        assert_eq!(format!("{}", expr), "x");
    }

    #[test]
    fn test_display_simple_add() {
        let expr = Expr::Add {
            lhs: Box::new(AddLhs::Var("a".to_string())),
            rhs: Box::new(AddRhs::Var("b".to_string())),
        };
        assert_eq!(format!("{}", expr), "(a + b)");
    }

    #[test]
    fn test_display_precedence() {
        // a + b * c
        let expr = Expr::Add {
            lhs: Box::new(AddLhs::Var("a".to_string())),
            rhs: Box::new(AddRhs::Mul {
                lhs: Box::new(MulLhs::Var("b".to_string())),
                rhs: Box::new(MulRhs::Var("c".to_string())),
            }),
        };
        assert_eq!(format!("{}", expr), "(a + (b * c))");
    }
}
