use std::str::FromStr;

use logos::Lexer;
use logos::Logos;
use logos::Skip;

// ============================================================================
// Position and Span Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: LineColumn,
    pub lines: usize,
    pub end_column: usize,
}

pub struct NewLineTracer {
    pub line: usize,
    pub last_newline_char_index: usize,
}

impl NewLineTracer {
    fn new() -> Self {
        Self {
            line: 1,
            last_newline_char_index: 0,
        }
    }
}

impl Default for NewLineTracer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Token Trait
// ============================================================================

/// Common trait for all token types
pub trait TokenTrait {
    /// Get the position of this token
    fn position(&self) -> LineColumn;

    /// Get the string representation of this token's value
    /// For fixed tokens (keywords, operators), this is the literal text
    /// For dynamic tokens (identifiers, literals), this is the parsed value
    fn value_str(&self) -> &str;
}

// ============================================================================
// Macro for Fixed-Content Tokens
// ============================================================================

/// Macro to generate token structs for fixed-content tokens
/// (keywords, operators, delimiters, punctuation)
macro_rules! fixed_token {
    ($name:ident, $text:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name {
            pub position: LineColumn,
        }

        impl $name {
            pub fn new(position: LineColumn) -> Self {
                Self { position }
            }
        }

        impl TokenTrait for $name {
            fn position(&self) -> LineColumn {
                self.position
            }

            fn value_str(&self) -> &str {
                $text
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", $text)
            }
        }
    };
}

// ============================================================================
// Keyword Tokens
// ============================================================================

fixed_token!(TokenStruct, "struct");
fixed_token!(TokenContainer, "container");
fixed_token!(TokenFn, "fn");
fixed_token!(TokenLet, "let");
fixed_token!(TokenFor, "for");
fixed_token!(TokenIn, "in");
fixed_token!(TokenWith, "with");
fixed_token!(TokenIf, "if");
fixed_token!(TokenElse, "else");
fixed_token!(TokenOr, "or");
fixed_token!(TokenAnd, "and");
fixed_token!(TokenReturn, "return");
fixed_token!(TokenTrue, "true");
fixed_token!(TokenFalse, "false");
fixed_token!(TokenSelf, "self");

// ============================================================================
// Operator Tokens
// ============================================================================

fixed_token!(TokenEquals, "=");
fixed_token!(TokenEqualsEquals, "==");
fixed_token!(TokenNotEquals, "!=");
fixed_token!(TokenLessThan, "<");
fixed_token!(TokenGreaterThan, ">");
fixed_token!(TokenLessEquals, "<=");
fixed_token!(TokenGreaterEquals, ">=");
fixed_token!(TokenPlus, "+");
fixed_token!(TokenMinus, "-");
fixed_token!(TokenMultiply, "*");
fixed_token!(TokenDivide, "/");
fixed_token!(TokenPower, "^");
fixed_token!(TokenModulo, "%");
fixed_token!(TokenAmpersand, "&");

// ============================================================================
// Punctuation Tokens
// ============================================================================

fixed_token!(TokenColon, ":");
fixed_token!(TokenSemiColon, ";");
fixed_token!(TokenComma, ",");
fixed_token!(TokenDot, ".");
fixed_token!(TokenDotDot, "..");
fixed_token!(TokenLeftParen, "(");
fixed_token!(TokenRightParen, ")");
fixed_token!(TokenLeftBracket, "[");
fixed_token!(TokenRightBracket, "]");
fixed_token!(TokenLeftBrace, "{");
fixed_token!(TokenRightBrace, "}");
fixed_token!(TokenPipe, "|");
fixed_token!(TokenArrow, "->");

// ============================================================================
// Built-in Type Tokens
// ============================================================================

fixed_token!(TokenBoolType, "bool");
fixed_token!(TokenI32Type, "i32");
fixed_token!(TokenF64Type, "f64");
fixed_token!(TokenRealType, "Real");
fixed_token!(TokenAlgebraicType, "Algebraic");

