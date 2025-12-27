//! Statement and type annotation parsers

use crate::ast::{Stmt, Type};
use crate::lexer::Token;
use crate::parser::ParseError;
use chumsky::prelude::*;

// ============================================================================
// Type Annotation Parser
// ============================================================================

/// Parse type annotations (bool, i32, f64, Real, Algebraic)
#[cfg_attr(not(test), allow(dead_code))] // Used internally by let_stmt, imported in tests
pub fn type_annotation<'src>()
-> impl Parser<'src, &'src [Token<'src>], Type, ParseError<'src>> + Clone {
    choice((
        select! { Token::BoolType(_) => Type::Bool },
        select! { Token::I32Type(_) => Type::I32 },
        select! { Token::F64Type(_) => Type::F64 },
        select! { Token::RealType(_) => Type::Real },
        select! { Token::AlgebraicType(_) => Type::Algebraic },
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
                Token::Identifier(t) => t.name.to_string(),
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
        .map(|((name, type_annotation), init)| Stmt::Let {
            name,
            type_annotation,
            init,
        })
        .labelled("let statement")
}
