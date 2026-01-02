use super::helpers::*;

// ============================================================================
// For Loop Tests
// ============================================================================

#[test]
fn test_for_loop_range() {
    // for i in 0..10 { }
    let input = "for i in 0..10 { }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::For {
            loop_var,
            iterator,
            body,
            ..
        } => {
            assert_eq!(loop_var, "i");
            assert_matches!(iterator, Expr::Range { .. });
            assert_eq!(body.len(), 0);
        }
        other => panic!("Expected Stmt::For, got {:?}", other),
    }
}

#[test]
fn test_for_loop_with_body() {
    // for i in 0..5 { let x: i32 = i; }
    let input = "for i in 0..5 { let x: i32 = i; }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::For {
            loop_var,
            iterator,
            body,
            ..
        } => {
            assert_eq!(loop_var, "i");
            assert_matches!(iterator, Expr::Range { .. });
            assert_eq!(body.len(), 1);
            assert_matches!(body[0], Stmt::Let { .. });
        }
        other => panic!("Expected Stmt::For, got {:?}", other),
    }
}

#[test]
fn test_for_loop_over_variable() {
    // for elem in (items) { }
    // Note: Parentheses disambiguate from empty struct literal "items {}"
    let input = "for elem in (items) { }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::For {
            loop_var,
            iterator,
            body,
            ..
        } => {
            assert_eq!(loop_var, "elem");
            // Parentheses create a Paren expression wrapping the variable
            match iterator {
                Expr::Paren { inner, .. } => match inner.as_ref() {
                    Expr::Var { name, .. } if *name == "items" => {} // OK
                    other => panic!("Expected Var inside Paren, got {:?}", other),
                },
                other => panic!("Expected Paren, got {:?}", other),
            }
            assert_eq!(body.len(), 0);
        }
        other => panic!("Expected Stmt::For, got {:?}", other),
    }
}

#[test]
fn test_for_loop_nested() {
    // for i in 0..3 { for j in 0..2 { } }
    let input = "for i in 0..3 { for j in 0..2 { } }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::For {
            loop_var,
            iterator,
            body,
            ..
        } => {
            assert_eq!(loop_var, "i");
            assert_matches!(iterator, Expr::Range { .. });
            assert_eq!(body.len(), 1);

            // Check inner for loop
            match &body[0] {
                Stmt::For {
                    loop_var: inner_var,
                    iterator: inner_iter,
                    body: inner_body,
                    ..
                } => {
                    assert_eq!(*inner_var, "j");
                    assert_matches!(inner_iter, Expr::Range { .. });
                    assert_eq!(inner_body.len(), 0);
                }
                other => panic!("Expected inner Stmt::For, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::For, got {:?}", other),
    }
}

#[test]
fn test_for_loop_with_multiple_statements() {
    // for i in 0..5 { let x: i32 = i; let y: i32 = x + 1; }
    let input = "for i in 0..5 { let x: i32 = i; let y: i32 = x + 1; }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::For {
            loop_var,
            iterator,
            body,
            ..
        } => {
            assert_eq!(loop_var, "i");
            assert_matches!(iterator, Expr::Range { .. });
            assert_eq!(body.len(), 2);
            assert_matches!(body[0], Stmt::Let { .. });
            assert_matches!(body[1], Stmt::Let { .. });
        }
        other => panic!("Expected Stmt::For, got {:?}", other),
    }
}

#[test]
fn test_for_loop_over_expression() {
    // for item in obj.items { }
    let input = "for item in obj.items { }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::For {
            loop_var,
            iterator,
            body,
            ..
        } => {
            assert_eq!(loop_var, "item");
            assert_matches!(iterator, Expr::FieldAccess { .. });
            assert_eq!(body.len(), 0);
        }
        other => panic!("Expected Stmt::For, got {:?}", other),
    }
}