// ============================================================================
// Dynamic Content Tokens (Literals and Identifiers)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct TokenFloatLiteral {
    pub value: f64,
    pub span: Span,
}

impl TokenFloatLiteral {
    pub fn new(value: f64, span: Span) -> Self {
        Self { value, span }
    }
}

impl TokenTrait for TokenFloatLiteral {
    fn position(&self) -> LineColumn {
        self.span.start
    }

    fn value_str(&self) -> &str {
        // For numeric literals, we return a string representation
        // Note: This creates a string on the stack, so for display purposes
        // the Display trait is more appropriate
        "float_literal"
    }
}

impl std::fmt::Display for TokenFloatLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenIntLiteral {
    pub value: i32,
    pub span: Span,
}

impl TokenIntLiteral {
    pub fn new(value: i32, span: Span) -> Self {
        Self { value, span }
    }
}

impl TokenTrait for TokenIntLiteral {
    fn position(&self) -> LineColumn {
        self.span.start
    }

    fn value_str(&self) -> &str {
        "int_literal"
    }
}

impl std::fmt::Display for TokenIntLiteral {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenIdentifier<'src> {
    pub name: &'src str,
    pub span: Span,
}

impl<'src> TokenIdentifier<'src> {
    pub fn new(name: &'src str, span: Span) -> Self {
        Self { name, span }
    }
}

impl<'src> TokenTrait for TokenIdentifier<'src> {
    fn position(&self) -> LineColumn {
        self.span.start
    }

    fn value_str(&self) -> &str {
        self.name
    }
}

impl<'src> std::fmt::Display for TokenIdentifier<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

// ============================================================================
// Wrapper Enum for All Token Types
// ============================================================================

/// Wrapper enum that can hold any token type for easier handling
#[derive(Debug, Clone, PartialEq)]
pub enum AnyToken<'src> {
    // Keywords
    Struct(TokenStruct),
    Container(TokenContainer),
    Fn(TokenFn),
    Let(TokenLet),
    For(TokenFor),
    In(TokenIn),
    With(TokenWith),
    If(TokenIf),
    Else(TokenElse),
    Or(TokenOr),
    And(TokenAnd),
    Return(TokenReturn),
    True(TokenTrue),
    False(TokenFalse),
    SelfKw(TokenSelf),

    // Operators
    Equals(TokenEquals),
    EqualsEquals(TokenEqualsEquals),
    NotEquals(TokenNotEquals),
    LessThan(TokenLessThan),
    GreaterThan(TokenGreaterThan),
    LessEquals(TokenLessEquals),
    GreaterEquals(TokenGreaterEquals),
    Plus(TokenPlus),
    Minus(TokenMinus),
    Multiply(TokenMultiply),
    Divide(TokenDivide),
    Power(TokenPower),
    Modulo(TokenModulo),
    Ampersand(TokenAmpersand),

    // Punctuation
    Colon(TokenColon),
    SemiColon(TokenSemiColon),
    Comma(TokenComma),
    Dot(TokenDot),
    DotDot(TokenDotDot),
    LeftParen(TokenLeftParen),
    RightParen(TokenRightParen),
    LeftBracket(TokenLeftBracket),
    RightBracket(TokenRightBracket),
    LeftBrace(TokenLeftBrace),
    RightBrace(TokenRightBrace),
    Pipe(TokenPipe),
    Arrow(TokenArrow),

    // Built-in types
    BoolType(TokenBoolType),
    I32Type(TokenI32Type),
    F64Type(TokenF64Type),
    RealType(TokenRealType),
    AlgebraicType(TokenAlgebraicType),

    // Literals
    FloatLiteral(TokenFloatLiteral),
    IntLiteral(TokenIntLiteral),
    Identifier(TokenIdentifier<'src>),
}

