use std::str::FromStr;

use logos::Lexer;
use logos::Logos;
use logos::Skip;

#[derive(Debug, PartialEq, Eq)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Logos, Debug, PartialEq)]
#[logos(extras = NewLineTracer)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(skip(r"//[^\n]*", newline_callback))]
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")]
pub enum Token<'src> {
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

    // Literals (order matters - float must come before int)
    #[regex(r"\d+\.\d+", parse_with_span_no_newlines)]
    FloatLiteral((f64, Span)),
    #[regex(r"\d+", parse_with_span_no_newlines)]
    IntLiteral((i32, Span)),
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", parse_ident)]
    Identifier((&'src str, Span)),

    // Built-in types
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
}

fn newline_callback<'src>(lex: &mut Lexer<'src, Token<'src>>) -> Skip {
    lex.extras.line += 1;
    lex.extras.last_newline_char_index = lex.span().end;
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

fn parse_with_span_no_newlines<'src, T: FromStr>(
    lex: &mut Lexer<'src, Token<'src>>,
) -> Option<(T, Span)> {
    let value = lex.slice().parse::<T>().ok()?;
    let span = derive_span_no_newline(lex);
    Some((value, span))
}
fn parse_ident<'src>(lex: &mut Lexer<'src, Token<'src>>) -> Option<(&'src str, Span)> {
    let value = lex.slice();
    let span = derive_span_no_newline(lex);
    Some((value, span))
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
                Token::Struct(LineColumn { line: 1, column: 1 }),
                Token::Container(LineColumn { line: 1, column: 8 }),
                Token::Fn(LineColumn {
                    line: 1,
                    column: 18
                }),
                Token::Let(LineColumn {
                    line: 1,
                    column: 21
                }),
                Token::For(LineColumn {
                    line: 1,
                    column: 25
                }),
                Token::In(LineColumn {
                    line: 1,
                    column: 29
                }),
                Token::With(LineColumn {
                    line: 1,
                    column: 32
                }),
                Token::If(LineColumn {
                    line: 1,
                    column: 37
                }),
                Token::Else(LineColumn {
                    line: 1,
                    column: 40
                }),
                Token::Or(LineColumn {
                    line: 1,
                    column: 45
                }),
                Token::And(LineColumn {
                    line: 1,
                    column: 48
                }),
                Token::Return(LineColumn {
                    line: 1,
                    column: 52
                }),
                Token::True(LineColumn {
                    line: 1,
                    column: 59
                }),
                Token::False(LineColumn {
                    line: 1,
                    column: 64
                }),
                Token::SelfKw(LineColumn {
                    line: 1,
                    column: 70
                }),
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
                Token::Equals(LineColumn { line: 1, column: 1 }),
                Token::EqualsEquals(LineColumn { line: 1, column: 3 }),
                Token::NotEquals(LineColumn { line: 1, column: 6 }),
                Token::LessThan(LineColumn { line: 1, column: 9 }),
                Token::GreaterThan(LineColumn {
                    line: 1,
                    column: 11
                }),
                Token::LessEquals(LineColumn {
                    line: 1,
                    column: 13
                }),
                Token::GreaterEquals(LineColumn {
                    line: 1,
                    column: 16
                }),
                Token::Plus(LineColumn {
                    line: 1,
                    column: 19
                }),
                Token::Minus(LineColumn {
                    line: 1,
                    column: 21
                }),
                Token::Multiply(LineColumn {
                    line: 1,
                    column: 23
                }),
                Token::Divide(LineColumn {
                    line: 1,
                    column: 25
                }),
                Token::Power(LineColumn {
                    line: 1,
                    column: 27
                }),
                Token::Modulo(LineColumn {
                    line: 1,
                    column: 29
                }),
                Token::Ampersand(LineColumn {
                    line: 1,
                    column: 31
                }),
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
                Token::Colon(LineColumn { line: 1, column: 1 }),
                Token::SemiColon(LineColumn { line: 1, column: 3 }),
                Token::Comma(LineColumn { line: 1, column: 5 }),
                Token::Dot(LineColumn { line: 1, column: 7 }),
                Token::DotDot(LineColumn { line: 1, column: 9 }),
                Token::LeftParen(LineColumn {
                    line: 1,
                    column: 12
                }),
                Token::RightParen(LineColumn {
                    line: 1,
                    column: 14
                }),
                Token::LeftBracket(LineColumn {
                    line: 1,
                    column: 16
                }),
                Token::RightBracket(LineColumn {
                    line: 1,
                    column: 18
                }),
                Token::LeftBrace(LineColumn {
                    line: 1,
                    column: 20
                }),
                Token::RightBrace(LineColumn {
                    line: 1,
                    column: 22
                }),
                Token::Pipe(LineColumn {
                    line: 1,
                    column: 24
                }),
                Token::Arrow(LineColumn {
                    line: 1,
                    column: 26
                }),
            ]
        );
    }

    #[test]
    fn test_literals() {
        let input = "123 3.45 identifier_name _private";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::IntLiteral((
                    123,
                    Span {
                        start: LineColumn { line: 1, column: 1 },
                        lines: 0,
                        end_column: 4
                    }
                )),
                Token::FloatLiteral((
                    3.45,
                    Span {
                        start: LineColumn { line: 1, column: 5 },
                        lines: 0,
                        end_column: 9
                    }
                )),
                Token::Identifier((
                    "identifier_name",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 10
                        },
                        lines: 0,
                        end_column: 25
                    }
                )),
                Token::Identifier((
                    "_private",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 26
                        },
                        lines: 0,
                        end_column: 34
                    }
                )),
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
                Token::BoolType(LineColumn { line: 1, column: 1 }),
                Token::I32Type(LineColumn { line: 1, column: 6 }),
                Token::F64Type(LineColumn {
                    line: 1,
                    column: 10
                }),
                Token::RealType(LineColumn {
                    line: 1,
                    column: 14
                }),
                Token::AlgebraicType(LineColumn {
                    line: 1,
                    column: 19
                }),
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
                Token::Let(LineColumn { line: 1, column: 1 }),
                Token::Identifier((
                    "x",
                    Span {
                        start: LineColumn { line: 1, column: 5 },
                        lines: 0,
                        end_column: 6
                    }
                )),
                Token::Colon(LineColumn { line: 1, column: 6 }),
                Token::I32Type(LineColumn { line: 1, column: 8 }),
                Token::Equals(LineColumn {
                    line: 1,
                    column: 12
                }),
                Token::IntLiteral((
                    42,
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 14
                        },
                        lines: 0,
                        end_column: 16
                    }
                )),
                Token::SemiColon(LineColumn {
                    line: 1,
                    column: 16
                }),
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
                Token::Struct(LineColumn { line: 1, column: 1 }),
                Token::Identifier((
                    "Point",
                    Span {
                        start: LineColumn { line: 1, column: 8 },
                        lines: 0,
                        end_column: 13
                    }
                )),
                Token::LeftBrace(LineColumn {
                    line: 1,
                    column: 14
                }),
                Token::Identifier((
                    "x",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 16
                        },
                        lines: 0,
                        end_column: 17
                    }
                )),
                Token::Colon(LineColumn {
                    line: 1,
                    column: 17
                }),
                Token::F64Type(LineColumn {
                    line: 1,
                    column: 19
                }),
                Token::Comma(LineColumn {
                    line: 1,
                    column: 22
                }),
                Token::Identifier((
                    "y",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 24
                        },
                        lines: 0,
                        end_column: 25
                    }
                )),
                Token::Colon(LineColumn {
                    line: 1,
                    column: 25
                }),
                Token::F64Type(LineColumn {
                    line: 1,
                    column: 27
                }),
                Token::RightBrace(LineColumn {
                    line: 1,
                    column: 31
                }),
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
                Token::Fn(LineColumn { line: 1, column: 1 }),
                Token::Identifier((
                    "distance",
                    Span {
                        start: LineColumn { line: 1, column: 4 },
                        lines: 0,
                        end_column: 12
                    }
                )),
                Token::LeftParen(LineColumn {
                    line: 1,
                    column: 12
                }),
                Token::Identifier((
                    "p1",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 13
                        },
                        lines: 0,
                        end_column: 15
                    }
                )),
                Token::Colon(LineColumn {
                    line: 1,
                    column: 15
                }),
                Token::Ampersand(LineColumn {
                    line: 1,
                    column: 17
                }),
                Token::Identifier((
                    "Point",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 18
                        },
                        lines: 0,
                        end_column: 23
                    }
                )),
                Token::Comma(LineColumn {
                    line: 1,
                    column: 23
                }),
                Token::Identifier((
                    "p2",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 25
                        },
                        lines: 0,
                        end_column: 27
                    }
                )),
                Token::Colon(LineColumn {
                    line: 1,
                    column: 27
                }),
                Token::Ampersand(LineColumn {
                    line: 1,
                    column: 29
                }),
                Token::Identifier((
                    "Point",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 30
                        },
                        lines: 0,
                        end_column: 35
                    }
                )),
                Token::RightParen(LineColumn {
                    line: 1,
                    column: 35
                }),
                Token::Arrow(LineColumn {
                    line: 1,
                    column: 37
                }),
                Token::F64Type(LineColumn {
                    line: 1,
                    column: 40
                }),
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
                Token::For(LineColumn { line: 1, column: 1 }),
                Token::Identifier((
                    "i",
                    Span {
                        start: LineColumn { line: 1, column: 5 },
                        lines: 0,
                        end_column: 6
                    }
                )),
                Token::In(LineColumn { line: 1, column: 7 }),
                Token::IntLiteral((
                    0,
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 10
                        },
                        lines: 0,
                        end_column: 11
                    }
                )),
                Token::DotDot(LineColumn {
                    line: 1,
                    column: 11
                }),
                Token::IntLiteral((
                    5,
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 13
                        },
                        lines: 0,
                        end_column: 14
                    }
                )),
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
                Token::With(LineColumn { line: 1, column: 1 }),
                Token::Identifier((
                    "transform",
                    Span {
                        start: LineColumn { line: 1, column: 6 },
                        lines: 0,
                        end_column: 15
                    }
                )),
                Token::LeftBrace(LineColumn {
                    line: 1,
                    column: 16
                }),
                Token::Dot(LineColumn {
                    line: 1,
                    column: 18
                }),
                Token::Identifier((
                    "point",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 19
                        },
                        lines: 0,
                        end_column: 24
                    }
                )),
                Token::Equals(LineColumn {
                    line: 1,
                    column: 25
                }),
                Token::Identifier((
                    "p1",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 27
                        },
                        lines: 0,
                        end_column: 29
                    }
                )),
                Token::SemiColon(LineColumn {
                    line: 1,
                    column: 29
                }),
                Token::RightBrace(LineColumn {
                    line: 1,
                    column: 31
                }),
            ]
        );
    }

    #[test]
    fn test_comments_are_skipped() {
        let input = "let x = 42; // This is a comment\nlet y = 3.45; /* Multi-line\n   comment */";
        let tokens = lex_tokens(input);
        assert_eq!(
            tokens,
            vec![
                Token::Let(LineColumn { line: 1, column: 1 }),
                Token::Identifier((
                    "x",
                    Span {
                        start: LineColumn { line: 1, column: 5 },
                        lines: 0,
                        end_column: 6
                    }
                )),
                Token::Equals(LineColumn { line: 1, column: 7 }),
                Token::IntLiteral((
                    42,
                    Span {
                        start: LineColumn { line: 1, column: 9 },
                        lines: 0,
                        end_column: 11
                    }
                )),
                Token::SemiColon(LineColumn {
                    line: 1,
                    column: 11
                }),
                Token::Let(LineColumn { line: 2, column: 2 }),
                Token::Identifier((
                    "y",
                    Span {
                        start: LineColumn { line: 2, column: 6 },
                        lines: 0,
                        end_column: 7
                    }
                )),
                Token::Equals(LineColumn { line: 2, column: 8 }),
                Token::FloatLiteral((
                    3.45,
                    Span {
                        start: LineColumn {
                            line: 2,
                            column: 10
                        },
                        lines: 0,
                        end_column: 14
                    }
                )),
                Token::SemiColon(LineColumn {
                    line: 2,
                    column: 14
                }),
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
                Token::Let(LineColumn { line: 1, column: 3 }),
                Token::Identifier((
                    "x",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 10
                        },
                        lines: 0,
                        end_column: 11
                    }
                )),
                Token::Equals(LineColumn {
                    line: 1,
                    column: 13
                }),
                Token::IntLiteral((
                    42,
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 16
                        },
                        lines: 0,
                        end_column: 18
                    }
                )),
                Token::SemiColon(LineColumn {
                    line: 1,
                    column: 20
                }),
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
                Token::Let(LineColumn { line: 1, column: 1 }),
                Token::Identifier((
                    "points",
                    Span {
                        start: LineColumn { line: 1, column: 5 },
                        lines: 0,
                        end_column: 11
                    }
                )),
                Token::Colon(LineColumn {
                    line: 1,
                    column: 11
                }),
                Token::LeftBracket(LineColumn {
                    line: 1,
                    column: 13
                }),
                Token::Identifier((
                    "Point",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 14
                        },
                        lines: 0,
                        end_column: 19
                    }
                )),
                Token::SemiColon(LineColumn {
                    line: 1,
                    column: 19
                }),
                Token::IntLiteral((
                    5,
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 21
                        },
                        lines: 0,
                        end_column: 22
                    }
                )),
                Token::RightBracket(LineColumn {
                    line: 1,
                    column: 22
                }),
                Token::Equals(LineColumn {
                    line: 1,
                    column: 24
                }),
                Token::LeftBracket(LineColumn {
                    line: 1,
                    column: 26
                }),
                Token::RightBracket(LineColumn {
                    line: 1,
                    column: 27
                }),
                Token::SemiColon(LineColumn {
                    line: 1,
                    column: 28
                }),
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
                Token::Identifier((
                    "circle",
                    Span {
                        start: LineColumn { line: 1, column: 1 },
                        lines: 0,
                        end_column: 7
                    }
                )),
                Token::Dot(LineColumn { line: 1, column: 7 }),
                Token::Identifier((
                    "area",
                    Span {
                        start: LineColumn { line: 1, column: 8 },
                        lines: 0,
                        end_column: 12
                    }
                )),
                Token::LeftParen(LineColumn {
                    line: 1,
                    column: 12
                }),
                Token::RightParen(LineColumn {
                    line: 1,
                    column: 13
                }),
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
                Token::Identifier((
                    "points",
                    Span {
                        start: LineColumn { line: 1, column: 1 },
                        lines: 0,
                        end_column: 7
                    }
                )),
                Token::Dot(LineColumn { line: 1, column: 7 }),
                Token::Identifier((
                    "map",
                    Span {
                        start: LineColumn { line: 1, column: 8 },
                        lines: 0,
                        end_column: 11
                    }
                )),
                Token::LeftParen(LineColumn {
                    line: 1,
                    column: 11
                }),
                Token::Pipe(LineColumn {
                    line: 1,
                    column: 12
                }),
                Token::Identifier((
                    "p",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 13
                        },
                        lines: 0,
                        end_column: 14
                    }
                )),
                Token::Pipe(LineColumn {
                    line: 1,
                    column: 14
                }),
                Token::Identifier((
                    "p",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 16
                        },
                        lines: 0,
                        end_column: 17
                    }
                )),
                Token::Dot(LineColumn {
                    line: 1,
                    column: 17
                }),
                Token::Identifier((
                    "x",
                    Span {
                        start: LineColumn {
                            line: 1,
                            column: 18
                        },
                        lines: 0,
                        end_column: 19
                    }
                )),
                Token::RightParen(LineColumn {
                    line: 1,
                    column: 19
                }),
            ]
        );
    }
}
