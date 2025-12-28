//! Logical expression parsers
//!
//! This module contains parsers for logical operations:
//! - AND (and)
//! - OR (or)
//!
//! These operators have the lowest precedence of all binary operators.

use crate::ast::HasSpan;
use crate::ast::*;
use crate::lexer::{Span, Token};
use chumsky::prelude::*;

use super::ParseError;

// ============================================================================
// Helper functions for span management
// ============================================================================

/// Combine two spans into a larger span that encompasses both
fn combine_spans(left: Span, right: Span) -> Span {
    Span {
        start: left.start,
        lines: if right.lines > 0 {
            left.lines + right.lines
        } else {
            left.lines
        },
        end_column: right.end_column,
    }
}

// ============================================================================
// Logical Operators Parser
// ============================================================================

/// Parser for logical operators (lower precedence than comparison)
pub fn log_parser<'src, C>(
    cmp_lhs: C,
) -> impl Parser<'src, &'src [Token<'src>], CmpLhs<'src>, ParseError<'src>> + Clone
where
    C: Parser<'src, &'src [Token<'src>], CmpLhs<'src>, ParseError<'src>> + Clone,
{
    let and_op = select! { Token::And(_) => "and" };
    let or_op = select! { Token::Or(_) => "or" };

    let log_atom = cmp_lhs.clone();

    // Left-associative logical operators (lower precedence than comparison)
    log_atom.foldl(
        choice((and_op, or_op)).then(cmp_lhs).repeated(),
        |lhs: CmpLhs<'src>, (op, rhs): (&str, CmpLhs<'src>)| {
            let lhs_span = lhs.span();
            let rhs_span = rhs.span();
            let span = combine_spans(lhs_span, rhs_span);
            let paren_span = rhs_span; // Use rhs span for Paren

            match op {
                "and" => CmpLhs::And {
                    lhs: Box::new(lhs),
                    rhs: Box::new(CmpRhs::Paren {
                        inner: Box::new(Expr::from(rhs)),
                        span: paren_span,
                    }),
                    span,
                },
                "or" => CmpLhs::Or {
                    lhs: Box::new(lhs),
                    rhs: Box::new(CmpRhs::Paren {
                        inner: Box::new(Expr::from(rhs)),
                        span: paren_span,
                    }),
                    span,
                },
                _ => unreachable!(),
            }
        },
    )
}
