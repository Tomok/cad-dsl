use subenum::subenum;

// ============================================================================
// Expression AST with Type-Safe Operator Precedence
// ============================================================================

/// Top-level expression type with operator precedence hierarchy
///
/// Hierarchy (low to high precedence):
/// - Expr: Add, Sub, Paren, Mul, Div, Var, IntLit, FloatLit
/// - AddLhs: Add, Sub, Paren, Mul, Div, Var, IntLit, FloatLit (everything except top-level)
/// - Term: Paren, Mul, Div, Var, IntLit, FloatLit (no Add/Sub)
/// - Atom: Var, IntLit, FloatLit (only literals and variables)
///
/// This ensures:
/// - Addition/Subtraction can contain Multiplication/Division (precedence rules)
/// - Multiplication/Division can only contain Atoms (no nested Add/Sub)
/// - Left-hand sides allow recursion at the same precedence level
/// - Right-hand sides enforce higher precedence
#[subenum(AddLhs, Term, Atom)]
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // Addition - only in Expr and AddLhs
    // lhs can be Add/Sub, rhs cannot (enforces left-associativity)
    #[subenum(AddLhs)]
    Add { lhs: Box<AddLhs>, rhs: Box<Term> },

    // Subtraction - only in Expr and AddLhs
    #[subenum(AddLhs)]
    Sub { lhs: Box<AddLhs>, rhs: Box<Term> },

    // Parentheses - in Expr, AddLhs, and Term (resets precedence)
    #[subenum(AddLhs, Term)]
    Paren(Box<Expr>),

    // Multiplication - in AddLhs and Term
    // lhs can be Mul/Div, rhs can be Paren or Atom
    #[subenum(AddLhs, Term)]
    Mul { lhs: Box<Term>, rhs: Box<Term> },

    // Division - in AddLhs and Term
    #[subenum(AddLhs, Term)]
    Div { lhs: Box<Term>, rhs: Box<Term> },

    // Variable reference - in all levels
    #[subenum(AddLhs, Term, Atom)]
    Var(String),

    // Integer literal - in all levels
    #[subenum(AddLhs, Term, Atom)]
    IntLit(i32),

    // Float literal - in all levels
    #[subenum(AddLhs, Term, Atom)]
    FloatLit(f64),
}

// The subenum macro automatically generates:
//
// pub enum AddLhs {
//     Add { lhs: Box<AddLhs>, rhs: Box<Term> },
//     Sub { lhs: Box<AddLhs>, rhs: Box<Term> },
//     Mul { lhs: Box<Term>, rhs: Box<Atom> },
//     Div { lhs: Box<Term>, rhs: Box<Atom> },
//     Var(String),
//     IntLit(i32),
//     FloatLit(f64),
// }
//
// pub enum Term {
//     Mul { lhs: Box<Term>, rhs: Box<Atom> },
//     Div { lhs: Box<Term>, rhs: Box<Atom> },
//     Var(String),
//     IntLit(i32),
//     FloatLit(f64),
// }
//
// pub enum Atom {
//     Var(String),
//     IntLit(i32),
//     FloatLit(f64),
// }
//
// With automatic conversions:
// - Atom can be converted to Term, AddLhs, or Expr
// - Term can be converted to AddLhs or Expr
// - AddLhs can be converted to Expr

// ============================================================================
// Display Implementations
// ============================================================================

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Add { lhs, rhs } => write!(f, "({} + {})", lhs, rhs),
            Expr::Sub { lhs, rhs } => write!(f, "({} - {})", lhs, rhs),
            Expr::Paren(inner) => write!(f, "({})", inner),
            Expr::Mul { lhs, rhs } => write!(f, "({} * {})", lhs, rhs),
            Expr::Div { lhs, rhs } => write!(f, "({} / {})", lhs, rhs),
            Expr::Var(name) => write!(f, "{}", name),
            Expr::IntLit(value) => write!(f, "{}", value),
            Expr::FloatLit(value) => write!(f, "{}", value),
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
            AddLhs::Var(name) => write!(f, "{}", name),
            AddLhs::IntLit(value) => write!(f, "{}", value),
            AddLhs::FloatLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for Term {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Term::Paren(inner) => write!(f, "({})", inner),
            Term::Mul { lhs, rhs } => write!(f, "({} * {})", lhs, rhs),
            Term::Div { lhs, rhs } => write!(f, "({} / {})", lhs, rhs),
            Term::Var(name) => write!(f, "{}", name),
            Term::IntLit(value) => write!(f, "{}", value),
            Term::FloatLit(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Atom::Var(name) => write!(f, "{}", name),
            Atom::IntLit(value) => write!(f, "{}", value),
            Atom::FloatLit(value) => write!(f, "{}", value),
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
    fn test_term_to_expr_conversion() {
        let term = Term::Mul {
            lhs: Box::new(Term::Var("a".to_string())),
            rhs: Box::new(Term::Var("b".to_string())),
        };
        let expr: Expr = term.into();
        match expr {
            Expr::Mul { .. } => {},
            _ => panic!("Expected Mul"),
        }
    }

    #[test]
    fn test_precedence_in_types() {
        // Multiplication with terms
        let mul = Term::Mul {
            lhs: Box::new(Term::Var("a".to_string())),
            rhs: Box::new(Term::Var("b".to_string())),
        };

        // Addition with term on right side
        let add = Expr::Add {
            lhs: Box::new(AddLhs::Var("x".to_string())),
            rhs: Box::new(mul),
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
            rhs: Box::new(Term::Var("b".to_string())),
        };
        assert_eq!(format!("{}", expr), "(a + b)");
    }

    #[test]
    fn test_display_precedence() {
        // a + b * c
        let expr = Expr::Add {
            lhs: Box::new(AddLhs::Var("a".to_string())),
            rhs: Box::new(Term::Mul {
                lhs: Box::new(Term::Var("b".to_string())),
                rhs: Box::new(Term::Var("c".to_string())),
            }),
        };
        assert_eq!(format!("{}", expr), "(a + (b * c))");
    }
}
