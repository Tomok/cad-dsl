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

/// Parse an integer literal token
pub fn int_lit<'src>() -> impl Parser<'src, &'src [Token<'src>], i32, ParseError<'src>> + Clone {
    select! {
        Token::IntLiteral(t) => t.value,
    }
    .labelled("integer")
}

/// Parse a float literal token
pub fn float_lit<'src>() -> impl Parser<'src, &'src [Token<'src>], f64, ParseError<'src>> + Clone {
    select! {
        Token::FloatLiteral(t) => t.value,
    }
    .labelled("float")
}

/// Parse a variable name (identifier token)
pub fn var<'src>() -> impl Parser<'src, &'src [Token<'src>], String, ParseError<'src>> + Clone {
    select! {
        Token::Identifier(t) => t.name.to_string(),
    }
    .labelled("variable")
}

/// Parse a boolean literal token
pub fn bool_lit<'src>() -> impl Parser<'src, &'src [Token<'src>], bool, ParseError<'src>> + Clone {
    select! {
        Token::True(_) => true,
        Token::False(_) => false,
    }
    .labelled("boolean")
}

/// Parse an atomic expression (Atom enum)
pub fn atom<'src>() -> impl Parser<'src, &'src [Token<'src>], Atom, ParseError<'src>> + Clone {
    choice((
        // Try float first (it's more specific)
        float_lit().map(Atom::FloatLit),
        // Then integer
        int_lit().map(Atom::IntLit),
        // Then boolean
        bool_lit().map(Atom::BoolLit),
        // Finally variable
        var().map(Atom::Var),
    ))
    .labelled("atom")
}
