use crate::ident::IdentId;
use crate::span::Span;
use logos::Logos;

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: ProcessedTokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: ProcessedTokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProcessedTokenKind {
    // Keywords
    Sketch,
    Struct,
    Fn,
    Let,
    For,
    In,
    With,
    If,
    Else,
    Return,
    Import,
    True,
    False,

    // Identifiers (with interned IDs)
    Ident(IdentId),

    // Numeric literals
    IntLiteral(i64),
    FloatLiteral(f64),

    // Length units
    Millimeter(f64),
    Centimeter(f64),
    Meter(f64),

    // Angle units
    Degree(f64),
    Radian(f64),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,

    // Comparison operators
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,

    // Assignment and arrows
    Assign,
    Arrow,

    // Logical operators
    And,
    Or,
    Not,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    // Punctuation
    Comma,
    Semicolon,
    Colon,
    Dot,
    Ampersand,
    DotDot,

    // Special
    Eof,
    Error,
}

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(skip r"//[^\n]*")]
#[logos(skip r"/\*([^*]|\*[^/])*\*/")]
pub enum TokenKind {
    // Keywords
    #[token("sketch")]
    Sketch,
    #[token("struct")]
    Struct,
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("with")]
    With,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("return")]
    Return,
    #[token("import")]
    Import,
    #[token("true")]
    True,
    #[token("false")]
    False,

    // Identifiers (will be post-processed to intern)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Ident, // Post-processed to add IdentId

    // Numeric literals
    #[regex(r"[0-9]+", parse_int)]
    IntLiteral(i64),

    #[regex(r"[0-9]*\.[0-9]+", parse_float)]
    FloatLiteral(f64),

    // Length units
    #[regex(r"[0-9]+mm", parse_length_mm)]
    #[regex(r"[0-9]*\.[0-9]+mm", parse_length_mm)]
    Millimeter(f64),

    #[regex(r"[0-9]+cm", parse_length_cm)]
    #[regex(r"[0-9]*\.[0-9]+cm", parse_length_cm)]
    Centimeter(f64),

    #[regex(r"[0-9]+m", parse_length_m)]
    #[regex(r"[0-9]*\.[0-9]+m", parse_length_m)]
    Meter(f64),

    // Angle units
    #[regex(r"[0-9]+deg", parse_angle_deg)]
    #[regex(r"[0-9]*\.[0-9]+deg", parse_angle_deg)]
    Degree(f64),

    #[regex(r"[0-9]+rad", parse_angle_rad)]
    #[regex(r"[0-9]*\.[0-9]+rad", parse_angle_rad)]
    Radian(f64),

    // Operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("^")]
    Caret,
    #[token("%")]
    Percent,

    // Comparison operators
    #[token("==")]
    Eq,
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,

    // Assignment and arrows
    #[token("=")]
    Assign,
    #[token("->")]
    Arrow,

    // Logical operators
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Not,

    // Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    // Punctuation
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token("&")]
    Ampersand,
    #[token("..")]
    DotDot,

    // Special
    Eof,
    Error,
}

// Helper functions for parsing numeric literals
fn parse_int(lex: &mut logos::Lexer<TokenKind>) -> Option<i64> {
    lex.slice().parse().ok()
}

fn parse_float(lex: &mut logos::Lexer<TokenKind>) -> Option<f64> {
    lex.slice().parse().ok()
}

// Helper functions for parsing numeric literals with units
fn parse_length_mm(lex: &mut logos::Lexer<TokenKind>) -> Option<f64> {
    let slice = lex.slice();
    let num_part = &slice[..slice.len() - 2]; // Remove "mm"
    num_part.parse().ok()
}

fn parse_length_cm(lex: &mut logos::Lexer<TokenKind>) -> Option<f64> {
    let slice = lex.slice();
    let num_part = &slice[..slice.len() - 2]; // Remove "cm"
    num_part.parse().ok()
}

fn parse_length_m(lex: &mut logos::Lexer<TokenKind>) -> Option<f64> {
    let slice = lex.slice();
    let num_part = &slice[..slice.len() - 1]; // Remove "m"
    num_part.parse().ok()
}

fn parse_angle_deg(lex: &mut logos::Lexer<TokenKind>) -> Option<f64> {
    let slice = lex.slice();
    let num_part = &slice[..slice.len() - 3]; // Remove "deg"
    num_part.parse().ok()
}

fn parse_angle_rad(lex: &mut logos::Lexer<TokenKind>) -> Option<f64> {
    let slice = lex.slice();
    let num_part = &slice[..slice.len() - 3]; // Remove "rad"
    num_part.parse().ok()
}

