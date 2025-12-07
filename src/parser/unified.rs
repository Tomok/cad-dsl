use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::span::Span;
use chumsky::prelude::*;
use std::ops::Range;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
}

#[derive(Debug, Clone)]
enum PostfixOp {
    FieldAccess(crate::ident::IdentId),
    ArrayIndex,
    FunctionCall(Vec<Expr>),
}

pub fn unified_expr_parser()
-> impl Parser<ProcessedTokenKind, Expr, Error = Simple<ProcessedTokenKind>> + Clone {
    recursive(|expr| {
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

        // Parenthesized expressions using the recursive expr
        let parenthesized = expr
            .clone()
            .delimited_by(
                just(ProcessedTokenKind::LParen),
                just(ProcessedTokenKind::RParen),
            )
            .map(|expr| PrimaryExpr::Parenthesized(Box::new(expr)));

        // Array literals using the recursive expr
        let array_literal = expr
            .clone()
            .separated_by(just(ProcessedTokenKind::Comma))
            .delimited_by(
                just(ProcessedTokenKind::LBracket),
                just(ProcessedTokenKind::RBracket),
            )
            .map_with_span(|elements, span| PrimaryExpr::ArrayLiteral {
                elements,
                span: range_to_span(span),
            });

        // Range expressions - simplify using From traits
        let range_expr = simple_ident
            .or(literal)
            .then_ignore(just(ProcessedTokenKind::DotDot))
            .then(simple_ident.or(literal))
            .map_with_span(|(start, end), span| PrimaryExpr::Range {
                start: Box::new(start.into()),
                end: Box::new(end.into()),
                span: range_to_span(span),
            });

        // Struct literals (copied from primary.rs)
        let struct_literal = ident
            .then_ignore(just(ProcessedTokenKind::LBrace))
            .then(
                ident
                    .then_ignore(just(ProcessedTokenKind::Colon))
                    .then(expr.clone())
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

        // Primary expressions
        let primary = choice((
            range_expr,
            struct_literal,
            literal,
            parenthesized,
            array_literal,
            simple_ident,
        ));

        // Postfix expressions (field access, array indexing, function calls)
        let postfix = primary
            .then(
                choice((
                    // Field access: obj.field
                    just(ProcessedTokenKind::Dot)
                        .ignore_then(ident)
                        .map(|field| (PostfixOp::FieldAccess(field), None)),
                    // Array indexing: arr[index]
                    just(ProcessedTokenKind::LBracket)
                        .ignore_then(expr.clone())
                        .then_ignore(just(ProcessedTokenKind::RBracket))
                        .map(|index| (PostfixOp::ArrayIndex, Some(index))),
                    // Function call: func(args...)
                    expr.clone()
                        .separated_by(just(ProcessedTokenKind::Comma))
                        .delimited_by(
                            just(ProcessedTokenKind::LParen),
                            just(ProcessedTokenKind::RParen),
                        )
                        .map(|args| (PostfixOp::FunctionCall(args), None)),
                ))
                .repeated(),
            )
            .map_with_span(|(base, ops), span: Range<usize>| {
                ops.into_iter().fold(base, |acc, (op, expr_opt)| {
                    let current_span = range_to_span(span.clone());
                    match op {
                        PostfixOp::FieldAccess(field) => PrimaryExpr::FieldAccess {
                            base: Box::new(acc),
                            field,
                            span: current_span,
                        },
                        PostfixOp::ArrayIndex => PrimaryExpr::ArrayIndex {
                            array: Box::new(acc),
                            index: Box::new(expr_opt.unwrap()),
                            span: current_span,
                        },
                        PostfixOp::FunctionCall(args) => PrimaryExpr::Call {
                            func: Box::new(acc),
                            args,
                            span: current_span,
                        },
                    }
                })
            });

        // Unary expressions
        let unary = recursive(|unary| {
            let negation = just(ProcessedTokenKind::Minus)
                .ignore_then(unary.clone())
                .map_with_span(|expr, span| UnaryExpr::Negation {
                    expr: Box::new(expr),
                    span: range_to_span(span),
                });

            let not = just(ProcessedTokenKind::Not)
                .ignore_then(unary.clone())
                .map_with_span(|expr, span| UnaryExpr::Not {
                    expr: Box::new(expr),
                    span: range_to_span(span),
                });

            let reference = just(ProcessedTokenKind::Ampersand)
                .ignore_then(unary.clone())
                .map_with_span(|expr, span| UnaryExpr::Reference {
                    expr: Box::new(expr),
                    span: range_to_span(span),
                });

            choice((negation, not, reference, postfix.map(UnaryExpr::Primary)))
        });

        // Power expressions (right-associative)
        let power = recursive(|power| {
            unary
                .clone()
                .then(
                    just(ProcessedTokenKind::Caret)
                        .ignore_then(power.clone())
                        .or_not(),
                )
                .map_with_span(|(left, right_opt), span| match right_opt {
                    Some(right) => PowerExpr::Power {
                        left: Box::new(left),
                        right: Box::new(right),
                        span: range_to_span(span),
                    },
                    None => PowerExpr::Unary(left),
                })
        });

        // Multiplication expressions (left-associative using folding)
        let multiplication = power
            .clone()
            .then(
                choice((
                    just(ProcessedTokenKind::Star).to(MultOp::Multiply),
                    just(ProcessedTokenKind::Slash).to(MultOp::Divide),
                    just(ProcessedTokenKind::Percent).to(MultOp::Modulo),
                ))
                .then(power)
                .repeated(),
            )
            .map_with_span(|(first, ops), span: Range<usize>| {
                ops.into_iter()
                    .fold(first.into(), |acc, (op, right)| match op {
                        MultOp::Multiply => MultiplicationExpr::Multiply {
                            left: Box::new(acc),
                            right: Box::new(right),
                            span: range_to_span(span.clone()),
                        },
                        MultOp::Divide => MultiplicationExpr::Divide {
                            left: Box::new(acc),
                            right: Box::new(right),
                            span: range_to_span(span.clone()),
                        },
                        MultOp::Modulo => MultiplicationExpr::Modulo {
                            left: Box::new(acc),
                            right: Box::new(right),
                            span: range_to_span(span.clone()),
                        },
                    })
            });

        // Addition expressions (left-associative using folding)
        let addition = multiplication
            .clone()
            .then(
                choice((
                    just(ProcessedTokenKind::Plus).to(AddOp::Add),
                    just(ProcessedTokenKind::Minus).to(AddOp::Subtract),
                ))
                .then(multiplication)
                .repeated(),
            )
            .map_with_span(|(first, ops), span: Range<usize>| {
                ops.into_iter()
                    .fold(first.into(), |acc, (op, right)| match op {
                        AddOp::Add => AdditionExpr::Add {
                            left: Box::new(acc),
                            right: Box::new(right),
                            span: range_to_span(span.clone()),
                        },
                        AddOp::Subtract => AdditionExpr::Subtract {
                            left: Box::new(acc),
                            right: Box::new(right),
                            span: range_to_span(span.clone()),
                        },
                    })
            });

        // Comparison expressions
        let comparison = addition
            .clone()
            .then(
                choice((
                    just(ProcessedTokenKind::Eq).to(CompOp::Equal),
                    just(ProcessedTokenKind::NotEq).to(CompOp::NotEqual),
                    just(ProcessedTokenKind::Lt).to(CompOp::LessThan),
                    just(ProcessedTokenKind::Gt).to(CompOp::GreaterThan),
                    just(ProcessedTokenKind::LtEq).to(CompOp::LessEqual),
                    just(ProcessedTokenKind::GtEq).to(CompOp::GreaterEqual),
                ))
                .then(addition)
                .or_not(),
            )
            .map_with_span(|(left, op_right_opt), span| match op_right_opt {
                Some((op, right)) => {
                    let span = range_to_span(span);
                    match op {
                        CompOp::Equal => ComparisonExpr::Equal {
                            left: Box::new(left),
                            right: Box::new(right),
                            span,
                        },
                        CompOp::NotEqual => ComparisonExpr::NotEqual {
                            left: Box::new(left),
                            right: Box::new(right),
                            span,
                        },
                        CompOp::LessThan => ComparisonExpr::LessThan {
                            left: Box::new(left),
                            right: Box::new(right),
                            span,
                        },
                        CompOp::GreaterThan => ComparisonExpr::GreaterThan {
                            left: Box::new(left),
                            right: Box::new(right),
                            span,
                        },
                        CompOp::LessEqual => ComparisonExpr::LessEqual {
                            left: Box::new(left),
                            right: Box::new(right),
                            span,
                        },
                        CompOp::GreaterEqual => ComparisonExpr::GreaterEqual {
                            left: Box::new(left),
                            right: Box::new(right),
                            span,
                        },
                    }
                }
                None => left.into(),
            });

        // Logical AND expressions (left-associative using folding)
        let logical_and = comparison
            .clone()
            .then(
                just(ProcessedTokenKind::And)
                    .ignore_then(comparison)
                    .repeated(),
            )
            .map_with_span(|(first, ops), span: Range<usize>| {
                ops.into_iter()
                    .fold(first.into(), |acc, right| LogicalAndExpr::LogicalAnd {
                        left: Box::new(acc),
                        right: Box::new(right),
                        span: range_to_span(span.clone()),
                    })
            });

        // Logical OR expressions (left-associative using recursive approach)
        let logical_or = recursive(|logical_or_inner| {
            logical_and
                .clone()
                .then(
                    just(ProcessedTokenKind::Or)
                        .ignore_then(logical_or_inner)
                        .or_not(),
                )
                .map_with_span(|(left, right_opt), span| match right_opt {
                    Some(right) => {
                        // For OR chains, we need to create a new LogicalAndExpr that contains
                        // a LogicalOrExpr::LogicalOr, since OR has lower precedence than AND
                        LogicalOrExpr::LogicalOr {
                            left: Box::new(left),
                            right: Box::new(match right {
                                LogicalOrExpr::LogicalAnd(and) => and,
                                LogicalOrExpr::LogicalOr {
                                    left: nested_left, ..
                                } => {
                                    // For left-associativity in chains, we want the left operand
                                    *nested_left
                                }
                            }),
                            span: range_to_span(span),
                        }
                    }
                    None => left.into(),
                })
        });

        // Final expression
        logical_or.map(Expr::LogicalOr)
    })
}

#[derive(Clone, Debug)]
enum MultOp {
    Multiply,
    Divide,
    Modulo,
}

#[derive(Clone, Debug)]
enum AddOp {
    Add,
    Subtract,
}

#[derive(Clone, Debug)]
enum CompOp {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessEqual,
    GreaterEqual,
}
