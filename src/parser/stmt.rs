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
    choice((
        select! { Token::BoolType(_) => () }.map_with_span(|_, span| Type::Bool { span }),
        select! { Token::I32Type(_) => () }.map_with_span(|_, span| Type::I32 { span }),
        select! { Token::F64Type(_) => () }.map_with_span(|_, span| Type::F64 { span }),
        select! { Token::RealType(_) => () }.map_with_span(|_, span| Type::Real { span }),
        select! { Token::AlgebraicType(_) => () }.map_with_span(|_, span| Type::Algebraic { span }),
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
    let let_kw = select! { Token::Let(_) => () };
    let colon = select! { Token::Colon(_) => () };
    let equals = select! { Token::Equals(_) => () };
    let semicolon = select! { Token::SemiColon(_) => () };

    let_kw
        .ignore_then(
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
        .then_ignore(semicolon)
        .map_with_span(
            |(((name, name_span), type_annotation), init), span| Stmt::Let {
                name,
                name_span,
                type_annotation,
                init,
                span,
            },
        )
        .labelled("let statement")
}