impl TokenKind {
    /// Check if this token is a keyword that should not be interned as an identifier
    pub fn is_keyword(s: &str) -> bool {
        matches!(
            s,
            "sketch"
                | "struct"
                | "fn"
                | "let"
                | "for"
                | "in"
                | "with"
                | "if"
                | "else"
                | "return"
                | "import"
                | "true"
                | "false"
        )
    }
}

impl Eq for ProcessedTokenKind {}

impl std::hash::Hash for ProcessedTokenKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            ProcessedTokenKind::Ident(id) => id.hash(state),
            ProcessedTokenKind::IntLiteral(n) => n.hash(state),
            ProcessedTokenKind::FloatLiteral(f) => f.to_bits().hash(state),
            ProcessedTokenKind::Millimeter(f) => f.to_bits().hash(state),
            ProcessedTokenKind::Centimeter(f) => f.to_bits().hash(state),
            ProcessedTokenKind::Meter(f) => f.to_bits().hash(state),
            ProcessedTokenKind::Degree(f) => f.to_bits().hash(state),
            ProcessedTokenKind::Radian(f) => f.to_bits().hash(state),
            _ => {}
        }
    }
}

impl std::fmt::Display for ProcessedTokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessedTokenKind::Sketch => write!(f, "sketch"),
            ProcessedTokenKind::Struct => write!(f, "struct"),
            ProcessedTokenKind::Fn => write!(f, "fn"),
            ProcessedTokenKind::Let => write!(f, "let"),
            ProcessedTokenKind::For => write!(f, "for"),
            ProcessedTokenKind::In => write!(f, "in"),
            ProcessedTokenKind::With => write!(f, "with"),
            ProcessedTokenKind::If => write!(f, "if"),
            ProcessedTokenKind::Else => write!(f, "else"),
            ProcessedTokenKind::Return => write!(f, "return"),
            ProcessedTokenKind::Import => write!(f, "import"),
            ProcessedTokenKind::True => write!(f, "true"),
            ProcessedTokenKind::False => write!(f, "false"),
            ProcessedTokenKind::Ident(_) => write!(f, "identifier"),
            ProcessedTokenKind::IntLiteral(n) => write!(f, "{}", n),
            ProcessedTokenKind::FloatLiteral(n) => write!(f, "{}", n),
            ProcessedTokenKind::Millimeter(n) => write!(f, "{}mm", n),
            ProcessedTokenKind::Centimeter(n) => write!(f, "{}cm", n),
            ProcessedTokenKind::Meter(n) => write!(f, "{}m", n),
            ProcessedTokenKind::Degree(n) => write!(f, "{}deg", n),
            ProcessedTokenKind::Radian(n) => write!(f, "{}rad", n),
            ProcessedTokenKind::Plus => write!(f, "+"),
            ProcessedTokenKind::Minus => write!(f, "-"),
            ProcessedTokenKind::Star => write!(f, "*"),
            ProcessedTokenKind::Slash => write!(f, "/"),
            ProcessedTokenKind::Caret => write!(f, "^"),
            ProcessedTokenKind::Percent => write!(f, "%"),
            ProcessedTokenKind::Eq => write!(f, "=="),
            ProcessedTokenKind::NotEq => write!(f, "!="),
            ProcessedTokenKind::Lt => write!(f, "<"),
            ProcessedTokenKind::Gt => write!(f, ">"),
            ProcessedTokenKind::LtEq => write!(f, "<="),
            ProcessedTokenKind::GtEq => write!(f, ">="),
            ProcessedTokenKind::Assign => write!(f, "="),
            ProcessedTokenKind::Arrow => write!(f, "->"),
            ProcessedTokenKind::And => write!(f, "&&"),
            ProcessedTokenKind::Or => write!(f, "||"),
            ProcessedTokenKind::Not => write!(f, "!"),
            ProcessedTokenKind::LParen => write!(f, "("),
            ProcessedTokenKind::RParen => write!(f, ")"),
            ProcessedTokenKind::LBrace => write!(f, "{{"),
            ProcessedTokenKind::RBrace => write!(f, "}}"),
            ProcessedTokenKind::LBracket => write!(f, "["),
            ProcessedTokenKind::RBracket => write!(f, "]"),
            ProcessedTokenKind::Comma => write!(f, ","),
            ProcessedTokenKind::Semicolon => write!(f, ";"),
            ProcessedTokenKind::Colon => write!(f, ":"),
            ProcessedTokenKind::Dot => write!(f, "."),
            ProcessedTokenKind::Ampersand => write!(f, "&"),
            ProcessedTokenKind::DotDot => write!(f, ".."),
            ProcessedTokenKind::Eof => write!(f, "EOF"),
            ProcessedTokenKind::Error => write!(f, "ERROR"),
        }
    }
}
