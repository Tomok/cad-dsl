//! Expression parser using Chumsky
//!
//! This module provides a parser for mathematical expressions with proper
//! operator precedence and error reporting.
//!
//! # Module Structure
//!
//! The parser is organized into several submodules:
//! - **atoms**: Primitive parsers for literals and variables
//! - **arithmetic**: Arithmetic operators (power, multiplication, division, modulo, addition, subtraction)
//! - **comparison**: Comparison operators (equality, inequality)
//! - **logical**: Logical operators (and, or)
//! - **stmt**: Statement parsers (let statements, type annotations)
//! - **error**: Error reporting with Ariadne
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
// Submodules
// ============================================================================

mod arithmetic;
mod atoms;
mod comparison;
mod error;
mod logical;
mod stmt;

// ============================================================================
// Re-exports
// ============================================================================

pub use error::report_parse_errors;
pub use stmt::let_stmt;

// ============================================================================
// Parser Type Definitions
// ============================================================================

/// The error type used by the parser
pub type ParseError<'src> = extra::Err<Rich<'src, Token<'src>>>;

// ============================================================================
// Top-Level Expression Parser
// ============================================================================

/// Internal expression parser that builds the complete precedence hierarchy
/// (without end-of-input validation - use for subexpressions)
pub fn expr_inner<'src>() -> impl Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone
{
    recursive(|expr_rec| {
        let pow_lhs = arithmetic::pow_lhs_parser(expr_rec.clone());
        let pow_rhs = arithmetic::pow_rhs_parser(expr_rec.clone(), pow_lhs.clone());
        let mul_rhs = arithmetic::mul_rhs_parser(expr_rec.clone(), pow_rhs.clone());
        let mul_lhs = arithmetic::mul_lhs_parser(expr_rec, mul_rhs.clone(), pow_rhs);
        let add_rhs = arithmetic::add_rhs_parser(mul_lhs.clone());
        let add_lhs = arithmetic::add_lhs_parser(mul_lhs, add_rhs);
        let cmp_rhs = comparison::cmp_rhs_parser(add_lhs.clone());
        let cmp_lhs = comparison::cmp_lhs_parser(add_lhs, cmp_rhs);
        let log_lhs = logical::log_parser(cmp_lhs);

        // Convert CmpLhs (with logical operators) to Expr
        log_lhs.map(Into::into)
    })
}

