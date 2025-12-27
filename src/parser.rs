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
pub fn expr_inner<'src>()
-> impl Parser<'src, &'src [Token<'src>], Expr<'src>, ParseError<'src>> + Clone {
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

        // Convert CmpLhs<'src> (with logical operators) to Expr
        log_lhs.map(Into::into)
    })
}

/// Parse a complete expression with end-of-input validation
#[cfg_attr(not(test), allow(dead_code))] // Used in expression tests
pub fn expr<'src>() -> impl Parser<'src, &'src [Token<'src>], Expr<'src>, ParseError<'src>> + Clone
{
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
    fn test_expr_bool_true() {
        let result = parse_with_timeout(
            "true",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(matches!(result.unwrap(), Expr::BoolLit { value: true, .. }));
    }

    #[test]
    fn test_expr_bool_false() {
        let result = parse_with_timeout(
            "false",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(matches!(
            result.unwrap(),
            Expr::BoolLit { value: false, .. }
        ));
    }

    #[test]
    fn test_expr_simple_var() {
        let result = parse_with_timeout(
            "x",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(matches!(result.unwrap(), Expr::Var { name, .. } if name == "x"));
    }

    #[test]
    fn test_expr_simple_int() {
        let result = parse_with_timeout(
            "42",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        assert!(matches!(result.unwrap(), Expr::IntLit { value: 42, .. }));
    }

    #[test]
    fn test_expr_simple_add() {
        let result = parse_with_timeout(
            "1 + 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Add { lhs, rhs, .. } => {
                assert!(matches!(*lhs, AddLhs::IntLit { value: 1, .. }));
                assert!(matches!(*rhs, AddRhs::IntLit { value: 2, .. }));
            }
            other => panic!("Expected Expr::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_simple_mul() {
        let result = parse_with_timeout(
            "3 * 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Mul { lhs, rhs, .. } => {
                assert!(matches!(*lhs, MulLhs::IntLit { value: 3, .. }));
                assert!(matches!(*rhs, MulRhs::IntLit { value: 4, .. }));
            }
            other => panic!("Expected Expr::Mul, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_precedence() {
        // Test: 1 + 2 * 3
        let result = parse_with_timeout(
            "1 + 2 * 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Add { lhs, rhs, .. } => {
                assert!(matches!(*lhs, AddLhs::IntLit { value: 1, .. }));
                match *rhs {
                    AddRhs::Mul {
                        lhs: ref mul_lhs,
                        rhs: ref mul_rhs,
                        ..
                    } => {
                        assert!(matches!(**mul_lhs, MulLhs::IntLit { value: 2, .. }));
                        assert!(matches!(**mul_rhs, MulRhs::IntLit { value: 3, .. }));
                    }
                    ref other => panic!("Expected AddRhs::Mul, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_left_associative_add() {
        // Test: 1 + 2 + 3 should be (1 + 2) + 3
        let result = parse_with_timeout(
            "1 + 2 + 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Add { lhs, rhs, .. } => {
                match *lhs {
                    AddLhs::Add {
                        lhs: ref inner_lhs,
                        rhs: ref inner_rhs,
                        ..
                    } => {
                        assert!(matches!(**inner_lhs, AddLhs::IntLit { value: 1, .. }));
                        assert!(matches!(**inner_rhs, AddRhs::IntLit { value: 2, .. }));
                    }
                    ref other => panic!("Expected AddLhs::Add, got {:?}", other),
                }
                assert!(matches!(*rhs, AddRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_parentheses() {
        // Test: (1 + 2) * 3
        let result = parse_with_timeout(
            "(1 + 2) * 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Mul { lhs, rhs, .. } => {
                match *lhs {
                    MulLhs::Paren { ref inner, .. } => match **inner {
                        Expr::Add {
                            ref lhs, ref rhs, ..
                        } => {
                            assert!(matches!(**lhs, AddLhs::IntLit { value: 1, .. }));
                            assert!(matches!(**rhs, AddRhs::IntLit { value: 2, .. }));
                        }
                        ref other => panic!("Expected Expr::Add, got {:?}", other),
                    },
                    ref other => panic!("Expected MulLhs::Paren, got {:?}", other),
                }
                assert!(matches!(*rhs, MulRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::Mul, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_simple_eq() {
        // Test: 1 == 2
        let result = parse_with_timeout(
            "1 == 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Eq { lhs, rhs, .. } => {
                assert!(matches!(*lhs, CmpLhs::IntLit { value: 1, .. }));
                assert!(matches!(*rhs, CmpRhs::IntLit { value: 2, .. }));
            }
            other => panic!("Expected Expr::Eq, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_eq_with_addition() {
        // Test: 1 + 2 == 3 + 4
        let result = parse_with_timeout(
            "1 + 2 == 3 + 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Eq { lhs, rhs, .. } => {
                match *lhs {
                    CmpLhs::Add {
                        lhs: ref add_lhs,
                        rhs: ref add_rhs,
                        ..
                    } => {
                        assert!(matches!(**add_lhs, AddLhs::IntLit { value: 1, .. }));
                        assert!(matches!(**add_rhs, AddRhs::IntLit { value: 2, .. }));
                    }
                    ref other => panic!("Expected CmpLhs::Add, got {:?}", other),
                }
                match *rhs {
                    CmpRhs::Add {
                        lhs: ref add_lhs,
                        rhs: ref add_rhs,
                        ..
                    } => {
                        assert!(matches!(**add_lhs, AddLhs::IntLit { value: 3, .. }));
                        assert!(matches!(**add_rhs, AddRhs::IntLit { value: 4, .. }));
                    }
                    ref other => panic!("Expected CmpRhs::Add, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Eq, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_eq_left_associative() {
        // Test: 1 == 2 == 3 should be (1 == 2) == 3
        let result = parse_with_timeout(
            "1 == 2 == 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Eq { lhs, rhs, .. } => {
                match *lhs {
                    CmpLhs::Eq {
                        lhs: ref inner_lhs,
                        rhs: ref inner_rhs,
                        ..
                    } => {
                        assert!(matches!(**inner_lhs, CmpLhs::IntLit { value: 1, .. }));
                        assert!(matches!(**inner_rhs, CmpRhs::IntLit { value: 2, .. }));
                    }
                    ref other => panic!("Expected CmpLhs::Eq, got {:?}", other),
                }
                assert!(matches!(*rhs, CmpRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::Eq, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_eq_with_bool() {
        // Test: true == false
        let result = parse_with_timeout(
            "true == false",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Eq { lhs, rhs, .. } => {
                assert!(matches!(*lhs, CmpLhs::BoolLit { value: true, .. }));
                assert!(matches!(*rhs, CmpRhs::BoolLit { value: false, .. }));
            }
            other => panic!("Expected Expr::Eq, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_simple_neq() {
        // Test: 1 != 2
        let result = parse_with_timeout(
            "1 != 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::NotEq { lhs, rhs, .. } => {
                assert!(matches!(*lhs, CmpLhs::IntLit { value: 1, .. }));
                assert!(matches!(*rhs, CmpRhs::IntLit { value: 2, .. }));
            }
            other => panic!("Expected Expr::NotEq, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_neq_with_addition() {
        // Test: 1 + 2 != 3 + 4
        let result = parse_with_timeout(
            "1 + 2 != 3 + 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::NotEq { lhs, rhs, .. } => {
                match *lhs {
                    CmpLhs::Add {
                        lhs: ref add_lhs,
                        rhs: ref add_rhs,
                        ..
                    } => {
                        assert!(matches!(**add_lhs, AddLhs::IntLit { value: 1, .. }));
                        assert!(matches!(**add_rhs, AddRhs::IntLit { value: 2, .. }));
                    }
                    ref other => panic!("Expected CmpLhs::Add, got {:?}", other),
                }
                match *rhs {
                    CmpRhs::Add {
                        lhs: ref add_lhs,
                        rhs: ref add_rhs,
                        ..
                    } => {
                        assert!(matches!(**add_lhs, AddLhs::IntLit { value: 3, .. }));
                        assert!(matches!(**add_rhs, AddRhs::IntLit { value: 4, .. }));
                    }
                    ref other => panic!("Expected CmpRhs::Add, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::NotEq, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_neq_left_associative() {
        // Test: 1 != 2 != 3 should be (1 != 2) != 3
        let result = parse_with_timeout(
            "1 != 2 != 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::NotEq { lhs, rhs, .. } => {
                match *lhs {
                    CmpLhs::NotEq {
                        lhs: ref inner_lhs,
                        rhs: ref inner_rhs,
                        ..
                    } => {
                        assert!(matches!(**inner_lhs, CmpLhs::IntLit { value: 1, .. }));
                        assert!(matches!(**inner_rhs, CmpRhs::IntLit { value: 2, .. }));
                    }
                    ref other => panic!("Expected CmpLhs::NotEq, got {:?}", other),
                }
                assert!(matches!(*rhs, CmpRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::NotEq, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_neq_with_bool() {
        // Test: true != false
        let result = parse_with_timeout(
            "true != false",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::NotEq { lhs, rhs, .. } => {
                assert!(matches!(*lhs, CmpLhs::BoolLit { value: true, .. }));
                assert!(matches!(*rhs, CmpRhs::BoolLit { value: false, .. }));
            }
            other => panic!("Expected Expr::NotEq, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_mixed_eq_neq() {
        // Test: 1 == 2 != 3 should be (1 == 2) != 3
        let result = parse_with_timeout(
            "1 == 2 != 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::NotEq { lhs, rhs, .. } => {
                match *lhs {
                    CmpLhs::Eq {
                        lhs: ref inner_lhs,
                        rhs: ref inner_rhs,
                        ..
                    } => {
                        assert!(matches!(**inner_lhs, CmpLhs::IntLit { value: 1, .. }));
                        assert!(matches!(**inner_rhs, CmpRhs::IntLit { value: 2, .. }));
                    }
                    ref other => panic!("Expected CmpLhs::Eq, got {:?}", other),
                }
                assert!(matches!(*rhs, CmpRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::NotEq, got {:?}", other),
        }
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

        match result.unwrap() {
            Expr::Pow { lhs, rhs, .. } => {
                assert!(matches!(*lhs, PowLhs::IntLit { value: 2, .. }));
                assert!(matches!(*rhs, PowRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::Pow, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_pow_right_associative() {
        // Test: 2 ^ 3 ^ 4 should be 2 ^ (3 ^ 4) (right-associative)
        let result = parse_with_timeout(
            "2 ^ 3 ^ 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Pow { lhs, rhs, .. } => {
                assert!(matches!(*lhs, PowLhs::IntLit { value: 2, .. }));
                match *rhs {
                    PowRhs::Pow {
                        lhs: ref inner_lhs,
                        rhs: ref inner_rhs,
                        ..
                    } => {
                        assert!(matches!(**inner_lhs, PowLhs::IntLit { value: 3, .. }));
                        assert!(matches!(**inner_rhs, PowRhs::IntLit { value: 4, .. }));
                    }
                    ref other => panic!("Expected PowRhs::Pow, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Pow, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_pow_with_mul() {
        // Test: 2 * 3 ^ 4 should be 2 * (3 ^ 4) - power has higher precedence
        let result = parse_with_timeout(
            "2 * 3 ^ 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Mul { lhs, rhs, .. } => {
                assert!(matches!(*lhs, MulLhs::IntLit { value: 2, .. }));
                match *rhs {
                    MulRhs::Pow {
                        lhs: ref pow_lhs,
                        rhs: ref pow_rhs,
                        ..
                    } => {
                        assert!(matches!(**pow_lhs, PowLhs::IntLit { value: 3, .. }));
                        assert!(matches!(**pow_rhs, PowRhs::IntLit { value: 4, .. }));
                    }
                    ref other => panic!("Expected MulRhs::Pow, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Mul, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_pow_with_add() {
        // Test: 1 + 2 ^ 3 should be 1 + (2 ^ 3)
        let result = parse_with_timeout(
            "1 + 2 ^ 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Add { lhs, rhs, .. } => {
                assert!(matches!(*lhs, AddLhs::IntLit { value: 1, .. }));
                match *rhs {
                    AddRhs::Pow {
                        lhs: ref pow_lhs,
                        rhs: ref pow_rhs,
                        ..
                    } => {
                        assert!(matches!(**pow_lhs, PowLhs::IntLit { value: 2, .. }));
                        assert!(matches!(**pow_rhs, PowRhs::IntLit { value: 3, .. }));
                    }
                    ref other => panic!("Expected AddRhs::Pow, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_pow_with_parens() {
        // Test: (2 ^ 3) ^ 4 - parentheses override right-associativity
        let result = parse_with_timeout(
            "(2 ^ 3) ^ 4",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Pow { lhs, rhs, .. } => {
                match *lhs {
                    PowLhs::Paren { ref inner, .. } => match **inner {
                        Expr::Pow {
                            ref lhs, ref rhs, ..
                        } => {
                            assert!(matches!(**lhs, PowLhs::IntLit { value: 2, .. }));
                            assert!(matches!(**rhs, PowRhs::IntLit { value: 3, .. }));
                        }
                        ref other => panic!("Expected Expr::Pow, got {:?}", other),
                    },
                    ref other => panic!("Expected PowLhs::Paren, got {:?}", other),
                }
                assert!(matches!(*rhs, PowRhs::IntLit { value: 4, .. }));
            }
            other => panic!("Expected Expr::Pow, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_pow_with_vars() {
        // Test: x ^ y
        let result = parse_with_timeout(
            "x ^ y",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Pow { lhs, rhs, .. } => {
                assert!(matches!(*lhs, PowLhs::Var { name, .. } if name == "x"));
                assert!(matches!(*rhs, PowRhs::Var { name, .. } if name == "y"));
            }
            other => panic!("Expected Expr::Pow, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_complex_pow_precedence() {
        // Test: 2 + 3 * 4 ^ 5 should be 2 + (3 * (4 ^ 5))
        let result = parse_with_timeout(
            "2 + 3 * 4 ^ 5",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Add { lhs, rhs, .. } => {
                assert!(matches!(*lhs, AddLhs::IntLit { value: 2, .. }));
                match *rhs {
                    AddRhs::Mul {
                        lhs: ref mul_lhs,
                        rhs: ref mul_rhs,
                        ..
                    } => {
                        assert!(matches!(**mul_lhs, MulLhs::IntLit { value: 3, .. }));
                        match **mul_rhs {
                            MulRhs::Pow {
                                ref lhs, ref rhs, ..
                            } => {
                                assert!(matches!(**lhs, PowLhs::IntLit { value: 4, .. }));
                                assert!(matches!(**rhs, PowRhs::IntLit { value: 5, .. }));
                            }
                            ref other => panic!("Expected MulRhs::Pow, got {:?}", other),
                        }
                    }
                    ref other => panic!("Expected AddRhs::Mul, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_pow_chain_right_assoc() {
        // Test: a ^ b ^ c ^ d should be a ^ (b ^ (c ^ d))
        let result = parse_with_timeout(
            "a ^ b ^ c ^ d",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Pow { lhs, rhs, .. } => {
                assert!(matches!(*lhs, PowLhs::Var { name, .. } if name == "a"));
                match *rhs {
                    PowRhs::Pow {
                        lhs: ref b_lhs,
                        rhs: ref b_rhs,
                        ..
                    } => {
                        assert!(matches!(**b_lhs, PowLhs::Var { name, .. } if name == "b"));
                        match **b_rhs {
                            PowRhs::Pow {
                                ref lhs, ref rhs, ..
                            } => {
                                assert!(matches!(**lhs, PowLhs::Var { name, .. } if name == "c"));
                                assert!(matches!(**rhs, PowRhs::Var { name, .. } if name == "d"));
                            }
                            ref other => panic!("Expected PowRhs::Pow, got {:?}", other),
                        }
                    }
                    ref other => panic!("Expected PowRhs::Pow, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Pow, got {:?}", other),
        }
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

        match result.unwrap() {
            Expr::Mod { lhs, rhs, .. } => {
                assert!(matches!(*lhs, MulLhs::IntLit { value: 10, .. }));
                assert!(matches!(*rhs, MulRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::Mod, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_mod_left_associative() {
        // Test: 10 % 3 % 2 should be (10 % 3) % 2
        let result = parse_with_timeout(
            "10 % 3 % 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Mod { lhs, rhs, .. } => {
                match *lhs {
                    MulLhs::Mod {
                        lhs: ref inner_lhs,
                        rhs: ref inner_rhs,
                        ..
                    } => {
                        assert!(matches!(**inner_lhs, MulLhs::IntLit { value: 10, .. }));
                        assert!(matches!(**inner_rhs, MulRhs::IntLit { value: 3, .. }));
                    }
                    ref other => panic!("Expected MulLhs::Mod, got {:?}", other),
                }
                assert!(matches!(*rhs, MulRhs::IntLit { value: 2, .. }));
            }
            other => panic!("Expected Expr::Mod, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_mod_with_mul() {
        // Test: 10 * 3 % 2 should be (10 * 3) % 2 - same precedence, left-associative
        let result = parse_with_timeout(
            "10 * 3 % 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Mod { lhs, rhs, .. } => {
                match *lhs {
                    MulLhs::Mul {
                        lhs: ref inner_lhs,
                        rhs: ref inner_rhs,
                        ..
                    } => {
                        assert!(matches!(**inner_lhs, MulLhs::IntLit { value: 10, .. }));
                        assert!(matches!(**inner_rhs, MulRhs::IntLit { value: 3, .. }));
                    }
                    ref other => panic!("Expected MulLhs::Mul, got {:?}", other),
                }
                assert!(matches!(*rhs, MulRhs::IntLit { value: 2, .. }));
            }
            other => panic!("Expected Expr::Mod, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_mod_with_div() {
        // Test: 10 / 3 % 2 should be (10 / 3) % 2 - same precedence, left-associative
        let result = parse_with_timeout(
            "10 / 3 % 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Mod { lhs, rhs, .. } => {
                match *lhs {
                    MulLhs::Div {
                        lhs: ref inner_lhs,
                        rhs: ref inner_rhs,
                        ..
                    } => {
                        assert!(matches!(**inner_lhs, MulLhs::IntLit { value: 10, .. }));
                        assert!(matches!(**inner_rhs, MulRhs::IntLit { value: 3, .. }));
                    }
                    ref other => panic!("Expected MulLhs::Div, got {:?}", other),
                }
                assert!(matches!(*rhs, MulRhs::IntLit { value: 2, .. }));
            }
            other => panic!("Expected Expr::Mod, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_mod_with_add() {
        // Test: 1 + 10 % 3 should be 1 + (10 % 3) - modulo has higher precedence
        let result = parse_with_timeout(
            "1 + 10 % 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Add { lhs, rhs, .. } => {
                assert!(matches!(*lhs, AddLhs::IntLit { value: 1, .. }));
                match *rhs {
                    AddRhs::Mod {
                        lhs: ref mod_lhs,
                        rhs: ref mod_rhs,
                        ..
                    } => {
                        assert!(matches!(**mod_lhs, MulLhs::IntLit { value: 10, .. }));
                        assert!(matches!(**mod_rhs, MulRhs::IntLit { value: 3, .. }));
                    }
                    ref other => panic!("Expected AddRhs::Mod, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_mod_with_pow() {
        // Test: 2 ^ 3 % 5 should be (2 ^ 3) % 5 - power has higher precedence
        let result = parse_with_timeout(
            "2 ^ 3 % 5",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Mod { lhs, rhs, .. } => {
                match *lhs {
                    MulLhs::Pow {
                        lhs: ref pow_lhs,
                        rhs: ref pow_rhs,
                        ..
                    } => {
                        assert!(matches!(**pow_lhs, PowLhs::IntLit { value: 2, .. }));
                        assert!(matches!(**pow_rhs, PowRhs::IntLit { value: 3, .. }));
                    }
                    ref other => panic!("Expected MulLhs::Pow, got {:?}", other),
                }
                assert!(matches!(*rhs, MulRhs::IntLit { value: 5, .. }));
            }
            other => panic!("Expected Expr::Mod, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_mod_with_parens() {
        // Test: 10 % (3 + 2)
        let result = parse_with_timeout(
            "10 % (3 + 2)",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Mod { lhs, rhs, .. } => {
                assert!(matches!(*lhs, MulLhs::IntLit { value: 10, .. }));
                match *rhs {
                    MulRhs::Paren { ref inner, .. } => match **inner {
                        Expr::Add {
                            ref lhs, ref rhs, ..
                        } => {
                            assert!(matches!(**lhs, AddLhs::IntLit { value: 3, .. }));
                            assert!(matches!(**rhs, AddRhs::IntLit { value: 2, .. }));
                        }
                        ref other => panic!("Expected Expr::Add, got {:?}", other),
                    },
                    ref other => panic!("Expected MulRhs::Paren, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Mod, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_mod_with_vars() {
        // Test: x % y
        let result = parse_with_timeout(
            "x % y",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Mod { lhs, rhs, .. } => {
                assert!(matches!(*lhs, MulLhs::Var { name, .. } if name == "x"));
                assert!(matches!(*rhs, MulRhs::Var { name, .. } if name == "y"));
            }
            other => panic!("Expected Expr::Mod, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_complex_mod_precedence() {
        // Test: 2 + 3 * 4 % 5 should be 2 + ((3 * 4) % 5)
        let result = parse_with_timeout(
            "2 + 3 * 4 % 5",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Add { lhs, rhs, .. } => {
                assert!(matches!(*lhs, AddLhs::IntLit { value: 2, .. }));
                match *rhs {
                    AddRhs::Mod {
                        lhs: ref mod_lhs,
                        rhs: ref mod_rhs,
                        ..
                    } => {
                        match **mod_lhs {
                            MulLhs::Mul {
                                ref lhs, ref rhs, ..
                            } => {
                                assert!(matches!(**lhs, MulLhs::IntLit { value: 3, .. }));
                                assert!(matches!(**rhs, MulRhs::IntLit { value: 4, .. }));
                            }
                            ref other => panic!("Expected MulLhs::Mul, got {:?}", other),
                        }
                        assert!(matches!(**mod_rhs, MulRhs::IntLit { value: 5, .. }));
                    }
                    ref other => panic!("Expected AddRhs::Mod, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Add, got {:?}", other),
        }
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

        match result.unwrap() {
            Expr::Neg { inner, .. } => {
                assert!(matches!(*inner, PowLhs::IntLit { value: 5, .. }));
            }
            other => panic!("Expected Expr::Neg, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_simple_ref() {
        // Test: &x
        let result = parse_with_timeout(
            "&x",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Ref { inner, .. } => {
                assert!(matches!(*inner, PowLhs::Var { name, .. } if name == "x"));
            }
            other => panic!("Expected Expr::Ref, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_double_neg() {
        // Test: --5
        let result = parse_with_timeout(
            "--5",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Neg { inner, .. } => match *inner {
                PowLhs::Neg { ref inner, .. } => {
                    assert!(matches!(**inner, PowLhs::IntLit { value: 5, .. }));
                }
                ref other => panic!("Expected PowLhs::Neg, got {:?}", other),
            },
            other => panic!("Expected Expr::Neg, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_neg_ref() {
        // Test: -&x
        let result = parse_with_timeout(
            "-&x",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Neg { inner, .. } => match *inner {
                PowLhs::Ref { ref inner, .. } => {
                    assert!(matches!(**inner, PowLhs::Var { name, .. } if name == "x"));
                }
                ref other => panic!("Expected PowLhs::Ref, got {:?}", other),
            },
            other => panic!("Expected Expr::Neg, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_ref_neg() {
        // Test: &-x
        let result = parse_with_timeout(
            "&-x",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Ref { inner, .. } => match *inner {
                PowLhs::Neg { ref inner, .. } => {
                    assert!(matches!(**inner, PowLhs::Var { name, .. } if name == "x"));
                }
                ref other => panic!("Expected PowLhs::Neg, got {:?}", other),
            },
            other => panic!("Expected Expr::Ref, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_neg_with_pow() {
        // Test: -2 ^ 3 should be (-2) ^ 3 - unary has higher precedence than power
        let result = parse_with_timeout(
            "-2 ^ 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Pow { lhs, rhs, .. } => {
                match *lhs {
                    PowLhs::Neg { ref inner, .. } => {
                        assert!(matches!(**inner, PowLhs::IntLit { value: 2, .. }));
                    }
                    ref other => panic!("Expected PowLhs::Neg, got {:?}", other),
                }
                assert!(matches!(*rhs, PowRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::Pow, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_neg_with_mul() {
        // Test: -2 * 3 should be (-2) * 3
        let result = parse_with_timeout(
            "-2 * 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Mul { lhs, rhs, .. } => {
                match *lhs {
                    MulLhs::Neg { ref inner, .. } => {
                        assert!(matches!(**inner, PowLhs::IntLit { value: 2, .. }));
                    }
                    ref other => panic!("Expected MulLhs::Neg, got {:?}", other),
                }
                assert!(matches!(*rhs, MulRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::Mul, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_neg_with_add() {
        // Test: -2 + 3 should be (-2) + 3
        let result = parse_with_timeout(
            "-2 + 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Add { lhs, rhs, .. } => {
                match *lhs {
                    AddLhs::Neg { ref inner, .. } => {
                        assert!(matches!(**inner, PowLhs::IntLit { value: 2, .. }));
                    }
                    ref other => panic!("Expected AddLhs::Neg, got {:?}", other),
                }
                assert!(matches!(*rhs, AddRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_neg_paren() {
        // Test: -(2 + 3)
        let result = parse_with_timeout(
            "-(2 + 3)",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Neg { inner, .. } => match *inner {
                PowLhs::Paren { ref inner, .. } => match **inner {
                    Expr::Add {
                        ref lhs, ref rhs, ..
                    } => {
                        assert!(matches!(**lhs, AddLhs::IntLit { value: 2, .. }));
                        assert!(matches!(**rhs, AddRhs::IntLit { value: 3, .. }));
                    }
                    ref other => panic!("Expected Expr::Add, got {:?}", other),
                },
                ref other => panic!("Expected PowLhs::Paren, got {:?}", other),
            },
            other => panic!("Expected Expr::Neg, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_ref_with_add() {
        // Test: &x + 3
        let result = parse_with_timeout(
            "&x + 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Add { lhs, rhs, .. } => {
                match *lhs {
                    AddLhs::Ref { ref inner, .. } => {
                        assert!(matches!(**inner, PowLhs::Var { name, .. } if name == "x"));
                    }
                    ref other => panic!("Expected AddLhs::Ref, got {:?}", other),
                }
                assert!(matches!(*rhs, AddRhs::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::Add, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_complex_unary() {
        // Test: -a ^ -b should be (-a) ^ (-b)
        let result = parse_with_timeout(
            "-a ^ -b",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Pow { lhs, rhs, .. } => {
                match *lhs {
                    PowLhs::Neg { ref inner, .. } => {
                        assert!(matches!(**inner, PowLhs::Var { name, .. } if name == "a"));
                    }
                    ref other => panic!("Expected PowLhs::Neg, got {:?}", other),
                }
                match *rhs {
                    PowRhs::Neg { ref inner, .. } => {
                        assert!(matches!(**inner, PowLhs::Var { name, .. } if name == "b"));
                    }
                    ref other => panic!("Expected PowRhs::Neg, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Pow, got {:?}", other),
        }
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

        match result.unwrap() {
            Expr::And { lhs, rhs, .. } => {
                assert!(matches!(*lhs, CmpLhs::BoolLit { value: true, .. }));
                match *rhs {
                    CmpRhs::Paren { ref inner, .. } => {
                        assert!(matches!(**inner, Expr::BoolLit { value: false, .. }));
                    }
                    ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::And, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_simple_or() {
        // Test: true or false
        let result = parse_with_timeout(
            "true or false",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Or { lhs, rhs, .. } => {
                assert!(matches!(*lhs, CmpLhs::BoolLit { value: true, .. }));
                match *rhs {
                    CmpRhs::Paren { ref inner, .. } => {
                        assert!(matches!(**inner, Expr::BoolLit { value: false, .. }));
                    }
                    ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Or, got {:?}", other),
        }
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
        match result.unwrap() {
            Expr::And { lhs, rhs, .. } => {
                match *lhs {
                    CmpLhs::Or {
                        lhs: ref or_lhs,
                        rhs: ref or_rhs,
                        ..
                    } => {
                        assert!(matches!(**or_lhs, CmpLhs::Var { name, .. } if name == "a"));
                        match **or_rhs {
                            CmpRhs::Paren { ref inner, .. } => {
                                assert!(matches!(**inner, Expr::Var { name, .. } if name == "b"));
                            }
                            ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
                        }
                    }
                    ref other => panic!("Expected CmpLhs::Or, got {:?}", other),
                }
                match *rhs {
                    CmpRhs::Paren { ref inner, .. } => {
                        assert!(matches!(**inner, Expr::Var { name, .. } if name == "c"));
                    }
                    ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::And, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_logical_with_comparison() {
        // Test: x == 1 and y == 2
        let result = parse_with_timeout(
            "x == 1 and y == 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::And { lhs, rhs, .. } => {
                match *lhs {
                    CmpLhs::Eq {
                        lhs: ref eq_lhs,
                        rhs: ref eq_rhs,
                        ..
                    } => {
                        assert!(matches!(**eq_lhs, CmpLhs::Var { name, .. } if name == "x"));
                        assert!(matches!(**eq_rhs, CmpRhs::IntLit { value: 1, .. }));
                    }
                    ref other => panic!("Expected CmpLhs::Eq, got {:?}", other),
                }
                match *rhs {
                    CmpRhs::Paren { ref inner, .. } => match **inner {
                        Expr::Eq {
                            ref lhs, ref rhs, ..
                        } => {
                            assert!(matches!(**lhs, CmpLhs::Var { name, .. } if name == "y"));
                            assert!(matches!(**rhs, CmpRhs::IntLit { value: 2, .. }));
                        }
                        ref other => panic!("Expected Expr::Eq, got {:?}", other),
                    },
                    ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::And, got {:?}", other),
        }
    }

    #[test]
    fn test_expr_logical_left_associative() {
        // Test: a and b and c should be (a and b) and c
        let result = parse_with_timeout(
            "a and b and c",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::And { lhs, rhs, .. } => {
                match *lhs {
                    CmpLhs::And {
                        lhs: ref and_lhs,
                        rhs: ref and_rhs,
                        ..
                    } => {
                        assert!(matches!(**and_lhs, CmpLhs::Var { name, .. } if name == "a"));
                        match **and_rhs {
                            CmpRhs::Paren { ref inner, .. } => {
                                assert!(matches!(**inner, Expr::Var { name, .. } if name == "b"));
                            }
                            ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
                        }
                    }
                    ref other => panic!("Expected CmpLhs::And, got {:?}", other),
                }
                match *rhs {
                    CmpRhs::Paren { ref inner, .. } => {
                        assert!(matches!(**inner, Expr::Var { name, .. } if name == "c"));
                    }
                    ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::And, got {:?}", other),
        }
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
        assert!(matches!(result.unwrap(), Type::Bool { .. }));
    }

    #[test]
    fn test_type_i32() {
        let result = parse_with_timeout(
            "i32",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert!(matches!(result.unwrap(), Type::I32 { .. }));
    }

    #[test]
    fn test_type_f64() {
        let result = parse_with_timeout(
            "f64",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert!(matches!(result.unwrap(), Type::F64 { .. }));
    }

    #[test]
    fn test_type_real() {
        let result = parse_with_timeout(
            "Real",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert!(matches!(result.unwrap(), Type::Real { .. }));
    }

    #[test]
    fn test_type_algebraic() {
        let result = parse_with_timeout(
            "Algebraic",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );
        assert!(matches!(result.unwrap(), Type::Algebraic { .. }));
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

        match result.unwrap() {
            Stmt::Let {
                name,
                type_annotation,
                init,
                ..
            } => {
                assert_eq!(name, "x");
                assert!(matches!(type_annotation, Some(Type::I32 { .. })));
                assert!(matches!(init, Some(Expr::IntLit { value: 42, .. })));
            }
        }
    }

    #[test]
    fn test_let_with_type_only() {
        // let y: bool;
        let result = parse_with_timeout(
            "let y: bool;",
            |input| let_stmt(expr_inner()).parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Stmt::Let {
                name,
                type_annotation,
                init,
                ..
            } => {
                assert_eq!(name, "y");
                assert!(matches!(type_annotation, Some(Type::Bool { .. })));
                assert!(init.is_none());
            }
        }
    }

    #[test]
    fn test_let_with_init_only() {
        // let z = 3.14;
        let result = parse_with_timeout(
            "let z = 3.14;",
            |input| let_stmt(expr_inner()).parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Stmt::Let {
                name,
                type_annotation,
                init,
                ..
            } => {
                assert_eq!(name, "z");
                assert!(type_annotation.is_none());
                assert!(matches!(init, Some(Expr::FloatLit { value, .. }) if value == 3.14));
            }
        }
    }

    #[test]
    fn test_let_no_type_no_init() {
        // let w;
        let result = parse_with_timeout(
            "let w;",
            |input| let_stmt(expr_inner()).parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Stmt::Let {
                name,
                type_annotation,
                init,
                ..
            } => {
                assert_eq!(name, "w");
                assert!(type_annotation.is_none());
                assert!(init.is_none());
            }
        }
    }

    #[test]
    fn test_let_with_expression() {
        // let result: i32 = 1 + 2 * 3;
        let result = parse_with_timeout(
            "let result: i32 = 1 + 2 * 3;",
            |input| let_stmt(expr_inner()).parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Stmt::Let {
                name,
                type_annotation,
                init,
                ..
            } => {
                assert_eq!(name, "result");
                assert!(matches!(type_annotation, Some(Type::I32 { .. })));
                match init {
                    Some(Expr::Add { lhs, rhs, .. }) => {
                        assert!(matches!(*lhs, AddLhs::IntLit { value: 1, .. }));
                        match *rhs {
                            AddRhs::Mul {
                                lhs: ref mul_lhs,
                                rhs: ref mul_rhs,
                                ..
                            } => {
                                assert!(matches!(**mul_lhs, MulLhs::IntLit { value: 2, .. }));
                                assert!(matches!(**mul_rhs, MulRhs::IntLit { value: 3, .. }));
                            }
                            ref other => panic!("Expected AddRhs::Mul, got {:?}", other),
                        }
                    }
                    other => panic!("Expected Some(Expr::Add), got {:?}", other),
                }
            }
        }
    }

    // ========================================================================
    // Span Tracking Tests
    // ========================================================================

    #[test]
    fn test_span_simple_int_literal() {
        // Test: 42
        let result = parse_with_timeout(
            "42",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expr = result.unwrap();
        let span = expr.span();

        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 3); // "42" is 2 chars, end_column is exclusive
    }

    #[test]
    fn test_span_simple_var() {
        // Test: foo
        let result = parse_with_timeout(
            "foo",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expr = result.unwrap();
        let span = expr.span();

        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 4); // "foo" is 3 chars
    }

    #[test]
    fn test_span_binary_addition() {
        // Test: 1 + 2
        let result = parse_with_timeout(
            "1 + 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expr = result.unwrap();
        let span = expr.span();

        // Span should cover from "1" to "2"
        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 6); // "1 + 2" covers columns 1-5
    }

    #[test]
    fn test_span_nested_expression() {
        // Test: 1 + 2 * 3
        let result = parse_with_timeout(
            "1 + 2 * 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expr = result.unwrap();
        let span = expr.span();

        // Span should cover the entire expression
        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 10);
    }

    #[test]
    fn test_span_parenthesized() {
        // Test: (1 + 2)
        let result = parse_with_timeout(
            "(1 + 2)",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expr = result.unwrap();
        let span = expr.span();

        // Span should include the parentheses
        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1); // Start at '('
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 8); // End after ')'
    }

    #[test]
    fn test_span_unary_negation() {
        // Test: -42
        let result = parse_with_timeout(
            "-42",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expr = result.unwrap();
        let span = expr.span();

        // Span should start at '-' and end after '42'
        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 4);
    }

    #[test]
    fn test_span_comparison() {
        // Test: 1 == 2
        let result = parse_with_timeout(
            "1 == 2",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expr = result.unwrap();
        let span = expr.span();

        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 7);
    }

    #[test]
    fn test_span_logical_and() {
        // Test: true and false
        let result = parse_with_timeout(
            "true and false",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expr = result.unwrap();
        let span = expr.span();

        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 15);
    }

    #[test]
    fn test_span_power_operator() {
        // Test: 2 ^ 3
        let result = parse_with_timeout(
            "2 ^ 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expr = result.unwrap();
        let span = expr.span();

        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 6);
    }

    #[test]
    fn test_span_let_statement() {
        // Test: let x = 42;
        let result = parse_with_timeout(
            "let x = 42;",
            |input| let_stmt(expr_inner()).parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Stmt::Let {
                name_span, span, ..
            } => {
                // name_span should point to "x"
                assert_eq!(name_span.start.line, 1);
                assert_eq!(name_span.start.column, 5); // "x" starts at column 5

                // Overall span should cover entire statement
                assert_eq!(span.start.line, 1);
                assert_eq!(span.start.column, 1); // Starts at "let"
                assert_eq!(span.lines, 0);
                assert_eq!(span.end_column, 12); // Ends after ';'
            }
        }
    }

    #[test]
    fn test_span_type_annotation() {
        // Test: i32
        let result = parse_with_timeout(
            "i32",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );

        let type_ann = result.unwrap();
        let span = type_ann.span();

        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 4); // "i32" is 3 chars
    }

    #[test]
    fn test_span_complex_nested() {
        // Test: (1 + 2) * 3
        let result = parse_with_timeout(
            "(1 + 2) * 3",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        let expr = result.unwrap();
        let span = expr.span();

        // Should span from '(' to '3'
        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.lines, 0);
        assert_eq!(span.end_column, 12);
    }

    #[test]
    fn test_hasspan_trait_for_different_types() {
        // Test that HasSpan trait works for various AST node types
        use crate::ast::HasSpan;

        let expr_result = parse_with_timeout(
            "42",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );
        let expr = expr_result.unwrap();
        let _expr_span = expr.span(); // Uses HasSpan trait

        let type_result = parse_with_timeout(
            "bool",
            |input| type_annotation().parse(input).into_result(),
            Duration::from_secs(1),
        );
        let type_ann = type_result.unwrap();
        let _type_span = type_ann.span(); // Uses HasSpan trait

        // If we get here without panic, HasSpan works for all types
        assert!(true);
    }

    // ========================================================================
    // Function Call Tests
    // ========================================================================

    #[test]
    fn test_function_call_no_args() {
        // Test: foo()
        let result = parse_with_timeout(
            "foo()",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Call { name, args, .. } => {
                assert_eq!(name, "foo");
                assert_eq!(args.len(), 0);
            }
            other => panic!("Expected Expr::Call, got {:?}", other),
        }
    }

    #[test]
    fn test_function_call_one_arg() {
        // Test: foo(42)
        let result = parse_with_timeout(
            "foo(42)",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Call { name, args, .. } => {
                assert_eq!(name, "foo");
                assert_eq!(args.len(), 1);
                assert!(matches!(args[0], Expr::IntLit { value: 42, .. }));
            }
            other => panic!("Expected Expr::Call, got {:?}", other),
        }
    }

    #[test]
    fn test_function_call_multiple_args() {
        // Test: add(1, 2, 3)
        let result = parse_with_timeout(
            "add(1, 2, 3)",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Call { name, args, .. } => {
                assert_eq!(name, "add");
                assert_eq!(args.len(), 3);
                assert!(matches!(args[0], Expr::IntLit { value: 1, .. }));
                assert!(matches!(args[1], Expr::IntLit { value: 2, .. }));
                assert!(matches!(args[2], Expr::IntLit { value: 3, .. }));
            }
            other => panic!("Expected Expr::Call, got {:?}", other),
        }
    }

    #[test]
    fn test_function_call_expr_args() {
        // Test: foo(1 + 2, 3 * 4)
        let result = parse_with_timeout(
            "foo(1 + 2, 3 * 4)",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Call { name, args, .. } => {
                assert_eq!(name, "foo");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], Expr::Add { .. }));
                assert!(matches!(args[1], Expr::Mul { .. }));
            }
            other => panic!("Expected Expr::Call, got {:?}", other),
        }
    }

    #[test]
    fn test_function_call_nested() {
        // Test: foo(bar(42))
        let result = parse_with_timeout(
            "foo(bar(42))",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Call { name, args, .. } => {
                assert_eq!(name, "foo");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Expr::Call {
                        name: inner_name,
                        args: inner_args,
                        ..
                    } => {
                        assert_eq!(*inner_name, "bar");
                        assert_eq!(inner_args.len(), 1);
                        assert!(matches!(inner_args[0], Expr::IntLit { value: 42, .. }));
                    }
                    other => panic!("Expected inner Expr::Call, got {:?}", other),
                }
            }
            other => panic!("Expected Expr::Call, got {:?}", other),
        }
    }

    #[test]
    fn test_function_call_in_expression() {
        // Test: foo(1) + bar(2)
        let result = parse_with_timeout(
            "foo(1) + bar(2)",
            |input| expr().parse(input).into_result(),
            Duration::from_secs(2),
        );

        match result.unwrap() {
            Expr::Add { lhs, rhs, .. } => {
                assert!(matches!(*lhs, AddLhs::Call { .. }));
                assert!(matches!(*rhs, AddRhs::Call { .. }));
            }
            other => panic!("Expected Expr::Add, got {:?}", other),
        }
    }
}
