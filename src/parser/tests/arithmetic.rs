use super::helpers::*;

#[test]
fn test_expr_simple_add() {
    let result = parse_with_timeout(
        "1 + 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            assert_matches!(*lhs, AddLhs::IntLit { value: 1, .. });
            assert_matches!(*rhs, AddRhs::IntLit { value: 2, .. });
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

#[test]
fn test_expr_simple_mul() {
    let result = parse_with_timeout(
        "3 * 4",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mul { lhs, rhs, .. } => {
            assert_matches!(*lhs, MulLhs::IntLit { value: 3, .. });
            assert_matches!(*rhs, MulRhs::IntLit { value: 4, .. });
        }
        other => panic!("Expected Expr::Mul, got {:?}", other),
    }
}

#[test]
fn test_expr_precedence() {
    // Test: 1 + 2 * 3
    let result = parse_with_timeout(
        "1 + 2 * 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
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
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

#[test]
fn test_expr_left_associative_add() {
    // Test: 1 + 2 + 3 should be (1 + 2) + 3
    let result = parse_with_timeout(
        "1 + 2 + 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            match *lhs {
                AddLhs::Add {
                    lhs: ref inner_lhs,
                    rhs: ref inner_rhs,
                    ..
                } => {
                    assert_matches!(**inner_lhs, AddLhs::IntLit { value: 1, .. });
                    assert_matches!(**inner_rhs, AddRhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected AddLhs::Add, got {:?}", other),
            }
            assert_matches!(*rhs, AddRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

#[test]
fn test_expr_parentheses() {
    // Test: (1 + 2) * 3
    let result = parse_with_timeout(
        "(1 + 2) * 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mul { lhs, rhs, .. } => {
            match *lhs {
                MulLhs::Paren { ref inner, .. } => match **inner {
                    Expr::Add {
                        ref lhs, ref rhs, ..
                    } => {
                        assert_matches!(**lhs, AddLhs::IntLit { value: 1, .. });
                        assert_matches!(**rhs, AddRhs::IntLit { value: 2, .. });
                    }
                    ref other => panic!("Expected Expr::Add, got {:?}", other),
                },
                ref other => panic!("Expected MulLhs::Paren, got {:?}", other),
            }
            assert_matches!(*rhs, MulRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Mul, got {:?}", other),
    }
}

// ========================================================================
// Power Operator Tests
// ========================================================================

#[test]
fn test_expr_simple_pow() {
    // Test: 2 ^ 3
    let result = parse_with_timeout(
        "2 ^ 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Pow { lhs, rhs, .. } => {
            assert_matches!(*lhs, PowLhs::IntLit { value: 2, .. });
            assert_matches!(*rhs, PowRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Pow, got {:?}", other),
    }
}

#[test]
fn test_expr_pow_right_associative() {
    // Test: 2 ^ 3 ^ 4 should be 2 ^ (3 ^ 4) (right-associative)
    let result = parse_with_timeout(
        "2 ^ 3 ^ 4",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Pow { lhs, rhs, .. } => {
            assert_matches!(*lhs, PowLhs::IntLit { value: 2, .. });
            match *rhs {
                PowRhs::Pow {
                    lhs: ref inner_lhs,
                    rhs: ref inner_rhs,
                    ..
                } => {
                    assert_matches!(**inner_lhs, PowLhs::IntLit { value: 3, .. });
                    assert_matches!(**inner_rhs, PowRhs::IntLit { value: 4, .. });
                }
                ref other => panic!("Expected PowRhs::Pow, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Pow, got {:?}", other),
    }
}

#[test]
fn test_expr_pow_with_mul() {
    // Test: 2 * 3 ^ 4 should be 2 * (3 ^ 4) - power has higher precedence
    let result = parse_with_timeout(
        "2 * 3 ^ 4",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mul { lhs, rhs, .. } => {
            assert_matches!(*lhs, MulLhs::IntLit { value: 2, .. });
            match *rhs {
                MulRhs::Pow {
                    lhs: ref pow_lhs,
                    rhs: ref pow_rhs,
                    ..
                } => {
                    assert_matches!(**pow_lhs, PowLhs::IntLit { value: 3, .. });
                    assert_matches!(**pow_rhs, PowRhs::IntLit { value: 4, .. });
                }
                ref other => panic!("Expected MulRhs::Pow, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Mul, got {:?}", other),
    }
}

#[test]
fn test_expr_pow_with_add() {
    // Test: 1 + 2 ^ 3 should be 1 + (2 ^ 3)
    let result = parse_with_timeout(
        "1 + 2 ^ 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            assert_matches!(*lhs, AddLhs::IntLit { value: 1, .. });
            match *rhs {
                AddRhs::Pow {
                    lhs: ref pow_lhs,
                    rhs: ref pow_rhs,
                    ..
                } => {
                    assert_matches!(**pow_lhs, PowLhs::IntLit { value: 2, .. });
                    assert_matches!(**pow_rhs, PowRhs::IntLit { value: 3, .. });
                }
                ref other => panic!("Expected AddRhs::Pow, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

#[test]
fn test_expr_pow_with_parens() {
    // Test: (2 ^ 3) ^ 4 - parentheses override right-associativity
    let result = parse_with_timeout(
        "(2 ^ 3) ^ 4",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Pow { lhs, rhs, .. } => {
            match *lhs {
                PowLhs::Paren { ref inner, .. } => match **inner {
                    Expr::Pow {
                        ref lhs, ref rhs, ..
                    } => {
                        assert_matches!(**lhs, PowLhs::IntLit { value: 2, .. });
                        assert_matches!(**rhs, PowRhs::IntLit { value: 3, .. });
                    }
                    ref other => panic!("Expected Expr::Pow, got {:?}", other),
                },
                ref other => panic!("Expected PowLhs::Paren, got {:?}", other),
            }
            assert_matches!(*rhs, PowRhs::IntLit { value: 4, .. });
        }
        other => panic!("Expected Expr::Pow, got {:?}", other),
    }
}

#[test]
fn test_expr_pow_with_vars() {
    // Test: x ^ y
    let result = parse_with_timeout(
        "x ^ y",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Pow { lhs, rhs, .. } => {
            assert_matches!(*lhs, PowLhs::Var { name, .. } if name == "x");
            assert_matches!(*rhs, PowRhs::Var { name, .. } if name == "y");
        }
        other => panic!("Expected Expr::Pow, got {:?}", other),
    }
}

#[test]
fn test_expr_complex_pow_precedence() {
    // Test: 2 + 3 * 4 ^ 5 should be 2 + (3 * (4 ^ 5))
    let result = parse_with_timeout(
        "2 + 3 * 4 ^ 5",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            assert_matches!(*lhs, AddLhs::IntLit { value: 2, .. });
            match *rhs {
                AddRhs::Mul {
                    lhs: ref mul_lhs,
                    rhs: ref mul_rhs,
                    ..
                } => {
                    assert_matches!(**mul_lhs, MulLhs::IntLit { value: 3, .. });
                    match **mul_rhs {
                        MulRhs::Pow {
                            ref lhs, ref rhs, ..
                        } => {
                            assert_matches!(**lhs, PowLhs::IntLit { value: 4, .. });
                            assert_matches!(**rhs, PowRhs::IntLit { value: 5, .. });
                        }
                        ref other => panic!("Expected MulRhs::Pow, got {:?}", other),
                    }
                }
                ref other => panic!("Expected AddRhs::Mul, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

#[test]
fn test_expr_pow_chain_right_assoc() {
    // Test: a ^ b ^ c ^ d should be a ^ (b ^ (c ^ d))
    let result = parse_with_timeout(
        "a ^ b ^ c ^ d",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Pow { lhs, rhs, .. } => {
            assert_matches!(*lhs, PowLhs::Var { name, .. } if name == "a");
            match *rhs {
                PowRhs::Pow {
                    lhs: ref b_lhs,
                    rhs: ref b_rhs,
                    ..
                } => {
                    assert_matches!(**b_lhs, PowLhs::Var { name, .. } if name == "b");
                    match **b_rhs {
                        PowRhs::Pow {
                            ref lhs, ref rhs, ..
                        } => {
                            assert_matches!(**lhs, PowLhs::Var { name, .. } if name == "c");
                            assert_matches!(**rhs, PowRhs::Var { name, .. } if name == "d");
                        }
                        ref other => panic!("Expected PowRhs::Pow, got {:?}", other),
                    }
                }
                ref other => panic!("Expected PowRhs::Pow, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Pow, got {:?}", other),
    }
}

// ========================================================================
// Modulo Operator Tests
// ========================================================================

#[test]
fn test_expr_simple_mod() {
    // Test: 10 % 3
    let result = parse_with_timeout(
        "10 % 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mod { lhs, rhs, .. } => {
            assert_matches!(*lhs, MulLhs::IntLit { value: 10, .. });
            assert_matches!(*rhs, MulRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Mod, got {:?}", other),
    }
}

#[test]
fn test_expr_mod_left_associative() {
    // Test: 10 % 3 % 2 should be (10 % 3) % 2
    let result = parse_with_timeout(
        "10 % 3 % 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mod { lhs, rhs, .. } => {
            match *lhs {
                MulLhs::Mod {
                    lhs: ref inner_lhs,
                    rhs: ref inner_rhs,
                    ..
                } => {
                    assert_matches!(**inner_lhs, MulLhs::IntLit { value: 10, .. });
                    assert_matches!(**inner_rhs, MulRhs::IntLit { value: 3, .. });
                }
                ref other => panic!("Expected MulLhs::Mod, got {:?}", other),
            }
            assert_matches!(*rhs, MulRhs::IntLit { value: 2, .. });
        }
        other => panic!("Expected Expr::Mod, got {:?}", other),
    }
}

#[test]
fn test_expr_mod_with_mul() {
    // Test: 10 * 3 % 2 should be (10 * 3) % 2 - same precedence, left-associative
    let result = parse_with_timeout(
        "10 * 3 % 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mod { lhs, rhs, .. } => {
            match *lhs {
                MulLhs::Mul {
                    lhs: ref inner_lhs,
                    rhs: ref inner_rhs,
                    ..
                } => {
                    assert_matches!(**inner_lhs, MulLhs::IntLit { value: 10, .. });
                    assert_matches!(**inner_rhs, MulRhs::IntLit { value: 3, .. });
                }
                ref other => panic!("Expected MulLhs::Mul, got {:?}", other),
            }
            assert_matches!(*rhs, MulRhs::IntLit { value: 2, .. });
        }
        other => panic!("Expected Expr::Mod, got {:?}", other),
    }
}

#[test]
fn test_expr_mod_with_div() {
    // Test: 10 / 3 % 2 should be (10 / 3) % 2 - same precedence, left-associative
    let result = parse_with_timeout(
        "10 / 3 % 2",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mod { lhs, rhs, .. } => {
            match *lhs {
                MulLhs::Div {
                    lhs: ref inner_lhs,
                    rhs: ref inner_rhs,
                    ..
                } => {
                    assert_matches!(**inner_lhs, MulLhs::IntLit { value: 10, .. });
                    assert_matches!(**inner_rhs, MulRhs::IntLit { value: 3, .. });
                }
                ref other => panic!("Expected MulLhs::Div, got {:?}", other),
            }
            assert_matches!(*rhs, MulRhs::IntLit { value: 2, .. });
        }
        other => panic!("Expected Expr::Mod, got {:?}", other),
    }
}

#[test]
fn test_expr_mod_with_add() {
    // Test: 1 + 10 % 3 should be 1 + (10 % 3) - modulo has higher precedence
    let result = parse_with_timeout(
        "1 + 10 % 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            assert_matches!(*lhs, AddLhs::IntLit { value: 1, .. });
            match *rhs {
                AddRhs::Mod {
                    lhs: ref mod_lhs,
                    rhs: ref mod_rhs,
                    ..
                } => {
                    assert_matches!(**mod_lhs, MulLhs::IntLit { value: 10, .. });
                    assert_matches!(**mod_rhs, MulRhs::IntLit { value: 3, .. });
                }
                ref other => panic!("Expected AddRhs::Mod, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

#[test]
fn test_expr_mod_with_pow() {
    // Test: 2 ^ 3 % 5 should be (2 ^ 3) % 5 - power has higher precedence
    let result = parse_with_timeout(
        "2 ^ 3 % 5",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mod { lhs, rhs, .. } => {
            match *lhs {
                MulLhs::Pow {
                    lhs: ref pow_lhs,
                    rhs: ref pow_rhs,
                    ..
                } => {
                    assert_matches!(**pow_lhs, PowLhs::IntLit { value: 2, .. });
                    assert_matches!(**pow_rhs, PowRhs::IntLit { value: 3, .. });
                }
                ref other => panic!("Expected MulLhs::Pow, got {:?}", other),
            }
            assert_matches!(*rhs, MulRhs::IntLit { value: 5, .. });
        }
        other => panic!("Expected Expr::Mod, got {:?}", other),
    }
}

#[test]
fn test_expr_mod_with_parens() {
    // Test: 10 % (3 + 2)
    let result = parse_with_timeout(
        "10 % (3 + 2)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mod { lhs, rhs, .. } => {
            assert_matches!(*lhs, MulLhs::IntLit { value: 10, .. });
            match *rhs {
                MulRhs::Paren { ref inner, .. } => match **inner {
                    Expr::Add {
                        ref lhs, ref rhs, ..
                    } => {
                        assert_matches!(**lhs, AddLhs::IntLit { value: 3, .. });
                        assert_matches!(**rhs, AddRhs::IntLit { value: 2, .. });
                    }
                    ref other => panic!("Expected Expr::Add, got {:?}", other),
                },
                ref other => panic!("Expected MulRhs::Paren, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Mod, got {:?}", other),
    }
}

#[test]
fn test_expr_mod_with_vars() {
    // Test: x % y
    let result = parse_with_timeout(
        "x % y",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mod { lhs, rhs, .. } => {
            assert_matches!(*lhs, MulLhs::Var { name, .. } if name == "x");
            assert_matches!(*rhs, MulRhs::Var { name, .. } if name == "y");
        }
        other => panic!("Expected Expr::Mod, got {:?}", other),
    }
}

#[test]
fn test_expr_complex_mod_precedence() {
    // Test: 2 + 3 * 4 % 5 should be 2 + ((3 * 4) % 5)
    let result = parse_with_timeout(
        "2 + 3 * 4 % 5",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            assert_matches!(*lhs, AddLhs::IntLit { value: 2, .. });
            match *rhs {
                AddRhs::Mod {
                    lhs: ref mod_lhs,
                    rhs: ref mod_rhs,
                    ..
                } => {
                    match **mod_lhs {
                        MulLhs::Mul {
                            ref lhs, ref rhs, ..
                        } => {
                            assert_matches!(**lhs, MulLhs::IntLit { value: 3, .. });
                            assert_matches!(**rhs, MulRhs::IntLit { value: 4, .. });
                        }
                        ref other => panic!("Expected MulLhs::Mul, got {:?}", other),
                    }
                    assert_matches!(**mod_rhs, MulRhs::IntLit { value: 5, .. });
                }
                ref other => panic!("Expected AddRhs::Mod, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

// ========================================================================
// Unary Operator Tests
// ========================================================================

#[test]
fn test_expr_simple_neg() {
    // Test: -5
    let result = parse_with_timeout(
        "-5",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Neg { inner, .. } => {
            assert_matches!(*inner, PowLhs::IntLit { value: 5, .. });
        }
        other => panic!("Expected Expr::Neg, got {:?}", other),
    }
}

#[test]
fn test_expr_simple_ref() {
    // Test: &x
    let result = parse_with_timeout(
        "&x",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Ref { inner, .. } => {
            assert_matches!(*inner, PowLhs::Var { name, .. } if name == "x");
        }
        other => panic!("Expected Expr::Ref, got {:?}", other),
    }
}

#[test]
fn test_expr_double_neg() {
    // Test: --5
    let result = parse_with_timeout(
        "--5",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Neg { inner, .. } => match *inner {
            PowLhs::Neg { ref inner, .. } => {
                assert_matches!(**inner, PowLhs::IntLit { value: 5, .. });
            }
            ref other => panic!("Expected PowLhs::Neg, got {:?}", other),
        },
        other => panic!("Expected Expr::Neg, got {:?}", other),
    }
}

#[test]
fn test_expr_neg_ref() {
    // Test: -&x
    let result = parse_with_timeout(
        "-&x",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Neg { inner, .. } => match *inner {
            PowLhs::Ref { ref inner, .. } => {
                assert_matches!(**inner, PowLhs::Var { name, .. } if name == "x");
            }
            ref other => panic!("Expected PowLhs::Ref, got {:?}", other),
        },
        other => panic!("Expected Expr::Neg, got {:?}", other),
    }
}

#[test]
fn test_expr_ref_neg() {
    // Test: &-x
    let result = parse_with_timeout(
        "&-x",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Ref { inner, .. } => match *inner {
            PowLhs::Neg { ref inner, .. } => {
                assert_matches!(**inner, PowLhs::Var { name, .. } if name == "x");
            }
            ref other => panic!("Expected PowLhs::Neg, got {:?}", other),
        },
        other => panic!("Expected Expr::Ref, got {:?}", other),
    }
}

#[test]
fn test_expr_neg_with_pow() {
    // Test: -2 ^ 3 should be (-2) ^ 3 - unary has higher precedence than power
    let result = parse_with_timeout(
        "-2 ^ 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Pow { lhs, rhs, .. } => {
            match *lhs {
                PowLhs::Neg { ref inner, .. } => {
                    assert_matches!(**inner, PowLhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected PowLhs::Neg, got {:?}", other),
            }
            assert_matches!(*rhs, PowRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Pow, got {:?}", other),
    }
}

#[test]
fn test_expr_neg_with_mul() {
    // Test: -2 * 3 should be (-2) * 3
    let result = parse_with_timeout(
        "-2 * 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Mul { lhs, rhs, .. } => {
            match *lhs {
                MulLhs::Neg { ref inner, .. } => {
                    assert_matches!(**inner, PowLhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected MulLhs::Neg, got {:?}", other),
            }
            assert_matches!(*rhs, MulRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Mul, got {:?}", other),
    }
}

#[test]
fn test_expr_neg_with_add() {
    // Test: -2 + 3 should be (-2) + 3
    let result = parse_with_timeout(
        "-2 + 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            match *lhs {
                AddLhs::Neg { ref inner, .. } => {
                    assert_matches!(**inner, PowLhs::IntLit { value: 2, .. });
                }
                ref other => panic!("Expected AddLhs::Neg, got {:?}", other),
            }
            assert_matches!(*rhs, AddRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

#[test]
fn test_expr_neg_paren() {
    // Test: -(2 + 3)
    let result = parse_with_timeout(
        "-(2 + 3)",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Neg { inner, .. } => match *inner {
            PowLhs::Paren { ref inner, .. } => match **inner {
                Expr::Add {
                    ref lhs, ref rhs, ..
                } => {
                    assert_matches!(**lhs, AddLhs::IntLit { value: 2, .. });
                    assert_matches!(**rhs, AddRhs::IntLit { value: 3, .. });
                }
                ref other => panic!("Expected Expr::Add, got {:?}", other),
            },
            ref other => panic!("Expected PowLhs::Paren, got {:?}", other),
        },
        other => panic!("Expected Expr::Neg, got {:?}", other),
    }
}

#[test]
fn test_expr_ref_with_add() {
    // Test: &x + 3
    let result = parse_with_timeout(
        "&x + 3",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Add { lhs, rhs, .. } => {
            match *lhs {
                AddLhs::Ref { ref inner, .. } => {
                    assert_matches!(**inner, PowLhs::Var { name, .. } if name == "x");
                }
                ref other => panic!("Expected AddLhs::Ref, got {:?}", other),
            }
            assert_matches!(*rhs, AddRhs::IntLit { value: 3, .. });
        }
        other => panic!("Expected Expr::Add, got {:?}", other),
    }
}

#[test]
fn test_expr_complex_unary() {
    // Test: -a ^ -b should be (-a) ^ (-b)
    let result = parse_with_timeout(
        "-a ^ -b",
        |input| expr().parse(input).into_result(),
        Duration::from_secs(2),
    );

    match result.unwrap() {
        Expr::Pow { lhs, rhs, .. } => {
            match *lhs {
                PowLhs::Neg { ref inner, .. } => {
                    assert_matches!(**inner, PowLhs::Var { name, .. } if name == "a");
                }
                ref other => panic!("Expected PowLhs::Neg, got {:?}", other),
            }
            match *rhs {
                PowRhs::Neg { ref inner, .. } => {
                    assert_matches!(**inner, PowLhs::Var { name, .. } if name == "b");
                }
                ref other => panic!("Expected PowRhs::Neg, got {:?}", other),
            }
        }
        other => panic!("Expected Expr::Pow, got {:?}", other),
    }
}
