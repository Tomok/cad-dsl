use subenum::subenum;

// ============================================================================
// Expression AST with Type-Safe Operator Precedence
// ============================================================================

/// Top-level expression type with operator precedence hierarchy
///
/// Hierarchy with separate Left/Right-hand side types:
/// - Expr: All variants (top-level)
/// - AddLhs: Add, Sub, Paren, Mul, Div, Var, IntLit, FloatLit (left side of +/-)
/// - AddRhs: Paren, Mul, Div, Var, IntLit, FloatLit (right side of +/-, NO Add/Sub)
/// - MulLhs: Paren, Mul, Div, Var, IntLit, FloatLit (left side of *//, NO Add/Sub)
/// - MulRhs: Paren, Var, IntLit, FloatLit (right side of *//, NO Mul/Div)
/// - Atom: Var, IntLit, FloatLit (only literals and variables)
///
/// This ensures:
/// - Addition RHS cannot contain addition/subtraction (enforces precedence)
/// - Multiplication RHS cannot contain multiplication/division (enforces precedence)
/// - Left-hand sides allow recursion at the same precedence level (left-associativity)
/// - Right-hand sides enforce higher precedence
#[subenum(AddLhs, AddRhs, MulLhs, MulRhs, Atom)]
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // Addition - only in Expr and AddLhs
    // lhs can be Add/Sub, rhs cannot (enforces left-associativity and precedence)
    #[subenum(AddLhs)]
    Add { lhs: Box<AddLhs>, rhs: Box<AddRhs> },

    // Subtraction - only in Expr and AddLhs
    #[subenum(AddLhs)]
    Sub { lhs: Box<AddLhs>, rhs: Box<AddRhs> },

    // Parentheses - in all contexts except Atom (resets precedence)
    #[subenum(AddLhs, AddRhs, MulLhs, MulRhs)]
    Paren(Box<Expr>),

    // Multiplication - in AddLhs, AddRhs, MulLhs
    // lhs can be Mul/Div, rhs cannot (enforces left-associativity)
    #[subenum(AddLhs, AddRhs, MulLhs)]
    Mul { lhs: Box<MulLhs>, rhs: Box<MulRhs> },

    // Division - in AddLhs, AddRhs, MulLhs
    #[subenum(AddLhs, AddRhs, MulLhs)]
    Div { lhs: Box<MulLhs>, rhs: Box<MulRhs> },

    // Variable reference - in all levels
    #[subenum(AddLhs, AddRhs, MulLhs, MulRhs, Atom)]
    Var(String),

    // Integer literal - in all levels
    #[subenum(AddLhs, AddRhs, MulLhs, MulRhs, Atom)]
    IntLit(i32),

    // Float literal - in all levels
    #[subenum(AddLhs, AddRhs, MulLhs, MulRhs, Atom)]
    FloatLit(f64),
}

// The subenum macro automatically generates:
//
// pub enum AddLhs {
//     Add { lhs: Box<AddLhs>, rhs: Box<AddRhs> },
//     Sub { lhs: Box<AddLhs>, rhs: Box<AddRhs> },
//     Paren(Box<Expr>),
//     Mul { lhs: Box<MulLhs>, rhs: Box<MulRhs> },
//     Div { lhs: Box<MulLhs>, rhs: Box<MulRhs> },
//     Var(String),
//     IntLit(i32),
//     FloatLit(f64),
// }
//
// pub enum AddRhs {
//     Paren(Box<Expr>),
//     Mul { lhs: Box<MulLhs>, rhs: Box<MulRhs> },
//     Div { lhs: Box<MulLhs>, rhs: Box<MulRhs> },
//     Var(String),
//     IntLit(i32),
//     FloatLit(f64),
// }
//
// pub enum MulLhs {
//     Paren(Box<Expr>),
//     Mul { lhs: Box<MulLhs>, rhs: Box<MulRhs> },
//     Div { lhs: Box<MulLhs>, rhs: Box<MulRhs> },
//     Var(String),
//     IntLit(i32),
//     FloatLit(f64),
// }
//
// pub enum MulRhs {
//     Paren(Box<Expr>),
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
// - Atom can be converted to MulRhs, MulLhs, AddRhs, AddLhs, or Expr
// - MulRhs can be converted to AddRhs, AddLhs, or Expr
// - MulLhs can be converted to AddRhs, AddLhs, or Expr
// - AddRhs can be converted to AddLhs or Expr
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
        let expr: Expr = self.clone().into();
        write!(f, "{}", expr)
    }
}

impl std::fmt::Display for AddRhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let expr: Expr = self.clone().into();
        write!(f, "{}", expr)
    }
}

impl std::fmt::Display for MulLhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let expr: Expr = self.clone().into();
        write!(f, "{}", expr)
    }
}

impl std::fmt::Display for MulRhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let expr: Expr = self.clone().into();
        write!(f, "{}", expr)
    }
}

impl std::fmt::Display for Atom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let expr: Expr = self.clone().into();
        write!(f, "{}", expr)
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
            Expr::Mul { .. } => {},
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
            Expr::Add { .. } => {},
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
