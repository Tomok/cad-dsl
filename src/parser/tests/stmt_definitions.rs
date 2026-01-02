use super::helpers::*;

// ============================================================================
// Function Definition Tests
// ============================================================================

#[test]
fn test_function_def_no_params() {
    let input = "fn compute() -> i32 { 42 }";
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
            assert_eq!(name, "compute");
            assert_eq!(params.len(), 0);
            assert_matches!(return_type, Type::I32 { .. });
            assert_eq!(body.len(), 0);
            assert!(return_expr.is_some());
            assert_matches!(return_expr.unwrap(), Expr::IntLit { value: 42, .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_function_def_with_one_param() {
    let input = "fn square(x: i32) -> i32 { x * x }";
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
            return_expr,
            ..
        } => {
            assert_eq!(name, "square");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");
            assert_matches!(params[0].type_annotation, Type::I32 { .. });
            assert_matches!(return_type, Type::I32 { .. });
            assert!(return_expr.is_some());
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_function_def_with_multiple_params() {
    let input = "fn add(x: i32, y: i32) -> i32 { x + y }";
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
            return_expr,
            ..
        } => {
            assert_eq!(name, "add");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "x");
            assert_eq!(params[1].name, "y");
            assert_matches!(params[0].type_annotation, Type::I32 { .. });
            assert_matches!(params[1].type_annotation, Type::I32 { .. });
            assert_matches!(return_type, Type::I32 { .. });
            assert!(return_expr.is_some());
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_function_def_with_reference_params() {
    let input = "fn distance(p1: &Point, p2: &Point) -> f64 { 0.0 }";
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
            ..
        } => {
            assert_eq!(name, "distance");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "p1");
            assert_eq!(params[1].name, "p2");

            // Check that both parameters are reference types
            assert_matches!(
                params[0].type_annotation,
                Type::Reference { ref inner, .. } if matches!(**inner, Type::UserDefined { ref name, .. } if name == "Point")
            );
            assert_matches!(
                params[1].type_annotation,
                Type::Reference { ref inner, .. } if matches!(**inner, Type::UserDefined { ref name, .. } if name == "Point")
            );

            assert_matches!(return_type, Type::F64 { .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_function_def_with_user_defined_return_type() {
    let input = "fn create_point() -> Point { point(0, 0) }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef {
            name, return_type, ..
        } => {
            assert_eq!(name, "create_point");
            assert_matches!(
                return_type,
                Type::UserDefined { ref name, .. } if name == "Point"
            );
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_function_def_with_body_statements() {
    let input = "fn calculate(x: i32) -> i32 { let y: i32 = x + 1; y * 2 }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef {
            name,
            body,
            return_expr,
            ..
        } => {
            assert_eq!(name, "calculate");
            assert_eq!(body.len(), 1);
            assert!(return_expr.is_some());

            // Check that the body contains a let statement
            assert_matches!(body[0], Stmt::Let { .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_function_def_no_return_expr() {
    let input = "fn init() -> bool { }";
    let result = parse_with_timeout(
        input,
        |tokens| function_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FunctionDef {
            name, return_expr, ..
        } => {
            assert_eq!(name, "init");
            assert!(return_expr.is_none());
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_function_def_mixed_param_types() {
    let input = "fn process(value: i32, scale: f64, ref: &Point) -> bool { true }";
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
            ..
        } => {
            assert_eq!(name, "process");
            assert_eq!(params.len(), 3);

            assert_eq!(params[0].name, "value");
            assert_matches!(params[0].type_annotation, Type::I32 { .. });

            assert_eq!(params[1].name, "scale");
            assert_matches!(params[1].type_annotation, Type::F64 { .. });

            assert_eq!(params[2].name, "ref");
            assert_matches!(params[2].type_annotation, Type::Reference { .. });

            assert_matches!(return_type, Type::Bool { .. });
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

#[test]
fn test_type_annotation_reference() {
    let input = "&Point";
    let tokens = lexer::tokenize(input).unwrap();
    let tokens_static: &'static [Token<'static>] = Box::leak(tokens.into_boxed_slice());

    let result = type_annotation().parse(tokens_static).into_result();

    match result.unwrap() {
        Type::Reference { inner, .. } => {
            assert_matches!(*inner, Type::UserDefined { ref name, .. } if name == "Point");
        }
        other => panic!("Expected Type::Reference, got {:?}", other),
    }
}

#[test]
fn test_type_annotation_user_defined() {
    let input = "Point";
    let tokens = lexer::tokenize(input).unwrap();
    let tokens_static: &'static [Token<'static>] = Box::leak(tokens.into_boxed_slice());

    let result = type_annotation().parse(tokens_static).into_result();

    assert_matches!(result.unwrap(), Type::UserDefined { ref name, .. } if name == "Point");
}

#[test]
fn test_function_def_complex_body() {
    let input =
        "fn area(width: f64, height: f64) -> f64 { let result: f64 = width * height; result }";
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
            assert_eq!(name, "area");
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "width");
            assert_eq!(params[1].name, "height");
            assert_matches!(return_type, Type::F64 { .. });
            assert_eq!(body.len(), 1);
            assert!(return_expr.is_some());
        }
        other => panic!("Expected Stmt::FunctionDef, got {:?}", other),
    }
}

// ============================================================================
// Struct Definition Tests
// ============================================================================

#[test]
fn test_struct_def_basic() {
    let input = "struct Point { x: f64, y: f64 }";
    let result = parse_with_timeout(
        input,
        |tokens| struct_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::StructDef {
            name,
            container,
            fields,
            methods,
            ..
        } => {
            assert_eq!(name, "Point");
            assert!(container.is_none());
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_matches!(fields[0].type_annotation, Type::F64 { .. });
            assert_eq!(fields[1].name, "y");
            assert_matches!(fields[1].type_annotation, Type::F64 { .. });
            assert_eq!(methods.len(), 0);
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }
}

#[test]
fn test_struct_def_with_container() {
    let input = "struct Sketch { container entities, origin: Point }";
    let result = parse_with_timeout(
        input,
        |tokens| struct_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::StructDef {
            name,
            container,
            fields,
            methods,
            ..
        } => {
            assert_eq!(name, "Sketch");
            assert!(container.is_some());
            assert_eq!(container.unwrap().0, "entities");
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "origin");
            assert_matches!(fields[0].type_annotation, Type::UserDefined { ref name, .. } if name == "Point");
            assert_eq!(methods.len(), 0);
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }
}

#[test]
fn test_struct_def_with_method() {
    let input = "struct Circle { radius: f64, fn area() -> f64 { radius * radius } }";
    let result = parse_with_timeout(
        input,
        |tokens| struct_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::StructDef {
            name,
            container,
            fields,
            methods,
            ..
        } => {
            assert_eq!(name, "Circle");
            assert!(container.is_none());
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "radius");
            assert_eq!(methods.len(), 1);

            match &methods[0] {
                Stmt::FunctionDef {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    assert_eq!(name, "area");
                    assert_eq!(params.len(), 0);
                    assert_matches!(return_type, Type::F64 { .. });
                }
                other => panic!("Expected FunctionDef, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }
}

#[test]
fn test_struct_def_with_transform() {
    let input = "struct Translate { offset_x: f64, offset_y: f64, fn __transform__(p: &Point) -> Point { p } }";
    let result = parse_with_timeout(
        input,
        |tokens| struct_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::StructDef {
            name,
            fields,
            methods,
            ..
        } => {
            assert_eq!(name, "Translate");
            assert_eq!(fields.len(), 2);
            assert_eq!(methods.len(), 1);

            match &methods[0] {
                Stmt::FunctionDef { name, params, .. } => {
                    assert_eq!(name, "__transform__");
                    assert_eq!(params.len(), 1);
                    assert_eq!(params[0].name, "p");
                    assert_matches!(params[0].type_annotation, Type::Reference { .. });
                }
                other => panic!("Expected FunctionDef, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }
}

#[test]
fn test_struct_def_complex() {
    let input = "struct Rectangle { container entities, width: f64, height: f64, fn area() -> f64 { width * height }, fn perimeter() -> f64 { width + height } }";
    let result = parse_with_timeout(
        input,
        |tokens| struct_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::StructDef {
            name,
            container,
            fields,
            methods,
            ..
        } => {
            assert_eq!(name, "Rectangle");
            assert!(container.is_some());
            assert_eq!(container.unwrap().0, "entities");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "width");
            assert_eq!(fields[1].name, "height");
            assert_eq!(methods.len(), 2);

            match &methods[0] {
                Stmt::FunctionDef { name, .. } => {
                    assert_eq!(name, "area");
                }
                other => panic!("Expected FunctionDef, got {:?}", other),
            }

            match &methods[1] {
                Stmt::FunctionDef { name, .. } => {
                    assert_eq!(name, "perimeter");
                }
                other => panic!("Expected FunctionDef, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }
}

#[test]
fn test_struct_def_empty() {
    let input = "struct Empty { }";
    let result = parse_with_timeout(
        input,
        |tokens| struct_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::StructDef {
            name,
            container,
            fields,
            methods,
            ..
        } => {
            assert_eq!(name, "Empty");
            assert!(container.is_none());
            assert_eq!(fields.len(), 0);
            assert_eq!(methods.len(), 0);
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }
}

#[test]
fn test_struct_def_trailing_comma() {
    let input = "struct Point { x: f64, y: f64, }";
    let result = parse_with_timeout(
        input,
        |tokens| struct_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::StructDef { name, fields, .. } => {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }
}

#[test]
fn test_struct_def_reference_types() {
    let input = "struct LineRef { start: &Point, end: &Point }";
    let result = parse_with_timeout(
        input,
        |tokens| struct_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::StructDef { name, fields, .. } => {
            assert_eq!(name, "LineRef");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "start");
            assert_matches!(fields[0].type_annotation, Type::Reference { .. });
            assert_eq!(fields[1].name, "end");
            assert_matches!(fields[1].type_annotation, Type::Reference { .. });
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }
}

#[test]
fn test_struct_def_with_self_reference() {
    let input =
        "struct Circle { center: Point, radius: f64, fn diameter() -> f64 { self.radius * 2.0 } }";
    let result = parse_with_timeout(
        input,
        |tokens| struct_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::StructDef {
            name,
            fields,
            methods,
            ..
        } => {
            assert_eq!(name, "Circle");
            assert_eq!(fields.len(), 2);
            assert_eq!(methods.len(), 1);

            match &methods[0] {
                Stmt::FunctionDef {
                    name, return_expr, ..
                } => {
                    assert_eq!(name, "diameter");
                    assert!(return_expr.is_some());
                    // The return expression should contain self.radius * 2.0
                    // We can check that it's a multiplication expression
                    assert_matches!(return_expr, Some(Expr::Mul { .. }));
                }
                other => panic!("Expected FunctionDef, got {:?}", other),
            }
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }
}

#[test]
fn test_struct_def_multiline() {
    let input = "struct Point {
    x: f64,
    y: f64
}";
    let result = parse_with_timeout(
        input,
        |tokens| struct_def(expr_inner()).parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::StructDef { name, fields, .. } => {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
        }
        other => panic!("Expected Stmt::StructDef, got {:?}", other),
    }
}
