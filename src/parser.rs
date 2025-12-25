use crate::ast::*;
use crate::lexer::Token;
use chumsky::prelude::*;

// ============================================================================
// Parser Type Definitions
// ============================================================================

/// The error type used by the parser
pub type ParseError<'src> = extra::Err<Rich<'src, Token<'src>>>;

// ============================================================================
// Atomic Parsers (No recursion)
// ============================================================================

/// Parse an integer literal token
fn int_lit<'src>() -> impl Parser<'src, &'src [Token<'src>], i32, ParseError<'src>> + Clone {
    select! {
        Token::IntLiteral(t) => t.value,
    }
    .labelled("integer")
}

/// Parse a float literal token
fn float_lit<'src>() -> impl Parser<'src, &'src [Token<'src>], f64, ParseError<'src>> + Clone {
    select! {
        Token::FloatLiteral(t) => t.value,
    }
    .labelled("float")
}

/// Parse a variable name (identifier token)
fn var<'src>() -> impl Parser<'src, &'src [Token<'src>], String, ParseError<'src>> + Clone {
    select! {
        Token::Identifier(t) => t.name.to_string(),
    }
    .labelled("variable")
}

/// Parse an atomic expression (Atom enum)
fn atom<'src>() -> impl Parser<'src, &'src [Token<'src>], Atom, ParseError<'src>> + Clone {
    choice((
        // Try float first (it's more specific)
        float_lit().map(Atom::FloatLit),
        // Then integer
        int_lit().map(Atom::IntLit),
        // Finally variable
        var().map(Atom::Var),
    ))
    .labelled("atom")
}

// ============================================================================
// Recursive Expression Parsers
// ============================================================================

fn expr_inner<'src>() -> impl Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone {
    recursive(|expr_rec| {
        // Parenthesis tokens
        let lparen = select! { Token::LeftParen(_) => () };
        let rparen = select! { Token::RightParen(_) => () };

        // Operator tokens
        let mul_op = select! { Token::Multiply(_) => '*' };
        let div_op = select! { Token::Divide(_) => '/' };
        let add_op = select! { Token::Plus(_) => '+' };
        let sub_op = select! { Token::Minus(_) => '-' };

        // MulRhs: Paren, Var, IntLit, FloatLit
        let mul_rhs = choice((
            atom().map(Into::into),
            expr_rec
                .clone()
                .delimited_by(lparen, rparen)
                .map(|e| MulRhs::Paren(Box::new(e))),
        ));

        // MulLhs: Paren, Mul, Div, Var, IntLit, FloatLit
        let mul_atom = choice((
            atom().map(Into::into),
            expr_rec
                .clone()
                .delimited_by(lparen, rparen)
                .map(|e| MulLhs::Paren(Box::new(e))),
        ));

        // Left-associative multiplication and division
        let mul_lhs = mul_atom.clone().foldl(
            choice((mul_op, div_op))
                .then(mul_rhs.clone())
                .repeated(),
            |lhs, (op, rhs)| {
                if op == '*' {
                    MulLhs::Mul {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    }
                } else {
                    MulLhs::Div {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    }
                }
            },
        );

        // AddRhs: Paren, Mul, Div, Var, IntLit, FloatLit
        let add_rhs = mul_lhs.clone().map(Into::into);

        // AddLhs: Add, Sub, Paren, Mul, Div, Var, IntLit, FloatLit
        let add_lhs = {
            let add_atom = mul_lhs.clone().map(Into::into);

            // Left-associative addition and subtraction
            add_atom.clone().foldl(
                choice((add_op, sub_op))
                    .then(add_rhs.clone())
                    .repeated(),
                |lhs, (op, rhs)| {
                    if op == '+' {
                        AddLhs::Add {
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        }
                    } else {
                        AddLhs::Sub {
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                        }
                    }
                },
            )
        };

        // Convert AddLhs to Expr
        add_lhs.map(Into::into)
    })
}

