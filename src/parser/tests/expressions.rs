use super::helpers::*;

// ========================================================================
// Function Call Tests
// ========================================================================

#[test]
fn test_function_call_no_args() {
    // Test: foo()
    let result = parse_with_timeout(
        "foo()",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Call { name, args, .. } => {
            assert_eq!(name, "foo");
            assert_eq!(args.len(), 0);
        }
        other => panic!("Expected Expr::Call, got {:?}", other),
    }
}

#[test]
fn test_function_call_one_arg() {
    // Test: foo(42)
    let result = parse_with_timeout(
        "foo(42)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Call { name, args, .. } => {
            assert_eq!(name, "foo");
            assert_eq!(args.len(), 1);
            assert_matches!(args[0], Expr::IntLit { value: 42, .. });
        }
        other => panic!("Expected Expr::Call, got {:?}", other),
    }
}

#[test]
fn test_function_call_multiple_args() {
    // Test: add(1, 2, 3)
    let result = parse_with_timeout(
        "add(1, 2, 3)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Call { name, args, .. } => {
            assert_eq!(name, "add");
            assert_eq!(args.len(), 3);
            assert_matches!(args[0], Expr::IntLit { value: 1, .. });
            assert_matches!(args[1], Expr::IntLit { value: 2, .. });
            assert_matches!(args[2], Expr::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Call, got {:?}", other),
    }
}

#[test]
fn test_function_call_expr_args() {
    // Test: foo(1 + 2, 3 * 4)
    let result = parse_with_timeout(
        "foo(1 + 2, 3 * 4)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Call { name, args, .. } => {
            assert_eq!(name, "foo");
            assert_eq!(args.len(), 2);
            assert_matches!(args[0], Expr::Add { .. });
            assert_matches!(args[1], Expr::Mul { .. });
        }
        other => panic!("Expected Expr::Call, got {:?}", other),
    }
}

