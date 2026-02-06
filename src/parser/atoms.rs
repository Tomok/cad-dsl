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

use crate::ast::{Atom, Expr, RuneBlock, RuneParam};
use crate::lexer::{Span, Token};
use chumsky::prelude::*;

use super::ParseError;

// ============================================================================
// Helper Types
// ============================================================================

/// Represents a postfix operation (method call, field access, or indexing)
#[derive(Debug, Clone)]
enum PostfixOp<'src> {
    MethodOrField((&'src str, Span), Option<(Vec<Expr<'src>>, SimpleSpan)>),
    Index(Expr<'src>, SimpleSpan),
}

// ============================================================================
// Rune Block Parser
// ============================================================================

/// Parse a rune block: rune(params) { body }
fn rune_block<'src>(
    expr: impl Parser<'src, &'src [Token<'src>], Expr<'src>, ParseError<'src>> + Clone,
    source: Option<&'src str>,
) -> impl Parser<'src, &'src [Token<'src>], Atom<'src>, ParseError<'src>> + Clone {
    // Parse the rune keyword
    select! { Token::Rune(t) => t.position }
        .then(
            // Parse parameters: (x) or (x=expr, y, z=100)
            {
                let param = select! { Token::Identifier(t) => (t.name, t.span) }
                    .then(
                        select! { Token::Equals(_) => () }
                            .ignore_then(expr.clone())
                            .or_not(),
                    )
                    .map(|((name, name_span), value)| RuneParam {
                        name,
                        value,
                        span: name_span,
                    });

                param
                    .separated_by(select! { Token::Comma(_) => () })
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(
                        select! { Token::LeftParen(_) => () },
                        select! { Token::RightParen(_) => () },
                    )
            },
        )
        .then(
            // Parse the body with bracket counting
            rune_body(),
        )
        .map_with(
            move |((rune_pos, params), (placeholder_body, body_span)), e| {
                let span_range = e.span();

                // Extract actual body text from source if available
                let body = if let Some(src) = source {
                    let full_body = extract_source_from_span(src, &body_span);
                    // Trim the surrounding braces { and }
                    full_body
                        .trim()
                        .strip_prefix('{')
                        .unwrap_or(full_body)
                        .trim()
                        .strip_suffix('}')
                        .unwrap_or(full_body)
                        .trim()
                } else {
                    placeholder_body // Fall back to placeholder if source not available
                };

                Atom::RuneBlock(Box::new(RuneBlock {
                    params,
                    body,
                    span: Span {
                        start: rune_pos,
                        lines: 0,
                        end_column: span_range.end + 1,
                    },
                }))
            },
        )
}

/// Parse the body of a rune block with bracket counting
/// Returns a placeholder body string and the span from opening to closing brace
/// The actual body text will be extracted during semantic analysis using the span
fn rune_body<'src>()
-> impl Parser<'src, &'src [Token<'src>], (&'src str, Span), ParseError<'src>> + Clone {
    // Implementation with bracket counting as required by Phase 1.4
    // Strategy:
    // 1. Parse opening {
    // 2. Recursively parse body content (handling nested braces)
    // 3. Parse closing } and store span

    // Parse body tokens: either nested braces or any other token
    let body_token = recursive(|body_content| {
        choice((
            // Nested brace block: { ... }
            select! { Token::LeftBrace(_) => () }
                .ignore_then(body_content.clone().repeated())
                .then_ignore(select! { Token::RightBrace(_) => () })
                .ignored(),
            // Any token except  braces
            any()
                .try_map(|token, span| match token {
                    Token::LeftBrace(_) | Token::RightBrace(_) => {
                        Err(Rich::custom(span, "Unexpected brace"))
                    }
                    _ => Ok(()),
                })
                .ignored(),
        ))
    });

    select! { Token::LeftBrace(t) => t.position }
        .then(body_token.repeated())
        .then(select! { Token::RightBrace(t) => t.position })
        .map(|((open_pos, _body_tokens), close_pos)| {
            let body_span = Span {
                start: open_pos,
                lines: 0,
                end_column: close_pos.column + 1,
            };
            // Placeholder - actual body will be extracted during semantic analysis
            ("", body_span)
        })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract source text from a span by converting line/column positions to byte offsets
