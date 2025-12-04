use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::span::Span;
use chumsky::prelude::*;
use std::ops::Range;

pub type TokenStream = ProcessedTokenKind;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
}

pub fn recursive_parser() -> impl Parser<TokenStream, UnresolvedAst, Error = Simple<ProcessedTokenKind>> + Clone {
    // Declare all recursive types first with explicit types
    let mut expr: Recursive<TokenStream, Expr, Simple<ProcessedTokenKind>> = Recursive::declare();
    let mut stmt: Recursive<TokenStream, Stmt, Simple<ProcessedTokenKind>> = Recursive::declare();
    let mut type_ref: Recursive<TokenStream, TypeRef, Simple<ProcessedTokenKind>> = Recursive::declare();

    // Helper parsers
    let ident = select! { ProcessedTokenKind::Ident(id) => id };

    // Literal parser
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

    // Define type_ref first since it's used by other parsers
    type_ref.define(choice((
        // Array type: [Type; size]
        just(ProcessedTokenKind::LBracket)
            .ignore_then(type_ref.clone())
            .then_ignore(just(ProcessedTokenKind::Semicolon))
            .then(expr.clone())
            .then_ignore(just(ProcessedTokenKind::RBracket))
            .map_with_span(|(ty, size), span| TypeRef {
                name: ty.name,
                is_reference: false,
                array_size: Some(Box::new(size)),
                span: range_to_span(span),
            }),
        // Reference type: &Type
        just(ProcessedTokenKind::Ampersand)
            .ignore_then(type_ref.clone())
            .map(|ty| TypeRef {
                name: ty.name,
                is_reference: true,
                array_size: ty.array_size,
                span: ty.span,
            }),
        // Basic type
        ident.clone().map_with_span(|name, span| TypeRef {
            name,
            is_reference: false,
            array_size: None,
            span: range_to_span(span),
        })
    )));

    // Define expr with proper precedence
    let atom = choice((
        literal,
        // Parenthesized expression
        expr.clone().delimited_by(
            just(ProcessedTokenKind::LParen),
            just(ProcessedTokenKind::RParen)
        ),
        // Array literal
        expr.clone()
            .separated_by(just(ProcessedTokenKind::Comma))
            .delimited_by(
                just(ProcessedTokenKind::LBracket),
                just(ProcessedTokenKind::RBracket)
            )
            .map_with_span(|elements, span| Expr::ArrayLiteral {
                elements,
                span: range_to_span(span),
            }),
        // Range expression (moved to binary expressions to handle precedence properly)
        // Identifier
        ident.clone().map_with_span(|name, span| Expr::Ident {
            name,
            span: range_to_span(span),
        })
    ));

    // More robust postfix expression parsing
    let postfix = choice((
        // Field access: ident.ident  
        ident.clone()
            .then_ignore(just(ProcessedTokenKind::Dot))
            .then(ident.clone())
            .map_with_span(|(base, field), span| Expr::FieldAccess {
                base: Box::new(Expr::Ident {
                    name: base,
                    span: range_to_span(span.clone()),
                }),
                field,
                span: range_to_span(span),
            }),
        // Function call: ident(args)
        ident.clone()
            .then(expr.clone()
                .separated_by(just(ProcessedTokenKind::Comma))
                .delimited_by(
                    just(ProcessedTokenKind::LParen),
                    just(ProcessedTokenKind::RParen)
                ))
            .map_with_span(|(name, args), span| Expr::Call {
                func: Box::new(Expr::Ident {
                    name,
                    span: range_to_span(span.clone()),
                }),
                args,
                span: range_to_span(span),
            }),
        // Array indexing: ident[expr]
        ident.clone()
            .then_ignore(just(ProcessedTokenKind::LBracket))
            .then(expr.clone())
            .then_ignore(just(ProcessedTokenKind::RBracket))
            .map_with_span(|(array, index), span| Expr::ArrayIndex {
                array: Box::new(Expr::Ident {
                    name: array,
                    span: range_to_span(span.clone()),
                }),
                index: Box::new(index),
                span: range_to_span(span),
            }),
        // Basic atom expressions
        atom,
    ));

    // Unary operators - simplified for now
    let unary = postfix;

    // Range and binary operators
    let range_or_binary = unary.clone()
        .then(choice((
            // Range expression
            just(ProcessedTokenKind::DotDot)
                .ignore_then(unary.clone())
                .map(|end| (None, end)), // Special marker for range
            // Binary operators
            just(ProcessedTokenKind::Plus).to(BinOp::Add)
                .then(unary.clone())
                .map(|(op, right)| (Some(op), right)),
            just(ProcessedTokenKind::Minus).to(BinOp::Sub)
                .then(unary.clone())
                .map(|(op, right)| (Some(op), right)),
            just(ProcessedTokenKind::Star).to(BinOp::Mul)
                .then(unary.clone())
                .map(|(op, right)| (Some(op), right)),
            just(ProcessedTokenKind::Slash).to(BinOp::Div)
                .then(unary.clone())
                .map(|(op, right)| (Some(op), right)),
        ))
        .or_not())
        .map_with_span(|(left, op_right), span: Range<usize>| {
            if let Some((op_opt, right)) = op_right {
                if let Some(op) = op_opt {
                    // Binary operation
                    Expr::BinaryOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                        span: range_to_span(span),
                    }
                } else {
                    // Range expression
                    Expr::Range {
                        start: Box::new(left),
                        end: Box::new(right),
                        span: range_to_span(span),
                    }
                }
            } else {
                left
            }
        });

    expr.define(range_or_binary);

    // Define statements
    let let_stmt = just(ProcessedTokenKind::Let)
        .ignore_then(ident.clone())
        .then_ignore(just(ProcessedTokenKind::Colon))
        .then(type_ref.clone())
        .then(just(ProcessedTokenKind::Assign)
            .ignore_then(expr.clone())
            .or_not())
        .then_ignore(just(ProcessedTokenKind::Semicolon))
        .map_with_span(|((name, ty), init), span| Stmt::Let {
            name,
            ty: Some(ty),
            init,
            span: range_to_span(span),
        });

    let assign_stmt = expr.clone()
        .then_ignore(just(ProcessedTokenKind::Assign))
        .then(expr.clone())
        .then_ignore(just(ProcessedTokenKind::Semicolon))
        .map_with_span(|(target, value), span| Stmt::Assign {
            target,
            value,
            span: range_to_span(span),
        });

    let for_stmt = just(ProcessedTokenKind::For)
        .ignore_then(ident.clone())
        .then_ignore(just(ProcessedTokenKind::In))
        .then(expr.clone())
        .then(stmt.clone()
            .repeated()
            .delimited_by(
                just(ProcessedTokenKind::LBrace),
                just(ProcessedTokenKind::RBrace)
            ))
        .map_with_span(|((var, range), body), span| Stmt::For {
            var,
            range,
            body,
            span: range_to_span(span),
        });

    let with_stmt = just(ProcessedTokenKind::With)
        .ignore_then(expr.clone())
        .then(stmt.clone()
            .repeated()
            .delimited_by(
                just(ProcessedTokenKind::LBrace),
                just(ProcessedTokenKind::RBrace)
            ))
        .map_with_span(|(view, body), span| Stmt::With {
            view,
            body,
            span: range_to_span(span),
        });

    let expr_stmt = expr.clone()
        .then_ignore(just(ProcessedTokenKind::Semicolon).or_not())
        .map(Stmt::Expr);

    stmt.define(choice((
        let_stmt,
        for_stmt,
        with_stmt,
        assign_stmt,
        expr_stmt,
    )));

    // Field definition
    let field_def = ident.clone()
        .then_ignore(just(ProcessedTokenKind::Colon))
        .then(type_ref.clone())
        .then_ignore(just(ProcessedTokenKind::Comma).or_not())
        .map_with_span(|(name, ty), span| FieldDef {
            name,
            ty,
            span: range_to_span(span),
        });

    // Parameter definition
    let param_def = ident.clone()
        .then_ignore(just(ProcessedTokenKind::Colon))
        .then(type_ref.clone())
        .map_with_span(|(name, ty), span| ParamDef {
            name,
            ty,
            span: range_to_span(span),
        });

    // Function definition
    let function_def = just(ProcessedTokenKind::Fn)
        .ignore_then(ident.clone())
        .then(param_def
            .separated_by(just(ProcessedTokenKind::Comma))
            .delimited_by(
                just(ProcessedTokenKind::LParen),
                just(ProcessedTokenKind::RParen)
            ))
        .then(just(ProcessedTokenKind::Arrow)
            .ignore_then(type_ref.clone())
            .or_not())
        .then(
            stmt.clone()
                .repeated()
                .delimited_by(
                    just(ProcessedTokenKind::LBrace),
                    just(ProcessedTokenKind::RBrace)
                )
        )
        .map_with_span(|(((name, params), return_type), body), span| FunctionDef {
            name,
            params,
            return_type,
            body,
            span: range_to_span(span),
        });

    // Struct definition
    let struct_def = just(ProcessedTokenKind::Struct)
        .ignore_then(ident.clone())
        .then(choice((
            field_def.map(|f| (Some(f), None)),
            function_def.clone().map(|f| (None, Some(f))),
        ))
        .repeated()
        .delimited_by(
            just(ProcessedTokenKind::LBrace),
            just(ProcessedTokenKind::RBrace)
        ))
        .map_with_span(|(name, items), span| {
            let mut fields = Vec::new();
            let mut methods = Vec::new();
            
            for (field, method) in items {
                if let Some(field) = field {
                    fields.push(field);
                }
                if let Some(method) = method {
                    methods.push(method);
                }
            }
            
            StructDef {
                name,
                fields,
                methods,
                span: range_to_span(span),
            }
        });

    // Sketch definition
    let sketch = just(ProcessedTokenKind::Sketch)
        .ignore_then(ident.clone())
        .then(stmt.clone()
            .repeated()
            .delimited_by(
                just(ProcessedTokenKind::LBrace),
                just(ProcessedTokenKind::RBrace)
            ))
        .map_with_span(|(name, body), span| SketchDef {
            name,
            body,
            span: range_to_span(span),
        });

    // Top-level parser
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