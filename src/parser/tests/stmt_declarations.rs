use super::helpers::*;

// ========================================================================
// Let Statement Tests
// ========================================================================

#[test]
fn test_let_with_type_and_init() {
    // let x: i32 = 42;
    let result = parse_with_timeout(
        "let x: i32 = 42;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(name_path.len(), 1);
            assert_eq!(name_path[0].0, "x");
            assert_matches!(type_annotation, Some(Type::I32 { .. }));
            assert_matches!(init, Some(Expr::IntLit { value: 42, .. }));
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

#[test]
fn test_let_with_type_only() {
    // let y: bool;
    let result = parse_with_timeout(
        "let y: bool;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(name_path.len(), 1);
            assert_eq!(name_path[0].0, "y");
            assert_matches!(type_annotation, Some(Type::Bool { .. }));
            assert!(init.is_none());
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

#[test]
fn test_let_with_init_only() {
    // let z = 3.14;
    let result = parse_with_timeout(
        "let z = 3.14;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(name_path.len(), 1);
            assert_eq!(name_path[0].0, "z");
            assert!(type_annotation.is_none());
            assert_matches!(init, Some(Expr::FloatLit { value, .. }) if value == 3.14);
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

#[test]
fn test_let_no_type_no_init() {
    // let w;
    let result = parse_with_timeout(
        "let w;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(name_path.len(), 1);
            assert_eq!(name_path[0].0, "w");
            assert!(type_annotation.is_none());
            assert!(init.is_none());
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

#[test]
fn test_let_with_expression() {
    // let result: i32 = 1 + 2 * 3;
    let result = parse_with_timeout(
        "let result: i32 = 1 + 2 * 3;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(name_path.len(), 1);
            assert_eq!(name_path[0].0, "result");
            assert_matches!(type_annotation, Some(Type::I32 { .. }));
            match init {
                Some(Expr::Add { lhs, rhs, .. }) => {
                    assert_matches!(*lhs, AddLhs::IntLit { value: 1, .. });
                    match *rhs {
                        AddRhs::Mul {
                            lhs: ref mul_lhs,
                            rhs: ref mul_rhs,
                            ..
                        } => {
                            assert_matches!(**mul_lhs, MulLhs::IntLit { value: 2, .. });
                            assert_matches!(**mul_rhs, MulRhs::IntLit { value: 3, .. });
                        }
                        ref other => panic!("Expected AddRhs::Mul, got {:?}", other),
                    }
                }
                other => panic!("Expected Some(Expr::Add), got {:?}", other),
            }
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

// ========================================================================
// Container Field Declaration Tests
// ========================================================================

#[test]
fn test_let_container_field_simple() {
    // let obj.field: i32 = 42;
    let result = parse_with_timeout(
        "let obj.field: i32 = 42;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(name_path.len(), 2);
            assert_eq!(name_path[0].0, "obj");
            assert_eq!(name_path[1].0, "field");
            assert_matches!(type_annotation, Some(Type::I32 { .. }));
            assert_matches!(init, Some(Expr::IntLit { value: 42, .. }));
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

#[test]
fn test_let_container_field_nested() {
    // let sketch.entities.p1: Point = point();
    let result = parse_with_timeout(
        "let sketch.entities.p1: Point;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(name_path.len(), 3);
            assert_eq!(name_path[0].0, "sketch");
            assert_eq!(name_path[1].0, "entities");
            assert_eq!(name_path[2].0, "p1");
            assert_matches!(
                type_annotation,
                Some(Type::UserDefined { name, .. }) if name == "Point"
            );
            assert!(init.is_none());
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

#[test]
fn test_let_container_field_with_expression() {
    // let obj.value: i32 = 10 + 20;
    let result = parse_with_timeout(
        "let obj.value: i32 = 10 + 20;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(name_path.len(), 2);
            assert_eq!(name_path[0].0, "obj");
            assert_eq!(name_path[1].0, "value");
            assert_matches!(type_annotation, Some(Type::I32 { .. }));
            assert_matches!(init, Some(Expr::Add { .. }));
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

#[test]
fn test_let_container_field_no_type() {
    // let obj.field = 3.14;
    let result = parse_with_timeout(
        "let obj.field = 3.14;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(name_path.len(), 2);
            assert_eq!(name_path[0].0, "obj");
            assert_eq!(name_path[1].0, "field");
            assert!(type_annotation.is_none());
            assert_matches!(init, Some(Expr::FloatLit { value, .. }) if value == 3.14);
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

#[test]
fn test_let_container_field_deeply_nested() {
    // let a.b.c.d.e: bool = true;
    let result = parse_with_timeout(
        "let a.b.c.d.e: bool = true;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(name_path.len(), 5);
            assert_eq!(name_path[0].0, "a");
            assert_eq!(name_path[1].0, "b");
            assert_eq!(name_path[2].0, "c");
            assert_eq!(name_path[3].0, "d");
            assert_eq!(name_path[4].0, "e");
            assert_matches!(type_annotation, Some(Type::Bool { .. }));
            assert_matches!(init, Some(Expr::BoolLit { value: true, .. }));
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

#[test]
fn test_let_container_field_span_tracking() {
    // let obj.field: i32 = 42;
    let result = parse_with_timeout(
        "let obj.field: i32 = 42;",
        |input| let_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix: _,
            name_path,
            span,
            ..
        } => {
            assert_eq!(name_path.len(), 2);

            // First segment "obj" should start at column 5
            assert_eq!(name_path[0].1.start.line, 1);
            assert_eq!(name_path[0].1.start.column, 5);

            // Second segment "field" should start at column 9 (after "obj.")
            assert_eq!(name_path[1].1.start.line, 1);
            assert_eq!(name_path[1].1.start.column, 9);

            // Overall span should cover entire statement
            assert_eq!(span.start.line, 1);
            assert_eq!(span.start.column, 1); // Starts at "let"
            assert_eq!(span.lines, 0);
            assert_eq!(span.end_column, 25); // Ends after ';'
        }
        Stmt::Assignment { .. } => panic!("Expected Stmt::Let, got Assignment"),
        Stmt::FieldAssignment { .. } => panic!("Expected Stmt::Let, got FieldAssignment"),
        Stmt::For { .. } => panic!("Expected Stmt::Let, got For"),
        Stmt::FunctionDef { .. } => panic!("Expected Stmt::Let, got FunctionDef"),
        Stmt::StructDef { .. } => panic!("Expected Stmt::Let, got StructDef"),
        Stmt::Return { .. } => panic!("Expected Stmt::Let, got Return"),
        Stmt::Expression { .. } => panic!("Expected Stmt::Let, got Expression"),
        Stmt::Block { .. } => panic!("Expected Stmt::Let, got Block"),
        Stmt::With { .. } => panic!("Expected Stmt::Let, got With"),
        Stmt::If { .. } => panic!("Expected Stmt::Let, got If"),
        Stmt::Optimize { .. } => panic!("Expected Stmt::Let, got Optimize"),
        Stmt::UnitDecl { .. }
        | Stmt::UnitDef { .. }
        | Stmt::UnitPrefixDecl { .. }
        | Stmt::Include { .. } => panic!("Unexpected unit/include stmt"),
        Stmt::GlobalRuneFn { .. } => panic!("Unexpected GlobalRuneFn stmt"),
    }
}

#[test]
fn test_let_dot_prefix_simple() {
    let result = parse_with_timeout(
        "let .field: i32 = 42;",
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(dot_prefix, true);
            assert_eq!(name_path.len(), 1);
            assert_eq!(name_path[0].0, "field");
            assert_matches!(type_annotation, Some(Type::I32 { .. }));
            assert_matches!(init, Some(Expr::IntLit { value: 42, .. }));
        }
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }
}

#[test]
fn test_let_dot_prefix_nested_path() {
    let result = parse_with_timeout(
        "let .p1.x: f64 = 10.0;",
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix,
            name_path,
            type_annotation,
            init,
            ..
        } => {
            assert_eq!(dot_prefix, true);
            assert_eq!(name_path.len(), 2);
            assert_eq!(name_path[0].0, "p1");
            assert_eq!(name_path[1].0, "x");
            assert_matches!(type_annotation, Some(Type::F64 { .. }));
            assert_matches!(init, Some(Expr::FloatLit { .. }));
        }
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }
}

#[test]
fn test_let_without_dot_prefix() {
    let result = parse_with_timeout(
        "let x: i32 = 42;",
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Let {
            dot_prefix,
            name_path,
            ..
        } => {
            assert_eq!(dot_prefix, false);
            assert_eq!(name_path.len(), 1);
            assert_eq!(name_path[0].0, "x");
        }
        other => panic!("Expected Stmt::Let, got {:?}", other),
    }
}

// ========================================================================
// Assignment Statement Tests
// ========================================================================

#[test]
fn test_assignment_simple_int() {
    // x = 42;
    let result = parse_with_timeout(
        "x = 42;",
        |input| assignment_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Assignment {
            name,
            name_span,
            value,
            span,
        } => {
            assert_eq!(name, "x");
            assert_eq!(name_span.start.line, 1);
            assert_eq!(name_span.start.column, 1);
            assert_matches!(value, Expr::IntLit { value: 42, .. });

            // Overall span should cover entire statement
            assert_eq!(span.start.line, 1);
            assert_eq!(span.start.column, 1);
            assert_eq!(span.lines, 0);
            assert_eq!(span.end_column, 8); // "x = 42;" is 7 chars
        }
        _ => panic!("Expected Stmt::Assignment"),
    }
}

#[test]
fn test_assignment_simple_float() {
    // width = 3.14;
    let result = parse_with_timeout(
        "width = 3.14;",
        |input| assignment_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Assignment { name, value, .. } => {
            assert_eq!(name, "width");
            assert_matches!(value, Expr::FloatLit { value, .. } if value == 3.14);
        }
        _ => panic!("Expected Stmt::Assignment"),
    }
}

#[test]
fn test_assignment_variable_to_variable() {
    // y = x;
    let result = parse_with_timeout(
        "y = x;",
        |input| assignment_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Assignment { name, value, .. } => {
            assert_eq!(name, "y");
            assert_matches!(value, Expr::Var { name, .. } if name == "x");
        }
        _ => panic!("Expected Stmt::Assignment"),
    }
}

#[test]
fn test_assignment_with_expression() {
    // result = 1 + 2 * 3;
    let result = parse_with_timeout(
        "result = 1 + 2 * 3;",
        |input| assignment_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Assignment { name, value, .. } => {
            assert_eq!(name, "result");
            assert_matches!(value, Expr::Add { .. });
        }
        _ => panic!("Expected Stmt::Assignment"),
    }
}

#[test]
fn test_assignment_boolean() {
    // flag = true;
    let result = parse_with_timeout(
        "flag = true;",
        |input| assignment_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Assignment { name, value, .. } => {
            assert_eq!(name, "flag");
            assert_matches!(value, Expr::BoolLit { value: true, .. });
        }
        _ => panic!("Expected Stmt::Assignment"),
    }
}

#[test]
fn test_assignment_complex_expression() {
    // area = width * height;
    let result = parse_with_timeout(
        "area = width * height;",
        |input| assignment_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Assignment { name, value, .. } => {
            assert_eq!(name, "area");
            assert_matches!(value, Expr::Mul { .. });
        }
        _ => panic!("Expected Stmt::Assignment"),
    }
}

#[test]
fn test_assignment_comparison_expression() {
    // is_positive = x > 0;
    let result = parse_with_timeout(
        "is_positive = x > 0;",
        |input| assignment_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Assignment { name, value, .. } => {
            assert_eq!(name, "is_positive");
            assert_matches!(value, Expr::Gt { .. });
        }
        _ => panic!("Expected Stmt::Assignment"),
    }
}

#[test]
fn test_assignment_logical_expression() {
    // condition = a and b;
    let result = parse_with_timeout(
        "condition = a and b;",
        |input| assignment_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Assignment { name, value, .. } => {
            assert_eq!(name, "condition");
            assert_matches!(value, Expr::And { .. });
        }
        _ => panic!("Expected Stmt::Assignment"),
    }
}

#[test]
fn test_assignment_multiline_expression() {
    // value =
    //     10 + 20;
    let result = parse_with_timeout(
        "value = \n    10 + 20;",
        |input| assignment_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Assignment {
            name, value, span, ..
        } => {
            assert_eq!(name, "value");
            assert_matches!(value, Expr::Add { .. });

            // Span should span multiple lines
            assert_eq!(span.start.line, 1);
            assert_eq!(span.start.column, 1);
            assert_eq!(span.lines, 1); // Spans 2 lines total (lines=1 means line2-line1)
        }
        _ => panic!("Expected Stmt::Assignment"),
    }
}

#[test]
fn test_assignment_parenthesized_expression() {
    // result = (a + b) * c;
    let result = parse_with_timeout(
        "result = (a + b) * c;",
        |input| assignment_stmt(expr_inner()).parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::Assignment { name, value, .. } => {
            assert_eq!(name, "result");
            assert_matches!(value, Expr::Mul { .. });
        }
        _ => panic!("Expected Stmt::Assignment"),
    }
}

// ========================================================================
// Field Assignment Tests
// ========================================================================

#[test]
fn test_field_assignment_simple() {
    // obj.field = 42;
    let result = parse_with_timeout(
        "obj.field = 42;",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment {
            dot_prefix: _,
            field_path,
            value,
            span,
        } => {
            assert_eq!(field_path.len(), 2);
            assert_eq!(field_path[0].0, "obj");
            assert_eq!(field_path[1].0, "field");
            assert_matches!(value, Expr::IntLit { value: 42, .. });

            // Overall span should cover entire statement
            assert_eq!(span.start.line, 1);
            assert_eq!(span.start.column, 1);
            assert_eq!(span.lines, 0);
            assert_eq!(span.end_column, 16); // "obj.field = 42;" is 15 chars
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_nested_two_levels() {
    // sketch.origin.x = 10;
    let result = parse_with_timeout(
        "sketch.origin.x = 10;",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment {
            dot_prefix: _,
            field_path,
            value,
            ..
        } => {
            assert_eq!(field_path.len(), 3);
            assert_eq!(field_path[0].0, "sketch");
            assert_eq!(field_path[1].0, "origin");
            assert_eq!(field_path[2].0, "x");
            assert_matches!(value, Expr::IntLit { value: 10, .. });
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_deeply_nested() {
    // sketch.entities.p1.x = 5;
    let result = parse_with_timeout(
        "sketch.entities.p1.x = 5;",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment {
            dot_prefix: _,
            field_path,
            value,
            ..
        } => {
            assert_eq!(field_path.len(), 4);
            assert_eq!(field_path[0].0, "sketch");
            assert_eq!(field_path[1].0, "entities");
            assert_eq!(field_path[2].0, "p1");
            assert_eq!(field_path[3].0, "x");
            assert_matches!(value, Expr::IntLit { value: 5, .. });
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_with_float() {
    // point.x = 3.14;
    let result = parse_with_timeout(
        "point.x = 3.14;",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment { value, .. } => {
            assert_matches!(value, Expr::FloatLit { value, .. } if value == 3.14);
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_with_variable() {
    // obj.field = other_var;
    let result = parse_with_timeout(
        "obj.field = other_var;",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment { value, .. } => {
            assert_matches!(value, Expr::Var { name, .. } if name == "other_var");
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_with_expression() {
    // obj.width = 10 + 20;
    let result = parse_with_timeout(
        "obj.width = 10 + 20;",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment { value, .. } => {
            assert_matches!(value, Expr::Add { .. });
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_with_complex_expression() {
    // obj.area = width * height;
    let result = parse_with_timeout(
        "obj.area = width * height;",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment { value, .. } => {
            assert_matches!(value, Expr::Mul { .. });
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_with_field_access() {
    // obj.x = other.y;
    let result = parse_with_timeout(
        "obj.x = other.y;",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment { value, .. } => {
            assert_matches!(value, Expr::FieldAccess { .. });
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_with_method_call() {
    // obj.value = other.compute();
    let result = parse_with_timeout(
        "obj.value = other.compute();",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment { value, .. } => {
            assert_matches!(value, Expr::MethodCall { .. });
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_multiline() {
    // obj.value =
    //     10 + 20;
    let result = parse_with_timeout(
        "obj.value = \n    10 + 20;",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment { value, span, .. } => {
            assert_matches!(value, Expr::Add { .. });
            // Span should span multiple lines
            assert_eq!(span.start.line, 1);
            assert_eq!(span.start.column, 1);
            assert_eq!(span.lines, 1); // Spans 2 lines total
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_span_tracking() {
    // a.b = 1;
    let result = parse_with_timeout(
        "a.b = 1;",
        |input| {
            field_assignment_stmt(expr_inner())
                .parse(input)
                .into_result()
        },
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment {
            dot_prefix: _,
            field_path,
            span,
            ..
        } => {
            // Check field path spans
            assert_eq!(field_path[0].1.start.line, 1);
            assert_eq!(field_path[0].1.start.column, 1);
            assert_eq!(field_path[1].1.start.line, 1);
            assert_eq!(field_path[1].1.start.column, 3);

            // Check overall span
            assert_eq!(span.start.line, 1);
            assert_eq!(span.start.column, 1);
            assert_eq!(span.lines, 0);
            assert_eq!(span.end_column, 9); // "a.b = 1;" is 8 chars
        }
        _ => panic!("Expected Stmt::FieldAssignment"),
    }
}

#[test]
fn test_field_assignment_dot_prefix_simple() {
    let result = parse_with_timeout(
        ".field = 42;",
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment {
            dot_prefix,
            field_path,
            value,
            ..
        } => {
            assert_eq!(dot_prefix, true);
            assert_eq!(field_path.len(), 1);
            assert_eq!(field_path[0].0, "field");
            assert_matches!(value, Expr::IntLit { value: 42, .. });
        }
        other => panic!("Expected Stmt::FieldAssignment, got {:?}", other),
    }
}

#[test]
fn test_field_assignment_without_dot_prefix() {
    let result = parse_with_timeout(
        "obj.field = 42;",
        |tokens| stmt_parser_for_tests().parse(tokens).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Stmt::FieldAssignment {
            dot_prefix,
            field_path,
            ..
        } => {
            assert_eq!(dot_prefix, false);
            assert_eq!(field_path.len(), 2);
            assert_eq!(field_path[0].0, "obj");
            assert_eq!(field_path[1].0, "field");
        }
        other => panic!("Expected Stmt::FieldAssignment, got {:?}", other),
    }
}
