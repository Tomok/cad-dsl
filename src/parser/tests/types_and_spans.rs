use super::helpers::*;

// ========================================================================
// Type Annotation Tests
// ========================================================================

#[test]
fn test_type_bool() {
    let result = parse_with_timeout(
        "bool",
        |input| type_annotation().parse(input).into_result(),
        Duration::from_secs(1),
    );
    assert_matches!(result.unwrap(), Type::Bool { .. });
}

#[test]
fn test_type_i32() {
    let result = parse_with_timeout(
        "i32",
        |input| type_annotation().parse(input).into_result(),
        Duration::from_secs(1),
    );
    assert_matches!(result.unwrap(), Type::I32 { .. });
}

#[test]
fn test_type_f64() {
    let result = parse_with_timeout(
        "f64",
        |input| type_annotation().parse(input).into_result(),
        Duration::from_secs(1),
    );
    assert_matches!(result.unwrap(), Type::F64 { .. });
}

#[test]
fn test_type_real() {
    let result = parse_with_timeout(
        "Real",
        |input| type_annotation().parse(input).into_result(),
        Duration::from_secs(1),
    );
    assert_matches!(result.unwrap(), Type::Real { .. });
}

#[test]
fn test_type_algebraic() {
    let result = parse_with_timeout(
        "Algebraic",
        |input| type_annotation().parse(input).into_result(),
        Duration::from_secs(1),
    );
    assert_matches!(result.unwrap(), Type::Algebraic { .. });
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

// ========================================================================
// Span Tracking Tests
// ========================================================================

#[test]
fn test_span_simple_int_literal() {
    // Test: 42
    let result = parse_with_timeout(
        "42",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    let expr = result.unwrap();
    let span = expr.span();

    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1);
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 3); // "42" is 2 chars, end_column is exclusive
}

#[test]
fn test_span_simple_var() {
    // Test: foo
    let result = parse_with_timeout(
        "foo",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    let expr = result.unwrap();
    let span = expr.span();

    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1);
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 4); // "foo" is 3 chars
}

#[test]
fn test_span_binary_addition() {
    // Test: 1 + 2
    let result = parse_with_timeout(
        "1 + 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    let expr = result.unwrap();
    let span = expr.span();

    // Span should cover from "1" to "2"
    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1);
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 6); // "1 + 2" covers columns 1-5
}

#[test]
fn test_span_nested_expression() {
    // Test: 1 + 2 * 3
    let result = parse_with_timeout(
        "1 + 2 * 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    let expr = result.unwrap();
    let span = expr.span();

    // Span should cover the entire expression
    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1);
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 10);
}

#[test]
fn test_span_parenthesized() {
    // Test: (1 + 2)
    let result = parse_with_timeout(
        "(1 + 2)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    let expr = result.unwrap();
    let span = expr.span();

    // Span should include the parentheses
    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1); // Start at '('
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 8); // End after ')'
}

#[test]
fn test_span_unary_negation() {
    // Test: -42
    let result = parse_with_timeout(
        "-42",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    let expr = result.unwrap();
    let span = expr.span();

    // Span should start at '-' and end after '42'
    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1);
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 4);
}

#[test]
fn test_span_comparison() {
    // Test: 1 == 2
    let result = parse_with_timeout(
        "1 == 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    let expr = result.unwrap();
    let span = expr.span();

    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1);
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 7);
}

#[test]
fn test_span_logical_and() {
    // Test: true and false
    let result = parse_with_timeout(
        "true and false",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    let expr = result.unwrap();
    let span = expr.span();

    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1);
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 15);
}

#[test]
fn test_span_power_operator() {
    // Test: 2 ^ 3
    let result = parse_with_timeout(
        "2 ^ 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    let expr = result.unwrap();
    let span = expr.span();

    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1);
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 6);
}

#[test]
fn test_span_let_statement() {
    // Test: let x = 42;
    let result = parse_with_timeout(
        "let x = 42;",
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
            // name_path should have one element "x"
            assert_eq!(name_path.len(), 1);
            let name_span = name_path[0].1;
            assert_eq!(name_span.start.line, 1);
            assert_eq!(name_span.start.column, 5); // "x" starts at column 5

            // Overall span should cover entire statement
            assert_eq!(span.start.line, 1);
            assert_eq!(span.start.column, 1); // Starts at "let"
            assert_eq!(span.lines, 0);
            assert_eq!(span.end_column, 12); // Ends after ';'
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
    }
}

#[test]
fn test_span_type_annotation() {
    // Test: i32
    let result = parse_with_timeout(
        "i32",
        |input| type_annotation().parse(input).into_result(),
        Duration::from_secs(1),
    );

    let type_ann = result.unwrap();
    let span = type_ann.span();

    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1);
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 4); // "i32" is 3 chars
}

#[test]
fn test_span_complex_nested() {
    // Test: (1 + 2) * 3
    let result = parse_with_timeout(
        "(1 + 2) * 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    let expr = result.unwrap();
    let span = expr.span();

    // Should span from '(' to '3'
    assert_eq!(span.start.line, 1);
    assert_eq!(span.start.column, 1);
    assert_eq!(span.lines, 0);
    assert_eq!(span.end_column, 12);
}

#[test]
fn test_hasspan_trait_for_different_types() {
    // Test that HasSpan trait works for various AST node types
    use crate::ast::HasSpan;

    let expr_result = parse_with_timeout(
        "42",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    let expr = expr_result.unwrap();
    let _expr_span = expr.span(); // Uses HasSpan trait

    let type_result = parse_with_timeout(
        "bool",
        |input| type_annotation().parse(input).into_result(),
        Duration::from_secs(1),
    );
    let type_ann = type_result.unwrap();
    let _type_span = type_ann.span(); // Uses HasSpan trait

    // If we get here without panic, HasSpan works for all types
    assert!(true);
}
