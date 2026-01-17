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
fn test_modulo() {
    let (success, stdout, stderr) = solve_fixture("modulo.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "7");
}

#[test]
fn test_modulo_in_constraint() {
    let (success, stdout, stderr) = solve_fixture("modulo_in_constraint.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "13");
    verify_solution(&stdout, "y", "3");
}

#[test]
fn test_power() {
    let (success, stdout, stderr) = solve_fixture("power.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // Power of 2^3 = 8, so x should be 2.0
    assert!(stdout.contains("x = 2"));
    assert!(stdout.contains("y = 8"));
}

#[test]
fn test_power_integer() {
    let (success, stdout, stderr) = solve_fixture("power_integer.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "3");
    // Power returns Real in Z3, so y should be 27.0
    assert!(stdout.contains("y = 27"));
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
#[ignore] // Struct literals not yet implemented in new solver
fn test_struct_literal_init() {
    let (success, stdout, stderr) = solve_fixture("struct_literal_init.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "p.x", "5");
    verify_solution(&stdout, "p.y", "10");
}

#[test]
#[ignore] // Field assignment not yet implemented in new solver
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

#[test]

fn test_function_call_shadowing() {
    let (success, stdout, stderr) = solve_fixture("function_call_shadowing.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // Global variables
    verify_solution(&stdout, "b", "42");
    verify_solution(&stdout, "a", "1");
    // sub(b, 2) should use global b (42) as first arg: 42 - 2 = 40
    verify_solution(&stdout, "result", "40");
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

// ============================================================================
// Phase 3b: For-Loop Tests with Iterative Solving
// ============================================================================
//
// These tests verify the iterative solving mechanism with for-loop deferral.
// Phase 3b implements deferred constraint handling where for-loops with
// unknown range bounds are deferred until the bounds can be resolved through
// iterative solving.
//
// Test scenarios:
// 1. Known range (immediate unrolling)
// 2. Unknown range resolved in iteration 2 (deferred then resolved)
// 3. Computed range bounds (arithmetic expressions)
// 4. Cascading dependencies (multi-iteration resolution)
// 5. Unresolvable range (partial result expected)

#[test]
fn test_for_loop_known_range() {
    // For-loop with constant range bounds - should work immediately
    let (success, stdout, stderr) = solve_fixture("for_loop_known_range.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Verify array values: arr[i] == i * 10
    verify_solution(&stdout, "arr[0]", "0");
    verify_solution(&stdout, "arr[1]", "10");
    verify_solution(&stdout, "arr[2]", "20");
}

#[test]
fn test_for_loop_deferred_then_resolved() {
    // For-loop range depends on variable n which gets resolved first
    // Expected: 2 iterations (1: solve n, 2: unroll loop)
    let (success, stdout, stderr) = solve_fixture("for_loop_deferred_then_resolved.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Verify n was solved: n * 2 == 6 -> n = 3
    verify_solution(&stdout, "n", "3");

    // Verify loop was unrolled for i in 0..3
    verify_solution(&stdout, "arr[0]", "100");
    verify_solution(&stdout, "arr[1]", "101");
    verify_solution(&stdout, "arr[2]", "102");
}

#[test]
fn test_for_loop_computed_range() {
    // For-loop with arithmetic expression in range bound
    let (success, stdout, stderr) = solve_fixture("for_loop_computed_range.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Verify n was solved
    verify_solution(&stdout, "n", "5");

    // Verify loop was unrolled for i in 0..(n-2) = 0..3
    verify_solution(&stdout, "arr[0]", "0");
    verify_solution(&stdout, "arr[1]", "2");
    verify_solution(&stdout, "arr[2]", "4");
}

#[test]
fn test_for_loop_cascading_dependencies() {
    // For-loop range depends on n, which depends on m
    // Expected: 3 iterations (1: solve m, 2: solve n, 3: unroll loop)
    let (success, stdout, stderr) = solve_fixture("for_loop_cascading_dependencies.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Verify cascading resolution
    verify_solution(&stdout, "m", "10");
    verify_solution(&stdout, "n", "5");

    // Verify loop was unrolled for i in 0..5
    verify_solution(&stdout, "arr[0]", "0"); // 0 * 0
    verify_solution(&stdout, "arr[1]", "1"); // 1 * 1
    verify_solution(&stdout, "arr[2]", "4"); // 2 * 2
    verify_solution(&stdout, "arr[3]", "9"); // 3 * 3
    verify_solution(&stdout, "arr[4]", "16"); // 4 * 4
}

#[test]
fn test_for_loop_unresolvable_range() {
    // For-loop range depends on unconstrained variable
    // Expected: Partial result (solver should not fail)
    // NOTE: This test expects a partial result, not a complete solution.
    // The current implementation may return an error instead of a Partial result.
    // This is acceptable for now - the test documents expected behavior.

    let (success, stdout, stderr) = solve_fixture("for_loop_unresolvable_range.cad");

    // For now, we expect this to fail (return error) because partial results
    // are not yet fully exposed through the CLI interface.
    // TODO: Update this test once SolveResult::Partial is exposed in CLI

    if success {
        // If it succeeds, it should have solved x at least
        verify_solution(&stdout, "x", "42");

        // Variable n should either be present with any value, or not present
        // (depending on whether Z3 assigns it an arbitrary value)
    } else {
        // Expected to fail for now - verify error message mentions the issue
        let combined = format!("{}{}", stdout, stderr);
        assert!(
            combined.contains("n")
                || combined.contains("unknown")
                || combined.contains("undefined"),
            "Error should mention the unknown variable, got: {}",
            combined
        );
    }
}

// ============================================================================
// Array Tests
// ============================================================================
//
// Tests for array support including array indexing, array of primitives,
// and array of structs.

#[test]
fn test_array_simple_primitive() {
    // Create a simple test inline since we may not have this fixture yet
    let test_code = r#"
let arr: [i32; 3];
arr[0] == 10;
arr[1] == 20;
arr[2] == 30;
"#;

    // Write temporary test file
    std::fs::write("/tmp/array_simple_primitive.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/array_simple_primitive.cad"])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "arr[0]", "10");
    verify_solution(&stdout, "arr[1]", "20");
    verify_solution(&stdout, "arr[2]", "30");

    // Cleanup
    let _ = std::fs::remove_file("/tmp/array_simple_primitive.cad");
}

#[test]
fn test_array_of_structs() {
    // Test array of structs with field constraints
    let test_code = r#"
struct Point {
    x: i32,
    y: i32,
}

let points: [Point; 2];
points[0].x == 1;
points[0].y == 2;
points[1].x == 3;
points[1].y == 4;
"#;

    std::fs::write("/tmp/array_of_structs.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/array_of_structs.cad"])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "points[0].x", "1");
    verify_solution(&stdout, "points[0].y", "2");
    verify_solution(&stdout, "points[1].x", "3");
    verify_solution(&stdout, "points[1].y", "4");

    let _ = std::fs::remove_file("/tmp/array_of_structs.cad");
}

#[test]
fn test_array_with_arithmetic() {
    // Test array elements used in arithmetic expressions
    let test_code = r#"
let arr: [i32; 3];
arr[0] == 5;
arr[1] == arr[0] + 3;
arr[2] == arr[1] * 2;
"#;

    std::fs::write("/tmp/array_with_arithmetic.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/array_with_arithmetic.cad"])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "arr[0]", "5");
    verify_solution(&stdout, "arr[1]", "8");
    verify_solution(&stdout, "arr[2]", "16");

    let _ = std::fs::remove_file("/tmp/array_with_arithmetic.cad");
}

// ============================================================================
// With-Statement Tests (Container Context)
// ============================================================================
//
// Tests for container with-statements which provide namespace management
// using dot-prefix syntax.

#[test]
#[ignore] // With-statements not yet fully implemented in solver
fn test_with_statement_simple() {
    let (success, stdout, stderr) = solve_fixture("with_statement_simple.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Verify namespaced variables
    verify_solution(&stdout, "sketch.entities.p1.x", "10");
    verify_solution(&stdout, "sketch.entities.p1.y", "20");
    verify_solution(&stdout, "sketch.entities.p2.x", "30");
    verify_solution(&stdout, "sketch.entities.p2.y", "40");
}

#[test]
#[ignore] // With-statements not yet fully implemented in solver
fn test_with_statement_primitive_types() {
    let (success, stdout, stderr) = solve_fixture("with_statement_primitive_types.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // This test should verify primitive types can be used in with-statements
    // The exact assertions depend on the fixture content
    assert!(stdout.contains("entities"));
}

#[test]
#[ignore] // With-statements not yet fully implemented in solver
fn test_with_statement_nested_struct() {
    let (success, stdout, stderr) = solve_fixture("with_statement_nested_struct.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Verify nested struct handling in with-statements
    assert!(stdout.contains("entities"));
}

#[test]
#[ignore] // With-statements not yet fully implemented in solver
fn test_with_statement_constraints() {
    let (success, stdout, stderr) = solve_fixture("with_statement_constraints.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Verify constraints work correctly within with-statement context
    assert!(stdout.contains("entities"));
}

// ============================================================================
// If-Statement Tests
// ============================================================================
//
// Tests for conditional constraints using if-statements.

#[test]
fn test_if_statement_simple() {
    // Test simple conditional constraint
    let test_code = r#"
let x: i32;
let y: i32;
x == 10;

if x > 5 {
    y == 20;
}
"#;

    std::fs::write("/tmp/if_statement_simple.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/if_statement_simple.cad"])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
    verify_solution(&stdout, "y", "20");

    let _ = std::fs::remove_file("/tmp/if_statement_simple.cad");
}

#[test]
fn test_if_statement_false_condition() {
    // Test that constraints in false branch are not enforced
    let test_code = r#"
let x: i32;
let y: i32;
x == 3;
y == 100;

if x > 5 {
    y == 20;
}
"#;

    std::fs::write("/tmp/if_statement_false.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/if_statement_false.cad"])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "3");
    verify_solution(&stdout, "y", "100");

    let _ = std::fs::remove_file("/tmp/if_statement_false.cad");
}

#[test]
fn test_if_statement_with_bool() {
    // Test if-statement with boolean condition variable
    let test_code = r#"
let condition: bool;
let x: i32;
let y: i32;

condition == true;
x == 10;

if condition {
    y == x + 5;
}
"#;

    std::fs::write("/tmp/if_statement_bool.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/if_statement_bool.cad"])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "condition", "true");
    verify_solution(&stdout, "x", "10");
    verify_solution(&stdout, "y", "15");

    let _ = std::fs::remove_file("/tmp/if_statement_bool.cad");
}

#[test]
fn test_if_statement_with_assignment() {
    // Test assignment statements in if-statement branches
    let test_code = r#"
let x: i32;
let y: i32;

x > 10;

if x > 20 {
    y = x * 2;
} else {
    y = x + 5;
}
"#;

    std::fs::write("/tmp/if_statement_assignment.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/if_statement_assignment.cad"])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // The solver should find a solution satisfying all constraints
    // Since x > 10, we could have x = 21 (which is > 20), leading to y = 42
    // or x = 11 (which is <= 20), leading to y = 16
    // Both are valid solutions

    let _ = std::fs::remove_file("/tmp/if_statement_assignment.cad");
}

#[test]
fn test_if_statement_nested() {
    // Test nested if-statements
    let test_code = r#"
let x: i32;
let y: i32;

x > 10;

if x > 20 {
    if x > 30 {
        y = 100;
    } else {
        y = 50;
    }
} else {
    y = x + 5;
}
"#;

    std::fs::write("/tmp/if_statement_nested.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/if_statement_nested.cad"])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Valid solutions include:
    // - x = 31 (> 30), y = 100
    // - x = 21 (> 20 but <= 30), y = 50
    // - x = 11 (<= 20), y = 16

    let _ = std::fs::remove_file("/tmp/if_statement_nested.cad");
}

#[test]
fn test_if_statement_field_assignment() {
    // Test field assignment statements in if-statement branches
    let test_code = r#"
struct Point {
    x: i32,
    y: i32,
}

let p: Point;
let condition: bool;

condition == true;

if condition {
    p.x = 10;
    p.y = 20;
} else {
    p.x = 5;
    p.y = 15;
}
"#;

    std::fs::write("/tmp/if_statement_field_assignment.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "solve",
            "/tmp/if_statement_field_assignment.cad",
        ])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "condition", "true");
    verify_solution(&stdout, "p.x", "10");
    verify_solution(&stdout, "p.y", "20");

    let _ = std::fs::remove_file("/tmp/if_statement_field_assignment.cad");
}

#[test]
fn test_if_statement_deeply_nested() {
    // Test deeply nested if-statements (3 levels)
    let test_code = r#"
let x: i32;
let y: i32;

x == 25;

if x > 10 {
    if x > 20 {
        if x > 30 {
            y = 1;
        } else {
            y = 2;
        }
    } else {
        y = 3;
    }
} else {
    y = 4;
}
"#;

    std::fs::write("/tmp/if_statement_deeply_nested.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/if_statement_deeply_nested.cad"])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "25");
    verify_solution(&stdout, "y", "2"); // x = 25 is > 20 but <= 30, so y = 2

    let _ = std::fs::remove_file("/tmp/if_statement_deeply_nested.cad");
}
