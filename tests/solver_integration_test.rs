//! Comprehensive integration tests for the Z3 solver
//!
//! This test suite covers the complete solver pipeline from CAD source code
//! to constraint solving with Z3. Tests are organized by scenario type.

use std::process::Command;

/// Helper function to run the solve command on a test fixture
fn solve_fixture(fixture_name: &str) -> (bool, String, String) {
    let path = format!("tests/fixtures/solve/{}", fixture_name);
    let output = Command::new("cargo")
        .args(["run", "--", "solve", &path])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stdout, stderr)
}

/// Helper to verify a variable has a specific value in the solution
fn verify_solution(solution: &str, var_name: &str, expected_value: &str) {
    let expected = format!("{} = {}", var_name, expected_value);
    assert!(
        solution.contains(&expected),
        "Expected '{}' in solution, got:\n{}",
        expected,
        solution
    );
}

/// Helper to verify a solution satisfies a constraint (for under-constrained systems)
fn extract_value(solution: &str, var_name: &str) -> i32 {
    for line in solution.lines() {
        if line.starts_with(&format!("{} = ", var_name)) {
            let value_str = line.split(" = ").nth(1).unwrap().trim();
            return value_str.parse().unwrap();
        }
    }
    panic!("Variable {} not found in solution", var_name);
}

// ============================================================================
// Basic Equations
// ============================================================================

