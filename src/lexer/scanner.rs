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
                let error = if !text.is_ascii() {
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
    use assert_matches::assert_matches;

    #[test]
    fn test_tokenize_keywords() {
        let source = "sketch struct fn let for";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens.len(), 6); // 5 keywords + EOF
        assert_matches!(tokens[0].kind, ProcessedTokenKind::Sketch);
        assert_matches!(tokens[1].kind, ProcessedTokenKind::Struct);
        assert_matches!(tokens[2].kind, ProcessedTokenKind::Fn);
        assert_matches!(tokens[3].kind, ProcessedTokenKind::Let);
        assert_matches!(tokens[4].kind, ProcessedTokenKind::For);
        assert_matches!(tokens[5].kind, ProcessedTokenKind::Eof);
    }

    #[test]
    fn test_tokenize_all_keywords() {
        let source = "sketch struct fn let for in with if else return import true false";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens.len(), 14); // 13 keywords + EOF
        assert_matches!(tokens[0].kind, ProcessedTokenKind::Sketch);
        assert_matches!(tokens[1].kind, ProcessedTokenKind::Struct);
        assert_matches!(tokens[2].kind, ProcessedTokenKind::Fn);
        assert_matches!(tokens[3].kind, ProcessedTokenKind::Let);
        assert_matches!(tokens[4].kind, ProcessedTokenKind::For);
        assert_matches!(tokens[5].kind, ProcessedTokenKind::In);
        assert_matches!(tokens[6].kind, ProcessedTokenKind::With);
        assert_matches!(tokens[7].kind, ProcessedTokenKind::If);
        assert_matches!(tokens[8].kind, ProcessedTokenKind::Else);
        assert_matches!(tokens[9].kind, ProcessedTokenKind::Return);
        assert_matches!(tokens[10].kind, ProcessedTokenKind::Import);
        assert_matches!(tokens[11].kind, ProcessedTokenKind::True);
        assert_matches!(tokens[12].kind, ProcessedTokenKind::False);
        assert_matches!(tokens[13].kind, ProcessedTokenKind::Eof);
    }

    #[test]
    fn test_tokenize_identifiers() {
        let source = "hello_world x y123 _private";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens.len(), 5); // 4 identifiers + EOF

        // Check that all non-EOF tokens are identifiers and verify their names
        assert_matches!(&tokens[0].kind, ProcessedTokenKind::Ident(id) if idents.resolve(*id) == "hello_world");
        assert_matches!(&tokens[1].kind, ProcessedTokenKind::Ident(id) if idents.resolve(*id) == "x");
        assert_matches!(&tokens[2].kind, ProcessedTokenKind::Ident(id) if idents.resolve(*id) == "y123");
        assert_matches!(&tokens[3].kind, ProcessedTokenKind::Ident(id) if idents.resolve(*id) == "_private");
        assert_matches!(tokens[4].kind, ProcessedTokenKind::Eof);
    }

    #[test]
    fn test_tokenize_numeric_literals() {
        let source = "42 3.14 10mm 5.5cm 1m 90deg 1.57rad";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens.len(), 8); // 7 literals + EOF
        assert_matches!(tokens[0].kind, ProcessedTokenKind::IntLiteral(42));
        assert_matches!(tokens[1].kind, ProcessedTokenKind::FloatLiteral(_));
        assert_matches!(tokens[2].kind, ProcessedTokenKind::Millimeter(_));
        assert_matches!(tokens[3].kind, ProcessedTokenKind::Centimeter(_));
        assert_matches!(tokens[4].kind, ProcessedTokenKind::Meter(_));
        assert_matches!(tokens[5].kind, ProcessedTokenKind::Degree(_));
        assert_matches!(tokens[6].kind, ProcessedTokenKind::Radian(_));
        assert_matches!(tokens[7].kind, ProcessedTokenKind::Eof);

        // Verify exact values
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

        assert_eq!(tokens.len(), 18);
        assert_matches!(tokens[0].kind, ProcessedTokenKind::Plus);
        assert_matches!(tokens[1].kind, ProcessedTokenKind::Minus);
        assert_matches!(tokens[2].kind, ProcessedTokenKind::Star);
        assert_matches!(tokens[3].kind, ProcessedTokenKind::Slash);
        assert_matches!(tokens[4].kind, ProcessedTokenKind::Caret);
        assert_matches!(tokens[5].kind, ProcessedTokenKind::Percent);
        assert_matches!(tokens[6].kind, ProcessedTokenKind::Eq);
        assert_matches!(tokens[7].kind, ProcessedTokenKind::NotEq);
        assert_matches!(tokens[8].kind, ProcessedTokenKind::Lt);
        assert_matches!(tokens[9].kind, ProcessedTokenKind::Gt);
        assert_matches!(tokens[10].kind, ProcessedTokenKind::LtEq);
        assert_matches!(tokens[11].kind, ProcessedTokenKind::GtEq);
        assert_matches!(tokens[12].kind, ProcessedTokenKind::Assign);
        assert_matches!(tokens[13].kind, ProcessedTokenKind::Arrow);
        assert_matches!(tokens[14].kind, ProcessedTokenKind::And);
        assert_matches!(tokens[15].kind, ProcessedTokenKind::Or);
        assert_matches!(tokens[16].kind, ProcessedTokenKind::Not);
        assert_matches!(tokens[17].kind, ProcessedTokenKind::Eof);
    }

    #[test]
    fn test_tokenize_delimiters() {
        let source = "() {} [] , ; : . & ..";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens.len(), 13);
        assert_matches!(tokens[0].kind, ProcessedTokenKind::LParen);
        assert_matches!(tokens[1].kind, ProcessedTokenKind::RParen);
        assert_matches!(tokens[2].kind, ProcessedTokenKind::LBrace);
        assert_matches!(tokens[3].kind, ProcessedTokenKind::RBrace);
        assert_matches!(tokens[4].kind, ProcessedTokenKind::LBracket);
        assert_matches!(tokens[5].kind, ProcessedTokenKind::RBracket);
        assert_matches!(tokens[6].kind, ProcessedTokenKind::Comma);
        assert_matches!(tokens[7].kind, ProcessedTokenKind::Semicolon);
        assert_matches!(tokens[8].kind, ProcessedTokenKind::Colon);
        assert_matches!(tokens[9].kind, ProcessedTokenKind::Dot);
        assert_matches!(tokens[10].kind, ProcessedTokenKind::Ampersand);
        assert_matches!(tokens[11].kind, ProcessedTokenKind::DotDot);
        assert_matches!(tokens[12].kind, ProcessedTokenKind::Eof);
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
        assert_matches!(tokens[0].kind, ProcessedTokenKind::Let);
        assert_matches!(&tokens[1].kind, ProcessedTokenKind::Ident(id) if idents.resolve(*id) == "x");
        assert_matches!(tokens[2].kind, ProcessedTokenKind::Assign);
        assert_matches!(tokens[3].kind, ProcessedTokenKind::Millimeter(_));
        assert_matches!(tokens[4].kind, ProcessedTokenKind::Semicolon);
        assert_matches!(tokens[5].kind, ProcessedTokenKind::Let);
        assert_matches!(&tokens[6].kind, ProcessedTokenKind::Ident(id) if idents.resolve(*id) == "y");
        assert_matches!(tokens[7].kind, ProcessedTokenKind::Assign);
        assert_matches!(tokens[8].kind, ProcessedTokenKind::Millimeter(_));
        assert_matches!(tokens[9].kind, ProcessedTokenKind::Semicolon);
        assert_matches!(tokens[10].kind, ProcessedTokenKind::Eof);
    }

    #[test]
    fn test_error_recovery() {
        let source = "let x = 10mm; # invalid char; let y = 20mm;";
        let mut idents = IdentArena::new();
        let result = tokenize(source, &mut idents);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);

        assert_matches!(
            &errors[0].error,
            LexError::InvalidChar('#') | LexError::InvalidToken
        );
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
        assert_matches!(tokens.last().unwrap().kind, ProcessedTokenKind::Eof);
    }

    // ========================================================================
    // Span tracking tests - explicitly test position determination
    // ========================================================================

    #[test]
    fn test_span_tracking_basic() {
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
    fn test_span_tracking_keywords() {
        let source = "sketch struct fn";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens[0].span, Span::new(0, 6)); // "sketch"
        assert_eq!(tokens[1].span, Span::new(7, 13)); // "struct"
        assert_eq!(tokens[2].span, Span::new(14, 16)); // "fn"
    }

    #[test]
    fn test_span_tracking_operators() {
        let source = "== != <= >=";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens[0].span, Span::new(0, 2)); // "=="
        assert_eq!(tokens[1].span, Span::new(3, 5)); // "!="
        assert_eq!(tokens[2].span, Span::new(6, 8)); // "<="
        assert_eq!(tokens[3].span, Span::new(9, 11)); // ">="
    }

    #[test]
    fn test_span_tracking_numeric_literals() {
        let source = "42 3.14 10mm 5.5cm";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens[0].span, Span::new(0, 2)); // "42"
        assert_eq!(tokens[1].span, Span::new(3, 7)); // "3.14"
        assert_eq!(tokens[2].span, Span::new(8, 12)); // "10mm"
        assert_eq!(tokens[3].span, Span::new(13, 18)); // "5.5cm"
    }

    #[test]
    fn test_span_tracking_identifiers() {
        let source = "hello_world x y123";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens[0].span, Span::new(0, 11)); // "hello_world"
        assert_eq!(tokens[1].span, Span::new(12, 13)); // "x"
        assert_eq!(tokens[2].span, Span::new(14, 18)); // "y123"
    }

    #[test]
    fn test_span_tracking_multiline() {
        let source = "let x = 10mm;\nlet y = 20mm;";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        // First line: "let x = 10mm;"
        assert_eq!(tokens[0].span, Span::new(0, 3)); // "let"
        assert_eq!(tokens[1].span, Span::new(4, 5)); // "x"
        assert_eq!(tokens[2].span, Span::new(6, 7)); // "="
        assert_eq!(tokens[3].span, Span::new(8, 12)); // "10mm"
        assert_eq!(tokens[4].span, Span::new(12, 13)); // ";"

        // Second line: "let y = 20mm;" (starts at position 14 after newline)
        assert_eq!(tokens[5].span, Span::new(14, 17)); // "let"
        assert_eq!(tokens[6].span, Span::new(18, 19)); // "y"
        assert_eq!(tokens[7].span, Span::new(20, 21)); // "="
        assert_eq!(tokens[8].span, Span::new(22, 26)); // "20mm"
        assert_eq!(tokens[9].span, Span::new(26, 27)); // ";"
    }

    #[test]
    fn test_span_tracking_with_comments() {
        let source = "let x = 10mm; // comment\nlet y = 20mm;";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        // First line: "let x = 10mm;" (comment is skipped)
        assert_eq!(tokens[0].span, Span::new(0, 3)); // "let"
        assert_eq!(tokens[1].span, Span::new(4, 5)); // "x"
        assert_eq!(tokens[2].span, Span::new(6, 7)); // "="
        assert_eq!(tokens[3].span, Span::new(8, 12)); // "10mm"
        assert_eq!(tokens[4].span, Span::new(12, 13)); // ";"

        // Second line: "let y = 20mm;" (starts at position 25 after comment and newline)
        assert_eq!(tokens[5].span, Span::new(25, 28)); // "let"
        assert_eq!(tokens[6].span, Span::new(29, 30)); // "y"
        assert_eq!(tokens[7].span, Span::new(31, 32)); // "="
        assert_eq!(tokens[8].span, Span::new(33, 37)); // "20mm"
        assert_eq!(tokens[9].span, Span::new(37, 38)); // ";"
    }

    #[test]
    fn test_span_tracking_delimiters() {
        let source = "(){}[]";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens[0].span, Span::new(0, 1)); // "("
        assert_eq!(tokens[1].span, Span::new(1, 2)); // ")"
        assert_eq!(tokens[2].span, Span::new(2, 3)); // "{"
        assert_eq!(tokens[3].span, Span::new(3, 4)); // "}"
        assert_eq!(tokens[4].span, Span::new(4, 5)); // "["
        assert_eq!(tokens[5].span, Span::new(5, 6)); // "]"
    }

    #[test]
    fn test_span_tracking_dotdot() {
        let source = ". .. ...";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        assert_eq!(tokens[0].span, Span::new(0, 1)); // "."
        assert_eq!(tokens[1].span, Span::new(2, 4)); // ".."
        // "..." should be tokenized as ".." followed by "."
        assert_eq!(tokens[2].span, Span::new(5, 7)); // ".."
        assert_eq!(tokens[3].span, Span::new(7, 8)); // "."
    }

    #[test]
    fn test_span_tracking_error_location() {
        let source = "let x = #;";
        let mut idents = IdentArena::new();
        let result = tokenize(source, &mut idents);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].span, Span::new(8, 9)); // "#" at position 8
    }

    #[test]
    fn test_span_tracking_eof() {
        let source = "let x";
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).unwrap();

        // EOF token should be at the end of the source
        let eof_token = tokens.last().unwrap();
        assert_matches!(eof_token.kind, ProcessedTokenKind::Eof);
        assert_eq!(eof_token.span, Span::new(source.len(), source.len()));
    }
}
