//! Logical expression parsers
//!
//! This module contains parsers for logical operations:
//! - AND (and)
//! - OR (or)
//!
//! These operators have the lowest precedence of all binary operators.

use crate::ast::*;
use crate::lexer::Token;
use chumsky::prelude::*;

use super::ParseError;

// ============================================================================
// Logical Operators Parser
// ============================================================================

/// Parser for logical operators (lower precedence than comparison)
pub fn log_parser<'src, C>(
    cmp_lhs: C,
) -> impl Parser<'src, &'src [Token<'src>], CmpLhs, ParseError<'src>> + Clone
where
    C: Parser<'src, &'src [Token<'src>], CmpLhs, ParseError<'src>> + Clone,
{
    let and_op = select! { Token::And(_) => "and" };
    let or_op = select! { Token::Or(_) => "or" };

    let log_atom = cmp_lhs.clone();

    // Left-associative logical operators (lower precedence than comparison)
    log_atom.foldl(
        choice((and_op, or_op)).then(cmp_lhs).repeated(),
        |lhs, (op, rhs)| match op {
            "and" => CmpLhs::And {
                lhs: Box::new(lhs),
                rhs: Box::new(CmpRhs::Paren(Box::new(Expr::from(rhs)))),
            },
            "or" => CmpLhs::Or {
                lhs: Box::new(lhs),
                rhs: Box::new(CmpRhs::Paren(Box::new(Expr::from(rhs)))),
            },
            _ => unreachable!(),
        },
    )
}
