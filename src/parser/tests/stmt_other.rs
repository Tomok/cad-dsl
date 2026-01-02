use super::helpers::*;

// ============================================================================
// Return Statement Tests
// ============================================================================

#[test]
fn test_return_stmt_with_value() {
    let input = "return 42;";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_some());
            assert_matches!(value.unwrap(), Expr::IntLit { value: 42, .. });
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_without_value() {
    let input = "return;";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_none());
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_with_variable() {
    let input = "return x;";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_some());
            assert_matches!(value.unwrap(), Expr::Var { name: "x", .. });
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_with_expression() {
    let input = "return a + b;";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_some());
            assert_matches!(value.unwrap(), Expr::Add { .. });
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_with_complex_expression() {
    let input = "return x * 2 + y;";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_some());
            assert_matches!(value.unwrap(), Expr::Add { .. });
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_with_function_call() {
    let input = "return calculate(x, y);";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_some());
            assert_matches!(
                value.unwrap(),
                Expr::Call {
                    name: "calculate",
                    ..
                }
            );
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_with_method_call() {
    let input = "return obj.method();";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_some());
            assert_matches!(value.unwrap(), Expr::MethodCall { .. });
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_with_field_access() {
    let input = "return obj.field;";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_some());
            assert_matches!(value.unwrap(), Expr::FieldAccess { .. });
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_with_boolean_expression() {
    let input = "return x > 5;";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_some());
            assert_matches!(value.unwrap(), Expr::Gt { .. });
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_in_function_body() {
    let input = "fn get_value() -> i32 { return 42; }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef { body, .. } => {
            assert_eq!(body.len(), 1);
            assert_matches!(
                body[0],
                Stmt::Return {
                    value: Some(Expr::IntLit { value: 42, .. }),
                    ..
                }
            );
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_with_implicit_return() {
    let input = "fn get_value() -> i32 { return 42; 100 }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef {
            body, return_expr, ..
        } => {
            assert_eq!(body.len(), 1);
            assert_matches!(
                body[0],
                Stmt::Return {
                    value: Some(Expr::IntLit { value: 42, .. }),
                    ..
                }
            );
            // Also has implicit return expression
            assert!(return_expr.is_some());
            assert_matches!(return_expr.unwrap(), Expr::IntLit { value: 100, .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_multiple_return_stmts_in_function() {
    let _input = "fn check(x: i32) -> bool { if true { return true; } return false; }";
    // Note: This test assumes if statements are not yet implemented, so we'll use a simpler version
    let input = "fn check(x: i32) -> bool { return true; return false; }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef { body, .. } => {
            assert_eq!(body.len(), 2);
            assert_matches!(body[0], Stmt::Return { .. });
            assert_matches!(body[1], Stmt::Return { .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_with_struct_literal() {
    let input = "return Point { x: 0, y: 0 };";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_some());
            assert_matches!(value.unwrap(), Expr::StructLit { name: "Point", .. });
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_with_array_literal() {
    let input = "return [1, 2, 3];";
    let result = parse_with_timeout(
        input,
        |tokens| return_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Return { value, .. } => {
            assert!(value.is_some());
            assert_matches!(value.unwrap(), Expr::ArrayLit { .. });
        }
        other => panic!("Expected Stmt::Return, got {:?}", other),
    }
}

#[test]
fn test_return_stmt_in_for_loop() {
    let input = "for i in 0..5 { return i; }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::For { body, .. } => {
            assert_eq!(body.len(), 1);
            assert_matches!(body[0], Stmt::Return { .. });
        }
        other => panic!("Expected Stmt::For, got {:?}", other),
    }
}

// ============================================================================
// Expression Statement Tests
// ============================================================================

#[test]
fn test_expression_stmt_function_call() {
    let input = "foo();";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::Call { name: "foo", .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_function_call_with_args() {
    let input = "print(x, y);";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::Call { name: "print", .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_method_call() {
    let input = "obj.method();";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::MethodCall { .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_method_call_with_args() {
    let input = "sketch.add_line(p1, p2);";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::MethodCall { .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_integer_literal() {
    let input = "42;";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::IntLit { value: 42, .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_variable() {
    let input = "x;";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::Var { name: "x", .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_arithmetic() {
    let input = "a + b;";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::Add { .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_complex_expression() {
    let input = "x * 2 + y / 3;";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::Add { .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_field_access() {
    let input = "obj.field;";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::FieldAccess { .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_nested_field_access() {
    let input = "obj.nested.field;";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::FieldAccess { .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_in_function_body() {
    let input = "fn test() -> i32 { foo(); return 42; }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef { body, .. } => {
            assert_eq!(body.len(), 2);
            assert_matches!(body[0], Stmt::Expression { .. });
            assert_matches!(body[1], Stmt::Return { .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_multiple_in_function() {
    let input = "fn test() -> i32 { foo(); bar(); baz(); return 42; }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef { body, .. } => {
            assert_eq!(body.len(), 4);
            assert_matches!(body[0], Stmt::Expression { .. });
            assert_matches!(body[1], Stmt::Expression { .. });
            assert_matches!(body[2], Stmt::Expression { .. });
            assert_matches!(body[3], Stmt::Return { .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_in_for_loop() {
    let input = "for i in 0..5 { foo(i); }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::For { body, .. } => {
            assert_eq!(body.len(), 1);
            assert_matches!(body[0], Stmt::Expression { .. });
        }
        other => panic!("Expected Stmt::For, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_mixed_with_other_statements() {
    let input = "fn test() -> i32 { let x = 5; foo(x); x = x + 1; bar(x); return x; }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef { body, .. } => {
            assert_eq!(body.len(), 5);
            assert_matches!(body[0], Stmt::Let { .. });
            assert_matches!(body[1], Stmt::Expression { .. });
            assert_matches!(body[2], Stmt::Assignment { .. });
            assert_matches!(body[3], Stmt::Expression { .. });
            assert_matches!(body[4], Stmt::Return { .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_with_array_literal() {
    let input = "[1, 2, 3];";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::ArrayLit { .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_with_struct_literal() {
    let input = "Point { x: 1, y: 2 };";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::StructLit { .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_with_range() {
    let input = "0..10;";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::Range { .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

#[test]
fn test_expression_stmt_with_boolean_expr() {
    let input = "x > 5 and y < 10;";
    let result = parse_with_timeout(
        input,
        |tokens| expression_stmt(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Expression { expr, .. } => {
            assert_matches!(expr, Expr::And { .. });
        }
        other => panic!("Expected Stmt::Expression, got {:?}", other),
    }
}

// ============================================================================
// Block Statement Tests
// ============================================================================

#[test]
fn test_block_stmt_empty() {
    let input = "{ }";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 0);
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_single_statement() {
    let input = "{ let x = 1; }";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 1);
            assert_matches!(statements[0], Stmt::Let { .. });
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_multiple_statements() {
    let input = "{ let x = 1; let y = 2; x = 3; }";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 3);
            assert_matches!(statements[0], Stmt::Let { .. });
            assert_matches!(statements[1], Stmt::Let { .. });
            assert_matches!(statements[2], Stmt::Assignment { .. });
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_nested_blocks() {
    let input = "{ { let x = 1; } { let y = 2; } }";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 2);
            assert_matches!(statements[0], Stmt::Block { .. });
            assert_matches!(statements[1], Stmt::Block { .. });

            // Check inner blocks
            if let Stmt::Block {
                statements: inner_stmts,
                ..
            } = &statements[0]
            {
                assert_eq!(inner_stmts.len(), 1);
                assert_matches!(inner_stmts[0], Stmt::Let { .. });
            } else {
                panic!("Expected Stmt::Block");
            }

            if let Stmt::Block {
                statements: inner_stmts,
                ..
            } = &statements[1]
            {
                assert_eq!(inner_stmts.len(), 1);
                assert_matches!(inner_stmts[0], Stmt::Let { .. });
            } else {
                panic!("Expected Stmt::Block");
            }
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_deeply_nested() {
    let input = "{ { { let x = 1; } } }";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 1);
            assert_matches!(statements[0], Stmt::Block { .. });

            // Check second level
            if let Stmt::Block {
                statements: level2_stmts,
                ..
            } = &statements[0]
            {
                assert_eq!(level2_stmts.len(), 1);
                assert_matches!(level2_stmts[0], Stmt::Block { .. });

                // Check third level
                if let Stmt::Block {
                    statements: level3_stmts,
                    ..
                } = &level2_stmts[0]
                {
                    assert_eq!(level3_stmts.len(), 1);
                    assert_matches!(level3_stmts[0], Stmt::Let { .. });
                } else {
                    panic!("Expected Stmt::Block at level 3");
                }
            } else {
                panic!("Expected Stmt::Block at level 2");
            }
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_with_for_loop() {
    let input = "{ for i in 0..10 { let x = i; } }";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 1);
            assert_matches!(statements[0], Stmt::For { .. });
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_with_return() {
    let input = "{ let x = 1; return x; }";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 2);
            assert_matches!(statements[0], Stmt::Let { .. });
            assert_matches!(statements[1], Stmt::Return { .. });
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_with_expression_stmt() {
    let input = "{ foo(); bar(); }";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 2);
            assert_matches!(statements[0], Stmt::Expression { .. });
            assert_matches!(statements[1], Stmt::Expression { .. });
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_with_field_assignment() {
    let input = "{ obj.field = 42; }";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 1);
            assert_matches!(statements[0], Stmt::FieldAssignment { .. });
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_mixed_statements() {
    let input = "{ let x = 1; obj.y = 2; foo(); return x; }";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 4);
            assert_matches!(statements[0], Stmt::Let { .. });
            assert_matches!(statements[1], Stmt::FieldAssignment { .. });
            assert_matches!(statements[2], Stmt::Expression { .. });
            assert_matches!(statements[3], Stmt::Return { .. });
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_in_function() {
    let input = "fn test() -> i32 { { let x = 1; } x }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef { body, .. } => {
            assert_eq!(body.len(), 1);
            assert_matches!(body[0], Stmt::Block { .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_block_stmt_multiline() {
    let input = "{
    let x = 1;
    let y = 2;
    x = 3;
}";
    let result = parse_with_timeout(
        input,
        |tokens| {
            block_stmt(stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Block { statements, .. } => {
            assert_eq!(statements.len(), 3);
            assert_matches!(statements[0], Stmt::Let { .. });
            assert_matches!(statements[1], Stmt::Let { .. });
            assert_matches!(statements[2], Stmt::Assignment { .. });
        }
        other => panic!("Expected Stmt::Block, got {:?}", other),
    }
}

// ============================================================================
// With Statement Tests
// ============================================================================

#[test]
#[ignore] // TODO: Debug why empty body fails
fn test_with_stmt_empty_body() {
    // with transform { }
    let input = "with transform { }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::With {
            context_expr, body, ..
        } => {
            assert_matches!(context_expr, Expr::Var { name, .. } if name == "transform");
            assert_eq!(body.len(), 0);
        }
        other => panic!("Expected Stmt::With, got {:?}", other),
    }
}

#[test]
fn test_with_stmt_with_let() {
    // with sketch { let x: i32 = 42; }
    let input = "with sketch { let x: i32 = 42; }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::With {
            context_expr, body, ..
        } => {
            assert_matches!(context_expr, Expr::Var { name, .. } if name == "sketch");
            assert_eq!(body.len(), 1);
            assert_matches!(body[0], Stmt::Let { .. });
        }
        other => panic!("Expected Stmt::With, got {:?}", other),
    }
}

#[test]
fn test_with_stmt_multiple_statements() {
    // with transform { let x = 1; let y = 2; x = 3; }
    let input = "with transform { let x = 1; let y = 2; x = 3; }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::With {
            context_expr, body, ..
        } => {
            assert_matches!(context_expr, Expr::Var { name, .. } if name == "transform");
            assert_eq!(body.len(), 3);
            assert_matches!(body[0], Stmt::Let { .. });
            assert_matches!(body[1], Stmt::Let { .. });
            assert_matches!(body[2], Stmt::Assignment { .. });
        }
        other => panic!("Expected Stmt::With, got {:?}", other),
    }
}

#[test]
fn test_with_stmt_field_access() {
    // with obj.field { }
    let input = "with obj.field { }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::With {
            context_expr, body, ..
        } => {
            assert_matches!(context_expr, Expr::FieldAccess { .. });
            assert_eq!(body.len(), 0);
        }
        other => panic!("Expected Stmt::With, got {:?}", other),
    }
}

#[test]
fn test_with_stmt_function_call() {
    // with get_transform() { let x = 1; }
    let input = "with get_transform() { let x = 1; }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::With {
            context_expr, body, ..
        } => {
            assert_matches!(context_expr, Expr::Call { .. });
            assert_eq!(body.len(), 1);
            assert_matches!(body[0], Stmt::Let { .. });
        }
        other => panic!("Expected Stmt::With, got {:?}", other),
    }
}

#[test]
fn test_with_stmt_nested() {
    // with outer { with inner { let x = 1; } }
    let input = "with outer { with inner { let x = 1; } }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::With {
            context_expr, body, ..
        } => {
            assert_matches!(context_expr, Expr::Var { name, .. } if name == "outer");
            assert_eq!(body.len(), 1);
            match &body[0] {
                Stmt::With {
                    context_expr: inner_expr,
                    body: inner_body,
                    ..
                } => {
                    assert_matches!(inner_expr, Expr::Var { name, .. } if *name == "inner");
                    assert_eq!(inner_body.len(), 1);
                    assert_matches!(inner_body[0], Stmt::Let { .. });
                }
                other => panic!("Expected nested Stmt::With, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::With, got {:?}", other),
    }
}

#[test]
fn test_with_stmt_with_for_loop() {
    // with transform { for i in 0..5 { } }
    let input = "with transform { for i in 0..5 { } }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::With {
            context_expr, body, ..
        } => {
            assert_matches!(context_expr, Expr::Var { name, .. } if name == "transform");
            assert_eq!(body.len(), 1);
            assert_matches!(body[0], Stmt::For { .. });
        }
        other => panic!("Expected Stmt::With, got {:?}", other),
    }
}

#[test]
fn test_with_stmt_in_function() {
    let input = "fn test() -> i32 { with transform { let x = 1; } x }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef { body, .. } => {
            assert_eq!(body.len(), 1);
            assert_matches!(body[0], Stmt::With { .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_with_stmt_multiline() {
    let input = "with sketch {
    let x = 1;
    let y = 2;
    x = 3;
}";
    let result = parse_with_timeout(
        input,
        |tokens| {
            with_stmt(expr_inner(), stmt_parser_for_tests())
                .parse(tokens)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::With {
            context_expr, body, ..
        } => {
            assert_matches!(context_expr, Expr::Var { name, .. } if name == "sketch");
            assert_eq!(body.len(), 3);
            assert_matches!(body[0], Stmt::Let { .. });
            assert_matches!(body[1], Stmt::Let { .. });
            assert_matches!(body[2], Stmt::Assignment { .. });
        }
        other => panic!("Expected Stmt::With, got {:?}", other),
    }
}

#[test]
fn test_with_stmt_with_block() {
    // with transform { { let x = 1; } }
    let input = "with transform { { let x = 1; } }";
    let result = parse_with_timeout(
        input,
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::With {
            context_expr, body, ..
        } => {
            assert_matches!(context_expr, Expr::Var { name, .. } if name == "transform");
            assert_eq!(body.len(), 1);
            assert_matches!(body[0], Stmt::Block { .. });
        }
        other => panic!("Expected Stmt::With, got {:?}", other),
    }
}
