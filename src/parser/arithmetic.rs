//! Arithmetic expression parsers
//!
//! This module contains parsers for arithmetic operations:
//! - Power (^) - right-associative, highest precedence
//! - Multiplication (*), Division (/), Modulo (%) - left-associative
//! - Addition (+), Subtraction (-) - left-associative, lowest precedence
//! - Unary negation (-) and reference (&) - highest precedence

use crate::ast::*;
use crate::lexer::{LineColumn, Span, Token};
use chumsky::prelude::*;

use super::ParseError;
use super::atoms::atom;

// ============================================================================
// Helper functions for span management
// ============================================================================

/// Helper to extract span from MulLhs
fn get_mullhs_span(node: &MulLhs) -> Span {
    match node {
        MulLhs::Paren { span, .. }
        | MulLhs::Mul { span, .. }
        | MulLhs::Div { span, .. }
        | MulLhs::Mod { span, .. }
        | MulLhs::Pow { span, .. }
        | MulLhs::Neg { span, .. }
        | MulLhs::Ref { span, .. }
        | MulLhs::Var { span, .. }
        | MulLhs::IntLit { span, .. }
        | MulLhs::FloatLit { span, .. }
        | MulLhs::BoolLit { span, .. } => *span,
    }
}

/// Helper to extract span from MulRhs
fn get_mulrhs_span(node: &MulRhs) -> Span {
    match node {
        MulRhs::Paren { span, .. }
        | MulRhs::Pow { span, .. }
        | MulRhs::Neg { span, .. }
        | MulRhs::Ref { span, .. }
        | MulRhs::Var { span, .. }
        | MulRhs::IntLit { span, .. }
        | MulRhs::FloatLit { span, .. }
        | MulRhs::BoolLit { span, .. } => *span,
    }
}

/// Helper to extract span from AddLhs
fn get_addlhs_span(node: &AddLhs) -> Span {
    match node {
        AddLhs::Add { span, .. }
        | AddLhs::Sub { span, .. }
        | AddLhs::Paren { span, .. }
        | AddLhs::Mul { span, .. }
        | AddLhs::Div { span, .. }
        | AddLhs::Mod { span, .. }
        | AddLhs::Pow { span, .. }
        | AddLhs::Neg { span, .. }
        | AddLhs::Ref { span, .. }
        | AddLhs::Var { span, .. }
        | AddLhs::IntLit { span, .. }
        | AddLhs::FloatLit { span, .. }
        | AddLhs::BoolLit { span, .. } => *span,
    }
}

/// Helper to extract span from AddRhs
fn get_addrhs_span(node: &AddRhs) -> Span {
    match node {
        AddRhs::Paren { span, .. }
        | AddRhs::Mul { span, .. }
        | AddRhs::Div { span, .. }
        | AddRhs::Mod { span, .. }
        | AddRhs::Pow { span, .. }
        | AddRhs::Neg { span, .. }
        | AddRhs::Ref { span, .. }
        | AddRhs::Var { span, .. }
        | AddRhs::IntLit { span, .. }
        | AddRhs::FloatLit { span, .. }
        | AddRhs::BoolLit { span, .. } => *span,
    }
}

/// Combine two spans into a larger span that encompasses both
fn combine_spans(left: Span, right: Span) -> Span {
    Span {
        start: left.start,
        lines: if right.lines > 0 {
            left.lines + right.lines
        } else {
            left.lines
        },
        end_column: if right.lines > 0 {
            right.end_column
        } else {
            right.end_column
        },
    }
}

// ============================================================================
// Power Parsers (Highest precedence arithmetic operator)
// ============================================================================

