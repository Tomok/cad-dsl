use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::ident::IdentId;
use crate::span::Span;
use chumsky::prelude::*;
use std::ops::Range;

pub type TokenStream = ProcessedTokenKind;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
}

// Simple non-recursive parsers for basic constructs
pub fn simple_parser() -> impl Parser<TokenStream, UnresolvedAst, Error = Simple<ProcessedTokenKind>> + Clone {
    let sketch = just(ProcessedTokenKind::Sketch)
        .ignore_then(ident())
        .then(
            simple_stmt()
                .repeated()
                .delimited_by(
                    just(ProcessedTokenKind::LBrace),
                    just(ProcessedTokenKind::RBrace)
                )
        )
        .map_with_span(|(name, body), span| {
            SketchDef {
                name,
                body,
                span: range_to_span(span),
            }
        });

    let function_def = just(ProcessedTokenKind::Fn)
        .ignore_then(ident())
        .then(
            just(ProcessedTokenKind::LParen)
                .ignore_then(just(ProcessedTokenKind::RParen))
        )
        .then(
            just(ProcessedTokenKind::Arrow)
                .ignore_then(simple_type())
                .or_not()
        )
        .then(
            simple_stmt()
                .repeated()
                .delimited_by(
                    just(ProcessedTokenKind::LBrace),
                    just(ProcessedTokenKind::RBrace)
                )
        )
        .map_with_span(|(((name, _), return_type), body), span| FunctionDef {
            name,
            params: Vec::new(), // Simplified - no parameters for now
            return_type,
            body,
            span: range_to_span(span),
        });

    let struct_def = just(ProcessedTokenKind::Struct)
        .ignore_then(ident())
        .then(
            choice((
                field_def().map(StructItem::Field),
                function_def.map(StructItem::Method),
            ))
            .repeated()
            .delimited_by(
                just(ProcessedTokenKind::LBrace),
                just(ProcessedTokenKind::RBrace)
            )
        )
        .map_with_span(|(name, items), span| {
            let mut fields = Vec::new();
            let mut methods = Vec::new();
            
            for item in items {
                match item {
                    StructItem::Field(field) => fields.push(field),
                    StructItem::Method(method) => methods.push(method),
                }
            }
            
            StructDef {
                name,
                fields,
                methods,
                span: range_to_span(span),
            }
        });

    choice((
        sketch.map(|s| (Some(s), None)),
        struct_def.map(|s| (None, Some(s))),
    ))
    .repeated()
    .then_ignore(just(ProcessedTokenKind::Eof))
    .map(|items| {
        let mut sketches = Vec::new();
        let mut structs = Vec::new();
        
        for (sketch, struct_def) in items {
            if let Some(s) = sketch {
                sketches.push(s);
            }
            if let Some(s) = struct_def {
                structs.push(s);
            }
        }
        
        UnresolvedAst {
            imports: Vec::new(),
            structs,
            sketches,
        }
    })
}

fn field_def() -> impl Parser<TokenStream, FieldDef, Error = Simple<ProcessedTokenKind>> + Clone {
    ident()
        .then_ignore(just(ProcessedTokenKind::Colon))
        .then(simple_type())
        .then_ignore(just(ProcessedTokenKind::Comma).or_not())
        .map_with_span(|(name, ty), span| FieldDef {
            name,
            ty,
            span: range_to_span(span),
        })
}

fn simple_type() -> impl Parser<TokenStream, TypeRef, Error = Simple<ProcessedTokenKind>> + Clone {
    choice((
        // Array type: [Type; size]
        just(ProcessedTokenKind::LBracket)
            .ignore_then(ident())
            .then_ignore(just(ProcessedTokenKind::Semicolon))
            .then(select! { ProcessedTokenKind::IntLiteral(n) => n })
            .then_ignore(just(ProcessedTokenKind::RBracket))
            .map_with_span(|(name, size), span| TypeRef {
                name,
                is_reference: false,
                array_size: Some(Box::new(Expr::Literal {
                    kind: LiteralKind::Int(size),
                    span: range_to_span(span.clone()),
                })),
                span: range_to_span(span),
            }),
        // Reference type: &Type
        just(ProcessedTokenKind::Ampersand)
            .ignore_then(ident())
            .map_with_span(|name, span| TypeRef {
                name,
                is_reference: true,
                array_size: None,
                span: range_to_span(span),
            }),
        // Basic type
        ident().map_with_span(|name, span| TypeRef {
            name,
            is_reference: false,
            array_size: None,
            span: range_to_span(span),
        })
    ))
}

