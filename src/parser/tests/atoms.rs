use super::helpers::*;

#[test]
fn test_int_lit() {
    let result = parse_with_timeout(
        "42",
        |input| atoms::int_lit().parse(input).into_result(),
        Duration::from_secs(1),
    );
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_float_lit() {
    let result = parse_with_timeout(
        "3.5",
        |input| atoms::float_lit().parse(input).into_result(),
        Duration::from_secs(1),
    );
    assert_eq!(result.unwrap(), 3.5);
}

#[test]
fn test_var() {
    let result = parse_with_timeout(
        "foo",
        |input| atoms::var().parse(input).into_result(),
        Duration::from_secs(1),
    );
    assert_eq!(result.unwrap(), "foo");
}

#[test]
fn test_bool_lit_true() {
    let result = parse_with_timeout(
        "true",
        |input| atoms::bool_lit().parse(input).into_result(),
        Duration::from_secs(1),
    );
    assert_eq!(result.unwrap(), true);
}

#[test]
fn test_bool_lit_false() {
    let result = parse_with_timeout(
        "false",
        |input| atoms::bool_lit().parse(input).into_result(),
        Duration::from_secs(1),
    );
    assert_eq!(result.unwrap(), false);
}

#[test]
fn test_expr_bool_true() {
    let result = parse_with_timeout(
        "true",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert_matches!(result.unwrap(), Expr::BoolLit { value: true, .. });
}

#[test]
fn test_expr_bool_false() {
    let result = parse_with_timeout(
        "false",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert_matches!(result.unwrap(), Expr::BoolLit { value: false, .. });
}

#[test]
fn test_expr_simple_var() {
    let result = parse_with_timeout(
        "x",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert_matches!(result.unwrap(), Expr::Var { name, .. } if name == "x");
}

#[test]
fn test_expr_simple_int() {
    let result = parse_with_timeout(
        "42",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert_matches!(result.unwrap(), Expr::IntLit { value: 42, .. });
}
