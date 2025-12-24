use chumsky::prelude::*;

use crate::ast::{AddRhs, Atom, Expr, MulLhs, MulRhs};
use crate::lexer::Token;

// ============================================================================
// Parser Error Type
// ============================================================================

pub type ParseError<'src> = extra::Err<Rich<'src, Token<'src>>>;

// ============================================================================
// Expression Parser
// ============================================================================

/// Parse an expression from a stream of tokens
pub fn expr_parser<'src>() -> impl Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> {
    recursive(|expr| {
        // Atom parser: variables and literals
        let atom = choice((
            // Variable
            select! {
                Token::Identifier(id) => Atom::Var(id.name.to_string())
            },
            // Integer literal
            select! {
                Token::IntLiteral(lit) => Atom::IntLit(lit.value)
            },
            // Float literal
            select! {
                Token::FloatLiteral(lit) => Atom::FloatLit(lit.value)
            },
        ));

        // Primary: atom or parenthesized expression
        let primary = choice((
            atom.map(|a| Expr::from(a)),
            expr.clone()
                .delimited_by(
                    select! { Token::LeftParen(_) => () },
                    select! { Token::RightParen(_) => () },
                )
                .map(|e| Expr::Paren(Box::new(e))),
        ));

        // MulRhs parser: handles right-hand side of multiplication/division
        // Can be: Paren, Atom (no Mul/Div allowed)
        let mulrhs_parser = primary.clone();

        // MulLhs parser: handles multiplication and division (left-associative)
        // lhs can be Mul/Div/Paren/Atom, rhs can only be MulRhs (Paren/Atom)
        let mullhs_parser = mulrhs_parser
            .clone()
            .foldl(
                choice((
                    select! { Token::Multiply(_) => () }
                        .then(mulrhs_parser.clone())
                        .map(|(_, rhs)| (BinOp::Mul, rhs)),
                    select! { Token::Divide(_) => () }
                        .then(mulrhs_parser.clone())
                        .map(|(_, rhs)| (BinOp::Div, rhs)),
                ))
                .repeated(),
                |lhs, (op, rhs)| {
                    // Convert Expr to MulLhs for left-hand side
                    let lhs_mullhs: MulLhs = lhs.try_into()
                        .expect("Left-hand side should be convertible to MulLhs");

                    // Convert Expr to MulRhs for right-hand side
                    let rhs_mulrhs: MulRhs = rhs.try_into()
                        .expect("Right-hand side should be convertible to MulRhs");

                    match op {
                        BinOp::Mul => Expr::Mul {
                            lhs: Box::new(lhs_mullhs),
                            rhs: Box::new(rhs_mulrhs),
                        },
                        BinOp::Div => Expr::Div {
                            lhs: Box::new(lhs_mullhs),
                            rhs: Box::new(rhs_mulrhs),
                        },
                        _ => unreachable!(),
                    }
                },
            );

        // AddRhs parser: right-hand side of addition/subtraction
        // Can be: Paren, Mul, Div, Atom (no Add/Sub allowed)
        let addrhs_parser = mullhs_parser.clone();

        // Expression parser: addition and subtraction (left-associative)
        // lhs can be Add/Sub/Mul/Div/Paren/Atom, rhs can only be AddRhs (Mul/Div/Paren/Atom)
        addrhs_parser
            .clone()
            .foldl(
                choice((
                    select! { Token::Plus(_) => () }
                        .then(addrhs_parser.clone())
                        .map(|(_, rhs)| (BinOp::Add, rhs)),
                    select! { Token::Minus(_) => () }
                        .then(addrhs_parser.clone())
                        .map(|(_, rhs)| (BinOp::Sub, rhs)),
                ))
                .repeated(),
                |lhs, (op, rhs)| {
                    // Convert Expr to AddLhs for left-hand side
                    let lhs_addlhs = lhs.try_into()
                        .expect("Left-hand side should be convertible to AddLhs");

                    // Convert Expr to AddRhs for right-hand side
                    let rhs_addrhs: AddRhs = rhs.try_into()
                        .expect("Right-hand side should be convertible to AddRhs");

                    match op {
                        BinOp::Add => Expr::Add {
                            lhs: Box::new(lhs_addlhs),
                            rhs: Box::new(rhs_addrhs),
                        },
                        BinOp::Sub => Expr::Sub {
                            lhs: Box::new(lhs_addlhs),
                            rhs: Box::new(rhs_addrhs),
                        },
                        _ => unreachable!(),
                    }
                },
            )
    })
}

// ============================================================================
// Helper Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
}

// ============================================================================
// Public Parse Function
// ============================================================================

