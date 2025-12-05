use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::span::Span;
use chumsky::prelude::*;
use std::ops::Range;

use super::multiplication::multiplication_expr_parser;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
}

#[derive(Clone, Debug)]
enum AddOp {
    Add,
    Subtract,
}

pub fn addition_expr_parser()
-> impl Parser<ProcessedTokenKind, AdditionExpr, Error = Simple<ProcessedTokenKind>> + Clone {
    let multiplication = multiplication_expr_parser();

    multiplication
        .clone()
        .then(
            choice((
                just(ProcessedTokenKind::Plus).to(AddOp::Add),
                just(ProcessedTokenKind::Minus).to(AddOp::Subtract),
            ))
            .then(multiplication)
            .repeated(),
        )
        .map_with_span(|(first, ops), span| {
            if ops.is_empty() {
                AdditionExpr::Multiplication(Box::new(first))
            } else {
                build_left_associative_addition(first, ops, range_to_span(span))
            }
        })
}

fn build_left_associative_addition(
    first: MultiplicationExpr,
    ops: Vec<(AddOp, MultiplicationExpr)>,
    span: Span,
) -> AdditionExpr {
    if ops.is_empty() {
        return AdditionExpr::Multiplication(Box::new(first));
    }

    // Build the first operation
    let (first_op, second) = ops[0].clone();
    let mut current = match first_op {
        AddOp::Add => AdditionExpr::Add {
            left: Box::new(first.into()),
            right: Box::new(second),
            span,
        },
        AddOp::Subtract => AdditionExpr::Subtract {
            left: Box::new(first.into()),
            right: Box::new(second),
            span,
        },
    };

    // For remaining operations, wrap current result as AdditionExpr
    for (op, right) in ops.into_iter().skip(1) {
        current = match op {
            AddOp::Add => AdditionExpr::Add {
                left: Box::new(current),
                right: Box::new(right),
                span,
            },
            AddOp::Subtract => AdditionExpr::Subtract {
                left: Box::new(current),
                right: Box::new(right),
                span,
            },
        };
    }
    current
}