#[test]
fn test_function_call_nested() {
    // Test: foo(bar(42))
    let result = parse_with_timeout(
        "foo(bar(42))",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Call { name, args, .. } => {
            assert_eq!(name, "foo");
            assert_eq!(args.len(), 1);
            match &args[0] {
                Expr::Call {
                    name: inner_name,
                    args: inner_args,
                    ..
                } => {
                    assert_eq!(*inner_name, "bar");
                    assert_eq!(inner_args.len(), 1);
                    assert_matches!(inner_args[0], Expr::IntLit { value: 42, .. });
                }
                other => panic!("Expected inner Expr::Call, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Call, got {:?}", other),
    }
}

#[test]
fn test_function_call_in_expression() {
    // Test: foo(1) + bar(2)
    let result = parse_with_timeout(
        "foo(1) + bar(2)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            assert_matches!(*lhs, AddLhs::Call { .. });
            assert_matches!(*rhs, AddRhs::Call { .. });
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

// ========================================================================
// Method Call Tests
// ========================================================================

#[test]
fn test_method_call_no_args() {
    // Test: obj.method()
    let result = parse_with_timeout(
        "obj.method()",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            assert_matches!(*receiver, Expr::Var { name, .. } if name == "obj");
            assert_eq!(method, "method");
            assert_eq!(args.len(), 0);
        }
        other => panic!("Expected Expr::MethodCall, got {:?}", other),
    }
}

#[test]
fn test_method_call_one_arg() {
    // Test: obj.method(42)
    let result = parse_with_timeout(
        "obj.method(42)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            assert_matches!(*receiver, Expr::Var { name, .. } if name == "obj");
            assert_eq!(method, "method");
            assert_eq!(args.len(), 1);
            assert_matches!(args[0], Expr::IntLit { value: 42, .. });
        }
        other => panic!("Expected Expr::MethodCall, got {:?}", other),
    }
}

#[test]
fn test_method_call_multiple_args() {
    // Test: obj.method(1, 2, 3)
    let result = parse_with_timeout(
        "obj.method(1, 2, 3)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            assert_matches!(*receiver, Expr::Var { name, .. } if name == "obj");
            assert_eq!(method, "method");
            assert_eq!(args.len(), 3);
            assert_matches!(args[0], Expr::IntLit { value: 1, .. });
            assert_matches!(args[1], Expr::IntLit { value: 2, .. });
            assert_matches!(args[2], Expr::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::MethodCall, got {:?}", other),
    }
}

#[test]
fn test_method_call_chaining() {
    // Test: obj.method1().method2()
    let result = parse_with_timeout(
        "obj.method1().method2()",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            assert_eq!(method, "method2");
            assert_eq!(args.len(), 0);
            match *receiver {
                Expr::MethodCall {
                    receiver: inner_receiver,
                    method: inner_method,
                    args: inner_args,
                    ..
                } => {
                    assert_matches!(*inner_receiver, Expr::Var { name, .. } if name == "obj");
                    assert_eq!(inner_method, "method1");
                    assert_eq!(inner_args.len(), 0);
                }
                other => panic!("Expected inner Expr::MethodCall, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::MethodCall, got {:?}", other),
    }
}

#[test]
fn test_method_call_on_function_call() {
    // Test: foo().bar()
    let result = parse_with_timeout(
        "foo().bar()",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            assert_eq!(method, "bar");
            assert_eq!(args.len(), 0);
            assert_matches!(*receiver, Expr::Call { name, .. } if name == "foo");
        }
        other => panic!("Expected Expr::MethodCall, got {:?}", other),
    }
}

#[test]
fn test_method_call_with_expr_args() {
    // Test: obj.method(1 + 2, 3 * 4)
    let result = parse_with_timeout(
        "obj.method(1 + 2, 3 * 4)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            assert_matches!(*receiver, Expr::Var { name, .. } if name == "obj");
            assert_eq!(method, "method");
            assert_eq!(args.len(), 2);
            assert_matches!(args[0], Expr::Add { .. });
            assert_matches!(args[1], Expr::Mul { .. });
        }
        other => panic!("Expected Expr::MethodCall, got {:?}", other),
    }
}

#[test]
fn test_method_call_in_expression() {
    // Test: obj.method(1) + 2
    let result = parse_with_timeout(
        "obj.method(1) + 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            assert_matches!(*lhs, AddLhs::MethodCall { .. });
            assert_matches!(*rhs, AddRhs::IntLit { value: 2, .. });
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

#[test]
fn test_field_access() {
    // Test: obj.field
    let result = parse_with_timeout(
        "obj.field",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::FieldAccess {
            receiver, field, ..
        } => {
            assert_matches!(*receiver, Expr::Var { name, .. } if name == "obj");
            assert_eq!(field, "field");
        }
        other => panic!("Expected Expr::FieldAccess, got {:?}", other),
    }
}

#[test]
fn test_field_access_chaining() {
    // Test: obj.field1.field2
    let result = parse_with_timeout(
        "obj.field1.field2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::FieldAccess {
            receiver, field, ..
        } => {
            assert_eq!(field, "field2");
            match *receiver {
                Expr::FieldAccess {
                    receiver: inner_receiver,
                    field: inner_field,
                    ..
                } => {
                    assert_matches!(*inner_receiver, Expr::Var { name, .. } if name == "obj");
                    assert_eq!(inner_field, "field1");
                }
                other => panic!("Expected inner Expr::FieldAccess, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::FieldAccess, got {:?}", other),
    }
}

#[test]
fn test_field_access_on_function_call() {
    // Test: foo().field
    let result = parse_with_timeout(
        "foo().field",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::FieldAccess {
            receiver, field, ..
        } => {
            assert_eq!(field, "field");
            assert_matches!(*receiver, Expr::Call { name, .. } if name == "foo");
        }
        other => panic!("Expected Expr::FieldAccess, got {:?}", other),
    }
}

#[test]
fn test_field_access_then_method_call() {
    // Test: obj.field.method()
    let result = parse_with_timeout(
        "obj.field.method()",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::MethodCall {
            receiver,
            method,
            args,
            ..
        } => {
            assert_eq!(method, "method");
            assert_eq!(args.len(), 0);
            match *receiver {
                Expr::FieldAccess {
                    receiver: inner_receiver,
                    field,
                    ..
                } => {
                    assert_matches!(*inner_receiver, Expr::Var { name, .. } if name == "obj");
                    assert_eq!(field, "field");
                }
                other => panic!("Expected Expr::FieldAccess, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::MethodCall, got {:?}", other),
    }
}

#[test]
fn test_method_call_then_field_access() {
    // Test: obj.method().field
    let result = parse_with_timeout(
        "obj.method().field",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::FieldAccess {
            receiver, field, ..
        } => {
            assert_eq!(field, "field");
            match *receiver {
                Expr::MethodCall {
                    receiver: inner_receiver,
                    method,
                    args,
                    ..
                } => {
                    assert_matches!(*inner_receiver, Expr::Var { name, .. } if name == "obj");
                    assert_eq!(method, "method");
                    assert_eq!(args.len(), 0);
                }
                other => panic!("Expected Expr::MethodCall, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::FieldAccess, got {:?}", other),
    }
}

#[test]
fn test_field_access_in_expression() {
    // Test: obj.field + 2
    let result = parse_with_timeout(
        "obj.field + 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            assert_matches!(*lhs, AddLhs::FieldAccess { .. });
            assert_matches!(*rhs, AddRhs::IntLit { value: 2, .. });
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

// ============================================================================
// Array Literal Tests
// ============================================================================

#[test]
fn test_empty_array_literal() {
    let result = parse_with_timeout(
        "[]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::ArrayLit { elements, .. } => {
            assert_eq!(elements.len(), 0);
        }
        other => panic!("Expected Expr::ArrayLit, got {:?}", other),
    }
}

#[test]
fn test_array_literal_single_element() {
    let result = parse_with_timeout(
        "[42]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::ArrayLit { elements, .. } => {
            assert_eq!(elements.len(), 1);
            assert_matches!(elements[0], Expr::IntLit { value: 42, .. });
        }
        other => panic!("Expected Expr::ArrayLit, got {:?}", other),
    }
}

#[test]
fn test_array_literal_multiple_elements() {
    let result = parse_with_timeout(
        "[1, 2, 3]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::ArrayLit { elements, .. } => {
            assert_eq!(elements.len(), 3);
            assert_matches!(elements[0], Expr::IntLit { value: 1, .. });
            assert_matches!(elements[1], Expr::IntLit { value: 2, .. });
            assert_matches!(elements[2], Expr::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::ArrayLit, got {:?}", other),
    }
}

#[test]
fn test_array_literal_with_expressions() {
    let result = parse_with_timeout(
        "[1 + 2, 3 * 4]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::ArrayLit { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert_matches!(elements[0], Expr::Add { .. });
            assert_matches!(elements[1], Expr::Mul { .. });
        }
        other => panic!("Expected Expr::ArrayLit, got {:?}", other),
    }
}

#[test]
fn test_array_literal_trailing_comma() {
    let result = parse_with_timeout(
        "[1, 2, 3,]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::ArrayLit { elements, .. } => {
            assert_eq!(elements.len(), 3);
        }
        other => panic!("Expected Expr::ArrayLit, got {:?}", other),
    }
}

#[test]
fn test_nested_array_literal() {
    let result = parse_with_timeout(
        "[[1, 2], [3, 4]]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::ArrayLit { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert_matches!(elements[0], Expr::ArrayLit { .. });
            assert_matches!(elements[1], Expr::ArrayLit { .. });
        }
        other => panic!("Expected Expr::ArrayLit, got {:?}", other),
    }
}

// ============================================================================
// Array Indexing Tests
// ============================================================================

#[test]
fn test_array_index_simple() {
    let result = parse_with_timeout(
        "arr[0]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Index { array, index, .. } => {
            assert_matches!(*array, Expr::Var { name, .. } if name == "arr");
            assert_matches!(*index, Expr::IntLit { value, .. } if value == 0);
        }
        other => panic!("Expected Expr::Index, got {:?}", other),
    }
}

#[test]
fn test_array_index_chained() {
    let result = parse_with_timeout(
        "matrix[i][j]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Index { array, index, .. } => {
            // Outer index should be j
            assert_matches!(*index, Expr::Var { name, .. } if name == "j");

            // Inner should be matrix[i]
            assert_matches!(*array, Expr::Index { array: inner_array, index: inner_index, .. } => {
                assert_matches!(*inner_array, Expr::Var { name, .. } if name == "matrix");
                assert_matches!(*inner_index, Expr::Var { name, .. } if name == "i");
            });
        }
        other => panic!("Expected Expr::Index, got {:?}", other),
    }
}

#[test]
fn test_array_index_with_expression() {
    let result = parse_with_timeout(
        "arr[i + 1]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Index { array, index, .. } => {
            assert_matches!(*array, Expr::Var { name, .. } if name == "arr");
            assert_matches!(*index, Expr::Add { .. });
        }
        other => panic!("Expected Expr::Index, got {:?}", other),
    }
}

#[test]
fn test_array_index_on_field() {
    let result = parse_with_timeout(
        "obj.items[0]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Index { array, index, .. } => {
            assert_matches!(*index, Expr::IntLit { value, .. } if value == 0);
            assert_matches!(*array, Expr::FieldAccess { field, .. } if field == "items");
        }
        other => panic!("Expected Expr::Index, got {:?}", other),
    }
}

// ============================================================================
// Range Expression Tests
// ============================================================================

#[test]
fn test_range_simple() {
    let result = parse_with_timeout(
        "0..5",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Range { start, end, .. } => {
            assert_matches!(*start, Expr::IntLit { value, .. } if value == 0);
            assert_matches!(*end, Expr::IntLit { value, .. } if value == 5);
        }
        other => panic!("Expected Expr::Range, got {:?}", other),
    }
}

#[test]
fn test_range_with_variables() {
    let result = parse_with_timeout(
        "start..end",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Range { start, end, .. } => {
            assert_matches!(*start, Expr::Var { name, .. } if name == "start");
            assert_matches!(*end, Expr::Var { name, .. } if name == "end");
        }
        other => panic!("Expected Expr::Range, got {:?}", other),
    }
}

#[test]
fn test_range_with_arithmetic() {
    let result = parse_with_timeout(
        "i*2..n",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Range { start, end, .. } => {
            // Start should be i*2 (multiplication has higher precedence than range)
            assert_matches!(*start, Expr::Mul { .. });
            // End should be just n
            assert_matches!(*end, Expr::Var { name, .. } if name == "n");
        }
        other => panic!("Expected Expr::Range, got {:?}", other),
    }
}

#[test]
fn test_range_in_array_literal() {
    let result = parse_with_timeout(
        "[0..5]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::ArrayLit { elements, .. } => {
            assert_eq!(elements.len(), 1);
            assert_matches!(elements[0], Expr::Range { .. });
        }
        other => panic!("Expected Expr::ArrayLit, got {:?}", other),
    }
}

// ============================================================================
// Closure Expression Tests
// ============================================================================

#[test]
fn test_closure_single_param() {
    let result = parse_with_timeout(
        "|x| x + 1",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Closure { params, body, .. } => {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0], "x");
            assert_matches!(*body, Expr::Add { .. });
        }
        other => panic!("Expected Expr::Closure, got {:?}", other),
    }
}

