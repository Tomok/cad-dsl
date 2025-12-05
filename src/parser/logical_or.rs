use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::span::Span;
use chumsky::prelude::*;
use std::ops::Range;

use super::logical_and::logical_and_expr_parser;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
}

pub fn logical_or_expr_parser()
-> impl Parser<ProcessedTokenKind, LogicalOrExpr, Error = Simple<ProcessedTokenKind>> + Clone {
    let logical_and = logical_and_expr_parser();

    logical_and
        .clone()
        .then(
            just(ProcessedTokenKind::Or)
                .ignore_then(logical_and)
                .repeated(),
        )
        .map_with_span(|(first, rest), span| {
            if rest.is_empty() {
                first.into() // Convert LogicalAndExpr to LogicalOrExpr
            } else {
                build_left_associative_logical_or(first, rest, range_to_span(span))
            }
        })
}

fn build_left_associative_logical_or(
    first: LogicalAndExpr,
    rest: Vec<LogicalAndExpr>,
    span: Span,
) -> LogicalOrExpr {
    if rest.is_empty() {
        return first.into();
    }

    // Build the first operation
    let second = rest[0].clone();
    let mut current = LogicalOrExpr::LogicalOr {
        left: Box::new(first),
        right: Box::new(second),
        span,
    };

    // For remaining operations, wrap current result as LogicalAndExpr::LogicalOr
    for right in rest.into_iter().skip(1) {
        let wrapped_current = LogicalAndExpr::LogicalOr(Box::new(current));
        current = LogicalOrExpr::LogicalOr {
            left: Box::new(wrapped_current),
            right: Box::new(right),
            span,
        };
    }
    current
}
