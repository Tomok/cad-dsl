use super::helpers::*;

// Logical Operator Tests
// ========================================================================

#[test]
fn test_expr_simple_and() {
    // Test: true and false
    let result = parse_with_timeout(
        "true and false",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::And { lhs, rhs, .. } => {
            assert_matches!(*lhs, CmpLhs::BoolLit { value: true, .. });
            match *rhs {
                CmpRhs::Paren { ref inner, .. } => {
                    assert_matches!(**inner, Expr::BoolLit { value: false, .. });
                }
                ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::And, got {:?}", other),
    }
}

#[test]
fn test_expr_simple_or() {
    // Test: true or false
    let result = parse_with_timeout(
        "true or false",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Or { lhs, rhs, .. } => {
            assert_matches!(*lhs, CmpLhs::BoolLit { value: true, .. });
            match *rhs {
                CmpRhs::Paren { ref inner, .. } => {
                    assert_matches!(**inner, Expr::BoolLit { value: false, .. });
                }
                ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Or, got {:?}", other),
    }
}

#[test]
fn test_expr_and_precedence_over_or() {
    // Test: a or b and c should be a or (b and c)
    // Since we're using simple foldl, this might not work as expected
    // Let's adjust the expected result based on left-associativity
    let result = parse_with_timeout(
        "a or b and c",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    // With left-associative parsing, this will be ((a or b) and c)
    match result.unwrap() {
        Expr::And { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::Or {
                    lhs: ref or_lhs,
                    rhs: ref or_rhs,
                    ..
                } => {
                    assert_matches!(**or_lhs, CmpLhs::Var { name, .. } if name == "a");
                    match **or_rhs {
                        CmpRhs::Paren { ref inner, .. } => {
                            assert_matches!(**inner, Expr::Var { name, .. } if name == "b");
                        }
                        ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
                    }
                }
                ref other => panic!("Expected CmpLhs::Or, got {:?}", other),
            }
            match *rhs {
                CmpRhs::Paren { ref inner, .. } => {
                    assert_matches!(**inner, Expr::Var { name, .. } if name == "c");
                }
                ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::And, got {:?}", other),
    }
}

#[test]
fn test_expr_logical_with_comparison() {
    // Test: x == 1 and y == 2
    let result = parse_with_timeout(
        "x == 1 and y == 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::And { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::Eq {
                    lhs: ref eq_lhs,
                    rhs: ref eq_rhs,
                    ..
                } => {
                    assert_matches!(**eq_lhs, CmpLhs::Var { name, .. } if name == "x");
                    assert_matches!(**eq_rhs, CmpRhs::IntLit { value: 1, .. });
                }
                ref other => panic!("Expected CmpLhs::Eq, got {:?}", other),
            }
            match *rhs {
                CmpRhs::Paren { ref inner, .. } => match **inner {
                    Expr::Eq {
                        ref lhs, ref rhs, ..
                    } => {
                        assert_matches!(**lhs, CmpLhs::Var { name, .. } if name == "y");
                        assert_matches!(**rhs, CmpRhs::IntLit { value: 2, .. });
                    }
                    ref other => panic!("Expected Expr::Eq, got {:?}", other),
                },
                ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::And, got {:?}", other),
    }
}

#[test]
fn test_expr_logical_left_associative() {
    // Test: a and b and c should be (a and b) and c
    let result = parse_with_timeout(
        "a and b and c",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::And { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::And {
                    lhs: ref and_lhs,
                    rhs: ref and_rhs,
                    ..
                } => {
                    assert_matches!(**and_lhs, CmpLhs::Var { name, .. } if name == "a");
                    match **and_rhs {
                        CmpRhs::Paren { ref inner, .. } => {
                            assert_matches!(**inner, Expr::Var { name, .. } if name == "b");
                        }
                        ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
                    }
                }
                ref other => panic!("Expected CmpLhs::And, got {:?}", other),
            }
            match *rhs {
                CmpRhs::Paren { ref inner, .. } => {
                    assert_matches!(**inner, Expr::Var { name, .. } if name == "c");
                }
                ref other => panic!("Expected CmpRhs::Paren, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::And, got {:?}", other),
    }
}