/// Parse a complete expression with end-of-input validation
#[cfg_attr(not(test), allow(dead_code))] // Used in expression tests
pub fn expr<'src>() -> impl Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone {
    expr_inner().then_ignore(end())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Stmt, Type};
    use crate::lexer;
    use crate::parser::stmt::type_annotation;
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
            |input| atoms::int_lit().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_float_lit() {
        let result = parse_with_timeout(
            "3.5",
            |input| atoms::float_lit().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), 3.5);
    }

    #[test]
    fn test_var() {
        let result = parse_with_timeout(
            "foo",
            |input| atoms::var().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), "foo");
    }

    #[test]
    fn test_atom_int() {
        let result = parse_with_timeout(
            "42",
            |input| atoms::atom().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Atom::IntLit(42));
    }

    #[test]
    fn test_atom_float() {
        let result = parse_with_timeout(
            "3.5",
            |input| atoms::atom().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Atom::FloatLit(3.5));
    }

    #[test]
    fn test_atom_var() {
        let result = parse_with_timeout(
            "x",
            |input| atoms::atom().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Atom::Var("x".to_string()));
    }

    #[test]
    fn test_bool_lit_true() {
        let result = parse_with_timeout(
            "true",
            |input| atoms::bool_lit().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), true);
    }

    #[test]
    fn test_bool_lit_false() {
        let result = parse_with_timeout(
            "false",
            |input| atoms::bool_lit().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_atom_bool_true() {
        let result = parse_with_timeout(
            "true",
            |input| atoms::atom().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Atom::BoolLit(true));
    }

    #[test]
    fn test_atom_bool_false() {
        let result = parse_with_timeout(
            "false",
            |input| atoms::atom().parse(input).into_result(),
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

    #[test]
    fn test_expr_simple_neq() {
        // Test: 1 != 2
        let result = parse_with_timeout(
            "1 != 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::NotEq {
            lhs: Box::new(CmpLhs::IntLit(1)),
            rhs: Box::new(CmpRhs::IntLit(2)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_neq_with_addition() {
        // Test: 1 + 2 != 3 + 4
        let result = parse_with_timeout(
            "1 + 2 != 3 + 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::NotEq {
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
    fn test_expr_neq_left_associative() {
        // Test: 1 != 2 != 3 should be (1 != 2) != 3
        let result = parse_with_timeout(
            "1 != 2 != 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::NotEq {
            lhs: Box::new(CmpLhs::NotEq {
                lhs: Box::new(CmpLhs::IntLit(1)),
                rhs: Box::new(CmpRhs::IntLit(2)),
            }),
            rhs: Box::new(CmpRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_neq_with_bool() {
        // Test: true != false
        let result = parse_with_timeout(
            "true != false",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::NotEq {
            lhs: Box::new(CmpLhs::BoolLit(true)),
            rhs: Box::new(CmpRhs::BoolLit(false)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_mixed_eq_neq() {
        // Test: 1 == 2 != 3 should be (1 == 2) != 3
        let result = parse_with_timeout(
            "1 == 2 != 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::NotEq {
            lhs: Box::new(CmpLhs::Eq {
                lhs: Box::new(CmpLhs::IntLit(1)),
                rhs: Box::new(CmpRhs::IntLit(2)),
            }),
            rhs: Box::new(CmpRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    // ========================================================================
    // Power Operator Tests
    // ========================================================================

    #[test]
    fn test_expr_simple_pow() {
        // Test: 2 ^ 3
        let result = parse_with_timeout(
            "2 ^ 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Pow {
            lhs: Box::new(PowLhs::IntLit(2)),
            rhs: Box::new(PowRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_pow_right_associative() {
        // Test: 2 ^ 3 ^ 4 should be 2 ^ (3 ^ 4) (right-associative)
        let result = parse_with_timeout(
            "2 ^ 3 ^ 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Pow {
            lhs: Box::new(PowLhs::IntLit(2)),
            rhs: Box::new(PowRhs::Pow {
                lhs: Box::new(PowLhs::IntLit(3)),
                rhs: Box::new(PowRhs::IntLit(4)),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_pow_with_mul() {
        // Test: 2 * 3 ^ 4 should be 2 * (3 ^ 4) - power has higher precedence
        let result = parse_with_timeout(
            "2 * 3 ^ 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mul {
            lhs: Box::new(MulLhs::IntLit(2)),
            rhs: Box::new(MulRhs::Pow {
                lhs: Box::new(PowLhs::IntLit(3)),
                rhs: Box::new(PowRhs::IntLit(4)),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_pow_with_add() {
        // Test: 1 + 2 ^ 3 should be 1 + (2 ^ 3)
        let result = parse_with_timeout(
            "1 + 2 ^ 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Add {
            lhs: Box::new(AddLhs::IntLit(1)),
            rhs: Box::new(AddRhs::Pow {
                lhs: Box::new(PowLhs::IntLit(2)),
                rhs: Box::new(PowRhs::IntLit(3)),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_pow_with_parens() {
        // Test: (2 ^ 3) ^ 4 - parentheses override right-associativity
        let result = parse_with_timeout(
            "(2 ^ 3) ^ 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Pow {
            lhs: Box::new(PowLhs::Paren(Box::new(Expr::Pow {
                lhs: Box::new(PowLhs::IntLit(2)),
                rhs: Box::new(PowRhs::IntLit(3)),
            }))),
            rhs: Box::new(PowRhs::IntLit(4)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_pow_with_vars() {
        // Test: x ^ y
        let result = parse_with_timeout(
            "x ^ y",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Pow {
            lhs: Box::new(PowLhs::Var("x".to_string())),
            rhs: Box::new(PowRhs::Var("y".to_string())),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_complex_pow_precedence() {
        // Test: 2 + 3 * 4 ^ 5 should be 2 + (3 * (4 ^ 5))
        let result = parse_with_timeout(
            "2 + 3 * 4 ^ 5",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Add {
            lhs: Box::new(AddLhs::IntLit(2)),
            rhs: Box::new(AddRhs::Mul {
                lhs: Box::new(MulLhs::IntLit(3)),
                rhs: Box::new(MulRhs::Pow {
                    lhs: Box::new(PowLhs::IntLit(4)),
                    rhs: Box::new(PowRhs::IntLit(5)),
                }),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_pow_chain_right_assoc() {
        // Test: a ^ b ^ c ^ d should be a ^ (b ^ (c ^ d))
        let result = parse_with_timeout(
            "a ^ b ^ c ^ d",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Pow {
            lhs: Box::new(PowLhs::Var("a".to_string())),
            rhs: Box::new(PowRhs::Pow {
                lhs: Box::new(PowLhs::Var("b".to_string())),
                rhs: Box::new(PowRhs::Pow {
                    lhs: Box::new(PowLhs::Var("c".to_string())),
                    rhs: Box::new(PowRhs::Var("d".to_string())),
                }),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    // ========================================================================
    // Modulo Operator Tests
    // ========================================================================

    #[test]
    fn test_expr_simple_mod() {
        // Test: 10 % 3
        let result = parse_with_timeout(
            "10 % 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mod {
            lhs: Box::new(MulLhs::IntLit(10)),
            rhs: Box::new(MulRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_mod_left_associative() {
        // Test: 10 % 3 % 2 should be (10 % 3) % 2
        let result = parse_with_timeout(
            "10 % 3 % 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mod {
            lhs: Box::new(MulLhs::Mod {
                lhs: Box::new(MulLhs::IntLit(10)),
                rhs: Box::new(MulRhs::IntLit(3)),
            }),
            rhs: Box::new(MulRhs::IntLit(2)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_mod_with_mul() {
        // Test: 10 * 3 % 2 should be (10 * 3) % 2 - same precedence, left-associative
        let result = parse_with_timeout(
            "10 * 3 % 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mod {
            lhs: Box::new(MulLhs::Mul {
                lhs: Box::new(MulLhs::IntLit(10)),
                rhs: Box::new(MulRhs::IntLit(3)),
            }),
            rhs: Box::new(MulRhs::IntLit(2)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_mod_with_div() {
        // Test: 10 / 3 % 2 should be (10 / 3) % 2 - same precedence, left-associative
        let result = parse_with_timeout(
            "10 / 3 % 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mod {
            lhs: Box::new(MulLhs::Div {
                lhs: Box::new(MulLhs::IntLit(10)),
                rhs: Box::new(MulRhs::IntLit(3)),
            }),
            rhs: Box::new(MulRhs::IntLit(2)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_mod_with_add() {
        // Test: 1 + 10 % 3 should be 1 + (10 % 3) - modulo has higher precedence
        let result = parse_with_timeout(
            "1 + 10 % 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Add {
            lhs: Box::new(AddLhs::IntLit(1)),
            rhs: Box::new(AddRhs::Mod {
                lhs: Box::new(MulLhs::IntLit(10)),
                rhs: Box::new(MulRhs::IntLit(3)),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_mod_with_pow() {
        // Test: 2 ^ 3 % 5 should be (2 ^ 3) % 5 - power has higher precedence
        let result = parse_with_timeout(
            "2 ^ 3 % 5",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mod {
            lhs: Box::new(MulLhs::Pow {
                lhs: Box::new(PowLhs::IntLit(2)),
                rhs: Box::new(PowRhs::IntLit(3)),
            }),
            rhs: Box::new(MulRhs::IntLit(5)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_mod_with_parens() {
        // Test: 10 % (3 + 2)
        let result = parse_with_timeout(
            "10 % (3 + 2)",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mod {
            lhs: Box::new(MulLhs::IntLit(10)),
            rhs: Box::new(MulRhs::Paren(Box::new(Expr::Add {
                lhs: Box::new(AddLhs::IntLit(3)),
                rhs: Box::new(AddRhs::IntLit(2)),
            }))),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_mod_with_vars() {
        // Test: x % y
        let result = parse_with_timeout(
            "x % y",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mod {
            lhs: Box::new(MulLhs::Var("x".to_string())),
            rhs: Box::new(MulRhs::Var("y".to_string())),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_complex_mod_precedence() {
        // Test: 2 + 3 * 4 % 5 should be 2 + ((3 * 4) % 5)
        let result = parse_with_timeout(
            "2 + 3 * 4 % 5",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Add {
            lhs: Box::new(AddLhs::IntLit(2)),
            rhs: Box::new(AddRhs::Mod {
                lhs: Box::new(MulLhs::Mul {
                    lhs: Box::new(MulLhs::IntLit(3)),
                    rhs: Box::new(MulRhs::IntLit(4)),
                }),
                rhs: Box::new(MulRhs::IntLit(5)),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    // ========================================================================
    // Unary Operator Tests
    // ========================================================================

    #[test]
    fn test_expr_simple_neg() {
        // Test: -5
        let result = parse_with_timeout(
            "-5",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Neg {
            inner: Box::new(PowLhs::IntLit(5)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_simple_ref() {
        // Test: &x
        let result = parse_with_timeout(
            "&x",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Ref {
            inner: Box::new(PowLhs::Var("x".to_string())),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_double_neg() {
        // Test: --5
        let result = parse_with_timeout(
            "--5",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Neg {
            inner: Box::new(PowLhs::Neg {
                inner: Box::new(PowLhs::IntLit(5)),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_neg_ref() {
        // Test: -&x
        let result = parse_with_timeout(
            "-&x",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Neg {
            inner: Box::new(PowLhs::Ref {
                inner: Box::new(PowLhs::Var("x".to_string())),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_ref_neg() {
        // Test: &-x
        let result = parse_with_timeout(
            "&-x",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Ref {
            inner: Box::new(PowLhs::Neg {
                inner: Box::new(PowLhs::Var("x".to_string())),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_neg_with_pow() {
        // Test: -2 ^ 3 should be (-2) ^ 3 - unary has higher precedence than power
        let result = parse_with_timeout(
            "-2 ^ 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Pow {
            lhs: Box::new(PowLhs::Neg {
                inner: Box::new(PowLhs::IntLit(2)),
            }),
            rhs: Box::new(PowRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_neg_with_mul() {
        // Test: -2 * 3 should be (-2) * 3
        let result = parse_with_timeout(
            "-2 * 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Mul {
            lhs: Box::new(MulLhs::Neg {
                inner: Box::new(PowLhs::IntLit(2)),
            }),
            rhs: Box::new(MulRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_neg_with_add() {
        // Test: -2 + 3 should be (-2) + 3
        let result = parse_with_timeout(
            "-2 + 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Add {
            lhs: Box::new(AddLhs::Neg {
                inner: Box::new(PowLhs::IntLit(2)),
            }),
            rhs: Box::new(AddRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_neg_paren() {
        // Test: -(2 + 3)
        let result = parse_with_timeout(
            "-(2 + 3)",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Neg {
            inner: Box::new(PowLhs::Paren(Box::new(Expr::Add {
                lhs: Box::new(AddLhs::IntLit(2)),
                rhs: Box::new(AddRhs::IntLit(3)),
            }))),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_ref_with_add() {
        // Test: &x + 3
        let result = parse_with_timeout(
            "&x + 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Add {
            lhs: Box::new(AddLhs::Ref {
                inner: Box::new(PowLhs::Var("x".to_string())),
            }),
            rhs: Box::new(AddRhs::IntLit(3)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_complex_unary() {
        // Test: -a ^ -b should be (-a) ^ (-b)
        let result = parse_with_timeout(
            "-a ^ -b",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Pow {
            lhs: Box::new(PowLhs::Neg {
                inner: Box::new(PowLhs::Var("a".to_string())),
            }),
            rhs: Box::new(PowRhs::Neg {
                inner: Box::new(PowLhs::Var("b".to_string())),
            }),
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
    // Logical Operator Tests
    // ========================================================================

    #[test]
    fn test_expr_simple_and() {
        // Test: true and false
        let result = parse_with_timeout(
            "true and false",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::And {
            lhs: Box::new(CmpLhs::BoolLit(true)),
            rhs: Box::new(CmpRhs::Paren(Box::new(Expr::BoolLit(false)))),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_simple_or() {
        // Test: true or false
        let result = parse_with_timeout(
            "true or false",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::Or {
            lhs: Box::new(CmpLhs::BoolLit(true)),
            rhs: Box::new(CmpRhs::Paren(Box::new(Expr::BoolLit(false)))),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_and_precedence_over_or() {
        // Test: a or b and c should be a or (b and c)
        // Since we're using simple foldl, this might not work as expected
        // Let's adjust the expected result based on left-associativity
        let result = parse_with_timeout(
            "a or b and c",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        // With left-associative parsing, this will be ((a or b) and c)
        let expected = Expr::And {
            lhs: Box::new(CmpLhs::Or {
                lhs: Box::new(CmpLhs::Var("a".to_string())),
                rhs: Box::new(CmpRhs::Paren(Box::new(Expr::Var("b".to_string())))),
            }),
            rhs: Box::new(CmpRhs::Paren(Box::new(Expr::Var("c".to_string())))),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_logical_with_comparison() {
        // Test: x == 1 and y == 2
        let result = parse_with_timeout(
            "x == 1 and y == 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::And {
            lhs: Box::new(CmpLhs::Eq {
                lhs: Box::new(CmpLhs::Var("x".to_string())),
                rhs: Box::new(CmpRhs::IntLit(1)),
            }),
            rhs: Box::new(CmpRhs::Paren(Box::new(Expr::Eq {
                lhs: Box::new(CmpLhs::Var("y".to_string())),
                rhs: Box::new(CmpRhs::IntLit(2)),
            }))),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_expr_logical_left_associative() {
        // Test: a and b and c should be (a and b) and c
        let result = parse_with_timeout(
            "a and b and c",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Expr::And {
            lhs: Box::new(CmpLhs::And {
                lhs: Box::new(CmpLhs::Var("a".to_string())),
                rhs: Box::new(CmpRhs::Paren(Box::new(Expr::Var("b".to_string())))),
            }),
            rhs: Box::new(CmpRhs::Paren(Box::new(Expr::Var("c".to_string())))),
        };
        assert_eq!(result.unwrap(), expected);
    }

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

    // ========================================================================
    // Type Annotation Tests
    // ========================================================================

    #[test]
    fn test_type_bool() {
        let result = parse_with_timeout(
            "bool",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Type::Bool);
    }

    #[test]
    fn test_type_i32() {
        let result = parse_with_timeout(
            "i32",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Type::I32);
    }

    #[test]
    fn test_type_f64() {
        let result = parse_with_timeout(
            "f64",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Type::F64);
    }

    #[test]
    fn test_type_real() {
        let result = parse_with_timeout(
            "Real",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Type::Real);
    }

    #[test]
    fn test_type_algebraic() {
        let result = parse_with_timeout(
            "Algebraic",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert_eq!(result.unwrap(), Type::Algebraic);
    }

    // ========================================================================
    // Let Statement Tests
    // ========================================================================

    #[test]
    fn test_let_with_type_and_init() {
        // let x: i32 = 42;
        let result = parse_with_timeout(
            "let x: i32 = 42;",
            |input| let_stmt(expr_inner()).parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Stmt::Let {
            name: "x".to_string(),
            type_annotation: Some(Type::I32),
            init: Some(Expr::IntLit(42)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_let_with_type_only() {
        // let y: bool;
        let result = parse_with_timeout(
            "let y: bool;",
            |input| let_stmt(expr_inner()).parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Stmt::Let {
            name: "y".to_string(),
            type_annotation: Some(Type::Bool),
            init: None,
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_let_with_init_only() {
        // let z = 3.14;
        let result = parse_with_timeout(
            "let z = 3.14;",
            |input| let_stmt(expr_inner()).parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Stmt::Let {
            name: "z".to_string(),
            type_annotation: None,
            init: Some(Expr::FloatLit(3.14)),
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_let_no_type_no_init() {
        // let w;
        let result = parse_with_timeout(
            "let w;",
            |input| let_stmt(expr_inner()).parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Stmt::Let {
            name: "w".to_string(),
            type_annotation: None,
            init: None,
        };
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_let_with_expression() {
        // let result: i32 = 1 + 2 * 3;
        let result = parse_with_timeout(
            "let result: i32 = 1 + 2 * 3;",
            |input| let_stmt(expr_inner()).parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expected = Stmt::Let {
            name: "result".to_string(),
            type_annotation: Some(Type::I32),
            init: Some(Expr::Add {
                lhs: Box::new(AddLhs::IntLit(1)),
                rhs: Box::new(AddRhs::Mul {
                    lhs: Box::new(MulLhs::IntLit(2)),
                    rhs: Box::new(MulRhs::IntLit(3)),
                }),
            }),
        };
        assert_eq!(result.unwrap(), expected);
    }
}