fn simple_stmt() -> impl Parser<TokenStream, Stmt, Error = Simple<ProcessedTokenKind>> + Clone {
    choice((
        simple_let_stmt(),
        simple_assign_stmt(),
        simple_expr_stmt(),
    ))
}

fn simple_let_stmt() -> impl Parser<TokenStream, Stmt, Error = Simple<ProcessedTokenKind>> + Clone {
    just(ProcessedTokenKind::Let)
        .ignore_then(ident())
        .then_ignore(just(ProcessedTokenKind::Colon))
        .then(simple_type())
        .then(
            just(ProcessedTokenKind::Assign)
                .ignore_then(simple_expr())
                .or_not(),
        )
        .then_ignore(just(ProcessedTokenKind::Semicolon))
        .map_with_span(|((name, ty), init), span| Stmt::Let {
            name,
            ty: Some(ty),
            init,
            span: range_to_span(span),
        })
}

fn simple_assign_stmt() -> impl Parser<TokenStream, Stmt, Error = Simple<ProcessedTokenKind>> + Clone {
    simple_expr()
        .then_ignore(just(ProcessedTokenKind::Assign))
        .then(simple_expr())
        .then_ignore(just(ProcessedTokenKind::Semicolon))
        .map_with_span(|(target, value), span| Stmt::Assign {
            target,
            value,
            span: range_to_span(span),
        })
}

fn simple_for_stmt() -> impl Parser<TokenStream, Stmt, Error = Simple<ProcessedTokenKind>> + Clone {
    just(ProcessedTokenKind::For)
        .ignore_then(ident())
        .then_ignore(just(ProcessedTokenKind::In))
        .then(simple_range_expr())
        .then(
            simple_stmt()
                .repeated()
                .delimited_by(
                    just(ProcessedTokenKind::LBrace),
                    just(ProcessedTokenKind::RBrace)
                )
        )
        .map_with_span(|((var, range), body), span| Stmt::For {
            var,
            range,
            body,
            span: range_to_span(span),
        })
}

fn simple_with_stmt() -> impl Parser<TokenStream, Stmt, Error = Simple<ProcessedTokenKind>> + Clone {
    just(ProcessedTokenKind::With)
        .ignore_then(ident())
        .then(
            simple_stmt()
                .repeated()
                .delimited_by(
                    just(ProcessedTokenKind::LBrace),
                    just(ProcessedTokenKind::RBrace)
                )
        )
        .map_with_span(|(view, body), span| Stmt::With {
            view: Expr::Ident { name: view, span: range_to_span(span.clone()) },
            body,
            span: range_to_span(span),
        })
}

fn simple_expr_stmt() -> impl Parser<TokenStream, Stmt, Error = Simple<ProcessedTokenKind>> + Clone {
    simple_expr()
        .then_ignore(just(ProcessedTokenKind::Semicolon))
        .map(Stmt::Expr)
}

fn simple_range_expr() -> impl Parser<TokenStream, Expr, Error = Simple<ProcessedTokenKind>> + Clone {
    select! { ProcessedTokenKind::IntLiteral(start) => start }
        .then_ignore(just(ProcessedTokenKind::DotDot))
        .then(select! { ProcessedTokenKind::IntLiteral(end) => end })
        .map_with_span(|(start, end), span: Range<usize>| Expr::Range {
            start: Box::new(Expr::Literal {
                kind: LiteralKind::Int(start),
                span: range_to_span(span.clone()),
            }),
            end: Box::new(Expr::Literal {
                kind: LiteralKind::Int(end),
                span: range_to_span(span.clone()),
            }),
            span: range_to_span(span),
        })
}