fn extract_source_from_span<'src>(source: &'src str, span: &Span) -> &'src str {
    // Convert line/column to byte offset
    let mut byte_offset = 0;
    let mut current_line = 1;
    let mut current_column = 1;

    // Find start position
    let start_line = span.start.line;
    let start_column = span.start.column;
    let end_column = span.end_column;

    let chars: Vec<char> = source.chars().collect();
    let mut start_byte = 0;
    let mut end_byte = 0;

    for &ch in chars.iter() {
        // Track position
        if current_line == start_line && current_column == start_column {
            start_byte = byte_offset;
        }

        // For single-line spans, end is on same line
        if current_line == start_line && current_column == end_column {
            end_byte = byte_offset;
            break;
        }

        // Advance position
        if ch == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }

        byte_offset += ch.len_utf8();
    }

    // Handle end of source
    if end_byte == 0 {
        end_byte = source.len();
    }

    &source[start_byte..end_byte]
}

// ============================================================================
// Atomic Parsers (with optional recursion for function calls)
// ============================================================================

/// Parse an atomic expression (Atom enum)
/// Takes an expression parser for parsing function call arguments
/// Takes optional source text for extracting rune block bodies
#[allow(dead_code)] // Public API, may be used by external parsers
pub fn atom<'src>(
    expr: impl Parser<'src, &'src [Token<'src>], Expr<'src>, ParseError<'src>> + Clone,
) -> impl Parser<'src, &'src [Token<'src>], Atom<'src>, ParseError<'src>> + Clone {
    atom_with_source(expr, None)
}

