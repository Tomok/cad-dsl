//! Expression parser using Chumsky
//!
//! This module provides a parser for mathematical expressions with proper
//! operator precedence and error reporting.
//!
//! # Error Reporting
//!
//! The parser uses Chumsky's `Rich<Token>` error type, which preserves:
//! - **Spans**: Exact token positions for error highlighting
//! - **Expected tokens**: What the parser was expecting
//! - **Found tokens**: What was actually encountered
//!
//! Use `report_parse_errors()` to convert parser errors into beautiful
//! Ariadne reports with colored output and helpful suggestions.
//!
//! # Example
//!
//! ```ignore
//! use crate::parser::{expr, report_parse_errors};
//! use crate::lexer::tokenize;
//!
//! let source = "1 + 2 * 3";
//! let tokens = tokenize(source)?;
//!
//! match expr().parse(&tokens).into_result() {
//!     Ok(ast) => println!("Parsed: {:?}", ast),
//!     Err(errors) => report_parse_errors("input.cad", source, errors),
//! }
//! ```

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

/// Parse a boolean literal token
fn bool_lit<'src>() -> impl Parser<'src, &'src [Token<'src>], bool, ParseError<'src>> + Clone {
    select! {
        Token::True(_) => true,
        Token::False(_) => false,
    }
    .labelled("boolean")
}

/// Parse an atomic expression (Atom enum)
fn atom<'src>() -> impl Parser<'src, &'src [Token<'src>], Atom, ParseError<'src>> + Clone {
    choice((
        // Try float first (it's more specific)
        float_lit().map(Atom::FloatLit),
        // Then integer
        int_lit().map(Atom::IntLit),
        // Then boolean
        bool_lit().map(Atom::BoolLit),
        // Finally variable
        var().map(Atom::Var),
    ))
    .labelled("atom")
}

// ============================================================================
// Recursive Expression Parsers
// ============================================================================