impl<'src> AnyToken<'src> {
    /// Get the position of any token variant
    pub fn position(&self) -> LineColumn {
        match self {
            AnyToken::Struct(t) => t.position(),
            AnyToken::Container(t) => t.position(),
            AnyToken::Fn(t) => t.position(),
            AnyToken::Let(t) => t.position(),
            AnyToken::For(t) => t.position(),
            AnyToken::In(t) => t.position(),
            AnyToken::With(t) => t.position(),
            AnyToken::If(t) => t.position(),
            AnyToken::Else(t) => t.position(),
            AnyToken::Or(t) => t.position(),
            AnyToken::And(t) => t.position(),
            AnyToken::Return(t) => t.position(),
            AnyToken::True(t) => t.position(),
            AnyToken::False(t) => t.position(),
            AnyToken::SelfKw(t) => t.position(),
            AnyToken::Equals(t) => t.position(),
            AnyToken::EqualsEquals(t) => t.position(),
            AnyToken::NotEquals(t) => t.position(),
            AnyToken::LessThan(t) => t.position(),
            AnyToken::GreaterThan(t) => t.position(),
            AnyToken::LessEquals(t) => t.position(),
            AnyToken::GreaterEquals(t) => t.position(),
            AnyToken::Plus(t) => t.position(),
            AnyToken::Minus(t) => t.position(),
            AnyToken::Multiply(t) => t.position(),
            AnyToken::Divide(t) => t.position(),
            AnyToken::Power(t) => t.position(),
            AnyToken::Modulo(t) => t.position(),
            AnyToken::Ampersand(t) => t.position(),
            AnyToken::Colon(t) => t.position(),
            AnyToken::SemiColon(t) => t.position(),
            AnyToken::Comma(t) => t.position(),
            AnyToken::Dot(t) => t.position(),
            AnyToken::DotDot(t) => t.position(),
            AnyToken::LeftParen(t) => t.position(),
            AnyToken::RightParen(t) => t.position(),
            AnyToken::LeftBracket(t) => t.position(),
            AnyToken::RightBracket(t) => t.position(),
            AnyToken::LeftBrace(t) => t.position(),
            AnyToken::RightBrace(t) => t.position(),
            AnyToken::Pipe(t) => t.position(),
            AnyToken::Arrow(t) => t.position(),
            AnyToken::BoolType(t) => t.position(),
            AnyToken::I32Type(t) => t.position(),
            AnyToken::F64Type(t) => t.position(),
            AnyToken::RealType(t) => t.position(),
            AnyToken::AlgebraicType(t) => t.position(),
            AnyToken::FloatLiteral(t) => t.position(),
            AnyToken::IntLiteral(t) => t.position(),
            AnyToken::Identifier(t) => t.position(),
        }
    }

