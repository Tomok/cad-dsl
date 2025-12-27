//! Atomic parsers for primitive values
//!
//! This module contains parsers for the most basic language elements:
//! - Integer literals
//! - Float literals
//! - Boolean literals
//! - Variable identifiers
//! - Atomic expressions (combination of all primitives)

use crate::ast::Atom;
use crate::lexer::Token;
use chumsky::prelude::*;

use super::ParseError;

// ============================================================================
// Atomic Parsers (No recursion)
// ============================================================================

/// Parse an atomic expression (Atom enum)
pub fn atom<'src>() -> impl Parser<'src, &'src [Token<'src>], Atom, ParseError<'src>> + Clone {
    choice((
        // Try float first (it's more specific)
        select! {
            Token::FloatLiteral(t) => Atom::FloatLit { value: t.value, span: t.span },
        },
        // Then integer
        select! {
            Token::IntLiteral(t) => Atom::IntLit { value: t.value, span: t.span },
        },
        // Then boolean
        select! {
            Token::True(t) => Atom::BoolLit { value: true, span: crate::lexer::Span { start: t.position, lines: 0, end_column: t.position.column + 4 } },
            Token::False(t) => Atom::BoolLit { value: false, span: crate::lexer::Span { start: t.position, lines: 0, end_column: t.position.column + 5 } },
        },
        // Finally variable
        select! {
            Token::Identifier(t) => Atom::Var { name: t.name.to_string(), span: t.span },
        },
    ))
    .labelled("atom")
}

// ============================================================================
// Value-only parsers (for testing)
// ============================================================================

/// Parse an integer literal value (extracts just the i32)
#[cfg(test)]
pub fn int_lit<'src>() -> impl Parser<'src, &'src [Token<'src>], i32, ParseError<'src>> + Clone {
    select! {
        Token::IntLiteral(t) => t.value,
    }
    .labelled("integer literal")
}

/// Parse a float literal value (extracts just the f64)
#[cfg(test)]
pub fn float_lit<'src>() -> impl Parser<'src, &'src [Token<'src>], f64, ParseError<'src>> + Clone {
    select! {
        Token::FloatLiteral(t) => t.value,
    }
    .labelled("float literal")
}

/// Parse a variable identifier (extracts just the String)
#[cfg(test)]
pub fn var<'src>() -> impl Parser<'src, &'src [Token<'src>], String, ParseError<'src>> + Clone {
    select! {
        Token::Identifier(t) => t.name.to_string(),
    }
    .labelled("variable")
}

/// Parse a boolean literal value (extracts just the bool)
#[cfg(test)]
pub fn bool_lit<'src>() -> impl Parser<'src, &'src [Token<'src>], bool, ParseError<'src>> + Clone {
    select! {
        Token::True(_) => true,
        Token::False(_) => false,
    }
    .labelled("boolean literal")
}