#[test]
fn test_closure_multiple_params() {
    let result = parse_with_timeout(
        "|x, y| x * y",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Closure { params, body, .. } => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0], "x");
            assert_eq!(params[1], "y");
            assert_matches!(*body, Expr::Mul { .. });
        }
        other => panic!("Expected Expr::Closure, got {:?}", other),
    }
}

#[test]
fn test_closure_no_params() {
    let result = parse_with_timeout(
        "|| 42",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Closure { params, body, .. } => {
            assert_eq!(params.len(), 0);
            assert_matches!(*body, Expr::IntLit { value, .. } if value == 42);
        }
        other => panic!("Expected Expr::Closure, got {:?}", other),
    }
}

#[test]
fn test_closure_in_method_call() {
    let result = parse_with_timeout(
        "points.map(|p| p.x)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::MethodCall { method, args, .. } => {
            assert_eq!(method, "map");
            assert_eq!(args.len(), 1);
            assert_matches!(args[0], Expr::Closure { .. });
        }
        other => panic!("Expected Expr::MethodCall, got {:?}", other),
    }
}

// ============================================================================
// Struct Literal Tests
// ============================================================================

#[test]
fn test_empty_struct_literal() {
    let result = parse_with_timeout(
        "Point {}",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::StructLit { name, fields, .. } => {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 0);
        }
        other => panic!("Expected Expr::StructLit, got {:?}", other),
    }
}

