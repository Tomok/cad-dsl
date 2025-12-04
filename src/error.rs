use crate::span::Span;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LexError {
    #[error("Invalid character: '{0}'")]
    InvalidChar(char),

    #[error("Malformed number literal")]
    MalformedNumber,

    #[error("Unterminated string")]
    UnterminatedString,

    #[error("Invalid token")]
    InvalidToken,
}

#[derive(Debug)]
pub struct LexErrorWithSpan {
    pub error: LexError,
    pub span: Span,
}

impl LexErrorWithSpan {
    pub fn new(error: LexError, span: Span) -> Self {
        Self { error, span }
    }
}
