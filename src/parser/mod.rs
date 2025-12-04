pub mod error;
pub mod minimal;
pub mod simple;
pub mod recursive;
pub mod tests;

pub use error::*;

use crate::Token;
use crate::ast::unresolved::UnresolvedAst;
use crate::ident::IdentArena;
use chumsky::{Parser, error::Simple};

pub fn parse(tokens: Vec<Token>, _idents: &IdentArena) -> (Option<UnresolvedAst>, Vec<ParseError>) {
    let token_stream: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();

    match recursive::recursive_parser().parse(token_stream) {
        Ok(ast) => (Some(ast), Vec::new()),
        Err(errors) => {
            let parse_errors = errors.into_iter().map(convert_simple_error).collect();
            (None, parse_errors)
        }
    }
}

fn convert_simple_error(error: Simple<crate::ProcessedTokenKind>) -> ParseError {
    use crate::span::Span;

    let span = Span::new(error.span().start, error.span().end);

    match error.reason() {
        chumsky::error::SimpleReason::Unexpected => {
            let found = error
                .found()
                .map(|t| format!("{:?}", t))
                .unwrap_or_else(|| "end of input".to_string());
            let expected = if error.expected().len() == 0 {
                "something else".to_string()
            } else {
                error
                    .expected()
                    .map(|e| match e {
                        Some(token) => format!("{:?}", token),
                        None => "end of input".to_string(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            ParseError::UnexpectedToken {
                expected,
                found,
                span,
            }
        }
        chumsky::error::SimpleReason::Unclosed { .. } => ParseError::UnterminatedDelimiter {
            delimiter: "delimiter".to_string(),
            span,
        },
        chumsky::error::SimpleReason::Custom(msg) => ParseError::InvalidSyntax {
            message: msg.clone(),
            span,
        },
    }
}
