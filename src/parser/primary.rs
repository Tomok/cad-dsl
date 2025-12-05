use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::span::Span;
use chumsky::prelude::*;
use std::ops::Range;

use super::expression::expr_parser;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
}

pub fn primary_expr_parser()
-> impl Parser<ProcessedTokenKind, PrimaryExpr, Error = Simple<ProcessedTokenKind>> + Clone {
    recursive(|_primary_expr| {
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
        .map_with_span(|kind, span| PrimaryExpr::Literal {
            kind,
            span: range_to_span(span),
        });

        // Simple identifier
        let simple_ident = ident.map_with_span(|name, span| PrimaryExpr::Ident {
            name,
            span: range_to_span(span),
        });

        // Parenthesized expressions
        let parenthesized = just(ProcessedTokenKind::LParen)
            .ignore_then(expr_parser())
            .then_ignore(just(ProcessedTokenKind::RParen))
            .map(|expr| PrimaryExpr::Parenthesized(Box::new(expr)));

        // Array literals
        let array_literal = just(ProcessedTokenKind::LBracket)
            .ignore_then(
                expr_parser()
                    .separated_by(just(ProcessedTokenKind::Comma))
                    .allow_trailing(),
            )
            .then_ignore(just(ProcessedTokenKind::RBracket))
            .map_with_span(|elements, span| PrimaryExpr::ArrayLiteral {
                elements,
                span: range_to_span(span),
            });

        // Range expressions
        let range_expr = expr_parser()
            .then_ignore(just(ProcessedTokenKind::DotDot))
            .then(expr_parser())
            .map_with_span(|(start, end), span| PrimaryExpr::Range {
                start: Box::new(start),
                end: Box::new(end),
                span: range_to_span(span),
            });

        // Struct literals (simplified - just identifier followed by brace)
        let struct_literal = ident
            .then_ignore(just(ProcessedTokenKind::LBrace))
            .then(
                ident
                    .then_ignore(just(ProcessedTokenKind::Colon))
                    .then(expr_parser())
                    .separated_by(just(ProcessedTokenKind::Comma))
                    .allow_trailing(),
            )
            .then_ignore(just(ProcessedTokenKind::RBrace))
            .map_with_span(|(ty_name, fields), span| {
                let span_value = range_to_span(span);
                PrimaryExpr::StructLiteral {
                    ty: TypeRef {
                        name: ty_name,
                        is_reference: false,
                        array_size: None,
                        span: span_value,
                    },
                    fields,
                    span: span_value,
                }
            });

        // Base expressions (choose from various types)
        let base = choice((
            literal,
            struct_literal,
            array_literal,
            range_expr,
            parenthesized,
            simple_ident,
        ));

        // Postfix operations (function calls, field access, array indexing)
        base.then(
            choice((
                // Function calls: expr(args...)
                just(ProcessedTokenKind::LParen)
                    .ignore_then(
                        expr_parser()
                            .separated_by(just(ProcessedTokenKind::Comma))
                            .allow_trailing(),
                    )
                    .then_ignore(just(ProcessedTokenKind::RParen))
                    .map(PostfixOp::Call),
                // Field access: expr.field
                just(ProcessedTokenKind::Dot)
                    .ignore_then(ident)
                    .map(PostfixOp::FieldAccess),
                // Array indexing: expr[index]
                just(ProcessedTokenKind::LBracket)
                    .ignore_then(expr_parser())
                    .then_ignore(just(ProcessedTokenKind::RBracket))
                    .map(|index| PostfixOp::ArrayIndex(Box::new(index))),
            ))
            .repeated(),
        )
        .map_with_span(|(mut base, ops), span| {
            let span_value = range_to_span(span);
            for op in ops {
                base = match op {
                    PostfixOp::Call(args) => PrimaryExpr::Call {
                        func: Box::new(base),
                        args,
                        span: span_value,
                    },
                    PostfixOp::FieldAccess(field) => PrimaryExpr::FieldAccess {
                        base: Box::new(base),
                        field,
                        span: span_value,
                    },
                    PostfixOp::ArrayIndex(index) => PrimaryExpr::ArrayIndex {
                        array: Box::new(base),
                        index,
                        span: span_value,
                    },
                };
            }
            base
        })
    })
}

#[derive(Clone, Debug)]
enum PostfixOp {
    Call(Vec<Expr>),
    FieldAccess(crate::ident::IdentId),
    ArrayIndex(Box<Expr>),
}