/// Parse a string into an expression AST
pub fn parse_expr(input: &str) -> Result<Expr, String> {
    // First tokenize
    let tokens = crate::lexer::tokenize(input)?;

    // Then parse
    let parser = expr_parser().then_ignore(end());
    parser
        .parse(&tokens)
        .into_result()
        .map_err(|errors| {
            errors
                .into_iter()
                .map(|e| format!("Parse error: {:?}", e))
                .collect::<Vec<_>>()
                .join("\n")
        })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::AddLhs;

    #[test]
    fn test_parse_simple_var() {
        let result = parse_expr("x");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Expr::Var("x".to_string()));
    }

    #[test]
    fn test_parse_int_literal() {
        let result = parse_expr("42");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Expr::IntLit(42));
    }

    #[test]
    fn test_parse_float_literal() {
        let result = parse_expr("3.14");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Expr::FloatLit(3.14));
    }

    #[test]
    fn test_parse_simple_add() {
        let result = parse_expr("a + b");
        assert!(result.is_ok());

        match result.unwrap() {
            Expr::Add { lhs, rhs } => {
                assert_eq!(*lhs, AddLhs::Var("a".to_string()));
                assert_eq!(*rhs, AddRhs::Var("b".to_string()));
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_simple_mul() {
        let result = parse_expr("a * b");
        assert!(result.is_ok());

        match result.unwrap() {
            Expr::Mul { lhs, rhs } => {
                assert_eq!(*lhs, MulLhs::Var("a".to_string()));
                assert_eq!(*rhs, MulRhs::Var("b".to_string()));
            }
            _ => panic!("Expected Mul"),
        }
    }

    #[test]
    fn test_parse_precedence_add_mul() {
        // a + b * c should parse as a + (b * c)
        let result = parse_expr("a + b * c");
        assert!(result.is_ok());

        match result.unwrap() {
            Expr::Add { lhs, rhs } => {
                assert_eq!(*lhs, AddLhs::Var("a".to_string()));
                match *rhs {
                    AddRhs::Mul { lhs: mul_lhs, rhs: mul_rhs } => {
                        assert_eq!(*mul_lhs, MulLhs::Var("b".to_string()));
                        assert_eq!(*mul_rhs, MulRhs::Var("c".to_string()));
                    }
                    _ => panic!("Expected Mul on rhs of Add"),
                }
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_precedence_mul_add() {
        // a * b + c should parse as (a * b) + c
        let result = parse_expr("a * b + c");
        assert!(result.is_ok());

        match result.unwrap() {
            Expr::Add { lhs, rhs } => {
                match *lhs {
                    AddLhs::Mul { lhs: mul_lhs, rhs: mul_rhs } => {
                        assert_eq!(*mul_lhs, MulLhs::Var("a".to_string()));
                        assert_eq!(*mul_rhs, MulRhs::Var("b".to_string()));
                    }
                    _ => panic!("Expected Mul on lhs of Add"),
                }
                assert_eq!(*rhs, AddRhs::Var("c".to_string()));
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_left_associative_add() {
        // a + b + c should parse as (a + b) + c
        let result = parse_expr("a + b + c");
        assert!(result.is_ok());

        match result.unwrap() {
            Expr::Add { lhs, rhs } => {
                match *lhs {
                    AddLhs::Add {
                        lhs: inner_lhs,
                        rhs: inner_rhs,
                    } => {
                        assert_eq!(*inner_lhs, AddLhs::Var("a".to_string()));
                        assert_eq!(*inner_rhs, AddRhs::Var("b".to_string()));
                    }
                    _ => panic!("Expected Add on lhs"),
                }
                assert_eq!(*rhs, AddRhs::Var("c".to_string()));
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_parse_left_associative_mul() {
        // a * b * c should parse as (a * b) * c
        let result = parse_expr("a * b * c");
        assert!(result.is_ok());

        match result.unwrap() {
            Expr::Mul { lhs, rhs } => {
                match *lhs {
                    MulLhs::Mul {
                        lhs: inner_lhs,
                        rhs: inner_rhs,
                    } => {
                        assert_eq!(*inner_lhs, MulLhs::Var("a".to_string()));
                        assert_eq!(*inner_rhs, MulRhs::Var("b".to_string()));
                    }
                    _ => panic!("Expected Mul on lhs"),
                }
                assert_eq!(*rhs, MulRhs::Var("c".to_string()));
            }
            _ => panic!("Expected Mul"),
        }
    }

    #[test]
    fn test_parse_parentheses() {
        // (a + b) * c should enforce the grouping
        let result = parse_expr("(a + b) * c");
        assert!(result.is_ok());

        match result.unwrap() {
            Expr::Mul { lhs, rhs } => {
                match *lhs {
                    MulLhs::Paren(inner) => match *inner {
                        Expr::Add { .. } => {}
                        _ => panic!("Expected Add inside Paren"),
                    },
                    _ => panic!("Expected Paren on lhs of Mul"),
                }
                assert_eq!(*rhs, MulRhs::Var("c".to_string()));
            }
            _ => panic!("Expected Mul"),
        }
    }

    #[test]
    fn test_parse_complex_expression() {
        // (a + b) * (c - d)
        let result = parse_expr("(a + b) * (c - d)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_nested_parentheses() {
        // ((a))
        let result = parse_expr("((a))");
        assert!(result.is_ok());
    }
}
