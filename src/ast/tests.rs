#[cfg(test)]
mod tests {
    use crate::ast::expr::*;
    use crate::lexer::{LineColumn, Span};

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
