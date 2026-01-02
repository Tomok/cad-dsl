use super::helpers::*;

#[test]
fn test_expr_simple_eq() {
    // Test: 1 == 2
    let result = parse_with_timeout(
        "1 == 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Eq { lhs, rhs, .. } => {
            assert_matches!(*lhs, CmpLhs::IntLit { value: 1, .. });
            assert_matches!(*rhs, CmpRhs::IntLit { value: 2, .. });
        }
        other => panic!("Expected Expr::Eq, got {:?}", other),
    }
}

#[test]
fn test_expr_eq_with_addition() {
    // Test: 1 + 2 == 3 + 4
    let result = parse_with_timeout(
        "1 + 2 == 3 + 4",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Eq { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::Add {
                    lhs: ref add_lhs,
                    rhs: ref add_rhs,
                    ..
                } => {
                    assert_matches!(**add_lhs, AddLhs::IntLit { value: 1, .. });
                    assert_matches!(**add_rhs, AddRhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected CmpLhs::Add, got {:?}", other),
            }
            match *rhs {
                CmpRhs::Add {
                    lhs: ref add_lhs,
                    rhs: ref add_rhs,
                    ..
                } => {
                    assert_matches!(**add_lhs, AddLhs::IntLit { value: 3, .. });
                    assert_matches!(**add_rhs, AddRhs::IntLit { value: 4, .. });
                }
                ref other => panic!("Expected CmpRhs::Add, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Eq, got {:?}", other),
    }
}

#[test]
fn test_expr_eq_left_associative() {
    // Test: 1 == 2 == 3 should be (1 == 2) == 3
    let result = parse_with_timeout(
        "1 == 2 == 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Eq { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::Eq {
                    lhs: ref inner_lhs,
                    rhs: ref inner_rhs,
                    ..
                } => {
                    assert_matches!(**inner_lhs, CmpLhs::IntLit { value: 1, .. });
                    assert_matches!(**inner_rhs, CmpRhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected CmpLhs::Eq, got {:?}", other),
            }
            assert_matches!(*rhs, CmpRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Eq, got {:?}", other),
    }
}

#[test]
fn test_expr_eq_with_bool() {
    // Test: true == false
    let result = parse_with_timeout(
        "true == false",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Eq { lhs, rhs, .. } => {
            assert_matches!(*lhs, CmpLhs::BoolLit { value: true, .. });
            assert_matches!(*rhs, CmpRhs::BoolLit { value: false, .. });
        }
        other => panic!("Expected Expr::Eq, got {:?}", other),
    }
}

#[test]
fn test_expr_simple_neq() {
    // Test: 1 != 2
    let result = parse_with_timeout(
        "1 != 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::NotEq { lhs, rhs, .. } => {
            assert_matches!(*lhs, CmpLhs::IntLit { value: 1, .. });
            assert_matches!(*rhs, CmpRhs::IntLit { value: 2, .. });
        }
        other => panic!("Expected Expr::NotEq, got {:?}", other),
    }
}

#[test]
fn test_expr_neq_with_addition() {
    // Test: 1 + 2 != 3 + 4
    let result = parse_with_timeout(
        "1 + 2 != 3 + 4",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::NotEq { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::Add {
                    lhs: ref add_lhs,
                    rhs: ref add_rhs,
                    ..
                } => {
                    assert_matches!(**add_lhs, AddLhs::IntLit { value: 1, .. });
                    assert_matches!(**add_rhs, AddRhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected CmpLhs::Add, got {:?}", other),
            }
            match *rhs {
                CmpRhs::Add {
                    lhs: ref add_lhs,
                    rhs: ref add_rhs,
                    ..
                } => {
                    assert_matches!(**add_lhs, AddLhs::IntLit { value: 3, .. });
                    assert_matches!(**add_rhs, AddRhs::IntLit { value: 4, .. });
                }
                ref other => panic!("Expected CmpRhs::Add, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::NotEq, got {:?}", other),
    }
}

#[test]
fn test_expr_neq_left_associative() {
    // Test: 1 != 2 != 3 should be (1 != 2) != 3
    let result = parse_with_timeout(
        "1 != 2 != 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::NotEq { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::NotEq {
                    lhs: ref inner_lhs,
                    rhs: ref inner_rhs,
                    ..
                } => {
                    assert_matches!(**inner_lhs, CmpLhs::IntLit { value: 1, .. });
                    assert_matches!(**inner_rhs, CmpRhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected CmpLhs::NotEq, got {:?}", other),
            }
            assert_matches!(*rhs, CmpRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::NotEq, got {:?}", other),
    }
}

#[test]
fn test_expr_neq_with_bool() {
    // Test: true != false
    let result = parse_with_timeout(
        "true != false",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::NotEq { lhs, rhs, .. } => {
            assert_matches!(*lhs, CmpLhs::BoolLit { value: true, .. });
            assert_matches!(*rhs, CmpRhs::BoolLit { value: false, .. });
        }
        other => panic!("Expected Expr::NotEq, got {:?}", other),
    }
}

#[test]
fn test_expr_mixed_eq_neq() {
    // Test: 1 == 2 != 3 should be (1 == 2) != 3
    let result = parse_with_timeout(
        "1 == 2 != 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::NotEq { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::Eq {
                    lhs: ref inner_lhs,
                    rhs: ref inner_rhs,
                    ..
                } => {
                    assert_matches!(**inner_lhs, CmpLhs::IntLit { value: 1, .. });
                    assert_matches!(**inner_rhs, CmpRhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected CmpLhs::Eq, got {:?}", other),
            }
            assert_matches!(*rhs, CmpRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::NotEq, got {:?}", other),
    }
}

#[test]
fn test_expr_simple_lt() {
    // Test: 1 < 2
    let result = parse_with_timeout(
        "1 < 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Lt { lhs, rhs, .. } => {
            assert_matches!(*lhs, CmpLhs::IntLit { value: 1, .. });
            assert_matches!(*rhs, CmpRhs::IntLit { value: 2, .. });
        }
        other => panic!("Expected Expr::Lt, got {:?}", other),
    }
}

#[test]
fn test_expr_simple_gt() {
    // Test: 2 > 1
    let result = parse_with_timeout(
        "2 > 1",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Gt { lhs, rhs, .. } => {
            assert_matches!(*lhs, CmpLhs::IntLit { value: 2, .. });
            assert_matches!(*rhs, CmpRhs::IntLit { value: 1, .. });
        }
        other => panic!("Expected Expr::Gt, got {:?}", other),
    }
}

#[test]
fn test_expr_simple_lteq() {
    // Test: 1 <= 2
    let result = parse_with_timeout(
        "1 <= 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::LtEq { lhs, rhs, .. } => {
            assert_matches!(*lhs, CmpLhs::IntLit { value: 1, .. });
            assert_matches!(*rhs, CmpRhs::IntLit { value: 2, .. });
        }
        other => panic!("Expected Expr::LtEq, got {:?}", other),
    }
}

#[test]
fn test_expr_simple_gteq() {
    // Test: 2 >= 1
    let result = parse_with_timeout(
        "2 >= 1",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::GtEq { lhs, rhs, .. } => {
            assert_matches!(*lhs, CmpLhs::IntLit { value: 2, .. });
            assert_matches!(*rhs, CmpRhs::IntLit { value: 1, .. });
        }
        other => panic!("Expected Expr::GtEq, got {:?}", other),
    }
}

#[test]
fn test_expr_lt_with_addition() {
    // Test: 1 + 2 < 3 + 4
    let result = parse_with_timeout(
        "1 + 2 < 3 + 4",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Lt { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::Add {
                    lhs: ref add_lhs,
                    rhs: ref add_rhs,
                    ..
                } => {
                    assert_matches!(**add_lhs, AddLhs::IntLit { value: 1, .. });
                    assert_matches!(**add_rhs, AddRhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected CmpLhs::Add, got {:?}", other),
            }
            match *rhs {
                CmpRhs::Add {
                    lhs: ref add_lhs,
                    rhs: ref add_rhs,
                    ..
                } => {
                    assert_matches!(**add_lhs, AddLhs::IntLit { value: 3, .. });
                    assert_matches!(**add_rhs, AddRhs::IntLit { value: 4, .. });
                }
                ref other => panic!("Expected CmpRhs::Add, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Lt, got {:?}", other),
    }
}

#[test]
fn test_expr_comparison_left_associative() {
    // Test: 1 < 2 < 3 should be (1 < 2) < 3
    let result = parse_with_timeout(
        "1 < 2 < 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Lt { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::Lt {
                    lhs: ref inner_lhs,
                    rhs: ref inner_rhs,
                    ..
                } => {
                    assert_matches!(**inner_lhs, CmpLhs::IntLit { value: 1, .. });
                    assert_matches!(**inner_rhs, CmpRhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected CmpLhs::Lt, got {:?}", other),
            }
            assert_matches!(*rhs, CmpRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Lt, got {:?}", other),
    }
}

#[test]
fn test_expr_mixed_comparisons() {
    // Test: 1 < 2 == 3 > 4 should be ((1 < 2) == 3) > 4
    let result = parse_with_timeout(
        "1 < 2 == 3 > 4",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Gt { lhs, rhs, .. } => {
            match *lhs {
                CmpLhs::Eq {
                    lhs: ref eq_lhs,
                    rhs: ref eq_rhs,
                    ..
                } => {
                    match **eq_lhs {
                        CmpLhs::Lt {
                            lhs: ref lt_lhs,
                            rhs: ref lt_rhs,
                            ..
                        } => {
                            assert_matches!(**lt_lhs, CmpLhs::IntLit { value: 1, .. });
                            assert_matches!(**lt_rhs, CmpRhs::IntLit { value: 2, .. });
                        }
                        ref other => panic!("Expected CmpLhs::Lt, got {:?}", other),
                    }
                    assert_matches!(**eq_rhs, CmpRhs::IntLit { value: 3, .. });
                }
                ref other => panic!("Expected CmpLhs::Eq, got {:?}", other),
            }
            assert_matches!(*rhs, CmpRhs::IntLit { value: 4, .. });
        }
        other => panic!("Expected Expr::Gt, got {:?}", other),
    }
}