    /// Get the string value of any token variant
    pub fn value_str(&self) -> &str {
        match self {
            AnyToken::Struct(t) => t.value_str(),
            AnyToken::Container(t) => t.value_str(),
            AnyToken::Fn(t) => t.value_str(),
            AnyToken::Let(t) => t.value_str(),
            AnyToken::For(t) => t.value_str(),
            AnyToken::In(t) => t.value_str(),
            AnyToken::With(t) => t.value_str(),
            AnyToken::If(t) => t.value_str(),
            AnyToken::Else(t) => t.value_str(),
            AnyToken::Or(t) => t.value_str(),
            AnyToken::And(t) => t.value_str(),
            AnyToken::Return(t) => t.value_str(),
            AnyToken::True(t) => t.value_str(),
            AnyToken::False(t) => t.value_str(),
            AnyToken::SelfKw(t) => t.value_str(),
            AnyToken::Equals(t) => t.value_str(),
            AnyToken::EqualsEquals(t) => t.value_str(),
            AnyToken::NotEquals(t) => t.value_str(),
            AnyToken::LessThan(t) => t.value_str(),
            AnyToken::GreaterThan(t) => t.value_str(),
            AnyToken::LessEquals(t) => t.value_str(),
            AnyToken::GreaterEquals(t) => t.value_str(),
            AnyToken::Plus(t) => t.value_str(),
            AnyToken::Minus(t) => t.value_str(),
            AnyToken::Multiply(t) => t.value_str(),
            AnyToken::Divide(t) => t.value_str(),
            AnyToken::Power(t) => t.value_str(),
            AnyToken::Modulo(t) => t.value_str(),
            AnyToken::Ampersand(t) => t.value_str(),
            AnyToken::Colon(t) => t.value_str(),
            AnyToken::SemiColon(t) => t.value_str(),
            AnyToken::Comma(t) => t.value_str(),
            AnyToken::Dot(t) => t.value_str(),
            AnyToken::DotDot(t) => t.value_str(),
            AnyToken::LeftParen(t) => t.value_str(),
            AnyToken::RightParen(t) => t.value_str(),
            AnyToken::LeftBracket(t) => t.value_str(),
            AnyToken::RightBracket(t) => t.value_str(),
            AnyToken::LeftBrace(t) => t.value_str(),
            AnyToken::RightBrace(t) => t.value_str(),
            AnyToken::Pipe(t) => t.value_str(),
            AnyToken::Arrow(t) => t.value_str(),
            AnyToken::BoolType(t) => t.value_str(),
            AnyToken::I32Type(t) => t.value_str(),
            AnyToken::F64Type(t) => t.value_str(),
            AnyToken::RealType(t) => t.value_str(),
            AnyToken::AlgebraicType(t) => t.value_str(),
            AnyToken::FloatLiteral(t) => t.value_str(),
            AnyToken::IntLiteral(t) => t.value_str(),
            AnyToken::Identifier(t) => t.value_str(),
        }
    }
}

// ============================================================================
// Logos Token Enum (Internal for Lexer)
// ============================================================================

#[derive(Logos, Debug, PartialEq)]
#[logos(extras = NewLineTracer)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(skip(r"//[^\n]*", newline_callback))]
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
enum LogosToken<'src> {
    // Keywords
    #[token("struct", derive_position)]
    Struct(LineColumn),
    #[token("container", derive_position)]
    Container(LineColumn),
    #[token("fn", derive_position)]
    Fn(LineColumn),
    #[token("let", derive_position)]
    Let(LineColumn),
    #[token("for", derive_position)]
    For(LineColumn),
    #[token("in", derive_position)]
    In(LineColumn),
    #[token("with", derive_position)]
    With(LineColumn),
    #[token("if", derive_position)]
    If(LineColumn),
    #[token("else", derive_position)]
    Else(LineColumn),
    #[token("or", derive_position)]
    Or(LineColumn),
    #[token("and", derive_position)]
    And(LineColumn),
    #[token("return", derive_position)]
    Return(LineColumn),
    #[token("true", derive_position)]
    True(LineColumn),
    #[token("false", derive_position)]
    False(LineColumn),
    #[token("self", derive_position)]
    SelfKw(LineColumn),

    // Operators
    #[token("=", derive_position)]
    Equals(LineColumn),
    #[token("==", derive_position)]
    EqualsEquals(LineColumn),
    #[token("!=", derive_position)]
    NotEquals(LineColumn),
    #[token("<", derive_position)]
    LessThan(LineColumn),
    #[token(">", derive_position)]
    GreaterThan(LineColumn),
    #[token("<=", derive_position)]
    LessEquals(LineColumn),
    #[token(">=", derive_position)]
    GreaterEquals(LineColumn),
    #[token("+", derive_position)]
    Plus(LineColumn),
    #[token("-", derive_position)]
    Minus(LineColumn),
    #[token("*", derive_position)]
    Multiply(LineColumn),
    #[token("/", derive_position)]
    Divide(LineColumn),
    #[token("^", derive_position)]
    Power(LineColumn),
    #[token("%", derive_position)]
    Modulo(LineColumn),
    #[token("&", derive_position)]
    Ampersand(LineColumn),

    // Punctuation
    #[token(":", derive_position)]
    Colon(LineColumn),
    #[token(";", derive_position)]
    SemiColon(LineColumn),
    #[token(",", derive_position)]
    Comma(LineColumn),
    #[token(".", derive_position)]
    Dot(LineColumn),
    #[token("..", derive_position)]
    DotDot(LineColumn),
    #[token("(", derive_position)]
    LeftParen(LineColumn),
    #[token(")", derive_position)]
    RightParen(LineColumn),
    #[token("[", derive_position)]
    LeftBracket(LineColumn),
    #[token("]", derive_position)]
    RightBracket(LineColumn),
    #[token("{", derive_position)]
    LeftBrace(LineColumn),
    #[token("}", derive_position)]
    RightBrace(LineColumn),
    #[token("|", derive_position)]
    Pipe(LineColumn),
    #[token("->", derive_position)]
    Arrow(LineColumn),

    // Built-in types (must come before Identifier regex)
    #[token("bool", derive_position)]
    BoolType(LineColumn),
    #[token("i32", derive_position)]
    I32Type(LineColumn),
    #[token("f64", derive_position)]
    F64Type(LineColumn),
    #[token("Real", derive_position)]
    RealType(LineColumn),
    #[token("Algebraic", derive_position)]
    AlgebraicType(LineColumn),

    // Literals (order matters - float must come before int, identifiers must be last)
    #[regex(r"\d+\.\d+", parse_with_span_no_newlines)]
    FloatLiteral((f64, Span)),
    #[regex(r"\d+", parse_with_span_no_newlines)]
    IntLiteral((i32, Span)),
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", parse_ident)]
    Identifier((&'src str, Span)),
}

// ============================================================================
// Helper Functions for Logos
// ============================================================================

fn newline_callback<'src>(lex: &mut Lexer<'src, LogosToken<'src>>) -> Skip {
    lex.extras.line += 1;
    lex.extras.last_newline_char_index = lex.span().end;
    Skip
}

