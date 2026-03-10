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
#[cfg_attr(test, allow(dead_code))] // Used in parser tests
pub(crate) mod atoms;
mod comparison;
mod error;
mod logical;
mod stmt;

// ============================================================================
// Re-exports
// ============================================================================

pub use error::report_parse_errors;
#[allow(unused_imports)] // Re-exported for public API and tests
pub use stmt::{
    assignment_stmt, block_stmt, expression_stmt, field_assignment_stmt, for_stmt, function_def,
    if_stmt, include_stmt, let_stmt, optimize_stmt, return_stmt, struct_def, unit_prefix_stmt,
    unit_stmt, with_stmt,
};

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
    expr_inner_with_source(None)
}

/// Internal expression parser with source text for rune block body extraction
pub fn expr_inner_with_source<'src>(
    source: Option<&'src str>,
) -> impl Parser<'src, &'src [Token<'src>], Expr<'src>, ParseError<'src>> + Clone {
    recursive(move |expr_rec| {
        let pow_lhs = arithmetic::pow_lhs_parser(expr_rec.clone(), source);
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

/// Parse a complete expression with source text access (for rune block body extraction)
#[cfg_attr(not(test), allow(dead_code))] // Used in tests
pub fn expr_with_source<'src>(
    source: &'src str,
) -> impl Parser<'src, &'src [Token<'src>], Expr<'src>, ParseError<'src>> + Clone {
    expr_inner_with_source(Some(source)).then_ignore(end())
}

// ============================================================================
// Program Parser
// ============================================================================

/// Parse a complete CAD-DSL program (sequence of statements) from a token slice.
///
/// Returns `Ok(stmts)` on success, `Err(errors)` on parse failure.
/// Used by the include resolver to parse included files with the same rules
/// as the main file.
///
/// Both `content` and `tokens` must share the same lifetime `'src`. In practice
/// both are arena-allocated (see `src/include_resolver.rs`) so `'src = 'arena`.
pub fn parse_program<'src>(
    content: &'src str,
    tokens: &'src [Token<'src>],
) -> Result<Vec<Stmt<'src>>, Vec<Rich<'src, Token<'src>>>> {
    use chumsky::IterParser;
    use chumsky::primitive::choice;

    let stmt_parser = recursive(|stmt_rec| {
        let expr = expr_inner_with_source(Some(content));
        choice((
            unit_prefix_stmt(),
            unit_stmt(),
            include_stmt(),
            struct_def(expr.clone()),
            function_def(expr.clone()),
            let_stmt(expr.clone()),
            assignment_stmt(expr.clone()),
            field_assignment_stmt(expr.clone()),
            with_stmt(expr.clone(), stmt_rec.clone()),
            for_stmt(expr.clone(), stmt_rec.clone()),
            if_stmt(expr.clone(), stmt_rec.clone()),
            optimize_stmt(expr.clone()),
            expression_stmt(expr),
        ))
    })
    .repeated()
    .collect::<Vec<_>>();

    stmt_parser.parse(tokens).into_result()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests;