/// Parser for multiplication right-hand side (MulRhs)
fn mul_rhs_parser<'src, E>(
    expr_rec: E,
) -> impl Parser<'src, &'src [Token<'src>], MulRhs, ParseError<'src>> + Clone
where
    E: Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone,
{
    let lparen = select! { Token::LeftParen(_) => () };
    let rparen = select! { Token::RightParen(_) => () };

    choice((
        atom().map(Into::into),
        expr_rec
            .delimited_by(lparen, rparen)
            .map(|e| MulRhs::Paren(Box::new(e))),
    ))
}

/// Parser for multiplication left-hand side (MulLhs) with operators
fn mul_lhs_parser<'src, E, R>(
    expr_rec: E,
    mul_rhs: R,
) -> impl Parser<'src, &'src [Token<'src>], MulLhs, ParseError<'src>> + Clone
where
    E: Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone,
    R: Parser<'src, &'src [Token<'src>], MulRhs, ParseError<'src>> + Clone,
{
    let lparen = select! { Token::LeftParen(_) => () };
    let rparen = select! { Token::RightParen(_) => () };
    let mul_op = select! { Token::Multiply(_) => '*' };
    let div_op = select! { Token::Divide(_) => '/' };

    let mul_atom = choice((
        atom().map(Into::into),
        expr_rec
            .delimited_by(lparen, rparen)
            .map(|e| MulLhs::Paren(Box::new(e))),
    ));

    // Left-associative multiplication and division
    mul_atom.foldl(
        choice((mul_op, div_op)).then(mul_rhs).repeated(),
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
    )
}

/// Parser for addition right-hand side (AddRhs)
fn add_rhs_parser<'src, M>(
    mul_lhs: M,
) -> impl Parser<'src, &'src [Token<'src>], AddRhs, ParseError<'src>> + Clone
where
    M: Parser<'src, &'src [Token<'src>], MulLhs, ParseError<'src>> + Clone,
{
    mul_lhs.map(Into::into)
}

/// Parser for addition left-hand side (AddLhs) with operators
fn add_lhs_parser<'src, M, R>(
    mul_lhs: M,
    add_rhs: R,
) -> impl Parser<'src, &'src [Token<'src>], AddLhs, ParseError<'src>> + Clone
where
    M: Parser<'src, &'src [Token<'src>], MulLhs, ParseError<'src>> + Clone,
    R: Parser<'src, &'src [Token<'src>], AddRhs, ParseError<'src>> + Clone,
{
    let add_op = select! { Token::Plus(_) => '+' };
    let sub_op = select! { Token::Minus(_) => '-' };

    let add_atom = mul_lhs.map(Into::into);

    // Left-associative addition and subtraction
    add_atom.foldl(
        choice((add_op, sub_op)).then(add_rhs).repeated(),
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
}

/// Parser for comparison right-hand side (CmpRhs)
fn cmp_rhs_parser<'src, A>(
    add_lhs: A,
) -> impl Parser<'src, &'src [Token<'src>], CmpRhs, ParseError<'src>> + Clone
where
    A: Parser<'src, &'src [Token<'src>], AddLhs, ParseError<'src>> + Clone,
{
    add_lhs.map(Into::into)
}

/// Parser for comparison left-hand side (CmpLhs) with operators
fn cmp_lhs_parser<'src, A, R>(
    add_lhs: A,
    cmp_rhs: R,
) -> impl Parser<'src, &'src [Token<'src>], CmpLhs, ParseError<'src>> + Clone
where
    A: Parser<'src, &'src [Token<'src>], AddLhs, ParseError<'src>> + Clone,
    R: Parser<'src, &'src [Token<'src>], CmpRhs, ParseError<'src>> + Clone,
{
    let eq_op = select! { Token::EqualsEquals(_) => "==" };

    let cmp_atom = add_lhs.map(Into::into);

    // Left-associative equality
    cmp_atom.foldl(eq_op.then(cmp_rhs).repeated(), |lhs, (_op, rhs)| {
        CmpLhs::Eq {
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    })
}

/// Internal expression parser that builds the complete precedence hierarchy
fn expr_inner<'src>() -> impl Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone {
    recursive(|expr_rec| {
        let mul_rhs = mul_rhs_parser(expr_rec.clone());
        let mul_lhs = mul_lhs_parser(expr_rec, mul_rhs.clone());
        let add_rhs = add_rhs_parser(mul_lhs.clone());
        let add_lhs = add_lhs_parser(mul_lhs, add_rhs);
        let cmp_rhs = cmp_rhs_parser(add_lhs.clone());
        let cmp_lhs = cmp_lhs_parser(add_lhs, cmp_rhs);

        // Convert CmpLhs to Expr
        cmp_lhs.map(Into::into)
    })
}

pub fn expr<'src>() -> impl Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone {
    expr_inner().then_ignore(end())
}

// ============================================================================
// Error Reporting with Ariadne
// ============================================================================

use ariadne::{Color, Label, Report, ReportKind, Source};

/// Convert parser errors to beautiful Ariadne reports
///
/// This function converts Chumsky's Rich errors into Ariadne reports with
/// proper spans and helpful error messages. All necessary information
/// (spans, labels, expected tokens) is preserved from the parser.
pub fn report_parse_errors<'src>(
    filename: &str,
    source: &'src str,
    errors: Vec<Rich<'src, Token<'src>>>,
) {
    for error in errors {
        let span = error.span();

        // Calculate byte offset from token position
        // Note: This requires the tokens to have proper span information
        let offset = span.start;

        let mut report =
            Report::build(ReportKind::Error, filename, offset).with_message("Parse error");

        // Add the main error label
        report = report.with_label(
            Label::new((filename, offset..offset + 1))
                .with_message(format!("Unexpected token: {:?}", error.found()))
                .with_color(Color::Red),
        );

        // Add expected tokens if available
        if !error.expected().collect::<Vec<_>>().is_empty() {
            let expected = error
                .expected()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join(", ");
            report = report.with_note(format!("Expected one of: {}", expected));
        }

        // Add help message based on error context
        if error.found().is_none() {
            report = report.with_help("Unexpected end of input");
        }

        report
            .finish()
            .eprint((filename, Source::from(source)))
            .unwrap();
    }
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
        parse_fn: impl FnOnce(
            &'static [Token<'static>],
        ) -> Result<T, Vec<Rich<'static, Token<'static>>>>
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
    fn test_bool_lit_true() {
        let result = parse_with_timeout(
            "true",
            |input| bool_lit().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_bool_lit_false() {
        let result = parse_with_timeout(
            "false",
            |input| bool_lit().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_atom_bool_true() {
        let result = parse_with_timeout(
            "true",
            |input| atom().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Atom::BoolLit(true));
    }

    #[test]
    fn test_atom_bool_false() {
        let result = parse_with_timeout(
            "false",
            |input| atom().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Atom::BoolLit(false));
    }

    #[test]
    fn test_expr_bool_true() {
        let result = parse_with_timeout(
            "true",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert_eq!(result.unwrap(), Expr::BoolLit(true));
    }

    #[test]
    fn test_expr_bool_false() {
        let result = parse_with_timeout(
            "false",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert_eq!(result.unwrap(), Expr::BoolLit(false));
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

    #[test]
    fn test_expr_simple_eq() {
        // Test: 1 == 2
        let result = parse_with_timeout(
            "1 == 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Eq {
            lhs: Box::new(CmpLhs::IntLit(1)),
            rhs: Box::new(CmpRhs::IntLit(2)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_eq_with_addition() {
        // Test: 1 + 2 == 3 + 4
        let result = parse_with_timeout(
            "1 + 2 == 3 + 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Eq {
            lhs: Box::new(CmpLhs::Add {
                lhs: Box::new(AddLhs::IntLit(1)),
                rhs: Box::new(AddRhs::IntLit(2)),
            }),
            rhs: Box::new(CmpRhs::Add {
                lhs: Box::new(AddLhs::IntLit(3)),
                rhs: Box::new(AddRhs::IntLit(4)),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_eq_left_associative() {
        // Test: 1 == 2 == 3 should be (1 == 2) == 3
        let result = parse_with_timeout(
            "1 == 2 == 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Eq {
            lhs: Box::new(CmpLhs::Eq {
                lhs: Box::new(CmpLhs::IntLit(1)),
                rhs: Box::new(CmpRhs::IntLit(2)),
            }),
            rhs: Box::new(CmpRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_eq_with_bool() {
        // Test: true == false
        let result = parse_with_timeout(
            "true == false",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Eq {
            lhs: Box::new(CmpLhs::BoolLit(true)),
            rhs: Box::new(CmpRhs::BoolLit(false)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    // ========================================================================
    // Error Case Tests
    // ========================================================================

    #[test]
    fn test_error_missing_right_operand() {
        // "1 +" should fail - missing right operand
        let result = parse_with_timeout(
            "1 +",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(result.is_err(), "Should fail with missing right operand");
    }

    #[test]
    fn test_error_double_operator() {
        // "1 + + 2" should fail - double operator
        let result = parse_with_timeout(
            "1 + + 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(result.is_err(), "Should fail with double operator");
    }

    #[test]
    fn test_error_missing_closing_paren() {
        // "(1 + 2" should fail - missing closing parenthesis
        let result = parse_with_timeout(
            "(1 + 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(result.is_err(), "Should fail with missing closing paren");
    }

    #[test]
    fn test_error_missing_operator() {
        // "1 2" should fail - missing operator
        let result = parse_with_timeout(
            "1 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(result.is_err(), "Should fail with missing operator");
    }

    #[test]
    fn test_error_missing_left_operand() {
        // "* 2" should fail - missing left operand
        let result = parse_with_timeout(
            "* 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(result.is_err(), "Should fail with missing left operand");
    }

    #[test]
    fn test_error_empty_input() {
        // "" should fail - empty input
        let result = parse_with_timeout(
            "",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(result.is_err(), "Should fail with empty input");
    }

    #[test]
    fn test_error_empty_parentheses() {
        // "()" should fail - empty parentheses
        let result = parse_with_timeout(
            "()",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(result.is_err(), "Should fail with empty parentheses");
    }

    #[test]
    fn test_error_unclosed_nested_parens() {
        // "((1 + 2)" should fail - unclosed nested parentheses
        let result = parse_with_timeout(
            "((1 + 2)",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(result.is_err(), "Should fail with unclosed nested parens");
    }

    // ========================================================================
    // Error Reporting Example (demonstrates Ariadne integration)
    // ========================================================================

    #[test]
    #[ignore] // Ignore by default as it prints to stderr
    fn test_ariadne_error_reporting_example() {
        // This test demonstrates how to use report_parse_errors
        // Run with: cargo test test_ariadne_error_reporting_example -- --ignored --nocapture

        use super::report_parse_errors;

        let source = "1 + + 2";
        let tokens = lexer::tokenize(source).unwrap();

        // Parse and capture errors
        match expr().parse(&tokens).into_result() {
            Ok(_) => panic!("Expected parse error"),
            Err(errors) => {
                println!("\n=== Ariadne Error Report Example ===\n");
                report_parse_errors("example.cad", source, errors);
                println!("\n=== End of Example ===\n");
            }
        }
    }
}
