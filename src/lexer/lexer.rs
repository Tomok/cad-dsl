use super::token::{ProcessedTokenKind, Token, TokenKind};
use crate::{
    error::{LexError, LexErrorWithSpan},
    ident::IdentArena,
    span::Span,
};
use logos::Logos;

pub fn tokenize(
    source: &str,
    idents: &mut IdentArena,
) -> Result<Vec<Token>, Vec<LexErrorWithSpan>> {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();
    let mut lexer = TokenKind::lexer(source);

    while let Some(result) = lexer.next() {
        let span = Span::new(lexer.span().start, lexer.span().end);

        match result {
            Ok(token_kind) => {
                let processed_kind = match token_kind {
                    TokenKind::Sketch => ProcessedTokenKind::Sketch,
                    TokenKind::Struct => ProcessedTokenKind::Struct,
                    TokenKind::Fn => ProcessedTokenKind::Fn,
                    TokenKind::Let => ProcessedTokenKind::Let,
                    TokenKind::For => ProcessedTokenKind::For,
                    TokenKind::In => ProcessedTokenKind::In,
                    TokenKind::With => ProcessedTokenKind::With,
                    TokenKind::View => ProcessedTokenKind::View,
                    TokenKind::If => ProcessedTokenKind::If,
                    TokenKind::Else => ProcessedTokenKind::Else,
                    TokenKind::Return => ProcessedTokenKind::Return,
                    TokenKind::Import => ProcessedTokenKind::Import,
                    TokenKind::True => ProcessedTokenKind::True,
                    TokenKind::False => ProcessedTokenKind::False,
                    TokenKind::Ident => {
                        let text = lexer.slice();
                        let id = idents.intern(text);
                        ProcessedTokenKind::Ident(id)
                    }
                    TokenKind::IntLiteral(n) => ProcessedTokenKind::IntLiteral(n),
                    TokenKind::FloatLiteral(n) => ProcessedTokenKind::FloatLiteral(n),
                    TokenKind::Millimeter(n) => ProcessedTokenKind::Millimeter(n),
                    TokenKind::Centimeter(n) => ProcessedTokenKind::Centimeter(n),
                    TokenKind::Meter(n) => ProcessedTokenKind::Meter(n),
                    TokenKind::Degree(n) => ProcessedTokenKind::Degree(n),
                    TokenKind::Radian(n) => ProcessedTokenKind::Radian(n),
                    TokenKind::Plus => ProcessedTokenKind::Plus,
                    TokenKind::Minus => ProcessedTokenKind::Minus,
                    TokenKind::Star => ProcessedTokenKind::Star,
                    TokenKind::Slash => ProcessedTokenKind::Slash,
                    TokenKind::Caret => ProcessedTokenKind::Caret,
                    TokenKind::Percent => ProcessedTokenKind::Percent,
                    TokenKind::Eq => ProcessedTokenKind::Eq,
                    TokenKind::NotEq => ProcessedTokenKind::NotEq,
                    TokenKind::Lt => ProcessedTokenKind::Lt,
                    TokenKind::Gt => ProcessedTokenKind::Gt,
                    TokenKind::LtEq => ProcessedTokenKind::LtEq,
                    TokenKind::GtEq => ProcessedTokenKind::GtEq,
                    TokenKind::Assign => ProcessedTokenKind::Assign,
                    TokenKind::Arrow => ProcessedTokenKind::Arrow,
                    TokenKind::And => ProcessedTokenKind::And,
                    TokenKind::Or => ProcessedTokenKind::Or,
                    TokenKind::Not => ProcessedTokenKind::Not,
                    TokenKind::LParen => ProcessedTokenKind::LParen,
                    TokenKind::RParen => ProcessedTokenKind::RParen,
                    TokenKind::LBrace => ProcessedTokenKind::LBrace,
                    TokenKind::RBrace => ProcessedTokenKind::RBrace,
                    TokenKind::LBracket => ProcessedTokenKind::LBracket,
                    TokenKind::RBracket => ProcessedTokenKind::RBracket,
                    TokenKind::Comma => ProcessedTokenKind::Comma,
                    TokenKind::Semicolon => ProcessedTokenKind::Semicolon,
                    TokenKind::Colon => ProcessedTokenKind::Colon,
                    TokenKind::Dot => ProcessedTokenKind::Dot,
                    TokenKind::Ampersand => ProcessedTokenKind::Ampersand,
                    TokenKind::DotDot => ProcessedTokenKind::DotDot,
                    TokenKind::Eof => ProcessedTokenKind::Eof,
                    TokenKind::Error => ProcessedTokenKind::Error,
                };

                tokens.push(Token::new(processed_kind, span));
            }
            Err(_) => {
                // Try to determine what kind of error this is
                let text = lexer.slice();
                let error = if text.chars().any(|c| !c.is_ascii()) {
                    LexError::InvalidChar(text.chars().next().unwrap_or('\0'))
                } else {
                    LexError::InvalidToken
                };

                errors.push(LexErrorWithSpan::new(error, span));

                // Skip this character and continue
                continue;
            }
        }
    }

    // Add EOF token
    let eof_span = Span::new(source.len(), source.len());
    tokens.push(Token::new(ProcessedTokenKind::Eof, eof_span));

    if errors.is_empty() {
        Ok(tokens)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_keywords() {
        let source = "sketch struct fn let for";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens.len(), 6); // 5 keywords + EOF
        assert_eq!(tokens[0].kind, ProcessedTokenKind::Sketch);
        assert_eq!(tokens[1].kind, ProcessedTokenKind::Struct);
        assert_eq!(tokens[2].kind, ProcessedTokenKind::Fn);
        assert_eq!(tokens[3].kind, ProcessedTokenKind::Let);
        assert_eq!(tokens[4].kind, ProcessedTokenKind::For);
        assert_eq!(tokens[5].kind, ProcessedTokenKind::Eof);
    }

    #[test]
    fn test_tokenize_identifiers() {
        let source = "hello_world x y123 _private";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens.len(), 5); // 4 identifiers + EOF

        for i in 0..4 {
            if let ProcessedTokenKind::Ident(id) = &tokens[i].kind {
                match i {
                    0 => assert_eq!(idents.resolve(*id), "hello_world"),
                    1 => assert_eq!(idents.resolve(*id), "x"),
                    2 => assert_eq!(idents.resolve(*id), "y123"),
                    3 => assert_eq!(idents.resolve(*id), "_private"),
                    _ => unreachable!(),
                }
            } else {
                panic!("Expected identifier token");
            }
        }
    }

    #[test]
    fn test_tokenize_numeric_literals() {
        let source = "42 3.14 10mm 5.5cm 1m 90deg 1.57rad";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens.len(), 8); // 7 literals + EOF
        assert_eq!(tokens[0].kind, ProcessedTokenKind::IntLiteral(42));
        assert_eq!(tokens[1].kind, ProcessedTokenKind::FloatLiteral(3.14));
        assert_eq!(tokens[2].kind, ProcessedTokenKind::Millimeter(10.0));
        assert_eq!(tokens[3].kind, ProcessedTokenKind::Centimeter(5.5));
        assert_eq!(tokens[4].kind, ProcessedTokenKind::Meter(1.0));
        assert_eq!(tokens[5].kind, ProcessedTokenKind::Degree(90.0));
        assert_eq!(tokens[6].kind, ProcessedTokenKind::Radian(1.57));
    }

    #[test]
    fn test_tokenize_operators() {
        let source = "+ - * / ^ % == != < > <= >= = -> && || !";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        let expected = vec![
            ProcessedTokenKind::Plus,
            ProcessedTokenKind::Minus,
            ProcessedTokenKind::Star,
            ProcessedTokenKind::Slash,
            ProcessedTokenKind::Caret,
            ProcessedTokenKind::Percent,
            ProcessedTokenKind::Eq,
            ProcessedTokenKind::NotEq,
            ProcessedTokenKind::Lt,
            ProcessedTokenKind::Gt,
            ProcessedTokenKind::LtEq,
            ProcessedTokenKind::GtEq,
            ProcessedTokenKind::Assign,
            ProcessedTokenKind::Arrow,
            ProcessedTokenKind::And,
            ProcessedTokenKind::Or,
            ProcessedTokenKind::Not,
            ProcessedTokenKind::Eof,
        ];

        assert_eq!(tokens.len(), expected.len());
        for (i, expected_kind) in expected.iter().enumerate() {
            assert_eq!(&tokens[i].kind, expected_kind);
        }
    }

    #[test]
    fn test_tokenize_delimiters() {
        let source = "() {} [] , ; : . & ..";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        let expected = vec![
            ProcessedTokenKind::LParen,
            ProcessedTokenKind::RParen,
            ProcessedTokenKind::LBrace,
            ProcessedTokenKind::RBrace,
            ProcessedTokenKind::LBracket,
            ProcessedTokenKind::RBracket,
            ProcessedTokenKind::Comma,
            ProcessedTokenKind::Semicolon,
            ProcessedTokenKind::Colon,
            ProcessedTokenKind::Dot,
            ProcessedTokenKind::Ampersand,
            ProcessedTokenKind::DotDot,
            ProcessedTokenKind::Eof,
        ];

        assert_eq!(tokens.len(), expected.len());
        for (i, expected_kind) in expected.iter().enumerate() {
            assert_eq!(&tokens[i].kind, expected_kind);
        }
    }

    #[test]
    fn test_tokenize_comments() {
        let source = r#"
            // Single line comment
            let x = 10mm; // End of line comment
            /*
             * Multi-line comment
             * spanning multiple lines
             */
            let y = 20mm;
        "#;
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        // Comments should be skipped
        assert_eq!(tokens.len(), 11); // let x = 10mm ; let y = 20mm ; EOF
        assert_eq!(tokens[0].kind, ProcessedTokenKind::Let);
        if let ProcessedTokenKind::Ident(id) = &tokens[1].kind {
            assert_eq!(idents.resolve(*id), "x");
        } else {
            panic!("Expected identifier token");
        }
        assert_eq!(tokens[2].kind, ProcessedTokenKind::Assign);
        assert_eq!(tokens[3].kind, ProcessedTokenKind::Millimeter(10.0));
        assert_eq!(tokens[4].kind, ProcessedTokenKind::Semicolon);
        assert_eq!(tokens[5].kind, ProcessedTokenKind::Let);
        if let ProcessedTokenKind::Ident(id) = &tokens[6].kind {
            assert_eq!(idents.resolve(*id), "y");
        } else {
            panic!("Expected identifier token");
        }
        assert_eq!(tokens[7].kind, ProcessedTokenKind::Assign);
        assert_eq!(tokens[8].kind, ProcessedTokenKind::Millimeter(20.0));
        assert_eq!(tokens[9].kind, ProcessedTokenKind::Semicolon);
        assert_eq!(tokens[10].kind, ProcessedTokenKind::Eof);
    }

    #[test]
    fn test_span_tracking() {
        let source = "let x = 10mm;";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens[0].span, Span::new(0, 3)); // "let"
        assert_eq!(tokens[1].span, Span::new(4, 5)); // "x"
        assert_eq!(tokens[2].span, Span::new(6, 7)); // "="
        assert_eq!(tokens[3].span, Span::new(8, 12)); // "10mm"
        assert_eq!(tokens[4].span, Span::new(12, 13)); // ";"
    }

    #[test]
    fn test_error_recovery() {
        let source = "let x = 10mm; # invalid char; let y = 20mm;";
        let mut idents = IdentArena::new();
        let result = tokenize(source, &mut idents);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);

        match &errors[0].error {
            LexError::InvalidChar('#') => {}
            LexError::InvalidToken => {}
            _ => panic!("Expected invalid character or token error"),
        }
    }

    #[test]
    fn test_complete_sketch() {
        let source = r#"
            sketch simple_triangle {
                let p1: Point = point(0mm, 0mm);
                let p2: Point = point(30mm, 0mm);
                let p3: Point = point();
                
                distance(&p1, &p3) = 40mm;
                distance(&p2, &p3) = 50mm;
            }
        "#;
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents);

        assert!(tokens.is_ok());
        let tokens = tokens.unwrap();
        assert!(tokens.len() > 30); // Should have many tokens
        assert_eq!(tokens.last().unwrap().kind, ProcessedTokenKind::Eof);
    }
}