fn derive_position<'src>(lex: &mut Lexer<'src, LogosToken<'src>>) -> LineColumn {
    let line = lex.extras.line;
    let column = lex.span().start - lex.extras.last_newline_char_index + 1;
    LineColumn { line, column }
}

fn derive_span_no_newline<'src>(lex: &mut Lexer<'src, LogosToken<'src>>) -> Span {
    let start = derive_position(lex);
    let end_column = lex.span().end - lex.extras.last_newline_char_index + 1;
    let lines = 0;
    Span {
        start,
        end_column,
        lines,
    }
}

fn parse_with_span_no_newlines<'src, T: FromStr>(
    lex: &mut Lexer<'src, LogosToken<'src>>,
) -> Option<(T, Span)> {
    let value = lex.slice().parse::<T>().ok()?;
    let span = derive_span_no_newline(lex);
    Some((value, span))
}

fn parse_ident<'src>(lex: &mut Lexer<'src, LogosToken<'src>>) -> Option<(&'src str, Span)> {
    let value = lex.slice();
    let span = derive_span_no_newline(lex);
    Some((value, span))
}

// ============================================================================
// Public Tokenizer Function
// ============================================================================

/// Tokenize input source code into a vector of AnyToken
pub fn tokenize<'src>(input: &'src str) -> Result<Vec<AnyToken<'src>>, String> {
    let mut lexer = LogosToken::lexer(input);
    let mut tokens = Vec::new();

    while let Some(result) = lexer.next() {
        match result {
            Ok(logos_token) => {
                let any_token = convert_logos_token(logos_token);
                tokens.push(any_token);
            }
            Err(_) => {
                let span_start = lexer.span().start;
                return Err(format!("Lexing error at position {}", span_start));
            }
        }
    }

    Ok(tokens)
}

