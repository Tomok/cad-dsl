use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(skip r"//[^\n]*")]
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
enum Token {
    // Keywords
    #[token("struct")]
    Struct,
    #[token("container")]
    Container,
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
    #[token("or")]
    Or,
    #[token("and")]
    And,
    #[token("return")]
    Return,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("self")]
    SelfKw,

    // Operators
    #[token("=")]
    Equals,
    #[token("==")]
    EqualsEquals,
    #[token("!=")]
    NotEquals,
    #[token("<")]
    LessThan,
    #[token(">")]
    GreaterThan,
    #[token("<=")]
    LessEquals,
    #[token(">=")]
    GreaterEquals,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Multiply,
    #[token("/")]
    Divide,
    #[token("^")]
    Power,
    #[token("%")]
    Modulo,
    #[token("&")]
    Ampersand,

    // Punctuation
    #[token(":")]
    Colon,
    #[token(";")]
    SemiColon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("..")]
    DotDot,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("|")]
    Pipe,
    #[token("->")]
    Arrow,

    // Literals (order matters - float must come before int)
    #[regex(r"\d+\.\d+", |lex| lex.slice().parse::<f64>().ok())]
    FloatLiteral(f64),
    #[regex(r"\d+", |lex| lex.slice().parse::<i32>().ok())]
    IntLiteral(i32),
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    // Built-in types
    #[token("bool")]
    BoolType,
    #[token("i32")]
    I32Type,
    #[token("f64")]
    F64Type,
    #[token("Real")]
    RealType,
    #[token("Algebraic")]
    AlgebraicType,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex_tokens(input: &str) -> Vec<Token> {
        let mut lexer = Token::lexer(input);
        let mut tokens = Vec::new();
        while let Some(token) = lexer.next() {
            match token {
                Ok(t) => tokens.push(t),
                Err(_) => panic!("Lexing error at position {}", lexer.span().start),
            }
        }
        tokens
    }

    #[test]
    fn test_keywords() {
        let input = "struct container fn let for in with if else or and return true false self";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Struct,
                Token::Container,
                Token::Fn,
                Token::Let,
                Token::For,
                Token::In,
                Token::With,
                Token::If,
                Token::Else,
                Token::Or,
                Token::And,
                Token::Return,
                Token::True,
                Token::False,
                Token::SelfKw,
            ]
        );
    }

    #[test]
    fn test_operators() {
        let input = "= == != < > <= >= + - * / ^ % &";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Equals,
                Token::EqualsEquals,
                Token::NotEquals,
                Token::LessThan,
                Token::GreaterThan,
                Token::LessEquals,
                Token::GreaterEquals,
                Token::Plus,
                Token::Minus,
                Token::Multiply,
                Token::Divide,
                Token::Power,
                Token::Modulo,
                Token::Ampersand,
            ]
        );
    }

    #[test]
    fn test_punctuation() {
        let input = ": ; , . .. ( ) [ ] { } | ->";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Colon,
                Token::SemiColon,
                Token::Comma,
                Token::Dot,
                Token::DotDot,
                Token::LeftParen,
                Token::RightParen,
                Token::LeftBracket,
                Token::RightBracket,
                Token::LeftBrace,
                Token::RightBrace,
                Token::Pipe,
                Token::Arrow,
            ]
        );
    }

    #[test]
    fn test_literals() {
        let input = "123 3.14 identifier_name _private";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::IntLiteral(123),
                Token::FloatLiteral(3.14),
                Token::Identifier,
                Token::Identifier,
            ]
        );
    }

    #[test]
    fn test_types() {
        let input = "bool i32 f64 Real Algebraic";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::BoolType,
                Token::I32Type,
                Token::F64Type,
                Token::RealType,
                Token::AlgebraicType,
            ]
        );
    }

    #[test]
    fn test_simple_expression() {
        let input = "let x: i32 = 42;";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Identifier,
                Token::Colon,
                Token::I32Type,
                Token::Equals,
                Token::IntLiteral(42),
                Token::SemiColon,
            ]
        );
    }

    #[test]
    fn test_struct_definition() {
        let input = "struct Point { x: f64, y: f64 }";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Struct,
                Token::Identifier,
                Token::LeftBrace,
                Token::Identifier,
                Token::Colon,
                Token::F64Type,
                Token::Comma,
                Token::Identifier,
                Token::Colon,
                Token::F64Type,
                Token::RightBrace,
            ]
        );
    }

    #[test]
    fn test_function_definition() {
        let input = "fn distance(p1: &Point, p2: &Point) -> f64";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Fn,
                Token::Identifier,
                Token::LeftParen,
                Token::Identifier,
                Token::Colon,
                Token::Ampersand,
                Token::Identifier,
                Token::Comma,
                Token::Identifier,
                Token::Colon,
                Token::Ampersand,
                Token::Identifier,
                Token::RightParen,
                Token::Arrow,
                Token::F64Type,
            ]
        );
    }

    #[test]
    fn test_range_syntax() {
        let input = "for i in 0..5";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::For,
                Token::Identifier,
                Token::In,
                Token::IntLiteral(0),
                Token::DotDot,
                Token::IntLiteral(5),
            ]
        );
    }

    #[test]
    fn test_with_statement() {
        let input = "with transform { .point = p1; }";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::With,
                Token::Identifier,
                Token::LeftBrace,
                Token::Dot,
                Token::Identifier,
                Token::Equals,
                Token::Identifier,
                Token::SemiColon,
                Token::RightBrace,
            ]
        );
    }

    #[test]
    fn test_comments_are_skipped() {
        let input = "let x = 42; // This is a comment\nlet y = 3.14; /* Multi-line\n   comment */";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Identifier,
                Token::Equals,
                Token::IntLiteral(42),
                Token::SemiColon,
                Token::Let,
                Token::Identifier,
                Token::Equals,
                Token::FloatLiteral(3.14),
                Token::SemiColon,
            ]
        );
    }

    #[test]
    fn test_whitespace_is_skipped() {
        let input = "  let    x  =  42  ;  ";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Identifier,
                Token::Equals,
                Token::IntLiteral(42),
                Token::SemiColon,
            ]
        );
    }

    #[test]
    fn test_array_syntax() {
        let input = "let points: [Point; 5] = [];";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Let,
                Token::Identifier,
                Token::Colon,
                Token::LeftBracket,
                Token::Identifier,
                Token::SemiColon,
                Token::IntLiteral(5),
                Token::RightBracket,
                Token::Equals,
                Token::LeftBracket,
                Token::RightBracket,
                Token::SemiColon,
            ]
        );
    }

    #[test]
    fn test_method_call() {
        let input = "circle.area()";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Identifier,
                Token::Dot,
                Token::Identifier,
                Token::LeftParen,
                Token::RightParen,
            ]
        );
    }

    #[test]
    fn test_closure_syntax() {
        let input = "points.map(|p| p.x)";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Identifier,
                Token::Dot,
                Token::Identifier,
                Token::LeftParen,
                Token::Pipe,
                Token::Identifier,
                Token::Pipe,
                Token::Identifier,
                Token::Dot,
                Token::Identifier,
                Token::RightParen,
            ]
        );
    }
}