#[test]
fn test_basic_linear() {
    let (success, stdout, stderr) = solve_fixture("basic_linear.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
}

#[test]
fn test_system_of_equations() {
    let (success, stdout, stderr) = solve_fixture("system_of_equations.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "20");
    verify_solution(&stdout, "y", "10");
}

#[test]
fn test_single_var_multiple_constraints() {
    let (success, stdout, stderr) = solve_fixture("single_var_multiple_constraints.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "15");
}

// ============================================================================
// Different Types
// ============================================================================

#[test]
fn test_integer_constraint() {
    let (success, stdout, stderr) = solve_fixture("integer_constraint.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "42");
}

#[test]
fn test_float_constraint() {
    let (success, stdout, stderr) = solve_fixture("float_constraint.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // Float might have slight variations, so we check for 3.14
    assert!(stdout.contains("x = 3.14"));
}

#[test]
fn test_bool_constraint() {
    let (success, stdout, stderr) = solve_fixture("bool_constraint.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "true");
}

#[test]
fn test_mixed_types() {
    let (success, stdout, stderr) = solve_fixture("mixed_types.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
    assert!(stdout.contains("y = 3.14"));
    verify_solution(&stdout, "z", "true");
}

// ============================================================================
// Arithmetic Operations
// ============================================================================

#[test]
fn test_addition() {
    let (success, stdout, stderr) = solve_fixture("addition.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "3");
    verify_solution(&stdout, "y", "7");
}

#[test]
fn test_subtraction() {
    let (success, stdout, stderr) = solve_fixture("subtraction.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
    verify_solution(&stdout, "y", "5");
}

#[test]
fn test_multiplication() {
    let (success, stdout, stderr) = solve_fixture("multiplication.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "5");
}

#[test]
fn test_division() {
    let (success, stdout, stderr) = solve_fixture("division.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // Float division should give us 10.0
    assert!(stdout.contains("x = 10"));
}

#[test]
fn test_negation() {
    let (success, stdout, stderr) = solve_fixture("negation.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
}

// ============================================================================
// Comparison Operators
// ============================================================================

#[test]
fn test_equality() {
    let (success, stdout, stderr) = solve_fixture("equality.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
}

#[test]
fn test_inequality() {
    let (success, stdout, stderr) = solve_fixture("inequality.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
}

#[test]
fn test_less_than() {
    let (success, stdout, stderr) = solve_fixture("less_than.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "5");
}

#[test]
fn test_greater_than() {
    let (success, stdout, stderr) = solve_fixture("greater_than.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
}

#[test]
fn test_less_equal() {
    let (success, stdout, stderr) = solve_fixture("less_equal.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
}

#[test]
fn test_greater_equal() {
    let (success, stdout, stderr) = solve_fixture("greater_equal.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "5");
}

// ============================================================================
// Constraint Systems
// ============================================================================

#[test]
fn test_over_constrained() {
    let (success, stdout, stderr) = solve_fixture("over_constrained.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
}

#[test]
fn test_under_constrained() {
    let (success, stdout, stderr) = solve_fixture("under_constrained.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Z3 will pick one solution - we just verify it satisfies the constraint
    let x = extract_value(&stdout, "x");
    let y = extract_value(&stdout, "y");
    assert_eq!(x + y, 20, "Solution should satisfy x + y = 20");
}

#[test]
fn test_exactly_constrained() {
    let (success, stdout, stderr) = solve_fixture("exactly_constrained.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
    verify_solution(&stdout, "y", "20");
}

// ============================================================================
// UNSAT Cases
// ============================================================================

#[test]
fn test_unsat_contradiction() {
    let (success, stdout, stderr) = solve_fixture("unsat_contradiction.cad");
    assert!(
        !success,
        "Expected solver to fail for contradictory constraints"
    );

    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("UNSAT")
            || combined.contains("Unsat")
            || combined.contains("cannot be satisfied")
            || combined.contains("Solver error"),
        "Expected UNSAT error, got: {}",
        combined
    );
}

#[test]
fn test_unsat_impossible_inequalities() {
    let (success, stdout, stderr) = solve_fixture("unsat_impossible_inequalities.cad");
    assert!(
        !success,
        "Expected solver to fail for impossible inequalities"
    );

    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("UNSAT")
            || combined.contains("Unsat")
            || combined.contains("cannot be satisfied")
            || combined.contains("Solver error"),
        "Expected UNSAT error, got: {}",
        combined
    );
}

#[test]
fn test_bool_contradiction() {
    let (success, stdout, stderr) = solve_fixture("bool_contradiction.cad");
    assert!(
        !success,
        "Expected solver to fail for boolean contradiction"
    );

    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("UNSAT")
            || combined.contains("Unsat")
            || combined.contains("cannot be satisfied")
            || combined.contains("Solver error"),
        "Expected UNSAT error, got: {}",
        combined
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_zero_value() {
    let (success, stdout, stderr) = solve_fixture("zero_value.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "0");
}

#[test]
fn test_negative_value() {
    let (success, stdout, stderr) = solve_fixture("negative_value.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "-10");
}

#[test]
fn test_large_number() {
    let (success, stdout, stderr) = solve_fixture("large_number.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "1000000");
}

#[test]
fn test_small_float() {
    let (success, stdout, stderr) = solve_fixture("small_float.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // Check for 0.001 or scientific notation
    assert!(
        stdout.contains("x = 0.001") || stdout.contains("x = 1/1000"),
        "Expected x = 0.001, got: {}",
        stdout
    );
}

// ============================================================================
// Additional Edge Cases and Error Conditions
// ============================================================================

#[test]
fn test_nonexistent_file() {
    let output = Command::new("cargo")
        .args(["run", "--", "solve", "tests/fixtures/solve/nonexistent.cad"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Failed to read file") || stderr.contains("No such file"),
        "Expected file not found error, got: {}",
        stderr
    );
}

// ============================================================================
// Struct Support Tests
// ============================================================================

#[test]
fn test_struct_simple_field_constraint() {
    let (success, stdout, stderr) = solve_fixture("struct_simple_field_constraint.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "p.x", "10");
    verify_solution(&stdout, "p.y", "5");
}

#[test]
fn test_struct_nested_field_constraint() {
    let (success, stdout, stderr) = solve_fixture("struct_nested_field_constraint.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "line.start.x", "5");
    verify_solution(&stdout, "line.end.x", "15");
}

#[test]
fn test_struct_literal_init() {
    let (success, stdout, stderr) = solve_fixture("struct_literal_init.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "p.x", "5");
    verify_solution(&stdout, "p.y", "10");
}

#[test]
fn test_struct_field_assignment() {
    let (success, stdout, stderr) = solve_fixture("struct_field_assignment.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "p.x", "5");
    verify_solution(&stdout, "p.y", "10");
}

#[test]
fn test_struct_unsat() {
    let (success, stdout, stderr) = solve_fixture("struct_unsat.cad");
    assert!(
        !success,
        "Expected solver to fail for contradictory struct field constraints"
    );

    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("UNSAT")
            || combined.contains("Unsat")
            || combined.contains("cannot be satisfied")
            || combined.contains("Solver error"),
        "Expected UNSAT error, got: {}",
        combined
    );
}

// ============================================================================
// Function Call Tests
// ============================================================================
//
// NOTE: These tests are currently ignored because function call support
// is not yet fully implemented in the constraint solver. The function inliner
// exists, but the constraint extractor needs to be updated to handle
// function definitions. See CLAUDE.md "Next Implementation Steps" for details.
//
// To enable these tests, remove the #[ignore] attribute once function call
// support is complete in the solver pipeline.

#[test]
fn test_function_call_simple() {
    let (success, stdout, stderr) = solve_fixture("function_call_simple.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "result", "10");
    verify_solution(&stdout, "x", "7");
}

#[test]
fn test_function_call_nested() {
    let (success, stdout, stderr) = solve_fixture("function_call_nested.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "z", "15");
    verify_solution(&stdout, "y", "5");
}

#[test]
fn test_function_call_in_constraint() {
    let (success, stdout, stderr) = solve_fixture("function_call_in_constraint.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // square(a) == 16 means a = 4 or a = -4
    // Z3 will pick one solution - we just verify it satisfies the constraint
    let a = extract_value(&stdout, "a");
    assert_eq!(a * a, 16, "Solution should satisfy square(a) = 16");
}

// ============================================================================
// Method Call Tests
// ============================================================================
//
// NOTE: These tests are currently ignored because method call support
// is not yet fully implemented in the constraint solver. The semantic
// analyzer now resolves method calls, and the function inliner supports
// them, but the constraint extractor needs to handle method definitions.
//
// To enable these tests, remove the #[ignore] attribute once method call
// support is complete in the solver pipeline.

#[test]
fn test_method_call_simple() {
    let (success, stdout, stderr) = solve_fixture("method_call_simple.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "c.radius", "5");
    verify_solution(&stdout, "a", "75");
}

#[test]
fn test_method_call_with_args() {
    let (success, stdout, stderr) = solve_fixture("method_call_with_args.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "r.width", "4");
    verify_solution(&stdout, "r.height", "3");
    verify_solution(&stdout, "a", "24");
}

#[test]
fn test_method_call_in_constraint() {
    let (success, stdout, stderr) = solve_fixture("method_call_in_constraint.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // s.area() == 16 means s.side = 4 or s.side = -4
    // Z3 will pick one solution - we just verify it satisfies the constraint
    let side = extract_value(&stdout, "s.side");
    assert_eq!(side * side, 16, "Solution should satisfy s.area() = 16");
}