/// Convert internal Logos token to public AnyToken
fn convert_logos_token(token: LogosToken) -> AnyToken {
    match token {
        LogosToken::Struct(pos) => AnyToken::Struct(TokenStruct::new(pos)),
        LogosToken::Container(pos) => AnyToken::Container(TokenContainer::new(pos)),
        LogosToken::Fn(pos) => AnyToken::Fn(TokenFn::new(pos)),
        LogosToken::Let(pos) => AnyToken::Let(TokenLet::new(pos)),
        LogosToken::For(pos) => AnyToken::For(TokenFor::new(pos)),
        LogosToken::In(pos) => AnyToken::In(TokenIn::new(pos)),
        LogosToken::With(pos) => AnyToken::With(TokenWith::new(pos)),
        LogosToken::If(pos) => AnyToken::If(TokenIf::new(pos)),
        LogosToken::Else(pos) => AnyToken::Else(TokenElse::new(pos)),
        LogosToken::Or(pos) => AnyToken::Or(TokenOr::new(pos)),
        LogosToken::And(pos) => AnyToken::And(TokenAnd::new(pos)),
        LogosToken::Return(pos) => AnyToken::Return(TokenReturn::new(pos)),
        LogosToken::True(pos) => AnyToken::True(TokenTrue::new(pos)),
        LogosToken::False(pos) => AnyToken::False(TokenFalse::new(pos)),
        LogosToken::SelfKw(pos) => AnyToken::SelfKw(TokenSelf::new(pos)),
        LogosToken::Equals(pos) => AnyToken::Equals(TokenEquals::new(pos)),
        LogosToken::EqualsEquals(pos) => AnyToken::EqualsEquals(TokenEqualsEquals::new(pos)),
        LogosToken::NotEquals(pos) => AnyToken::NotEquals(TokenNotEquals::new(pos)),
        LogosToken::LessThan(pos) => AnyToken::LessThan(TokenLessThan::new(pos)),
        LogosToken::GreaterThan(pos) => AnyToken::GreaterThan(TokenGreaterThan::new(pos)),
        LogosToken::LessEquals(pos) => AnyToken::LessEquals(TokenLessEquals::new(pos)),
        LogosToken::GreaterEquals(pos) => AnyToken::GreaterEquals(TokenGreaterEquals::new(pos)),
        LogosToken::Plus(pos) => AnyToken::Plus(TokenPlus::new(pos)),
        LogosToken::Minus(pos) => AnyToken::Minus(TokenMinus::new(pos)),
        LogosToken::Multiply(pos) => AnyToken::Multiply(TokenMultiply::new(pos)),
        LogosToken::Divide(pos) => AnyToken::Divide(TokenDivide::new(pos)),
        LogosToken::Power(pos) => AnyToken::Power(TokenPower::new(pos)),
        LogosToken::Modulo(pos) => AnyToken::Modulo(TokenModulo::new(pos)),
        LogosToken::Ampersand(pos) => AnyToken::Ampersand(TokenAmpersand::new(pos)),
        LogosToken::Colon(pos) => AnyToken::Colon(TokenColon::new(pos)),
        LogosToken::SemiColon(pos) => AnyToken::SemiColon(TokenSemiColon::new(pos)),
        LogosToken::Comma(pos) => AnyToken::Comma(TokenComma::new(pos)),
        LogosToken::Dot(pos) => AnyToken::Dot(TokenDot::new(pos)),
        LogosToken::DotDot(pos) => AnyToken::DotDot(TokenDotDot::new(pos)),
        LogosToken::LeftParen(pos) => AnyToken::LeftParen(TokenLeftParen::new(pos)),
        LogosToken::RightParen(pos) => AnyToken::RightParen(TokenRightParen::new(pos)),
        LogosToken::LeftBracket(pos) => AnyToken::LeftBracket(TokenLeftBracket::new(pos)),
        LogosToken::RightBracket(pos) => AnyToken::RightBracket(TokenRightBracket::new(pos)),
        LogosToken::LeftBrace(pos) => AnyToken::LeftBrace(TokenLeftBrace::new(pos)),
        LogosToken::RightBrace(pos) => AnyToken::RightBrace(TokenRightBrace::new(pos)),
        LogosToken::Pipe(pos) => AnyToken::Pipe(TokenPipe::new(pos)),
        LogosToken::Arrow(pos) => AnyToken::Arrow(TokenArrow::new(pos)),
        LogosToken::BoolType(pos) => AnyToken::BoolType(TokenBoolType::new(pos)),
        LogosToken::I32Type(pos) => AnyToken::I32Type(TokenI32Type::new(pos)),
        LogosToken::F64Type(pos) => AnyToken::F64Type(TokenF64Type::new(pos)),
        LogosToken::RealType(pos) => AnyToken::RealType(TokenRealType::new(pos)),
        LogosToken::AlgebraicType(pos) => AnyToken::AlgebraicType(TokenAlgebraicType::new(pos)),
        LogosToken::FloatLiteral((value, span)) => {
            AnyToken::FloatLiteral(TokenFloatLiteral::new(value, span))
        }
        LogosToken::IntLiteral((value, span)) => {
            AnyToken::IntLiteral(TokenIntLiteral::new(value, span))
        }
        LogosToken::Identifier((name, span)) => {
            AnyToken::Identifier(TokenIdentifier::new(name, span))
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let input = "struct container fn let for in with if else or and return true false self";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 15);

        // Test that each keyword token has the correct position and value
        assert!(matches!(tokens[0], AnyToken::Struct(_)));
        assert_eq!(tokens[0].value_str(), "struct");
        assert_eq!(tokens[0].position(), LineColumn { line: 1, column: 1 });

        assert!(matches!(tokens[1], AnyToken::Container(_)));
        assert_eq!(tokens[1].value_str(), "container");
    }

    #[test]
    fn test_operators() {
        let input = "= == != < > <= >= + - * / ^ % &";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 14);

        assert!(matches!(tokens[0], AnyToken::Equals(_)));
        assert_eq!(tokens[0].value_str(), "=");

        assert!(matches!(tokens[1], AnyToken::EqualsEquals(_)));
        assert_eq!(tokens[1].value_str(), "==");
    }

    #[test]
    fn test_punctuation() {
        let input = ": ; , . .. ( ) [ ] { } | ->";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 13);

        assert!(matches!(tokens[0], AnyToken::Colon(_)));
        assert_eq!(tokens[0].value_str(), ":");
    }

    #[test]
    fn test_literals() {
        let input = "123 3.45 identifier_name _private";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 4);

        if let AnyToken::IntLiteral(token) = &tokens[0] {
            assert_eq!(token.value, 123);
            assert_eq!(token.position(), LineColumn { line: 1, column: 1 });
        } else {
            panic!("Expected IntLiteral");
        }

        if let AnyToken::FloatLiteral(token) = &tokens[1] {
            assert_eq!(token.value, 3.45);
        } else {
            panic!("Expected FloatLiteral");
        }

        if let AnyToken::Identifier(token) = &tokens[2] {
            assert_eq!(token.name, "identifier_name");
            assert_eq!(token.value_str(), "identifier_name");
        } else {
            panic!("Expected Identifier");
        }
    }

    #[test]
    fn test_types() {
        let input = "bool i32 f64 Real Algebraic";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 5);

        assert!(matches!(tokens[0], AnyToken::BoolType(_)));
        assert!(matches!(tokens[1], AnyToken::I32Type(_)));
        assert!(matches!(tokens[2], AnyToken::F64Type(_)));
        assert!(matches!(tokens[3], AnyToken::RealType(_)));
        assert!(matches!(tokens[4], AnyToken::AlgebraicType(_)));
    }

    #[test]
    fn test_simple_expression() {
        let input = "let x: i32 = 42;";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 7);

        assert!(matches!(tokens[0], AnyToken::Let(_)));
        assert!(matches!(tokens[1], AnyToken::Identifier(_)));
        assert!(matches!(tokens[2], AnyToken::Colon(_)));
        assert!(matches!(tokens[3], AnyToken::I32Type(_)));
        assert!(matches!(tokens[4], AnyToken::Equals(_)));
        assert!(matches!(tokens[5], AnyToken::IntLiteral(_)));
        assert!(matches!(tokens[6], AnyToken::SemiColon(_)));
    }

    #[test]
    fn test_struct_definition() {
        let input = "struct Point { x: f64, y: f64 }";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 11);

        assert!(matches!(tokens[0], AnyToken::Struct(_)));

        if let AnyToken::Identifier(token) = &tokens[1] {
            assert_eq!(token.name, "Point");
        }
    }

    #[test]
    fn test_function_definition() {
        let input = "fn distance(p1: &Point, p2: &Point) -> f64";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 15);

        assert!(matches!(tokens[0], AnyToken::Fn(_)));
        assert!(matches!(tokens[14], AnyToken::F64Type(_)));
    }

    #[test]
    fn test_range_syntax() {
        let input = "for i in 0..5";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 6);

        assert!(matches!(tokens[0], AnyToken::For(_)));
        assert!(matches!(tokens[1], AnyToken::Identifier(_)));
        assert!(matches!(tokens[2], AnyToken::In(_)));
        assert!(matches!(tokens[3], AnyToken::IntLiteral(_)));
        assert!(matches!(tokens[4], AnyToken::DotDot(_)));
        assert!(matches!(tokens[5], AnyToken::IntLiteral(_)));
    }

    #[test]
    fn test_with_statement() {
        let input = "with transform { .point = p1; }";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 9);

        assert!(matches!(tokens[0], AnyToken::With(_)));
        assert!(matches!(tokens[8], AnyToken::RightBrace(_)));
    }

    #[test]
    fn test_comments_are_skipped() {
        let input = "let x = 42; // This is a comment\nlet y = 3.45; /* Multi-line\n   comment */";
        let tokens = tokenize(input).unwrap();

        // Should have: let x = 42 ; let y = 3.45 ;
        assert_eq!(tokens.len(), 10);

        assert!(matches!(tokens[0], AnyToken::Let(_)));
        assert_eq!(tokens[0].position().line, 1);

        assert!(matches!(tokens[5], AnyToken::Let(_)));
        assert_eq!(tokens[5].position().line, 2);
    }

    #[test]
    fn test_whitespace_is_skipped() {
        let input = "  let    x  =  42  ;  ";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 5);
    }

    #[test]
    fn test_array_syntax() {
        let input = "let points: [Point; 5] = [];";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 12);

        assert!(matches!(tokens[0], AnyToken::Let(_)));
        assert!(matches!(tokens[11], AnyToken::SemiColon(_)));
    }

    #[test]
    fn test_method_call() {
        let input = "circle.area()";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 5);

        assert!(matches!(tokens[0], AnyToken::Identifier(_)));
        assert!(matches!(tokens[1], AnyToken::Dot(_)));
    }

    #[test]
    fn test_closure_syntax() {
        let input = "points.map(|p| p.x)";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 11);

        assert!(matches!(tokens[0], AnyToken::Identifier(_))); // points
        assert!(matches!(tokens[1], AnyToken::Dot(_))); // .
        assert!(matches!(tokens[2], AnyToken::Identifier(_))); // map
        assert!(matches!(tokens[3], AnyToken::LeftParen(_))); // (
        assert!(matches!(tokens[4], AnyToken::Pipe(_))); // |
        assert!(matches!(tokens[5], AnyToken::Identifier(_))); // p
        assert!(matches!(tokens[6], AnyToken::Pipe(_))); // |
        assert!(matches!(tokens[7], AnyToken::Identifier(_))); // p
        assert!(matches!(tokens[8], AnyToken::Dot(_))); // .
        assert!(matches!(tokens[9], AnyToken::Identifier(_))); // x
        assert!(matches!(tokens[10], AnyToken::RightParen(_))); // )
    }

    #[test]
    fn test_token_trait() {
        let token = TokenStruct::new(LineColumn { line: 1, column: 5 });
        assert_eq!(token.position(), LineColumn { line: 1, column: 5 });
        assert_eq!(token.value_str(), "struct");
    }
}
