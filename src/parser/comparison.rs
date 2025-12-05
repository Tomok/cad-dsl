use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::span::Span;
use chumsky::prelude::*;
use std::ops::Range;

use super::addition::addition_expr_parser;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
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

pub fn comparison_expr_parser()
-> impl Parser<ProcessedTokenKind, ComparisonExpr, Error = Simple<ProcessedTokenKind>> + Clone {
    let addition = addition_expr_parser();

    // Comparison operators are typically non-associative (no chaining)
    addition
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
        .map_with_span(|(left, op_right_opt), span| {
            match op_right_opt {
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
                None => left.into(), // Convert AdditionExpr to ComparisonExpr
            }
        })
}
