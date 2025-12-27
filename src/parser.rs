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
mod tests;
