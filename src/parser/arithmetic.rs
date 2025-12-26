//! Arithmetic expression parsers
//!
//! This module contains parsers for arithmetic operations:
//! - Power (^) - right-associative, highest precedence
//! - Multiplication (*), Division (/), Modulo (%) - left-associative
//! - Addition (+), Subtraction (-) - left-associative, lowest precedence
//! - Unary negation (-) and reference (&) - highest precedence

use crate::ast::*;
use crate::lexer::Token;
use chumsky::prelude::*;

use super::ParseError;
use super::atoms::atom;

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
                .map(|inner| PowLhs::Neg {
                    inner: Box::new(inner),
                }),
            // Unary reference: &<expr>
            ref_op.ignore_then(unary_rec).map(|inner| PowLhs::Ref {
                inner: Box::new(inner),
            }),
            // Atom (base case)
            atom().map(Into::into),
            // Parenthesized expression
            expr_rec
                .delimited_by(lparen, rparen)
                .map(|e| PowLhs::Paren(Box::new(e))),
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
            .map(|(base, rest)| {
                match rest {
                    None => base.into(), // No power operator, just return base as PowRhs
                    Some((_, rhs)) => {
                        // Build Pow node
                        PowRhs::Pow {
                            lhs: Box::new(base),
                            rhs: Box::new(rhs),
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
                PowRhs::Pow { lhs, rhs } => MulRhs::Pow { lhs, rhs },
                PowRhs::Paren(e) => MulRhs::Paren(e),
                PowRhs::Neg { inner } => MulRhs::Neg { inner },
                PowRhs::Ref { inner } => MulRhs::Ref { inner },
                PowRhs::Var(s) => MulRhs::Var(s),
                PowRhs::IntLit(i) => MulRhs::IntLit(i),
                PowRhs::FloatLit(f) => MulRhs::FloatLit(f),
                PowRhs::BoolLit(b) => MulRhs::BoolLit(b),
            }
        }),
        expr_rec
            .delimited_by(lparen, rparen)
            .map(|e| MulRhs::Paren(Box::new(e))),
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
                PowRhs::Pow { lhs, rhs } => MulLhs::Pow { lhs, rhs },
                PowRhs::Paren(e) => MulLhs::Paren(e),
                PowRhs::Neg { inner } => MulLhs::Neg { inner },
                PowRhs::Ref { inner } => MulLhs::Ref { inner },
                PowRhs::Var(s) => MulLhs::Var(s),
                PowRhs::IntLit(i) => MulLhs::IntLit(i),
                PowRhs::FloatLit(f) => MulLhs::FloatLit(f),
                PowRhs::BoolLit(b) => MulLhs::BoolLit(b),
            }
        }),
        expr_rec
            .delimited_by(lparen, rparen)
            .map(|e| MulLhs::Paren(Box::new(e))),
    ));

    // Left-associative multiplication, division, and modulo
    mul_atom.foldl(
        choice((mul_op, div_op, mod_op)).then(mul_rhs).repeated(),
        |lhs, (op, rhs)| {
            if op == '*' {
                MulLhs::Mul {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            } else if op == '/' {
                MulLhs::Div {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            } else {
                MulLhs::Mod {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
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
