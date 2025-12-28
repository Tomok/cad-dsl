//! Statement and type annotation parsers

use crate::ast::span::HasSpan;
use crate::ast::{FunctionParam, Stmt, Type};
use crate::lexer::Token;
use crate::parser::ParseError;
use chumsky::prelude::*;

// ============================================================================
// Type Annotation Parser
// ============================================================================

/// Parse type annotations (bool, i32, f64, Real, Algebraic, &Type, UserType)
pub fn type_annotation<'src>()
-> impl Parser<'src, &'src [Token<'src>], Type, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let base_type = choice((
        select! {
            Token::BoolType(t) => Type::Bool {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 4 }
            },
        },
        select! {
            Token::I32Type(t) => Type::I32 {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 3 }
            },
        },
        select! {
            Token::F64Type(t) => Type::F64 {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 3 }
            },
        },
        select! {
            Token::RealType(t) => Type::Real {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 4 }
            },
        },
        select! {
            Token::AlgebraicType(t) => Type::Algebraic {
                span: Span { start: t.position, lines: 0, end_column: t.position.column + 9 }
            },
        },
        select! {
            Token::Identifier(t) => Type::UserDefined {
                name: t.name.to_string(),
                span: t.span,
            },
        },
    ));

    // Reference type: &Type
    // Only support single-level references (no &&Type)
    let reference_type = select! {
        Token::Ampersand(t) => t.position,
    }
    .then(base_type.clone())
    .map(|(amp_pos, inner_type)| {
        let inner_span = inner_type.span();
        let span = if amp_pos.line == inner_span.start.line {
            Span {
                start: amp_pos,
                lines: 0,
                end_column: inner_span.end_column,
            }
        } else {
            Span {
                start: amp_pos,
                lines: inner_span.start.line - amp_pos.line + inner_span.lines,
                end_column: inner_span.end_column,
            }
        };
        Type::Reference {
            inner: Box::new(inner_type),
            span,
        }
    });

    choice((reference_type, base_type)).labelled("type annotation")
}

// ============================================================================
// Statement Parser
// ============================================================================

/// Parse a let statement
///
/// Syntax:
///   let <name>: <type> = <expr>;
///   let <name>: <type>;
///   let <name> = <expr>;
///   let <name>;
pub fn let_stmt<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let colon = select! { Token::Colon(_) => () };
    let equals = select! { Token::Equals(_) => () };

    select! {
        Token::Let(t) => t.position,
    }
    .then(
        select! {
            Token::Identifier(t) => (t.name, t.span),
        }
        .labelled("variable name"),
    )
    .then(
        // Optional type annotation: : <type>
        colon.ignore_then(type_annotation()).or_not(),
    )
    .then(
        // Optional initialization: = <expr>
        equals.ignore_then(expr_parser).or_not(),
    )
    .then(select! {
        Token::SemiColon(t) => t.position,
    })
    .map(
        |((((let_pos, (name, name_span)), type_annotation), init), semi_pos)| {
            // Construct span from let keyword to semicolon
            let span = if let_pos.line == semi_pos.line {
                // Same line
                Span {
                    start: let_pos,
                    lines: 0,
                    end_column: semi_pos.column + 1,
                }
            } else {
                // Multiple lines
                Span {
                    start: let_pos,
                    lines: semi_pos.line - let_pos.line,
                    end_column: semi_pos.column + 1,
                }
            };

            Stmt::Let {
                name,
                name_span,
                type_annotation,
                init,
                span,
            }
        },
    )
    .labelled("let statement")
}

// ============================================================================
// Function Definition Parser
// ============================================================================

/// Parse a function parameter: name: Type
pub fn function_param<'src>()
-> impl Parser<'src, &'src [Token<'src>], FunctionParam, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let colon = select! { Token::Colon(_) => () };

    select! {
        Token::Identifier(t) => (t.name.to_string(), t.span),
    }
    .labelled("parameter name")
    .then_ignore(colon)
    .then(type_annotation())
    .map(|((name, name_span), type_annotation)| {
        let type_span = type_annotation.span();
        let span = if name_span.start.line == type_span.start.line {
            Span {
                start: name_span.start,
                lines: 0,
                end_column: type_span.end_column,
            }
        } else {
            Span {
                start: name_span.start,
                lines: type_span.start.line - name_span.start.line + type_span.lines,
                end_column: type_span.end_column,
            }
        };
        FunctionParam {
            name,
            name_span,
            type_annotation,
            span,
        }
    })
    .labelled("function parameter")
}

/// Parse a function definition
///
/// Syntax:
///   fn name(param1: Type1, param2: Type2) -> ReturnType { body }
///   fn name() -> ReturnType { body }
pub fn function_def<'src>(
    expr_parser: impl Parser<'src, &'src [Token<'src>], crate::ast::Expr<'src>, ParseError<'src>>
    + Clone
    + 'src,
) -> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    use crate::lexer::Span;

    let left_paren = select! { Token::LeftParen(_) => () };
    let right_paren = select! { Token::RightParen(_) => () };
    let comma = select! { Token::Comma(_) => () };
    let arrow = select! { Token::Arrow(_) => () };
    let left_brace = select! { Token::LeftBrace(_) => () };
    let right_brace = select! { Token::RightBrace(t) => t.position };

    // For now, function bodies only contain let statements
    // (nested function definitions will be added later if needed)
    let stmt_parser = let_stmt(expr_parser.clone());

    select! {
        Token::Fn(t) => t.position,
    }
    .then(
        select! {
            Token::Identifier(t) => (t.name.to_string(), t.span),
        }
        .labelled("function name"),
    )
    .then_ignore(left_paren)
    .then(
        // Parameter list: param1: Type1, param2: Type2, ...
        function_param()
            .separated_by(comma)
            .allow_trailing()
            .collect::<Vec<_>>(),
    )
    .then_ignore(right_paren)
    .then_ignore(arrow)
    .then(type_annotation().labelled("return type"))
    .then_ignore(left_brace)
    .then(
        // Function body: statements followed by optional return expression
        stmt_parser
            .repeated()
            .collect::<Vec<_>>()
            .then(expr_parser.or_not()),
    )
    .then(right_brace)
    .map(
        |(
            ((((fn_pos, (name, name_span)), params), return_type), (body, return_expr)),
            brace_pos,
        )| {
            // Construct span from fn keyword to closing brace
            let span = if fn_pos.line == brace_pos.line {
                Span {
                    start: fn_pos,
                    lines: 0,
                    end_column: brace_pos.column + 1,
                }
            } else {
                Span {
                    start: fn_pos,
                    lines: brace_pos.line - fn_pos.line,
                    end_column: brace_pos.column + 1,
                }
            };

            Stmt::FunctionDef {
                name,
                name_span,
                params,
                return_type,
                body,
                return_expr,
                span,
            }
        },
    )
    .labelled("function definition")
}
