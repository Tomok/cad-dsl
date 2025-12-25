use crate::ast::*;
use chumsky::prelude::*;

// ============================================================================
// Parser Type Definitions
// ============================================================================

/// The error type used by the parser
pub type ParseError<'a> = extra::Err<Rich<'a, char>>;

// ============================================================================
// Atomic Parsers (No recursion)
// ============================================================================

/// Parse an integer literal
fn int_lit<'a>() -> impl Parser<'a, &'a str, i32, ParseError<'a>> + Clone {
    text::int(10)
        .try_map(|s: &str, span| {
            s.parse::<i32>()
                .map_err(|e| Rich::custom(span, format!("Invalid integer: {}", e)))
        })
        .labelled("integer")
}

/// Parse a float literal
fn float_lit<'a>() -> impl Parser<'a, &'a str, f64, ParseError<'a>> + Clone {
    text::int(10)
        .then_ignore(just('.'))
        .then(text::int(10))
        .to_slice()
        .try_map(|s: &str, span| {
            s.parse::<f64>()
                .map_err(|e| Rich::custom(span, format!("Invalid float: {}", e)))
        })
        .labelled("float")
}

/// Parse a variable name (identifier)
fn var<'a>() -> impl Parser<'a, &'a str, String, ParseError<'a>> + Clone {
    text::ident()
        .map(|s: &str| s.to_string())
        .labelled("variable")
}

/// Parse an atomic expression (Atom enum)
fn atom<'a>() -> impl Parser<'a, &'a str, Atom, ParseError<'a>> + Clone {
    choice((
        // Try float first (it's more specific with the decimal point requirement)
        float_lit().map(Atom::FloatLit),
        // Then integer
        int_lit().map(Atom::IntLit),
        // Finally variable
        var().map(Atom::Var),
    ))
    .padded()
    .labelled("atom")
}

// ============================================================================
// Recursive Expression Parsers
// ============================================================================

/// Internal expression parser without end-of-input check
fn expr_inner<'a>() -> impl Parser<'a, &'a str, Expr, ParseError<'a>> + Clone {
    recursive(|expr_rec| {
        // MulRhs: Paren, Var, IntLit, FloatLit
        let mul_rhs = choice((
            atom().map(|a| match a {
                Atom::Var(s) => MulRhs::Var(s),
                Atom::IntLit(i) => MulRhs::IntLit(i),
                Atom::FloatLit(f) => MulRhs::FloatLit(f),
            }),
            expr_rec
                .clone()
                .delimited_by(just('('), just(')'))
                .map(|e| MulRhs::Paren(Box::new(e))),
        ))
        .padded();

        // MulLhs: Paren, Mul, Div, Var, IntLit, FloatLit
        let mul_atom = choice((
            atom().map(|a| match a {
                Atom::Var(s) => MulLhs::Var(s),
                Atom::IntLit(i) => MulLhs::IntLit(i),
                Atom::FloatLit(f) => MulLhs::FloatLit(f),
            }),
            expr_rec
                .clone()
                .delimited_by(just('('), just(')'))
                .map(|e| MulLhs::Paren(Box::new(e))),
        ))
        .padded();

        // Left-associative multiplication and division
        let mul_lhs = mul_atom.clone().foldl(
            choice((just('*').padded(), just('/').padded()))
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
        let add_rhs = choice((
            mul_lhs.clone().map(|m| match m {
                MulLhs::Paren(e) => AddRhs::Paren(e),
                MulLhs::Mul { lhs, rhs } => AddRhs::Mul { lhs, rhs },
                MulLhs::Div { lhs, rhs } => AddRhs::Div { lhs, rhs },
                MulLhs::Var(s) => AddRhs::Var(s),
                MulLhs::IntLit(i) => AddRhs::IntLit(i),
                MulLhs::FloatLit(f) => AddRhs::FloatLit(f),
            }),
        ))
        .padded();

        // AddLhs: Add, Sub, Paren, Mul, Div, Var, IntLit, FloatLit
        let add_lhs = {
            let add_atom = mul_lhs.clone().map(|m| match m {
                MulLhs::Paren(e) => AddLhs::Paren(e),
                MulLhs::Mul { lhs, rhs } => AddLhs::Mul { lhs, rhs },
                MulLhs::Div { lhs, rhs } => AddLhs::Div { lhs, rhs },
                MulLhs::Var(s) => AddLhs::Var(s),
                MulLhs::IntLit(i) => AddLhs::IntLit(i),
                MulLhs::FloatLit(f) => AddLhs::FloatLit(f),
            });

            // Left-associative addition and subtraction
            add_atom.clone().foldl(
                choice((just('+').padded(), just('-').padded()))
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
        add_lhs.map(|a| match a {
            AddLhs::Add { lhs, rhs } => Expr::Add { lhs, rhs },
            AddLhs::Sub { lhs, rhs } => Expr::Sub { lhs, rhs },
            AddLhs::Paren(e) => Expr::Paren(e),
            AddLhs::Mul { lhs, rhs } => Expr::Mul { lhs, rhs },
            AddLhs::Div { lhs, rhs } => Expr::Div { lhs, rhs },
            AddLhs::Var(s) => Expr::Var(s),
            AddLhs::IntLit(i) => Expr::IntLit(i),
            AddLhs::FloatLit(f) => Expr::FloatLit(f),
        })
    })
}

/// Parse a complete expression with all precedence levels
///
/// This is an example showing how to use `recursive()` to handle
/// circular dependencies in the grammar.
pub fn expr<'a>() -> impl Parser<'a, &'a str, Expr, ParseError<'a>> + Clone {
    expr_inner().padded().then_ignore(end())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    /// Helper function to run a parser test with a timeout
    /// This prevents tests from hanging indefinitely if there's infinite recursion
    fn parse_with_timeout<T: Send + 'static>(
        input: &'static str,
        parser: impl Fn(&str) -> Result<T, Vec<Rich<char>>> + Send + 'static,
        timeout: Duration,
    ) -> Result<T, String> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let result = parser(input);
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
            "3.14",
            |input| float_lit().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), 3.14);
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
            "3.14",
            |input| atom().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Atom::FloatLit(3.14));
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