#[test]
fn test_struct_literal_single_field() {
    let result = parse_with_timeout(
        "Point { x: 10 }",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::StructLit { name, fields, .. } => {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 1);
            assert_matches!(
                &fields[0],
                StructLitField::Field {
                    name: "x",
                    value: Expr::IntLit { value: 10, .. },
                    ..
                }
            );
        }
        other => panic!("Expected Expr::StructLit, got {:?}", other),
    }
}

#[test]
fn test_struct_literal_multiple_fields() {
    let result = parse_with_timeout(
        "Point { x: 10, y: 20 }",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::StructLit { name, fields, .. } => {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
            assert_matches!(
                &fields[0],
                StructLitField::Field {
                    name: "x",
                    value: Expr::IntLit { value: 10, .. },
                    ..
                }
            );
            assert_matches!(
                &fields[1],
                StructLitField::Field {
                    name: "y",
                    value: Expr::IntLit { value: 20, .. },
                    ..
                }
            );
        }
        other => panic!("Expected Expr::StructLit, got {:?}", other),
    }
}

#[test]
fn test_struct_literal_with_expressions() {
    let result = parse_with_timeout(
        "Circle { center: point(0, 0), radius: 5 * 2 }",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::StructLit { name, fields, .. } => {
            assert_eq!(name, "Circle");
            assert_eq!(fields.len(), 2);
            assert_matches!(
                &fields[0],
                StructLitField::Field {
                    name: "center",
                    value: Expr::Call { .. },
                    ..
                }
            );
            assert_matches!(
                &fields[1],
                StructLitField::Field {
                    name: "radius",
                    value: Expr::Mul { .. },
                    ..
                }
            );
        }
        other => panic!("Expected Expr::StructLit, got {:?}", other),
    }
}

