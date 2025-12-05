pub mod addition;
pub mod comparison;
pub mod error;
pub mod expression;
pub mod logical_and;
pub mod logical_or;
pub mod multiplication;
pub mod power;
pub mod primary;
pub mod recursive;
pub mod tests;
pub mod unary;
pub mod unified;

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
    use crate::ProcessedTokenKind;
    use crate::span::Span;

    let span = Span::new(error.span().start, error.span().end);

    match error.reason() {
        chumsky::error::SimpleReason::Unexpected => {
            let found = error
                .found()
                .map(|t| format!("{:?}", t))
                .unwrap_or_else(|| "end of input".to_string());

            let expected_tokens = error.expected().collect::<Vec<_>>();

            // Check for specific error patterns and provide better messages

            // Missing semicolon pattern
            if expected_tokens
                .iter()
                .any(|e| **e == Some(ProcessedTokenKind::Semicolon))
            {
                if found != "Semicolon" {
                    return ParseError::MissingSemicolon { span };
                }
            }

            // Left-associativity chain pattern - detect when we have operators that could form chains
            if let Some(found_token) = error.found() {
                match found_token {
                    ProcessedTokenKind::Star
                    | ProcessedTokenKind::Slash
                    | ProcessedTokenKind::Percent => {
                        // This might be a multiplication chain that's not implemented yet
                        if expected_tokens.iter().any(|e| {
                            matches!(
                                *e,
                                Some(ProcessedTokenKind::Semicolon)
                                    | Some(ProcessedTokenKind::RBrace)
                            )
                        }) {
                            return ParseError::LeftAssociativityNotImplemented {
                                operation: "multiplication/division chain".to_string(),
                                span,
                            };
                        }
                    }
                    ProcessedTokenKind::Plus | ProcessedTokenKind::Minus => {
                        // This might be an addition chain
                        if expected_tokens.iter().any(|e| {
                            matches!(
                                *e,
                                Some(ProcessedTokenKind::Semicolon)
                                    | Some(ProcessedTokenKind::RBrace)
                            )
                        }) {
                            // Only suggest this if it's likely a chain, not just invalid syntax
                            return ParseError::LeftAssociativityNotImplemented {
                                operation: "addition/subtraction chain".to_string(),
                                span,
                            };
                        }
                    }
                    ProcessedTokenKind::Or => {
                        return ParseError::LeftAssociativityNotImplemented {
                            operation: "logical OR chain".to_string(),
                            span,
                        };
                    }
                    _ => {}
                }
            }

            let expected = if expected_tokens.len() == 0 {
                "something else".to_string()
            } else {
                expected_tokens
                    .iter()
                    .map(|e| match e {
                        Some(token) => {
                            // Provide more user-friendly names for common tokens
                            match token {
                                ProcessedTokenKind::Semicolon => "semicolon (;)".to_string(),
                                ProcessedTokenKind::LBrace => "opening brace ({)".to_string(),
                                ProcessedTokenKind::RBrace => "closing brace (})".to_string(),
                                ProcessedTokenKind::LParen => "opening parenthesis (()".to_string(),
                                ProcessedTokenKind::RParen => "closing parenthesis ())".to_string(),
                                ProcessedTokenKind::LBracket => "opening bracket ([)".to_string(),
                                ProcessedTokenKind::RBracket => "closing bracket (])".to_string(),
                                ProcessedTokenKind::Assign => "assignment (=)".to_string(),
                                ProcessedTokenKind::Colon => "colon (:)".to_string(),
                                _ => format!("{:?}", token),
                            }
                        }
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
        chumsky::error::SimpleReason::Unclosed { delimiter, .. } => {
            let delimiter_name = match delimiter {
                ProcessedTokenKind::LBrace => "brace".to_string(),
                ProcessedTokenKind::LParen => "parenthesis".to_string(),
                ProcessedTokenKind::LBracket => "bracket".to_string(),
                _ => "delimiter".to_string(),
            };
            ParseError::UnterminatedDelimiter {
                delimiter: delimiter_name,
                span,
            }
        }
        chumsky::error::SimpleReason::Custom(msg) => ParseError::InvalidSyntax {
            message: msg.clone(),
            span,
        },
    }
}
