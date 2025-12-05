use crate::ProcessedTokenKind;
use crate::span::Span;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("Unexpected token: expected {expected}, found {found}")]
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Missing {token}")]
    MissingToken { token: String, span: Span },

    #[error("Invalid expression")]
    InvalidExpression { span: Span },

    #[error("Unterminated delimiter")]
    UnterminatedDelimiter { delimiter: String, span: Span },

    #[error("Invalid syntax")]
    InvalidSyntax { message: String, span: Span },

    #[error("Missing semicolon after statement")]
    MissingSemicolon { span: Span },

    #[error("Left-associativity chain not yet implemented: {operation}")]
    LeftAssociativityNotImplemented { operation: String, span: Span },

    #[error("Expression chain requires parentheses for clarity")]
    ExpressionChainAmbiguous { span: Span },

    #[error("Operator precedence violation")]
    PrecedenceViolation { span: Span },
}

impl ParseError {
    pub fn span(&self) -> Span {
        match self {
            ParseError::UnexpectedToken { span, .. } => *span,
            ParseError::MissingToken { span, .. } => *span,
            ParseError::InvalidExpression { span } => *span,
            ParseError::UnterminatedDelimiter { span, .. } => *span,
            ParseError::InvalidSyntax { span, .. } => *span,
            ParseError::MissingSemicolon { span } => *span,
            ParseError::LeftAssociativityNotImplemented { span, .. } => *span,
            ParseError::ExpressionChainAmbiguous { span } => *span,
            ParseError::PrecedenceViolation { span } => *span,
        }
    }

    pub fn unexpected_token(
        expected: impl Into<String>,
        found: ProcessedTokenKind,
        span: Span,
    ) -> Self {
        ParseError::UnexpectedToken {
            expected: expected.into(),
            found: found.to_string(),
            span,
        }
    }

    pub fn missing_token(token: impl Into<String>, span: Span) -> Self {
        ParseError::MissingToken {
            token: token.into(),
            span,
        }
    }
}