/// Parser for power base (PowLhs) - atoms, parens, and unary operators
pub fn pow_lhs_parser<'src, E>(
    expr_rec: E,
) -> impl Parser<'src, &'src [Token<'src>], PowLhs, ParseError<'src>> + Clone
where
    E: Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone + 'src,
{
    let lparen = select! { Token::LeftParen(_) => () };
    let rparen = select! { Token::RightParen(_) => () };

    // Recursive parser for unary operators (allows stacking like --x or &-x)
    recursive(|unary_rec| {
        let neg_op = select! { Token::Minus(_) => () };
        let ref_op = select! { Token::Ampersand(_) => () };

        choice((
            // Unary negation: -<expr>
            neg_op
                .ignore_then(unary_rec.clone())
                .map_with_span(|inner, span| PowLhs::Neg {
                    inner: Box::new(inner),
                    span,
                }),
            // Unary reference: &<expr>
            ref_op
                .ignore_then(unary_rec)
                .map_with_span(|inner, span| PowLhs::Ref {
                    inner: Box::new(inner),
                    span,
                }),
            // Atom (base case)
            atom().map(Into::into),
            // Parenthesized expression
            expr_rec
                .delimited_by(lparen, rparen)
                .map_with_span(|e, span| PowLhs::Paren {
                    inner: Box::new(e),
                    span,
                }),
        ))
    })
}

/// Parser for power right-hand side (PowRhs) - can contain Pow recursively
pub fn pow_rhs_parser<'src, E>(
    _expr_rec: E,
    pow_lhs: impl Parser<'src, &'src [Token<'src>], PowLhs, ParseError<'src>> + Clone + 'src,
) -> impl Parser<'src, &'src [Token<'src>], PowRhs, ParseError<'src>> + Clone
where
    E: Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone,
{
    let pow_op = select! { Token::Power(_) => () };

    // Right-associative: base (^ rhs)?
    // rhs can recursively contain power operations
    recursive(|pow_rhs_rec| {
        let base_parser = pow_lhs.clone();
        base_parser
            .then(pow_op.then(pow_rhs_rec).or_not())
            .map_with_span(|(base, rest), span| {
                match rest {
                    None => base.into(), // No power operator, just return base as PowRhs
                    Some((_, rhs)) => {
                        // Build Pow node
                        PowRhs::Pow {
                            lhs: Box::new(base),
                            rhs: Box::new(rhs),
                            span,
                        }
                    }
                }
            })
    })
}

// ============================================================================
// Multiplication/Division/Modulo Parsers
// ============================================================================

/// Parser for multiplication right-hand side (MulRhs)
pub fn mul_rhs_parser<'src, E>(
    expr_rec: E,
    pow_rhs: impl Parser<'src, &'src [Token<'src>], PowRhs, ParseError<'src>> + Clone,
) -> impl Parser<'src, &'src [Token<'src>], MulRhs, ParseError<'src>> + Clone
where
    E: Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone,
{
    let lparen = select! { Token::LeftParen(_) => () };
    let rparen = select! { Token::RightParen(_) => () };

    choice((
        pow_rhs.map(|p| {
            // Convert PowRhs to MulRhs
            match p {
                PowRhs::Pow { lhs, rhs, span } => MulRhs::Pow { lhs, rhs, span },
                PowRhs::Paren { inner, span } => MulRhs::Paren { inner, span },
                PowRhs::Neg { inner, span } => MulRhs::Neg { inner, span },
                PowRhs::Ref { inner, span } => MulRhs::Ref { inner, span },
                PowRhs::Var { name, span } => MulRhs::Var { name, span },
                PowRhs::IntLit { value, span } => MulRhs::IntLit { value, span },
                PowRhs::FloatLit { value, span } => MulRhs::FloatLit { value, span },
                PowRhs::BoolLit { value, span } => MulRhs::BoolLit { value, span },
            }
        }),
        expr_rec
            .delimited_by(lparen, rparen)
            .map_with_span(|e, span| MulRhs::Paren {
                inner: Box::new(e),
                span,
            }),
    ))
}

/// Parser for multiplication left-hand side (MulLhs) with operators
pub fn mul_lhs_parser<'src, E, R, P>(
    expr_rec: E,
    mul_rhs: R,
    pow_rhs: P,
) -> impl Parser<'src, &'src [Token<'src>], MulLhs, ParseError<'src>> + Clone
where
    E: Parser<'src, &'src [Token<'src>], Expr, ParseError<'src>> + Clone,
    R: Parser<'src, &'src [Token<'src>], MulRhs, ParseError<'src>> + Clone,
    P: Parser<'src, &'src [Token<'src>], PowRhs, ParseError<'src>> + Clone,
{
    let lparen = select! { Token::LeftParen(_) => () };
    let rparen = select! { Token::RightParen(_) => () };
    let mul_op = select! { Token::Multiply(_) => '*' };
    let div_op = select! { Token::Divide(_) => '/' };
    let mod_op = select! { Token::Modulo(_) => '%' };

    // mul_atom now uses pow_rhs which handles power operations
    let mul_atom = choice((
        pow_rhs.map(|p| {
            // Convert PowRhs to MulLhs
            match p {
                PowRhs::Pow { lhs, rhs, span } => MulLhs::Pow { lhs, rhs, span },
                PowRhs::Paren { inner, span } => MulLhs::Paren { inner, span },
                PowRhs::Neg { inner, span } => MulLhs::Neg { inner, span },
                PowRhs::Ref { inner, span } => MulLhs::Ref { inner, span },
                PowRhs::Var { name, span } => MulLhs::Var { name, span },
                PowRhs::IntLit { value, span } => MulLhs::IntLit { value, span },
                PowRhs::FloatLit { value, span } => MulLhs::FloatLit { value, span },
                PowRhs::BoolLit { value, span } => MulLhs::BoolLit { value, span },
            }
        }),
        expr_rec
            .delimited_by(lparen, rparen)
            .map_with_span(|e, span| MulLhs::Paren {
                inner: Box::new(e),
                span,
            }),
    ));

    // Left-associative multiplication, division, and modulo
    mul_atom.foldl(
        choice((mul_op, div_op, mod_op)).then(mul_rhs).repeated(),
        |lhs, (op, rhs)| {
            let lhs_span = get_mullhs_span(&lhs);
            let rhs_span = get_mulrhs_span(&rhs);
            let span = combine_spans(lhs_span, rhs_span);

            if op == '*' {
                MulLhs::Mul {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                }
            } else if op == '/' {
                MulLhs::Div {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                }
            } else {
                MulLhs::Mod {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                }
            }
        },
    )
}

// ============================================================================
// Addition/Subtraction Parsers
// ============================================================================

/// Parser for addition right-hand side (AddRhs)
pub fn add_rhs_parser<'src, M>(
    mul_lhs: M,
) -> impl Parser<'src, &'src [Token<'src>], AddRhs, ParseError<'src>> + Clone
where
    M: Parser<'src, &'src [Token<'src>], MulLhs, ParseError<'src>> + Clone,
{
    mul_lhs.map(Into::into)
}

/// Parser for addition left-hand side (AddLhs) with operators
pub fn add_lhs_parser<'src, M, R>(
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
            let lhs_span = get_addlhs_span(&lhs);
            let rhs_span = get_addrhs_span(&rhs);
            let span = combine_spans(lhs_span, rhs_span);

            if op == '+' {
                AddLhs::Add {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                }
            } else {
                AddLhs::Sub {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                }
            }
        },
    )
}
