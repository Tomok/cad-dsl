use super::helpers::*;

#[test]
fn test_error_missing_right_operand() {
    // "1 +" should fail - missing right operand
    let result = parse_with_timeout(
        "1 +",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert!(result.is_err(), "Should fail with missing right operand");
}

#[test]
fn test_error_double_operator() {
    // "1 + + 2" should fail - double operator
    let result = parse_with_timeout(
        "1 + + 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert!(result.is_err(), "Should fail with double operator");
}

#[test]
fn test_error_missing_closing_paren() {
    // "(1 + 2" should fail - missing closing parenthesis
    let result = parse_with_timeout(
        "(1 + 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert!(result.is_err(), "Should fail with missing closing paren");
}

#[test]
fn test_error_missing_operator() {
    // "1 2" should fail - missing operator
    let result = parse_with_timeout(
        "1 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert!(result.is_err(), "Should fail with missing operator");
}

#[test]
fn test_error_missing_left_operand() {
    // "* 2" should fail - missing left operand
    let result = parse_with_timeout(
        "* 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert!(result.is_err(), "Should fail with missing left operand");
}

#[test]
fn test_error_empty_input() {
    // "" should fail - empty input
    let result = parse_with_timeout(
        "",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert!(result.is_err(), "Should fail with empty input");
}

#[test]
fn test_error_empty_parentheses() {
    // "()" should fail - empty parentheses
    let result = parse_with_timeout(
        "()",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert!(result.is_err(), "Should fail with empty parentheses");
}

#[test]
fn test_error_unclosed_nested_parens() {
    // "((1 + 2)" should fail - unclosed nested parentheses
    let result = parse_with_timeout(
        "((1 + 2)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );
    assert!(result.is_err(), "Should fail with unclosed nested parens");
}

// ========================================================================
// Error Reporting Example (demonstrates Ariadne integration)
// ========================================================================

#[test]
#[ignore] // Ignore by default as it prints to stderr
fn test_ariadne_error_reporting_example() {
    // This test demonstrates how to use report_parse_errors
    // Run with: cargo test test_ariadne_error_reporting_example -- --ignored --nocapture

    use super::report_parse_errors;

    let source = "1 + + 2";
    let tokens = lexer::tokenize(source).unwrap();

    // Parse and capture errors
    match expr().parse(&tokens).into_result() {
        Ok(_) => panic!("Expected parse error"),
        Err(errors) => {
            println!("\n=== Ariadne Error Report Example ===\n");
            report_parse_errors("example.cad", source, errors);
            println!("\n=== End of Example ===\n");
        }
    }
}
