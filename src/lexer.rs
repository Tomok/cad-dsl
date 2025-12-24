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
    ($name:ident, $variant:ident, $text:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name {
            pub position: LineColumn,
        }

        impl $name {
            pub fn new(position: LineColumn) -> Self {
                Self { position }
            }

            pub fn from_lexer<'src>(lex: &mut Lexer<'src, Token<'src>>) -> Self {
                Self::new(derive_position(lex))
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

        impl<'src> From<$name> for Token<'src> {
            fn from(token: $name) -> Self {
                Token::$variant(token)
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

fixed_token!(TokenStruct, Struct, "struct");
fixed_token!(TokenContainer, Container, "container");
fixed_token!(TokenFn, Fn, "fn");
fixed_token!(TokenLet, Let, "let");
fixed_token!(TokenFor, For, "for");
fixed_token!(TokenIn, In, "in");
fixed_token!(TokenWith, With, "with");
fixed_token!(TokenIf, If, "if");
fixed_token!(TokenElse, Else, "else");
fixed_token!(TokenOr, Or, "or");
fixed_token!(TokenAnd, And, "and");
fixed_token!(TokenReturn, Return, "return");
fixed_token!(TokenTrue, True, "true");
fixed_token!(TokenFalse, False, "false");
fixed_token!(TokenSelf, SelfKw, "self");

// ============================================================================
// Operator Tokens
// ============================================================================

fixed_token!(TokenEquals, Equals, "=");
fixed_token!(TokenEqualsEquals, EqualsEquals, "==");
fixed_token!(TokenNotEquals, NotEquals, "!=");
fixed_token!(TokenLessThan, LessThan, "<");
fixed_token!(TokenGreaterThan, GreaterThan, ">");
fixed_token!(TokenLessEquals, LessEquals, "<=");
fixed_token!(TokenGreaterEquals, GreaterEquals, ">=");
fixed_token!(TokenPlus, Plus, "+");
fixed_token!(TokenMinus, Minus, "-");
fixed_token!(TokenMultiply, Multiply, "*");
fixed_token!(TokenDivide, Divide, "/");
fixed_token!(TokenPower, Power, "^");
fixed_token!(TokenModulo, Modulo, "%");
fixed_token!(TokenAmpersand, Ampersand, "&");

// ============================================================================
// Punctuation Tokens
// ============================================================================

fixed_token!(TokenColon, Colon, ":");
fixed_token!(TokenSemiColon, SemiColon, ";");
fixed_token!(TokenComma, Comma, ",");
fixed_token!(TokenDot, Dot, ".");
fixed_token!(TokenDotDot, DotDot, "..");
fixed_token!(TokenLeftParen, LeftParen, "(");
fixed_token!(TokenRightParen, RightParen, ")");
fixed_token!(TokenLeftBracket, LeftBracket, "[");
fixed_token!(TokenRightBracket, RightBracket, "]");
fixed_token!(TokenLeftBrace, LeftBrace, "{");
fixed_token!(TokenRightBrace, RightBrace, "}");
fixed_token!(TokenPipe, Pipe, "|");
fixed_token!(TokenArrow, Arrow, "->");

// ============================================================================
// Built-in Type Tokens
// ============================================================================

fixed_token!(TokenBoolType, BoolType, "bool");
fixed_token!(TokenI32Type, I32Type, "i32");
fixed_token!(TokenF64Type, F64Type, "f64");
fixed_token!(TokenRealType, RealType, "Real");
fixed_token!(TokenAlgebraicType, AlgebraicType, "Algebraic");

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

    pub fn from_lexer<'src>(lex: &mut Lexer<'src, Token<'src>>) -> Option<Self> {
        let value = lex.slice().parse::<f64>().ok()?;
        let span = derive_span_no_newline(lex);
        Some(Self::new(value, span))
    }
}

impl TokenTrait for TokenFloatLiteral {
    fn position(&self) -> LineColumn {
        self.span.start
    }

    fn value_str(&self) -> &str {
        "float_literal"
    }
}

impl<'src> From<TokenFloatLiteral> for Token<'src> {
    fn from(token: TokenFloatLiteral) -> Self {
        Token::FloatLiteral(token)
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

    pub fn from_lexer<'src>(lex: &mut Lexer<'src, Token<'src>>) -> Option<Self> {
        let value = lex.slice().parse::<i32>().ok()?;
        let span = derive_span_no_newline(lex);
        Some(Self::new(value, span))
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

impl<'src> From<TokenIntLiteral> for Token<'src> {
    fn from(token: TokenIntLiteral) -> Self {
        Token::IntLiteral(token)
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

    pub fn from_lexer(lex: &mut Lexer<'src, Token<'src>>) -> Self {
        let name = lex.slice();
        let span = derive_span_no_newline(lex);
        Self::new(name, span)
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

impl<'src> From<TokenIdentifier<'src>> for Token<'src> {
    fn from(token: TokenIdentifier<'src>) -> Self {
        Token::Identifier(token)
    }
}

impl<'src> std::fmt::Display for TokenIdentifier<'src> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

// ============================================================================
// Main Token Enum
// ============================================================================

/// Token enum with direct Logos integration
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(extras = NewLineTracer)]
#[logos(skip(r"[ \t\f]+"))]
#[logos(skip(r"\n", newline_callback))]
#[logos(skip(r"//[^\n]*"))]
#[logos(skip(r"/\*([^*]|\*+[^*/])*\*+/", multiline_comment_callback))]
pub enum Token<'src> {
    // Keywords
    #[token("struct", TokenStruct::from_lexer)]
    Struct(TokenStruct),
    #[token("container", TokenContainer::from_lexer)]
    Container(TokenContainer),
    #[token("fn", TokenFn::from_lexer)]
    Fn(TokenFn),
    #[token("let", TokenLet::from_lexer)]
    Let(TokenLet),
    #[token("for", TokenFor::from_lexer)]
    For(TokenFor),
    #[token("in", TokenIn::from_lexer)]
    In(TokenIn),
    #[token("with", TokenWith::from_lexer)]
    With(TokenWith),
    #[token("if", TokenIf::from_lexer)]
    If(TokenIf),
    #[token("else", TokenElse::from_lexer)]
    Else(TokenElse),
    #[token("or", TokenOr::from_lexer)]
    Or(TokenOr),
    #[token("and", TokenAnd::from_lexer)]
    And(TokenAnd),
    #[token("return", TokenReturn::from_lexer)]
    Return(TokenReturn),
    #[token("true", TokenTrue::from_lexer)]
    True(TokenTrue),
    #[token("false", TokenFalse::from_lexer)]
    False(TokenFalse),
    #[token("self", TokenSelf::from_lexer)]
    SelfKw(TokenSelf),

    // Operators
    #[token("=", TokenEquals::from_lexer)]
    Equals(TokenEquals),
    #[token("==", TokenEqualsEquals::from_lexer)]
    EqualsEquals(TokenEqualsEquals),
    #[token("!=", TokenNotEquals::from_lexer)]
    NotEquals(TokenNotEquals),
    #[token("<", TokenLessThan::from_lexer)]
    LessThan(TokenLessThan),
    #[token(">", TokenGreaterThan::from_lexer)]
    GreaterThan(TokenGreaterThan),
    #[token("<=", TokenLessEquals::from_lexer)]
    LessEquals(TokenLessEquals),
    #[token(">=", TokenGreaterEquals::from_lexer)]
    GreaterEquals(TokenGreaterEquals),
    #[token("+", TokenPlus::from_lexer)]
    Plus(TokenPlus),
    #[token("-", TokenMinus::from_lexer)]
    Minus(TokenMinus),
    #[token("*", TokenMultiply::from_lexer)]
    Multiply(TokenMultiply),
    #[token("/", TokenDivide::from_lexer)]
    Divide(TokenDivide),
    #[token("^", TokenPower::from_lexer)]
    Power(TokenPower),
    #[token("%", TokenModulo::from_lexer)]
    Modulo(TokenModulo),
    #[token("&", TokenAmpersand::from_lexer)]
    Ampersand(TokenAmpersand),

    // Punctuation
    #[token(":", TokenColon::from_lexer)]
    Colon(TokenColon),
    #[token(";", TokenSemiColon::from_lexer)]
    SemiColon(TokenSemiColon),
    #[token(",", TokenComma::from_lexer)]
    Comma(TokenComma),
    #[token(".", TokenDot::from_lexer)]
    Dot(TokenDot),
    #[token("..", TokenDotDot::from_lexer)]
    DotDot(TokenDotDot),
    #[token("(", TokenLeftParen::from_lexer)]
    LeftParen(TokenLeftParen),
    #[token(")", TokenRightParen::from_lexer)]
    RightParen(TokenRightParen),
    #[token("[", TokenLeftBracket::from_lexer)]
    LeftBracket(TokenLeftBracket),
    #[token("]", TokenRightBracket::from_lexer)]
    RightBracket(TokenRightBracket),
    #[token("{", TokenLeftBrace::from_lexer)]
    LeftBrace(TokenLeftBrace),
    #[token("}", TokenRightBrace::from_lexer)]
    RightBrace(TokenRightBrace),
    #[token("|", TokenPipe::from_lexer)]
    Pipe(TokenPipe),
    #[token("->", TokenArrow::from_lexer)]
    Arrow(TokenArrow),

    // Built-in types (must come before Identifier regex)
    #[token("bool", TokenBoolType::from_lexer)]
    BoolType(TokenBoolType),
    #[token("i32", TokenI32Type::from_lexer)]
    I32Type(TokenI32Type),
    #[token("f64", TokenF64Type::from_lexer)]
    F64Type(TokenF64Type),
    #[token("Real", TokenRealType::from_lexer)]
    RealType(TokenRealType),
    #[token("Algebraic", TokenAlgebraicType::from_lexer)]
    AlgebraicType(TokenAlgebraicType),

    // Literals (order matters - float must come before int, identifiers must be last)
    #[regex(r"\d+\.\d+", TokenFloatLiteral::from_lexer)]
    FloatLiteral(TokenFloatLiteral),
    #[regex(r"\d+", TokenIntLiteral::from_lexer)]
    IntLiteral(TokenIntLiteral),
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", TokenIdentifier::from_lexer)]
    Identifier(TokenIdentifier<'src>),
}

// Implement TokenTrait for Token enum - delegates to inner token structs
impl<'src> TokenTrait for Token<'src> {
    fn position(&self) -> LineColumn {
        match self {
            Token::Struct(t) => t.position(),
            Token::Container(t) => t.position(),
            Token::Fn(t) => t.position(),
            Token::Let(t) => t.position(),
            Token::For(t) => t.position(),
            Token::In(t) => t.position(),
            Token::With(t) => t.position(),
            Token::If(t) => t.position(),
            Token::Else(t) => t.position(),
            Token::Or(t) => t.position(),
            Token::And(t) => t.position(),
            Token::Return(t) => t.position(),
            Token::True(t) => t.position(),
            Token::False(t) => t.position(),
            Token::SelfKw(t) => t.position(),
            Token::Equals(t) => t.position(),
            Token::EqualsEquals(t) => t.position(),
            Token::NotEquals(t) => t.position(),
            Token::LessThan(t) => t.position(),
            Token::GreaterThan(t) => t.position(),
            Token::LessEquals(t) => t.position(),
            Token::GreaterEquals(t) => t.position(),
            Token::Plus(t) => t.position(),
            Token::Minus(t) => t.position(),
            Token::Multiply(t) => t.position(),
            Token::Divide(t) => t.position(),
            Token::Power(t) => t.position(),
            Token::Modulo(t) => t.position(),
            Token::Ampersand(t) => t.position(),
            Token::Colon(t) => t.position(),
            Token::SemiColon(t) => t.position(),
            Token::Comma(t) => t.position(),
            Token::Dot(t) => t.position(),
            Token::DotDot(t) => t.position(),
            Token::LeftParen(t) => t.position(),
            Token::RightParen(t) => t.position(),
            Token::LeftBracket(t) => t.position(),
            Token::RightBracket(t) => t.position(),
            Token::LeftBrace(t) => t.position(),
            Token::RightBrace(t) => t.position(),
            Token::Pipe(t) => t.position(),
            Token::Arrow(t) => t.position(),
            Token::BoolType(t) => t.position(),
            Token::I32Type(t) => t.position(),
            Token::F64Type(t) => t.position(),
            Token::RealType(t) => t.position(),
            Token::AlgebraicType(t) => t.position(),
            Token::FloatLiteral(t) => t.position(),
            Token::IntLiteral(t) => t.position(),
            Token::Identifier(t) => t.position(),
        }
    }

    fn value_str(&self) -> &str {
        match self {
            Token::Struct(t) => t.value_str(),
            Token::Container(t) => t.value_str(),
            Token::Fn(t) => t.value_str(),
            Token::Let(t) => t.value_str(),
            Token::For(t) => t.value_str(),
            Token::In(t) => t.value_str(),
            Token::With(t) => t.value_str(),
            Token::If(t) => t.value_str(),
            Token::Else(t) => t.value_str(),
            Token::Or(t) => t.value_str(),
            Token::And(t) => t.value_str(),
            Token::Return(t) => t.value_str(),
            Token::True(t) => t.value_str(),
            Token::False(t) => t.value_str(),
            Token::SelfKw(t) => t.value_str(),
            Token::Equals(t) => t.value_str(),
            Token::EqualsEquals(t) => t.value_str(),
            Token::NotEquals(t) => t.value_str(),
            Token::LessThan(t) => t.value_str(),
            Token::GreaterThan(t) => t.value_str(),
            Token::LessEquals(t) => t.value_str(),
            Token::GreaterEquals(t) => t.value_str(),
            Token::Plus(t) => t.value_str(),
            Token::Minus(t) => t.value_str(),
            Token::Multiply(t) => t.value_str(),
            Token::Divide(t) => t.value_str(),
            Token::Power(t) => t.value_str(),
            Token::Modulo(t) => t.value_str(),
            Token::Ampersand(t) => t.value_str(),
            Token::Colon(t) => t.value_str(),
            Token::SemiColon(t) => t.value_str(),
            Token::Comma(t) => t.value_str(),
            Token::Dot(t) => t.value_str(),
            Token::DotDot(t) => t.value_str(),
            Token::LeftParen(t) => t.value_str(),
            Token::RightParen(t) => t.value_str(),
            Token::LeftBracket(t) => t.value_str(),
            Token::RightBracket(t) => t.value_str(),
            Token::LeftBrace(t) => t.value_str(),
            Token::RightBrace(t) => t.value_str(),
            Token::Pipe(t) => t.value_str(),
            Token::Arrow(t) => t.value_str(),
            Token::BoolType(t) => t.value_str(),
            Token::I32Type(t) => t.value_str(),
            Token::F64Type(t) => t.value_str(),
            Token::RealType(t) => t.value_str(),
            Token::AlgebraicType(t) => t.value_str(),
            Token::FloatLiteral(t) => t.value_str(),
            Token::IntLiteral(t) => t.value_str(),
            Token::Identifier(t) => t.value_str(),
        }
    }
}

// ============================================================================
// Helper Functions for Logos
// ============================================================================

fn newline_callback<'src>(lex: &mut Lexer<'src, Token<'src>>) -> Skip {
    lex.extras.line += 1;
    lex.extras.last_newline_char_index = lex.span().end;
    Skip
}

fn multiline_comment_callback<'src>(lex: &mut Lexer<'src, Token<'src>>) -> Skip {
    let text = lex.slice();
    for (i, c) in text.char_indices() {
        if c == '\n' {
            lex.extras.line += 1;
            lex.extras.last_newline_char_index = lex.span().start + i + 1;
        }
    }
    Skip
}

fn derive_position<'src>(lex: &mut Lexer<'src, Token<'src>>) -> LineColumn {
    let line = lex.extras.line;
    let column = lex.span().start - lex.extras.last_newline_char_index + 1;
    LineColumn { line, column }
}

fn derive_span_no_newline<'src>(lex: &mut Lexer<'src, Token<'src>>) -> Span {
    let start = derive_position(lex);
    let end_column = lex.span().end - lex.extras.last_newline_char_index + 1;
    let lines = 0;
    Span {
        start,
        end_column,
        lines,
    }
}

// ============================================================================
// Public Tokenizer Function
// ============================================================================

/// Tokenize input source code into a vector of Token
pub fn tokenize<'src>(input: &'src str) -> Result<Vec<Token<'src>>, String> {
    let mut lexer = Token::lexer(input);
    let mut tokens = Vec::new();

    while let Some(result) = lexer.next() {
        match result {
            Ok(token) => tokens.push(token),
            Err(_) => {
                let span_start = lexer.span().start;
                return Err(format!("Lexing error at position {}", span_start));
            }
        }
    }

    Ok(tokens)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn test_keywords() {
        let input = "struct container fn let for in with if else or and return true false self";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 15);

        assert_matches!(tokens[0], Token::Struct(_));
        assert_matches!(tokens[1], Token::Container(_));
        assert_matches!(tokens[2], Token::Fn(_));
        assert_matches!(tokens[3], Token::Let(_));
        assert_matches!(tokens[4], Token::For(_));
        assert_matches!(tokens[5], Token::In(_));
        assert_matches!(tokens[6], Token::With(_));
        assert_matches!(tokens[7], Token::If(_));
        assert_matches!(tokens[8], Token::Else(_));
        assert_matches!(tokens[9], Token::Or(_));
        assert_matches!(tokens[10], Token::And(_));
        assert_matches!(tokens[11], Token::Return(_));
        assert_matches!(tokens[12], Token::True(_));
        assert_matches!(tokens[13], Token::False(_));
        assert_matches!(tokens[14], Token::SelfKw(_));
    }

    #[test]
    fn test_operators() {
        let input = "= == != < > <= >= + - * / ^ % &";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 14);

        assert_matches!(tokens[0], Token::Equals(_));
        assert_matches!(tokens[1], Token::EqualsEquals(_));
        assert_matches!(tokens[2], Token::NotEquals(_));
        assert_matches!(tokens[3], Token::LessThan(_));
        assert_matches!(tokens[4], Token::GreaterThan(_));
        assert_matches!(tokens[5], Token::LessEquals(_));
        assert_matches!(tokens[6], Token::GreaterEquals(_));
        assert_matches!(tokens[7], Token::Plus(_));
        assert_matches!(tokens[8], Token::Minus(_));
        assert_matches!(tokens[9], Token::Multiply(_));
        assert_matches!(tokens[10], Token::Divide(_));
        assert_matches!(tokens[11], Token::Power(_));
        assert_matches!(tokens[12], Token::Modulo(_));
        assert_matches!(tokens[13], Token::Ampersand(_));
    }

    #[test]
    fn test_punctuation() {
        let input = ": ; , . .. ( ) [ ] { } | ->";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 13);

        assert_matches!(tokens[0], Token::Colon(_));
        assert_matches!(tokens[1], Token::SemiColon(_));
        assert_matches!(tokens[2], Token::Comma(_));
        assert_matches!(tokens[3], Token::Dot(_));
        assert_matches!(tokens[4], Token::DotDot(_));
        assert_matches!(tokens[5], Token::LeftParen(_));
        assert_matches!(tokens[6], Token::RightParen(_));
        assert_matches!(tokens[7], Token::LeftBracket(_));
        assert_matches!(tokens[8], Token::RightBracket(_));
        assert_matches!(tokens[9], Token::LeftBrace(_));
        assert_matches!(tokens[10], Token::RightBrace(_));
        assert_matches!(tokens[11], Token::Pipe(_));
        assert_matches!(tokens[12], Token::Arrow(_));
    }

    #[test]
    fn test_literals() {
        let input = "123 3.45 identifier_name _private";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 4);

        assert_matches!(tokens[0], Token::IntLiteral(ref t) if t.value == 123);
        assert_matches!(tokens[1], Token::FloatLiteral(ref t) if t.value == 3.45);
        assert_matches!(tokens[2], Token::Identifier(ref t) if t.name == "identifier_name");
        assert_matches!(tokens[3], Token::Identifier(ref t) if t.name == "_private");
    }

    #[test]
    fn test_types() {
        let input = "bool i32 f64 Real Algebraic";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 5);

        assert_matches!(tokens[0], Token::BoolType(_));
        assert_matches!(tokens[1], Token::I32Type(_));
        assert_matches!(tokens[2], Token::F64Type(_));
        assert_matches!(tokens[3], Token::RealType(_));
        assert_matches!(tokens[4], Token::AlgebraicType(_));
    }

    #[test]
    fn test_simple_expression() {
        let input = "let x: i32 = 42;";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 7);

        assert_matches!(tokens[0], Token::Let(_));
        assert_matches!(tokens[1], Token::Identifier(ref t) if t.name == "x");
        assert_matches!(tokens[2], Token::Colon(_));
        assert_matches!(tokens[3], Token::I32Type(_));
        assert_matches!(tokens[4], Token::Equals(_));
        assert_matches!(tokens[5], Token::IntLiteral(ref t) if t.value == 42);
        assert_matches!(tokens[6], Token::SemiColon(_));
    }

    #[test]
    fn test_struct_definition() {
        let input = "struct Point { x: f64, y: f64 }";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 11);

        assert_matches!(tokens[0], Token::Struct(_));
        assert_matches!(tokens[1], Token::Identifier(ref t) if t.name == "Point");
        assert_matches!(tokens[2], Token::LeftBrace(_));
        assert_matches!(tokens[3], Token::Identifier(ref t) if t.name == "x");
        assert_matches!(tokens[4], Token::Colon(_));
        assert_matches!(tokens[5], Token::F64Type(_));
        assert_matches!(tokens[6], Token::Comma(_));
        assert_matches!(tokens[7], Token::Identifier(ref t) if t.name == "y");
        assert_matches!(tokens[8], Token::Colon(_));
        assert_matches!(tokens[9], Token::F64Type(_));
        assert_matches!(tokens[10], Token::RightBrace(_));
    }

    #[test]
    fn test_function_definition() {
        let input = "fn distance(p1: &Point, p2: &Point) -> f64";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 15);

        assert_matches!(tokens[0], Token::Fn(_));
        assert_matches!(tokens[1], Token::Identifier(ref t) if t.name == "distance");
        assert_matches!(tokens[14], Token::F64Type(_));
    }

    #[test]
    fn test_range_syntax() {
        let input = "for i in 0..5";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 6);

        assert_matches!(tokens[0], Token::For(_));
        assert_matches!(tokens[1], Token::Identifier(ref t) if t.name == "i");
        assert_matches!(tokens[2], Token::In(_));
        assert_matches!(tokens[3], Token::IntLiteral(ref t) if t.value == 0);
        assert_matches!(tokens[4], Token::DotDot(_));
        assert_matches!(tokens[5], Token::IntLiteral(ref t) if t.value == 5);
    }

    #[test]
    fn test_with_statement() {
        let input = "with transform { .point = p1; }";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 9);

        assert_matches!(tokens[0], Token::With(_));
        assert_matches!(tokens[1], Token::Identifier(ref t) if t.name == "transform");
        assert_matches!(tokens[8], Token::RightBrace(_));
    }

    #[test]
    fn test_comments_are_skipped() {
        let input = "let x = 42; // This is a comment\nlet y = 3.45;";
        let tokens = tokenize(input).unwrap();

        // Should have: let x = 42 ; let y = 3.45 ;
        assert_eq!(tokens.len(), 10);

        assert_matches!(tokens[0], Token::Let(_));
        assert_matches!(tokens[1], Token::Identifier(ref t) if t.name == "x");
        assert_matches!(tokens[2], Token::Equals(_));
        assert_matches!(tokens[3], Token::IntLiteral(ref t) if t.value == 42);
        assert_matches!(tokens[4], Token::SemiColon(_));
        assert_matches!(tokens[5], Token::Let(_));
        assert_matches!(tokens[6], Token::Identifier(ref t) if t.name == "y");
        assert_matches!(tokens[7], Token::Equals(_));
        assert_matches!(tokens[8], Token::FloatLiteral(ref t) if t.value == 3.45);
        assert_matches!(tokens[9], Token::SemiColon(_));
    }

    #[test]
    fn test_whitespace_is_skipped() {
        let input = "  let    x  =  42  ;  ";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 5);

        assert_matches!(tokens[0], Token::Let(_));
        assert_matches!(tokens[1], Token::Identifier(ref t) if t.name == "x");
        assert_matches!(tokens[2], Token::Equals(_));
        assert_matches!(tokens[3], Token::IntLiteral(ref t) if t.value == 42);
        assert_matches!(tokens[4], Token::SemiColon(_));
    }

    #[test]
    fn test_array_syntax() {
        let input = "let points: [Point; 5] = [];";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 12);

        assert_matches!(tokens[0], Token::Let(_));
        assert_matches!(tokens[1], Token::Identifier(ref t) if t.name == "points");
        assert_matches!(tokens[2], Token::Colon(_));
        assert_matches!(tokens[3], Token::LeftBracket(_));
        assert_matches!(tokens[4], Token::Identifier(ref t) if t.name == "Point");
        assert_matches!(tokens[5], Token::SemiColon(_));
        assert_matches!(tokens[6], Token::IntLiteral(ref t) if t.value == 5);
        assert_matches!(tokens[7], Token::RightBracket(_));
        assert_matches!(tokens[8], Token::Equals(_));
        assert_matches!(tokens[9], Token::LeftBracket(_));
        assert_matches!(tokens[10], Token::RightBracket(_));
        assert_matches!(tokens[11], Token::SemiColon(_));
    }

    #[test]
    fn test_method_call() {
        let input = "circle.area()";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 5);

        assert_matches!(tokens[0], Token::Identifier(ref t) if t.name == "circle");
        assert_matches!(tokens[1], Token::Dot(_));
        assert_matches!(tokens[2], Token::Identifier(ref t) if t.name == "area");
        assert_matches!(tokens[3], Token::LeftParen(_));
        assert_matches!(tokens[4], Token::RightParen(_));
    }

    #[test]
    fn test_closure_syntax() {
        let input = "points.map(|p| p.x)";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 11);

        assert_matches!(tokens[0], Token::Identifier(ref t) if t.name == "points");
        assert_matches!(tokens[1], Token::Dot(_));
        assert_matches!(tokens[2], Token::Identifier(ref t) if t.name == "map");
        assert_matches!(tokens[3], Token::LeftParen(_));
        assert_matches!(tokens[4], Token::Pipe(_));
        assert_matches!(tokens[5], Token::Identifier(ref t) if t.name == "p");
        assert_matches!(tokens[6], Token::Pipe(_));
        assert_matches!(tokens[7], Token::Identifier(ref t) if t.name == "p");
        assert_matches!(tokens[8], Token::Dot(_));
        assert_matches!(tokens[9], Token::Identifier(ref t) if t.name == "x");
        assert_matches!(tokens[10], Token::RightParen(_));
    }

    // ========================================================================
    // Position Tracking Tests
    // ========================================================================

    #[test]
    fn test_position_single_line() {
        let input = "let x = 42;";
        let tokens = tokenize(input).unwrap();

        assert_eq!(tokens[0].position(), LineColumn { line: 1, column: 1 }); // let
        assert_eq!(tokens[1].position(), LineColumn { line: 1, column: 5 }); // x
        assert_eq!(tokens[2].position(), LineColumn { line: 1, column: 7 }); // =
        assert_eq!(tokens[3].position(), LineColumn { line: 1, column: 9 }); // 42
        assert_eq!(
            tokens[4].position(),
            LineColumn {
                line: 1,
                column: 11
            }
        ); // ;
    }

    #[test]
    fn test_position_multiple_lines() {
        let input = "let x = 10;\nlet y = 20;";
        let tokens = tokenize(input).unwrap();

        assert_eq!(tokens[0].position(), LineColumn { line: 1, column: 1 }); // let (line 1)
        assert_eq!(tokens[1].position(), LineColumn { line: 1, column: 5 }); // x
        assert_eq!(tokens[5].position(), LineColumn { line: 2, column: 1 }); // let (line 2)
        assert_eq!(tokens[6].position(), LineColumn { line: 2, column: 5 }); // y
    }

    #[test]
    fn test_position_after_single_line_comment() {
        let input = "let x = 10; // comment\nlet y = 20;";
        let tokens = tokenize(input).unwrap();

        assert_eq!(tokens[0].position(), LineColumn { line: 1, column: 1 }); // let
        assert_eq!(tokens[5].position(), LineColumn { line: 2, column: 1 }); // let (after comment)
        assert_eq!(tokens[6].position(), LineColumn { line: 2, column: 5 }); // y
    }

    #[test]
    fn test_position_after_multiline_comment() {
        let input = "let x = 10;\n/* comment\n   on multiple\n   lines */\nlet y = 20;";
        let tokens = tokenize(input).unwrap();

        assert_eq!(tokens[0].position(), LineColumn { line: 1, column: 1 }); // let
        assert_eq!(tokens[1].position(), LineColumn { line: 1, column: 5 }); // x
        assert_eq!(tokens[5].position(), LineColumn { line: 5, column: 1 }); // let (after multi-line comment)
        assert_eq!(tokens[6].position(), LineColumn { line: 5, column: 5 }); // y
    }

    #[test]
    fn test_position_complex_multiline_comment() {
        let input = "struct Point {\n/* This is a\n * multi-line\n * comment\n */\nx: f64\n}";
        let tokens = tokenize(input).unwrap();

        assert_eq!(tokens[0].position(), LineColumn { line: 1, column: 1 }); // struct
        assert_eq!(tokens[1].position(), LineColumn { line: 1, column: 8 }); // Point
        assert_eq!(
            tokens[2].position(),
            LineColumn {
                line: 1,
                column: 14
            }
        ); // {
        assert_eq!(tokens[3].position(), LineColumn { line: 6, column: 1 }); // x (after comment)
        assert_eq!(tokens[4].position(), LineColumn { line: 6, column: 2 }); // :
        assert_eq!(tokens[5].position(), LineColumn { line: 6, column: 4 }); // f64
    }

    #[test]
    fn test_span_for_literals() {
        let input = "123 45.67";
        let tokens = tokenize(input).unwrap();

        if let Token::IntLiteral(ref t) = tokens[0] {
            assert_eq!(t.span.start, LineColumn { line: 1, column: 1 });
            assert_eq!(t.span.end_column, 4);
            assert_eq!(t.span.lines, 0);
        } else {
            panic!("Expected IntLiteral");
        }

        if let Token::FloatLiteral(ref t) = tokens[1] {
            assert_eq!(t.span.start, LineColumn { line: 1, column: 5 });
            assert_eq!(t.span.end_column, 10);
            assert_eq!(t.span.lines, 0);
        } else {
            panic!("Expected FloatLiteral");
        }
    }

    #[test]
    fn test_span_for_identifiers() {
        let input = "hello world";
        let tokens = tokenize(input).unwrap();

        if let Token::Identifier(ref t) = tokens[0] {
            assert_eq!(t.name, "hello");
            assert_eq!(t.span.start, LineColumn { line: 1, column: 1 });
            assert_eq!(t.span.end_column, 6);
            assert_eq!(t.span.lines, 0);
        } else {
            panic!("Expected Identifier");
        }

        if let Token::Identifier(ref t) = tokens[1] {
            assert_eq!(t.name, "world");
            assert_eq!(t.span.start, LineColumn { line: 1, column: 7 });
            assert_eq!(t.span.end_column, 12);
            assert_eq!(t.span.lines, 0);
        } else {
            panic!("Expected Identifier");
        }
    }
}
