//! Comparison expression parsers
//!
//! This module contains parsers for comparison operations:
//! - Equality (==)
//! - Inequality (!=)
//!
//! These operators have lower precedence than arithmetic operators
//! but higher precedence than logical operators.

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
// Comparison Parsers
// ============================================================================

/// Parser for comparison right-hand side (CmpRhs<'src>)
pub fn cmp_rhs_parser<'src, A>(
    add_lhs: A,
) -> impl Parser<'src, &'src [Token<'src>], CmpRhs<'src>, ParseError<'src>> + Clone
where
    A: Parser<'src, &'src [Token<'src>], AddLhs<'src>, ParseError<'src>> + Clone,
{
    add_lhs.map(Into::into)
}

/// Parser for comparison left-hand side (CmpLhs<'src>) with operators
pub fn cmp_lhs_parser<'src, A, R>(
    add_lhs: A,
    cmp_rhs: R,
) -> impl Parser<'src, &'src [Token<'src>], CmpLhs<'src>, ParseError<'src>> + Clone
where
    A: Parser<'src, &'src [Token<'src>], AddLhs<'src>, ParseError<'src>> + Clone,
    R: Parser<'src, &'src [Token<'src>], CmpRhs<'src>, ParseError<'src>> + Clone,
{
    let eq_op = select! { Token::EqualsEquals(_) => "==" };
    let neq_op = select! { Token::NotEquals(_) => "!=" };

    let cmp_atom = add_lhs.map(Into::into);

    // Left-associative equality and not-equal operators (higher precedence than logical)
    cmp_atom.foldl(
        choice((eq_op, neq_op)).then(cmp_rhs).repeated(),
        |lhs: CmpLhs<'src>, (op, rhs): (&str, CmpRhs<'src>)| {
            let lhs_span = lhs.span();
            let rhs_span = rhs.span();
            let span = combine_spans(lhs_span, rhs_span);

            match op {
                "==" => CmpLhs::Eq {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                },
                "!=" => CmpLhs::NotEq {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                },
                _ => unreachable!(),
            }
        },
    )
}