#[test]
fn test_struct_literal_trailing_comma() {
    let result = parse_with_timeout(
        "Point { x: 10, y: 20, }",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::StructLit { name, fields, .. } => {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
        }
        other => panic!("Expected Expr::StructLit, got {:?}", other),
    }
}

#[test]
fn test_nested_struct_literal() {
    let result = parse_with_timeout(
        "Line { start: Point { x: 0, y: 0 }, end: Point { x: 10, y: 10 } }",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::StructLit { name, fields, .. } => {
            assert_eq!(name, "Line");
            assert_eq!(fields.len(), 2);
            assert_matches!(
                &fields[0],
                StructLitField::Field {
                    name: "start",
                    value: Expr::StructLit { .. },
                    ..
                }
            );
            assert_matches!(
                &fields[1],
                StructLitField::Field {
                    name: "end",
                    value: Expr::StructLit { .. },
                    ..
                }
            );
        }
        other => panic!("Expected Expr::StructLit, got {:?}", other),
    }
}

#[test]
fn test_array_of_struct_literals() {
    let result = parse_with_timeout(
        "[Point { x: 0, y: 0 }, Point { x: 10, y: 10 }]",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::ArrayLit { elements, .. } => {
            assert_eq!(elements.len(), 2);
            assert_matches!(elements[0], Expr::StructLit { .. });
            assert_matches!(elements[1], Expr::StructLit { .. });
        }
        other => panic!("Expected Expr::ArrayLit, got {:?}", other),
    }
}

#[test]
fn test_struct_literal_computed_property() {
    let result = parse_with_timeout(
        "Rect { area() = 500 }",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::StructLit { name, fields, .. } => {
            assert_eq!(name, "Rect");
            assert_eq!(fields.len(), 1);
            assert_matches!(
                &fields[0],
                StructLitField::ComputedProperty {
                    name: "area",
                    value: Expr::IntLit { value: 500, .. },
                    ..
                }
            );
        }
        other => panic!("Expected Expr::StructLit, got {:?}", other),
    }
}

#[test]
fn test_struct_literal_mixed_fields_and_computed() {
    let result = parse_with_timeout(
        "Rect { width: 100, area() = 500, height: 50 }",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::StructLit { name, fields, .. } => {
            assert_eq!(name, "Rect");
            assert_eq!(fields.len(), 3);
            assert_matches!(
                &fields[0],
                StructLitField::Field {
                    name: "width",
                    value: Expr::IntLit { value: 100, .. },
                    ..
                }
            );
            assert_matches!(
                &fields[1],
                StructLitField::ComputedProperty {
                    name: "area",
                    value: Expr::IntLit { value: 500, .. },
                    ..
                }
            );
            assert_matches!(
                &fields[2],
                StructLitField::Field {
                    name: "height",
                    value: Expr::IntLit { value: 50, .. },
                    ..
                }
            );
        }
        other => panic!("Expected Expr::StructLit, got {:?}", other),
    }
}

#[test]
fn test_struct_literal_computed_property_expression() {
    let result = parse_with_timeout(
        "Circle { center: point(0, 0), circumference() = 2 * 3.14 * r }",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::StructLit { name, fields, .. } => {
            assert_eq!(name, "Circle");
            assert_eq!(fields.len(), 2);
            assert_matches!(
                &fields[0],
                StructLitField::Field {
                    name: "center",
                    value: Expr::Call { .. },
                    ..
                }
            );
            assert_matches!(
                &fields[1],
                StructLitField::ComputedProperty {
                    name: "circumference",
                    value: Expr::Mul { .. },
                    ..
                }
            );
        }
        other => panic!("Expected Expr::StructLit, got {:?}", other),
    }
}