pub fn expr<'src>() -> impl Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone {
    expr_inner().then_ignore(end())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    /// Helper function to parse with timeout
    /// This prevents tests from hanging indefinitely if there's infinite recursion
    ///
    /// Note: input must be 'static for thread safety
    fn parse_with_timeout<T: Send + 'static>(
        input: &'static str,
        parse_fn: impl FnOnce(&'static [Token<'static>]) -> Result<T, Vec<Rich<'static, Token<'static>>>>
            + Send
            + 'static,
        timeout: Duration,
    ) -> Result<T, String> {
        // First tokenize the input - since input is 'static, tokens will be too
        let tokens = lexer::tokenize(input).map_err(|e| format!("Lexer error: {}", e))?;

        // Make tokens static by leaking (only for tests)
        let tokens_static: &'static [Token<'static>] = Box::leak(tokens.into_boxed_slice());

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let result = parse_fn(tokens_static);
            let _ = tx.send(result);
        });

        rx.recv_timeout(timeout)
            .map_err(|_| "Test timeout - possible infinite recursion".to_string())
            .and_then(|r| r.map_err(|e| format!("Parse error: {:?}", e)))
    }

    #[test]
    fn test_int_lit() {
        let result = parse_with_timeout(
            "42",
            |input| int_lit().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_float_lit() {
        let result = parse_with_timeout(
            "3.5",
            |input| float_lit().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), 3.5);
    }

    #[test]
    fn test_var() {
        let result = parse_with_timeout(
            "foo",
            |input| var().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), "foo");
    }

    #[test]
    fn test_atom_int() {
        let result = parse_with_timeout(
            "42",
            |input| atom().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Atom::IntLit(42));
    }

    #[test]
    fn test_atom_float() {
        let result = parse_with_timeout(
            "3.5",
            |input| atom().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Atom::FloatLit(3.5));
    }

    #[test]
    fn test_atom_var() {
        let result = parse_with_timeout(
            "x",
            |input| atom().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Atom::Var("x".to_string()));
    }

    #[test]
    fn test_expr_simple_var() {
        let result = parse_with_timeout(
            "x",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert_eq!(result.unwrap(), Expr::Var("x".to_string()));
    }

    #[test]
    fn test_expr_simple_int() {
        let result = parse_with_timeout(
            "42",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert_eq!(result.unwrap(), Expr::IntLit(42));
    }

    #[test]
    fn test_expr_simple_add() {
        let result = parse_with_timeout(
            "1 + 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Add {
            lhs: Box::new(AddLhs::IntLit(1)),
            rhs: Box::new(AddRhs::IntLit(2)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_simple_mul() {
        let result = parse_with_timeout(
            "3 * 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mul {
            lhs: Box::new(MulLhs::IntLit(3)),
            rhs: Box::new(MulRhs::IntLit(4)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_precedence() {
        // Test: 1 + 2 * 3
        let result = parse_with_timeout(
            "1 + 2 * 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Add {
            lhs: Box::new(AddLhs::IntLit(1)),
            rhs: Box::new(AddRhs::Mul {
                lhs: Box::new(MulLhs::IntLit(2)),
                rhs: Box::new(MulRhs::IntLit(3)),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_left_associative_add() {
        // Test: 1 + 2 + 3 should be (1 + 2) + 3
        let result = parse_with_timeout(
            "1 + 2 + 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Add {
            lhs: Box::new(AddLhs::Add {
                lhs: Box::new(AddLhs::IntLit(1)),
                rhs: Box::new(AddRhs::IntLit(2)),
            }),
            rhs: Box::new(AddRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_parentheses() {
        // Test: (1 + 2) * 3
        let result = parse_with_timeout(
            "(1 + 2) * 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mul {
            lhs: Box::new(MulLhs::Paren(Box::new(Expr::Add {
                lhs: Box::new(AddLhs::IntLit(1)),
                rhs: Box::new(AddRhs::IntLit(2)),
            }))),
            rhs: Box::new(MulRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }
}
