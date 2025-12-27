//! Statement and type annotation parsers

use crate::ast::{Stmt, Type};
use crate::lexer::Token;
use crate::parser::ParseError;
use chumsky::prelude::*;

// ============================================================================
// Type Annotation Parser
// ============================================================================

/// Parse type annotations (bool, i32, f64, Real, Algebraic)
pub fn type_annotation<'src>()
-> impl Parser<'src, &'src [Token<'src>], Type, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    choice((
        select! {
            Token::BoolType(t) => Type::Bool {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 4 }
            },
        },
        select! {
            Token::I32Type(t) => Type::I32 {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 3 }
            },
        },
        select! {
            Token::F64Type(t) => Type::F64 {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 3 }
            },
        },
        select! {
            Token::RealType(t) => Type::Real {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 4 }
            },
        },
        select! {
            Token::AlgebraicType(t) => Type::Algebraic {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 9 }
            },
        },
    ))
    .labelled("type annotation")
}

// ============================================================================
// Statement Parser
// ============================================================================

/// Parse a let statement
///
/// Syntax:
///   let <name>: <type> = <expr>;
///   let <name>: <type>;
///   let <name> = <expr>;
///   let <name>;
pub fn let_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr, ParseError<'src>> + Clone,
) -> impl Parser<'src, &'src [Token<'src>], Stmt, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let colon = select! { Token::Colon(_) => () };
    let equals = select! { Token::Equals(_) => () };

    select! {
        Token::Let(t) => t.position,
    }
    .then(
        select! {
            Token::Identifier(t) => (t.name.to_string(), t.span),
        }
        .labelled("variable name"),
    )
    .then(
        // Optional type annotation: : <type>
        colon.ignore_then(type_annotation()).or_not(),
    )
    .then(
        // Optional initialization: = <expr>
        equals.ignore_then(expr_parser).or_not(),
    )
    .then(select! {
        Token::SemiColon(t) => t.position,
    })
    .map(
        |((((let_pos, (name, name_span)), type_annotation), init), semi_pos)| {
            // Construct span from let keyword to semicolon
            let span = if let_pos.line == semi_pos.line {
                // Same line
                Span {
                    start: let_pos,
                    lines: 0,
                    end_column: semi_pos.column + 1,
                }
            } else {
                // Multiple lines
                Span {
                    start: let_pos,
                    lines: semi_pos.line - let_pos.line,
                    end_column: semi_pos.column + 1,
                }
            };

            Stmt::Let {
                name,
                name_span,
                type_annotation,
                init,
                span,
            }
        },
    )
    .labelled("let statement")
}
