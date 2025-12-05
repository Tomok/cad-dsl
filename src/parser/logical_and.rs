use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::span::Span;
use chumsky::prelude::*;
use std::ops::Range;

use super::comparison::comparison_expr_parser;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
}

pub fn logical_and_expr_parser()
-> impl Parser<ProcessedTokenKind, LogicalAndExpr, Error = Simple<ProcessedTokenKind>> + Clone {
    let comparison = comparison_expr_parser();

    comparison
        .clone()
        .then(
            just(ProcessedTokenKind::And)
                .ignore_then(comparison)
                .repeated(),
        )
        .map_with_span(|(first, rest), span| {
            if rest.is_empty() {
                first.into() // Convert ComparisonExpr to LogicalAndExpr
            } else {
                build_left_associative_logical_and(first, rest, range_to_span(span))
            }
        })
}

fn build_left_associative_logical_and(
    first: ComparisonExpr,
    rest: Vec<ComparisonExpr>,
    span: Span,
) -> LogicalAndExpr {
    if rest.is_empty() {
        return first.into();
    }

    // Build the first operation
    let second = rest[0].clone();
    let mut current = LogicalAndExpr::LogicalAnd {
        left: Box::new(first.into()),
        right: Box::new(second),
        span,
    };

    // For remaining operations, use the current result directly
    for right in rest.into_iter().skip(1) {
        current = LogicalAndExpr::LogicalAnd {
            left: Box::new(current),
            right: Box::new(right),
            span,
        };
    }
    current
}
