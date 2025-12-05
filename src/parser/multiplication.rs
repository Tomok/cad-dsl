use crate::ProcessedTokenKind;
use crate::ast::unresolved::*;
use crate::span::Span;
use chumsky::prelude::*;
use std::ops::Range;

use super::power::power_expr_parser;

fn range_to_span(range: Range<usize>) -> Span {
    Span::new(range.start, range.end)
}

#[derive(Clone, Debug)]
enum MultOp {
    Multiply,
    Divide,
    Modulo,
}

pub fn multiplication_expr_parser()
-> impl Parser<ProcessedTokenKind, MultiplicationExpr, Error = Simple<ProcessedTokenKind>> + Clone {
    let power = power_expr_parser();

    power
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
        .map_with_span(|(first, ops), span| {
            if ops.is_empty() {
                first.into() // Convert PowerExpr to MultiplicationExpr
            } else {
                // For now, only handle a single operation due to hierarchical type constraints
                // TODO: Implement proper left-associativity - requires AST modification or alternative approach
                let (op, right) = ops.into_iter().next().unwrap();
                match op {
                    MultOp::Multiply => MultiplicationExpr::Multiply {
                        left: Box::new(first.into()),
                        right: Box::new(right),
                        span: range_to_span(span),
                    },
                    MultOp::Divide => MultiplicationExpr::Divide {
                        left: Box::new(first.into()),
                        right: Box::new(right),
                        span: range_to_span(span),
                    },
                    MultOp::Modulo => MultiplicationExpr::Modulo {
                        left: Box::new(first.into()),
                        right: Box::new(right),
                        span: range_to_span(span),
                    },
                }
            }
        })
}
