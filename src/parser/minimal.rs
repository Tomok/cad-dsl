use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::ident::IdentId;
use crate::span::Span;
use chumsky::prelude::*;
use chumsky::recursive::Recursive;
use std::ops::Range;

pub type TokenStream = ProcessedTokenKind;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
}

pub fn minimal_parser()
-> impl Parser<TokenStream, UnresolvedAst, Error = Simple<ProcessedTokenKind>> + Clone {
    let sketch = just(ProcessedTokenKind::Sketch)
        .ignore_then(ident())
        .then(stmt_parser().repeated().delimited_by(
            just(ProcessedTokenKind::LBrace),
            just(ProcessedTokenKind::RBrace),
        ))
        .map_with_span(|(name, body), span| {
            TopLevelItem::Sketch(SketchDef {
                name,
                body,
                span: range_to_span(span),
            })
        });

    let struct_def = just(ProcessedTokenKind::Struct)
        .ignore_then(ident())
        .then(
            choice((
                field_def().map(StructItem::Field),
                function_def().map(StructItem::Method),
            ))
            .repeated()
            .delimited_by(
                just(ProcessedTokenKind::LBrace),
                just(ProcessedTokenKind::RBrace),
            ),
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

            TopLevelItem::Struct(StructDef {
                name,
                fields,
                methods,
                span: range_to_span(span),
            })
        });

    choice((sketch, struct_def))
        .repeated()
        .then_ignore(just(ProcessedTokenKind::Eof))
        .map(|items| {
            let mut sketches = Vec::new();
            let mut structs = Vec::new();

            for item in items {
                match item {
                    TopLevelItem::Sketch(sketch) => sketches.push(sketch),
                    TopLevelItem::Struct(struct_def) => structs.push(struct_def),
                }
            }

            UnresolvedAst {
                imports: Vec::new(),
                structs,
                sketches,
            }
        })
}

#[derive(Debug, Clone)]
enum TopLevelItem {
    Sketch(SketchDef),
    Struct(StructDef),
}

#[derive(Debug, Clone)]
enum StructItem {
    Field(FieldDef),
    Method(FunctionDef),
}

fn field_def() -> impl Parser<TokenStream, FieldDef, Error = Simple<ProcessedTokenKind>> + Clone {
    ident()
        .then_ignore(just(ProcessedTokenKind::Colon))
        .then(type_ref_parser()) // Use proper type parser
        .then_ignore(just(ProcessedTokenKind::Comma).or_not())
        .map_with_span(|(name, ty), span| FieldDef {
            name,
            ty,
            span: range_to_span(span),
        })
}

fn function_def()
-> impl Parser<TokenStream, FunctionDef, Error = Simple<ProcessedTokenKind>> + Clone {
    just(ProcessedTokenKind::Fn)
        .ignore_then(ident())
        .then(
            param_def()
                .separated_by(just(ProcessedTokenKind::Comma))
                .delimited_by(
                    just(ProcessedTokenKind::LParen),
                    just(ProcessedTokenKind::RParen),
                ),
        )
        .then(
            just(ProcessedTokenKind::Arrow)
                .ignore_then(type_ref_parser())
                .or_not(),
        )
        .then(stmt_parser().repeated().delimited_by(
            just(ProcessedTokenKind::LBrace),
            just(ProcessedTokenKind::RBrace),
        ))
        .map_with_span(|(((name, params), return_type), body), span| FunctionDef {
            name,
            params,
            return_type,
            body,
            span: range_to_span(span),
        })
}

fn param_def() -> impl Parser<TokenStream, ParamDef, Error = Simple<ProcessedTokenKind>> + Clone {
    ident()
        .then_ignore(just(ProcessedTokenKind::Colon))
        .then(type_ref_parser())
        .map_with_span(|(name, ty), span| ParamDef {
            name,
            ty,
            span: range_to_span(span),
        })
}