fn simple_expr() -> impl Parser<TokenStream, Expr, Error = Simple<ProcessedTokenKind>> + Clone {
    let literal = choice((
        select! { ProcessedTokenKind::IntLiteral(n) => LiteralKind::Int(n) },
        select! { ProcessedTokenKind::FloatLiteral(f) => LiteralKind::Float(f) },
        select! { ProcessedTokenKind::True => LiteralKind::Bool(true) },
        select! { ProcessedTokenKind::False => LiteralKind::Bool(false) },
        select! { ProcessedTokenKind::Millimeter(f) => LiteralKind::Length {
            value: f,
            unit: LengthUnit::Millimeter
        }},
        select! { ProcessedTokenKind::Centimeter(f) => LiteralKind::Length {
            value: f,
            unit: LengthUnit::Centimeter
        }},
        select! { ProcessedTokenKind::Meter(f) => LiteralKind::Length {
            value: f,
            unit: LengthUnit::Meter
        }},
        select! { ProcessedTokenKind::Degree(f) => LiteralKind::Angle {
            value: f,
            unit: AngleUnit::Degree
        }},
        select! { ProcessedTokenKind::Radian(f) => LiteralKind::Angle {
            value: f,
            unit: AngleUnit::Radian
        }},
    ))
    .map_with_span(|kind, span| Expr::Literal {
        kind,
        span: range_to_span(span),
    });

    let ident_expr = ident().map_with_span(|name, span| Expr::Ident {
        name,
        span: range_to_span(span),
    });

    // Simple function call (non-recursive for now)
    let call_expr = ident()
        .then(
            ident_expr.clone()
                .separated_by(just(ProcessedTokenKind::Comma))
                .delimited_by(
                    just(ProcessedTokenKind::LParen),
                    just(ProcessedTokenKind::RParen)
                )
                .or_not()
        )
        .map_with_span(|(name, args), span| {
            if let Some(args) = args {
                Expr::Call {
                    func: Box::new(Expr::Ident { 
                        name, 
                        span: range_to_span(span.clone()) 
                    }),
                    args,
                    span: range_to_span(span),
                }
            } else {
                Expr::Ident {
                    name,
                    span: range_to_span(span),
                }
            }
        });

    // Add support for parentheses and simple binary operators
    let primary = choice((literal, ident_expr.clone()));
    
    // Parenthesized expressions - but avoid recursion by only allowing literals and identifiers inside
    let paren_expr = choice((literal.clone(), ident_expr.clone()))
        .delimited_by(
            just(ProcessedTokenKind::LParen),
            just(ProcessedTokenKind::RParen)
        );
    
    let atom = choice((paren_expr, primary, call_expr));
    
    // Simple binary operators (left-to-right, no precedence for now)
    let binary_expr = atom.clone()
        .then(
            choice((
                just(ProcessedTokenKind::Plus).to(BinOp::Add),
                just(ProcessedTokenKind::Minus).to(BinOp::Sub),
                just(ProcessedTokenKind::Star).to(BinOp::Mul),
                just(ProcessedTokenKind::Slash).to(BinOp::Div),
            ))
            .then(atom)
            .or_not()
        )
        .map_with_span(|(left, op_right), span| {
            if let Some((op, right)) = op_right {
                Expr::BinaryOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                    span: range_to_span(span),
                }
            } else {
                left
            }
        });
        
    binary_expr
}

fn ident() -> impl Parser<TokenStream, IdentId, Error = Simple<ProcessedTokenKind>> + Clone {
    select! { ProcessedTokenKind::Ident(id) => id }
}

#[derive(Debug, Clone)]
enum StructItem {
    Field(FieldDef),
    Method(FunctionDef),
}