//! Atomic parsers for primitive values
//!
//! This module contains parsers for the most basic language elements:
//! - Integer literals
//! - Float literals
//! - Boolean literals
//! - Variable identifiers
//! - Function calls
//! - Method calls
//! - Atomic expressions (combination of all primitives)

use crate::ast::{Atom, Expr};
use crate::lexer::{Span, Token};
use chumsky::prelude::*;

use super::ParseError;

// ============================================================================
// Atomic Parsers (with optional recursion for function calls)
// ============================================================================

/// Parse an atomic expression (Atom enum)
/// Takes an expression parser for parsing function call arguments
pub fn atom<'src>(
    expr: impl Parser<'src, &'src [Token<'src>], Expr<'src>, ParseError<'src>> + Clone,
) -> impl Parser<'src, &'src [Token<'src>], Atom<'src>, ParseError<'src>> + Clone {
    // First, parse a base atom (literal, variable, or function call)
    let base_atom = choice((
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
            Token::True(t) => Atom::BoolLit { value: true, span: Span { start: t.position, lines: 0, end_column: t.position.column + 4 } },
            Token::False(t) => Atom::BoolLit { value: false, span: Span { start: t.position, lines: 0, end_column: t.position.column + 5 } },
        },
        // Array literal: [elem1, elem2, ...]
        expr.clone()
            .separated_by(select! { Token::Comma(_) => () })
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(
                select! { Token::LeftBracket(_) => () },
                select! { Token::RightBracket(_) => () },
            )
            .map_with(|elements, e| {
                let span_range = e.span();
                Atom::ArrayLit {
                    elements,
                    span: Span {
                        start: crate::lexer::LineColumn {
                            line: 1,
                            column: span_range.start + 1,
                        },
                        lines: 0,
                        end_column: span_range.end + 1,
                    },
                }
            }),
        // Struct literal: StructName { field1: value1, field2: value2, ... }
        select! {
            Token::Identifier(t) => (t.name, t.span),
        }
        .then(
            select! { Token::Identifier(t) => t.name }
                .then_ignore(select! { Token::Colon(_) => () })
                .then(expr.clone())
                .separated_by(select! { Token::Comma(_) => () })
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(
                    select! { Token::LeftBrace(_) => () },
                    select! { Token::RightBrace(_) => () },
                ),
        )
        .map_with(|((name, name_span), fields), e| {
            let span_range = e.span();
            Atom::StructLit {
                name,
                fields,
                span: Span {
                    start: name_span.start,
                    lines: 0,
                    end_column: span_range.end + 1,
                },
            }
        }),
        // Function call: identifier followed by parentheses with comma-separated arguments
        select! {
            Token::Identifier(t) => (t.name, t.span),
        }
        .then(
            expr.clone()
                .separated_by(select! { Token::Comma(_) => () })
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(
                    select! { Token::LeftParen(t) => t.position },
                    select! { Token::RightParen(t) => t.position },
                )
                .map_with(|args, e| {
                    let span_range = e.span();
                    (args, span_range)
                }),
        )
        .map(|((name, name_span), (args, call_span))| {
            // Combine name span and call span for full function call span
            Atom::Call {
                name,
                args,
                span: Span {
                    start: name_span.start,
                    lines: 0, // Assuming single line for now
                    end_column: name_span.end_column + (call_span.end - call_span.start),
                },
            }
        }),
        // Finally plain variable (no function call)
        select! {
            Token::Identifier(t) => Atom::Var { name: t.name, span: t.span },
        },
    ));

    // Then parse zero or more method calls or field accesses as postfix operations
    // Method call: .identifier(args)
    // Field access: .identifier (without parentheses)
    let method_or_field_suffix = select! { Token::Dot(_) => () }
        .ignore_then(select! {
            Token::Identifier(t) => (t.name, t.span),
        })
        .then(
            expr.clone()
                .separated_by(select! { Token::Comma(_) => () })
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(
                    select! { Token::LeftParen(_) => () },
                    select! { Token::RightParen(t) => t.position },
                )
                .map_with(|args, e| {
                    let span_range = e.span();
                    (args, span_range)
                })
                .or_not(),
        );

    // Combine base atom with repeated method calls or field accesses
    base_atom
        .then(method_or_field_suffix.repeated().collect::<Vec<_>>())
        .map(|(mut atom, suffixes)| {
            // Apply each suffix (method call or field access) in sequence
            for ((name, name_span), args_and_span) in suffixes {
                let start = match &atom {
                    Atom::Var { span, .. } => span.start,
                    Atom::IntLit { span, .. } => span.start,
                    Atom::FloatLit { span, .. } => span.start,
                    Atom::BoolLit { span, .. } => span.start,
                    Atom::Call { span, .. } => span.start,
                    Atom::MethodCall { span, .. } => span.start,
                    Atom::FieldAccess { span, .. } => span.start,
                    Atom::ArrayLit { span, .. } => span.start,
                    Atom::StructLit { span, .. } => span.start,
                };

                atom = match args_and_span {
                    // Method call: has arguments
                    Some((args, call_span)) => Atom::MethodCall {
                        receiver: Box::new(atom.into()),
                        method: name,
                        args,
                        span: Span {
                            start,
                            lines: 0,
                            end_column: call_span.end,
                        },
                    },
                    // Field access: no arguments
                    None => Atom::FieldAccess {
                        receiver: Box::new(atom.into()),
                        field: name,
                        span: Span {
                            start,
                            lines: 0,
                            end_column: name_span.end_column,
                        },
                    },
                };
            }
            atom
        })
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

/// Parse a variable identifier (extracts just the &str)
#[cfg(test)]
pub fn var<'src>() -> impl Parser<'src, &'src [Token<'src>], &'src str, ParseError<'src>> + Clone {
    select! {
        Token::Identifier(t) => t.name,
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