/// Parse an atomic expression with source text access for rune block body extraction
pub fn atom_with_source<'src>(
    expr: impl Parser<'src, &'src [Token<'src>], Expr<'src>, ParseError<'src>> + Clone,
    source: Option<&'src str>,
) -> impl Parser<'src, &'src [Token<'src>], Atom<'src>, ParseError<'src>> + Clone {
    // First, parse a base atom (literal, variable, or function call)
    let base_atom = choice((
        // Rune block: rune(params) { body }
        rune_block(expr.clone(), source),
        // Dot-prefixed field access: .identifier(.identifier)*
        // For container field access in with blocks (e.g., .field or .field.x)
        select! { Token::Dot(t) => t.position }
            .then(
                select! { Token::Identifier(t) => t.name }
                    .separated_by(select! { Token::Dot(_) => () })
                    .at_least(1)
                    .collect::<Vec<_>>(),
            )
            .map(|(dot_pos, field_path)| {
                let last_field_len = field_path.last().map_or(0, |f| f.len());
                let span = Span {
                    start: dot_pos,
                    lines: 0,
                    end_column: dot_pos.column
                        + 1
                        + field_path
                            .iter()
                            .take(field_path.len() - 1)
                            .map(|f| f.len() + 1) // field + dot
                            .sum::<usize>()
                        + last_field_len,
                };
                Atom::ContainerFieldAccess { field_path, span }
            }),
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
        // Closure: |param1, param2| expr
        select! { Token::Pipe(t) => t.position }
            .then(
                select! { Token::Identifier(t) => t.name }
                    .separated_by(select! { Token::Comma(_) => () })
                    .allow_trailing()
                    .collect::<Vec<_>>(),
            )
            .then_ignore(select! { Token::Pipe(_) => () })
            .then(expr.clone())
            .map_with(|((start_pos, params), body), e| {
                let span_range = e.span();
                Atom::Closure {
                    params,
                    body: Box::new(body),
                    span: Span {
                        start: start_pos,
                        lines: 0,
                        end_column: span_range.end,
                    },
                }
            }),
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
        // Struct literal: StructName { field1: value1, method() = value2, ... }
        select! {
            Token::Identifier(t) => (t.name, t.span),
        }
        .then({
            // Field can be:
            // 1. Regular field: identifier : expr
            // 2. Computed property: identifier () = expr
            let field_parser = select! { Token::Identifier(t) => (t.name, t.span) }
                .then(choice((
                    // Computed property: identifier() = expr
                    select! { Token::LeftParen(_) => () }
                        .ignore_then(select! { Token::RightParen(_) => () })
                        .ignore_then(select! { Token::Equals(_) => () })
                        .ignore_then(expr.clone())
                        .map(|value| (true, value)),
                    // Regular field: identifier: expr
                    select! { Token::Colon(_) => () }
                        .ignore_then(expr.clone())
                        .map(|value| (false, value)),
                )))
                .map_with(|((name, name_span), (is_computed, value)), e| {
                    use crate::ast::StructLitField;
                    let span_range = e.span();
                    if is_computed {
                        StructLitField::ComputedProperty {
                            name,
                            value,
                            span: Span {
                                start: name_span.start,
                                lines: 0,
                                end_column: span_range.end,
                            },
                        }
                    } else {
                        StructLitField::Field {
                            name,
                            value,
                            span: Span {
                                start: name_span.start,
                                lines: 0,
                                end_column: span_range.end,
                            },
                        }
                    }
                });

            field_parser
                .separated_by(select! { Token::Comma(_) => () })
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(
                    select! { Token::LeftBrace(_) => () },
                    select! { Token::RightBrace(_) => () },
                )
        })
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
        // self keyword
        select! {
            Token::SelfKw(t) => Atom::Var { name: "self", span: Span { start: t.position, lines: 0, end_column: t.position.column + 4 } },
        },
        // Finally plain variable (no function call)
        select! {
            Token::Identifier(t) => Atom::Var { name: t.name, span: t.span },
        },
    ));

    // Then parse zero or more method calls, field accesses, or array indexing as postfix operations
    // Method call: .identifier(args)
    // Field access: .identifier (without parentheses)
    // Array indexing: [expr]

    // Method/field suffix: .identifier with optional args
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
        )
        .map(|(name_span, args_and_span)| PostfixOp::MethodOrField(name_span, args_and_span));

    // Index suffix: [expr]
    let index_suffix = expr
        .clone()
        .delimited_by(
            select! { Token::LeftBracket(_) => () },
            select! { Token::RightBracket(t) => t.position },
        )
        .map_with(|index, e| {
            let span_range = e.span();
            PostfixOp::Index(index, span_range)
        });

    // Combine both postfix operations
    let postfix_op = choice((method_or_field_suffix, index_suffix));

    // Combine base atom with repeated postfix operations
    base_atom
        .then(postfix_op.repeated().collect::<Vec<_>>())
        .map(|(mut atom, suffixes)| {
            // Apply each postfix operation in sequence
            for suffix in suffixes {
                let start = match &atom {
                    Atom::Var { span, .. } => span.start,
                    Atom::IntLit { span, .. } => span.start,
                    Atom::FloatLit { span, .. } => span.start,
                    Atom::BoolLit { span, .. } => span.start,
                    Atom::Call { span, .. } => span.start,
                    Atom::MethodCall { span, .. } => span.start,
                    Atom::FieldAccess { span, .. } => span.start,
                    Atom::ContainerFieldAccess { span, .. } => span.start,
                    Atom::ArrayLit { span, .. } => span.start,
                    Atom::StructLit { span, .. } => span.start,
                    Atom::Index { span, .. } => span.start,
                    Atom::Range { span, .. } => span.start,

                    Atom::Closure { span, .. } => span.start,
                    Atom::RuneBlock(block) => block.span.start,
                };

                atom = match suffix {
                    PostfixOp::MethodOrField((name, name_span), args_and_span) => {
                        match args_and_span {
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
                        }
                    }
                    PostfixOp::Index(index_expr, index_span) => Atom::Index {
                        array: Box::new(atom.into()),
                        index: Box::new(index_expr),
                        span: Span {
                            start,
                            lines: 0,
                            end_column: index_span.end,
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