fn stmt_parser() -> impl Parser<TokenStream, Stmt, Error = Simple<ProcessedTokenKind>> + Clone {
    let mut stmt = Recursive::declare();
    let expr = expr_parser();

    {
        let let_stmt = just(ProcessedTokenKind::Let)
            .ignore_then(ident())
            .then_ignore(just(ProcessedTokenKind::Colon))
            .then(type_ref_parser())
            .then(
                just(ProcessedTokenKind::Assign)
                    .ignore_then(expr.clone())
                    .or_not(),
            )
            .then_ignore(just(ProcessedTokenKind::Semicolon))
            .map_with_span(|((name, ty), init), span| Stmt::Let {
                name,
                ty: Some(ty),
                init,
                span: range_to_span(span),
            });

        let for_stmt = just(ProcessedTokenKind::For)
            .ignore_then(ident())
            .then_ignore(just(ProcessedTokenKind::In))
            .then(expr.clone())
            .then(stmt.clone().repeated().delimited_by(
                just(ProcessedTokenKind::LBrace),
                just(ProcessedTokenKind::RBrace),
            ))
            .map_with_span(|((var, range), body), span| Stmt::For {
                var,
                range,
                body,
                span: range_to_span(span),
            });

        let with_stmt = just(ProcessedTokenKind::With)
            .ignore_then(expr.clone())
            .then(stmt.clone().repeated().delimited_by(
                just(ProcessedTokenKind::LBrace),
                just(ProcessedTokenKind::RBrace),
            ))
            .map_with_span(|(view, body), span| Stmt::With {
                view,
                body,
                span: range_to_span(span),
            });

        let assign_stmt = expr
            .clone()
            .then_ignore(just(ProcessedTokenKind::Assign))
            .then(expr.clone())
            .then_ignore(just(ProcessedTokenKind::Semicolon))
            .map_with_span(|(target, value), span| Stmt::Assign {
                target,
                value,
                span: range_to_span(span),
            });

        let expr_stmt = expr
            .clone()
            .then_ignore(just(ProcessedTokenKind::Semicolon))
            .map(Stmt::Expr);

        stmt.define(
            choice((let_stmt, for_stmt, with_stmt, assign_stmt, expr_stmt)).labelled("statement"),
        );
    }
    stmt
}