#[test]
fn test_for_loop_multiline() {
    let input = "for i in 0..10 {
    let x: i32 = i;
    let y: i32 = x + 1;
}";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::For {
            loop_var,
            iterator,
            body,
            ..
        } => {
            assert_eq!(loop_var, "i");
            assert_matches!(iterator, Expr::Range { .. });
            assert_eq!(body.len(), 2);
        }
        other => panic!("Expected Stmt::For, got {:?}", other),
    }
}

#[test]
fn test_for_loop_over_array_literal() {
    // for x in [1, 2, 3] { }
    let input = "for x in [1, 2, 3] { }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::For {
            loop_var,
            iterator,
            body,
            ..
        } => {
            assert_eq!(loop_var, "x");
            assert_matches!(iterator, Expr::ArrayLit { .. });
            assert_eq!(body.len(), 0);
        }
        other => panic!("Expected Stmt::For, got {:?}", other),
    }
}

// ============================================================================
// If Statement Tests
// ============================================================================

#[test]
fn test_if_stmt_without_else() {
    let input = "if x > 0 { return x; }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(condition, Expr::Gt { .. });
            assert_eq!(then_branch.len(), 1);
            assert_matches!(then_branch[0], Stmt::Return { .. });
            assert!(else_branch.is_none());
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_with_else() {
    let input = "if condition { x = 1; } else { x = 2; }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(
                condition,
                Expr::Var {
                    name: "condition",
                    ..
                }
            );
            assert_eq!(then_branch.len(), 1);
            assert_matches!(then_branch[0], Stmt::Assignment { .. });
            assert!(else_branch.is_some());
            let else_stmts = else_branch.unwrap();
            assert_eq!(else_stmts.len(), 1);
            assert_matches!(else_stmts[0], Stmt::Assignment { .. });
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_else_if_chain() {
    let input = "if x > 0 { positive(); } else { if x < 0 { negative(); } else { zero(); } }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(condition, Expr::Gt { .. });
            assert_eq!(then_branch.len(), 1);
            assert_matches!(then_branch[0], Stmt::Expression { .. });

            // Else branch contains another if statement
            assert!(else_branch.is_some());
            let else_stmts = else_branch.unwrap();
            assert_eq!(else_stmts.len(), 1);

            // The else branch should contain another if statement
            match &else_stmts[0] {
                Stmt::If {
                    condition: inner_cond,
                    then_branch: inner_then,
                    else_branch: inner_else,
                    ..
                } => {
                    assert_matches!(inner_cond, Expr::Lt { .. });
                    assert_eq!(inner_then.len(), 1);
                    assert_matches!(inner_then[0], Stmt::Expression { .. });

                    // Final else clause
                    assert!(inner_else.is_some());
                    let final_else = inner_else.as_ref().unwrap();
                    assert_eq!(final_else.len(), 1);
                    assert_matches!(final_else[0], Stmt::Expression { .. });
                }
                other => panic!("Expected nested Stmt::If in else branch, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_nested() {
    let input = "if a { if b { x = 1; } }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(condition, Expr::Var { name: "a", .. });
            assert_eq!(then_branch.len(), 1);
            assert!(else_branch.is_none());

            // Check nested if statement
            match &then_branch[0] {
                Stmt::If {
                    condition: inner_cond,
                    then_branch: inner_then,
                    else_branch: inner_else,
                    ..
                } => {
                    assert_matches!(inner_cond, Expr::Var { name: "b", .. });
                    assert_eq!(inner_then.len(), 1);
                    assert_matches!(inner_then[0], Stmt::Assignment { .. });
                    assert!(inner_else.is_none());
                }
                other => panic!("Expected nested Stmt::If, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_with_boolean_expression() {
    let input = "if x > 0 and y < 10 { doSomething(); }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(condition, Expr::And { .. });
            assert_eq!(then_branch.len(), 1);
            assert_matches!(then_branch[0], Stmt::Expression { .. });
            assert!(else_branch.is_none());
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_with_equality() {
    let input = "if x == 5 { return true; }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(condition, Expr::Eq { .. });
            assert_eq!(then_branch.len(), 1);
            assert_matches!(then_branch[0], Stmt::Return { .. });
            assert!(else_branch.is_none());
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_with_multiple_statements() {
    let input = "if condition { let x = 1; let y = 2; return x + y; }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(
                condition,
                Expr::Var {
                    name: "condition",
                    ..
                }
            );
            assert_eq!(then_branch.len(), 3);
            assert_matches!(then_branch[0], Stmt::Let { .. });
            assert_matches!(then_branch[1], Stmt::Let { .. });
            assert_matches!(then_branch[2], Stmt::Return { .. });
            assert!(else_branch.is_none());
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_in_function() {
    let input = "fn test(x: i32) -> i32 { if x > 0 { return x; } else { return 0; } }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef {
            name,
            params,
            return_type,
            body,
            return_expr,
            ..
        } => {
            assert_eq!(name, "test");
            assert_eq!(params.len(), 1);
            assert_matches!(return_type, Type::I32 { .. });
            assert_eq!(body.len(), 1);
            assert!(return_expr.is_none());

            // Check the if statement in the function body
            match &body[0] {
                Stmt::If {
                    condition,
                    then_branch,
                    else_branch,
                    ..
                } => {
                    assert_matches!(condition, Expr::Gt { .. });
                    assert_eq!(then_branch.len(), 1);
                    assert_matches!(then_branch[0], Stmt::Return { .. });
                    assert!(else_branch.is_some());
                    let else_stmts = else_branch.as_ref().unwrap();
                    assert_eq!(else_stmts.len(), 1);
                    assert_matches!(else_stmts[0], Stmt::Return { .. });
                }
                other => panic!("Expected Stmt::If in function body, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_with_for_loop() {
    let input = "if condition { for i in 0..10 { process(i); } }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(
                condition,
                Expr::Var {
                    name: "condition",
                    ..
                }
            );
            assert_eq!(then_branch.len(), 1);
            assert_matches!(then_branch[0], Stmt::For { .. });
            assert!(else_branch.is_none());
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_multiline() {
    let input = "if x > 0 {
    let result = x * 2;
    return result;
} else {
    return 0;
}";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(condition, Expr::Gt { .. });
            assert_eq!(then_branch.len(), 2);
            assert_matches!(then_branch[0], Stmt::Let { .. });
            assert_matches!(then_branch[1], Stmt::Return { .. });
            assert!(else_branch.is_some());
            let else_stmts = else_branch.unwrap();
            assert_eq!(else_stmts.len(), 1);
            assert_matches!(else_stmts[0], Stmt::Return { .. });
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_with_block_in_branches() {
    let input = "if condition { { let x = 1; } } else { { let y = 2; } }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(
                condition,
                Expr::Var {
                    name: "condition",
                    ..
                }
            );
            assert_eq!(then_branch.len(), 1);
            assert_matches!(then_branch[0], Stmt::Block { .. });
            assert!(else_branch.is_some());
            let else_stmts = else_branch.unwrap();
            assert_eq!(else_stmts.len(), 1);
            assert_matches!(else_stmts[0], Stmt::Block { .. });
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_with_field_access_condition() {
    let input = "if obj.field > 0 { doSomething(); }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(condition, Expr::Gt { .. });
            assert_eq!(then_branch.len(), 1);
            assert_matches!(then_branch[0], Stmt::Expression { .. });
            assert!(else_branch.is_none());
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}

#[test]
fn test_if_stmt_with_function_call_condition() {
    let input = "if isValid(x) { process(x); }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            assert_matches!(
                condition,
                Expr::Call {
                    name: "isValid",
                    ..
                }
            );
            assert_eq!(then_branch.len(), 1);
            assert_matches!(then_branch[0], Stmt::Expression { .. });
            assert!(else_branch.is_none());
        }
        other => panic!("Expected Stmt::If, got {:?}", other),
    }
}
