// Import and re-export everything needed for tests
#[allow(unused_imports)]
pub(super) use super::super::*; // Parser module items
#[allow(unused_imports)]
pub(super) use crate::ast::{Stmt, Type};
#[allow(unused_imports)]
pub(super) use crate::lexer::{self, Token};
#[allow(unused_imports)]
pub(super) use crate::parser::atoms;
#[allow(unused_imports)]
pub(super) use crate::parser::stmt::{
    assignment_stmt, block_stmt, field_assignment_stmt, for_stmt, function_def, if_stmt, let_stmt,
    return_stmt, struct_def, type_annotation, with_stmt,
};
#[allow(unused_imports)]
pub(super) use crate::parser::{ParseError, expr, expr_inner};
#[allow(unused_imports)]
pub(super) use assert_matches::assert_matches;
#[allow(unused_imports)]
pub(super) use chumsky::prelude::*;
#[allow(unused_imports)]
pub(super) use std::time::Duration;

/// Helper function to parse with timeout
/// This prevents tests from hanging indefinitely if there's infinite recursion
///
/// Note: input must be 'static for thread safety
pub fn parse_with_timeout<T: Send + 'static>(
    input: &'static str,
    parse_fn: impl FnOnce(&'static [Token<'static>]) -> Result<T, Vec<Rich<'static, Token<'static>>>>
    + Send
    + 'static,
    timeout: Duration,
) -> Result<T, String> {
    // First tokenize the input - since input is 'static, tokens will be too
    let tokens = lexer::tokenize(input).map_err(|e| format!("Lexer error: {}", e))?;

    // Make tokens static by leaking (only for tests)
    let tokens_static: &'static [Token<'static>] = Box::leak(tokens.into_boxed_slice());

    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = parse_fn(tokens_static);
        let _ = tx.send(result);
    });

    rx.recv_timeout(timeout)
        .map_err(|_| "Test timeout - possible infinite recursion".to_string())
        .and_then(|r| r.map_err(|e| format!("Parse error: {:?}", e)))
}

/// Helper to create a recursive statement parser for testing for loops, with statements, if statements, and blocks
/// Supports let statements, assignment statements, return statements, nested for loops, with statements, if statements, blocks, and expression statements
pub fn stmt_parser_for_tests<'src>()
-> impl Parser<'src, &'src [Token<'src>], Stmt<'src>, ParseError<'src>> + Clone {
    recursive(|stmt_rec| {
        choice((
            let_stmt(expr_inner()),
            field_assignment_stmt(expr_inner()),
            assignment_stmt(expr_inner()),
            return_stmt(expr_inner()),
            for_stmt(expr_inner(), stmt_rec.clone()),
            with_stmt(expr_inner(), stmt_rec.clone()),
            if_stmt(expr_inner(), stmt_rec.clone()),
            block_stmt(stmt_rec),
            expression_stmt(expr_inner()),
        ))
    })
}