fn expr_parser() -> impl Parser<TokenStream, Expr, Error = Simple<ProcessedTokenKind>> + Clone {
    let mut expr = Recursive::declare();

    {
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

        // Array literal
        let array_literal = expr
            .clone()
            .separated_by(just(ProcessedTokenKind::Comma))
            .delimited_by(
                just(ProcessedTokenKind::LBracket),
                just(ProcessedTokenKind::RBracket),
            )
            .map_with_span(|elements, span| Expr::ArrayLiteral {
                elements,
                span: range_to_span(span),
            });

        // Range expression (e.g., 0..5)
        let range_expr = expr
            .clone()
            .then_ignore(just(ProcessedTokenKind::DotDot))
            .then(expr.clone())
            .map_with_span(|(start, end), span| Expr::Range {
                start: Box::new(start),
                end: Box::new(end),
                span: range_to_span(span),
            });

        let parenthesized = expr.clone().delimited_by(
            just(ProcessedTokenKind::LParen),
            just(ProcessedTokenKind::RParen),
        );

        // Base atoms
        let atom = choice((literal, ident_expr, parenthesized));

        // Struct literal
        let struct_literal = ident()
            .then(
                ident()
                    .then_ignore(just(ProcessedTokenKind::Colon))
                    .then(expr.clone())
                    .separated_by(just(ProcessedTokenKind::Comma))
                    .delimited_by(
                        just(ProcessedTokenKind::LBrace),
                        just(ProcessedTokenKind::RBrace),
                    ),
            )
            .map_with_span(|(type_name, fields), span| Expr::StructLiteral {
                ty: TypeRef {
                    name: type_name,
                    is_reference: false,
                    array_size: None,
                    span: range_to_span(span.clone()),
                },
                fields,
                span: range_to_span(span),
            });

        // Complex literals that can be primary expressions
        let complex_literals = choice((struct_literal, array_literal, range_expr));

        // Primary expressions (atoms and complex literals)
        let primary = choice((atom, complex_literals));

        // Prefix operators (unary, reference, dereference)
        let prefix = choice((
            // Unary minus
            just(ProcessedTokenKind::Minus)
                .ignore_then(expr.clone())
                .map_with_span(|expr, span| Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                    span: range_to_span(span),
                }),
            // Unary not
            just(ProcessedTokenKind::Not)
                .ignore_then(expr.clone())
                .map_with_span(|expr, span| Expr::UnaryOp {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                    span: range_to_span(span),
                }),
            // Reference
            just(ProcessedTokenKind::Ampersand)
                .ignore_then(expr.clone())
                .map_with_span(|expr, span| Expr::Reference {
                    expr: Box::new(expr),
                    span: range_to_span(span),
                }),
            // Dereference
            just(ProcessedTokenKind::Star)
                .ignore_then(expr.clone())
                .map_with_span(|expr, span| Expr::Dereference {
                    expr: Box::new(expr),
                    span: range_to_span(span),
                }),
            // Primary expressions
            primary.clone(),
        ));

        // Postfix operators: function calls, field access, array indexing
        let postfix = prefix
            .clone()
            .then(
                choice((
                    // Function call
                    expr.clone()
                        .separated_by(just(ProcessedTokenKind::Comma))
                        .delimited_by(
                            just(ProcessedTokenKind::LParen),
                            just(ProcessedTokenKind::RParen),
                        )
                        .map(PostfixOp::Call),
                    // Field access
                    just(ProcessedTokenKind::Dot)
                        .ignore_then(ident())
                        .map(PostfixOp::FieldAccess),
                    // Array indexing
                    expr.clone()
                        .delimited_by(
                            just(ProcessedTokenKind::LBracket),
                            just(ProcessedTokenKind::RBracket),
                        )
                        .map(PostfixOp::ArrayIndex),
                ))
                .repeated(),
            )
            .map(|(base, ops)| {
                ops.into_iter().fold(base, |acc, op| {
                    let span = acc.span();
                    match op {
                        PostfixOp::Call(args) => Expr::Call {
                            func: Box::new(acc),
                            args,
                            span,
                        },
                        PostfixOp::FieldAccess(field) => Expr::FieldAccess {
                            base: Box::new(acc),
                            field,
                            span,
                        },
                        PostfixOp::ArrayIndex(index) => Expr::ArrayIndex {
                            array: Box::new(acc),
                            index: Box::new(index),
                            span,
                        },
                    }
                })
            });

        // Binary operators with precedence
        // Exponentiation (highest precedence, right-associative)
        let exp = postfix
            .clone()
            .then(
                just(ProcessedTokenKind::Caret)
                    .ignore_then(postfix.clone())
                    .repeated(),
            )
            .map(|(left, rights)| {
                rights.into_iter().fold(left, |left, right| {
                    let span = Span::new(left.span().start, right.span().end);
                    Expr::BinaryOp {
                        op: BinOp::Pow,
                        left: Box::new(left),
                        right: Box::new(right),
                        span,
                    }
                })
            });

        // Multiplicative operators
        let factor = exp
            .clone()
            .then(
                choice((
                    just(ProcessedTokenKind::Star).to(BinOp::Mul),
                    just(ProcessedTokenKind::Slash).to(BinOp::Div),
                    just(ProcessedTokenKind::Percent).to(BinOp::Mod),
                ))
                .then(exp.clone())
                .repeated(),
            )
            .map(|(left, ops)| {
                ops.into_iter().fold(left, |left, (op, right)| {
                    let span = Span::new(left.span().start, right.span().end);
                    Expr::BinaryOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                        span,
                    }
                })
            });

        // Additive operators
        let term = factor
            .clone()
            .then(
                choice((
                    just(ProcessedTokenKind::Plus).to(BinOp::Add),
                    just(ProcessedTokenKind::Minus).to(BinOp::Sub),
                ))
                .then(factor.clone())
                .repeated(),
            )
            .map(|(left, ops)| {
                ops.into_iter().fold(left, |left, (op, right)| {
                    let span = Span::new(left.span().start, right.span().end);
                    Expr::BinaryOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                        span,
                    }
                })
            });

        // Comparison operators
        let comparison = term
            .clone()
            .then(
                choice((
                    just(ProcessedTokenKind::Eq).to(BinOp::Eq),
                    just(ProcessedTokenKind::NotEq).to(BinOp::NotEq),
                    just(ProcessedTokenKind::Lt).to(BinOp::Lt),
                    just(ProcessedTokenKind::Gt).to(BinOp::Gt),
                    just(ProcessedTokenKind::LtEq).to(BinOp::LtEq),
                    just(ProcessedTokenKind::GtEq).to(BinOp::GtEq),
                ))
                .then(term.clone())
                .repeated(),
            )
            .map(|(left, ops)| {
                ops.into_iter().fold(left, |left, (op, right)| {
                    let span = Span::new(left.span().start, right.span().end);
                    Expr::BinaryOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                        span,
                    }
                })
            });

        // Logical AND
        let and_expr = comparison
            .clone()
            .then(
                just(ProcessedTokenKind::And)
                    .to(BinOp::And)
                    .then(comparison.clone())
                    .repeated(),
            )
            .map(|(left, ops)| {
                ops.into_iter().fold(left, |left, (op, right)| {
                    let span = Span::new(left.span().start, right.span().end);
                    Expr::BinaryOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                        span,
                    }
                })
            });

        // Logical OR (lowest precedence)
        let or_expr = and_expr
            .clone()
            .then(
                just(ProcessedTokenKind::Or)
                    .to(BinOp::Or)
                    .then(and_expr.clone())
                    .repeated(),
            )
            .map(|(left, ops)| {
                ops.into_iter().fold(left, |left, (op, right)| {
                    let span = Span::new(left.span().start, right.span().end);
                    Expr::BinaryOp {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                        span,
                    }
                })
            });

        expr.define(or_expr);
    }
    expr
}

#[derive(Debug, Clone)]
enum PostfixOp {
    Call(Vec<Expr>),
    FieldAccess(IdentId),
    ArrayIndex(Expr),
}

fn type_ref_parser() -> impl Parser<TokenStream, TypeRef, Error = Simple<ProcessedTokenKind>> + Clone
{
    let mut type_ref = Recursive::declare();
    {
        let basic_type = ident().map_with_span(|name, span| TypeRef {
            name,
            is_reference: false,
            array_size: None,
            span: range_to_span(span),
        });

        // Reference type: &Type
        let reference_type = just(ProcessedTokenKind::Ampersand)
            .ignore_then(type_ref.clone())
            .map_with_span(|mut inner: TypeRef, span| {
                inner.is_reference = true;
                inner.span = range_to_span(span);
                inner
            });

        // Array type: [Type; size]
        let array_type = just(ProcessedTokenKind::LBracket)
            .ignore_then(type_ref.clone())
            .then_ignore(just(ProcessedTokenKind::Semicolon))
            .then(expr_parser()) // Array size expression
            .then_ignore(just(ProcessedTokenKind::RBracket))
            .map_with_span(|(mut inner, size): (TypeRef, _), span| {
                inner.array_size = Some(Box::new(size));
                inner.span = range_to_span(span);
                inner
            });

        type_ref.define(choice((array_type, reference_type, basic_type)));
    }
    type_ref
}

fn ident() -> impl Parser<TokenStream, IdentId, Error = Simple<ProcessedTokenKind>> + Clone {
    select! { ProcessedTokenKind::Ident(id) => id }
}
