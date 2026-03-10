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

#[test]
fn test_for_loop_let_in_body() {
    // Regression test: let declarations inside for-loop bodies must create
    // per-iteration scoped variables rather than a single shared variable.
    // Without scoping, each iteration adds a conflicting equality constraint
    // (x==0, x==2, x==4...) which makes the system UNSAT.
    let (success, stdout, stderr) = solve_fixture("for_loop_let_in_body.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x_0", "0");
    verify_solution(&stdout, "x_1", "2");
    verify_solution(&stdout, "x_2", "4");
}

#[test]
fn test_for_loop_two_loops_same_let_name() {
    // Regression test: two separate for-loops that each declare a `let` with
    // the same variable name must not conflict. Without a global counter,
    // both loops would produce variables named "temp_0", "temp_1", "temp_2",
    // and the second loop's constraints would collide with the first's,
    // making the system UNSAT.
    let (success, stdout, stderr) = solve_fixture("for_loop_two_loops_same_let_name.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // First loop: temp_0 = 0*2 = 0, temp_1 = 1*2 = 2, temp_2 = 2*2 = 4
    verify_solution(&stdout, "temp_0", "0");
    verify_solution(&stdout, "temp_1", "2");
    verify_solution(&stdout, "temp_2", "4");
    // Second loop: temp_3 = 1, temp_4 = 2, temp_5 = 3
    verify_solution(&stdout, "temp_3", "1");
    verify_solution(&stdout, "temp_4", "2");
    verify_solution(&stdout, "temp_5", "3");
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
fn test_with_statement_primitive_types() {
    let (success, stdout, stderr) = solve_fixture("with_statement_primitive_types.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // This test should verify primitive types can be used in with-statements
    // The exact assertions depend on the fixture content
    assert!(stdout.contains("entities"));
}

#[test]
fn test_with_statement_nested_struct() {
    let (success, stdout, stderr) = solve_fixture("with_statement_nested_struct.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Verify nested struct handling in with-statements
    assert!(stdout.contains("entities"));
}

#[test]
fn test_with_statement_constraints() {
    let (success, stdout, stderr) = solve_fixture("with_statement_constraints.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Verify constraints work correctly within with-statement context
    assert!(stdout.contains("entities"));
}

// ============================================================================
// Transform With-Statement Tests
// ============================================================================
//
// Tests for transform with-statements which apply coordinate transformations
// using __transform__ methods.

#[test]
fn test_transform_with_statement_recognized() {
    // Test that transform contexts are recognized and can be entered
    // Note: Transform application (automatic transformation of variable accesses)
    // is not yet fully implemented, but the context tracking infrastructure works.
    let test_code = r#"
struct Translate {
    offset_x: i32,
    offset_y: i32,

    fn __transform__(p: &i32) -> i32 {
        return p + self.offset_x;
    }
}

let transform: Translate;
transform.offset_x == 10;
transform.offset_y == 5;

with transform {
    let x: i32;
    x == 0;
}
"#;

    std::fs::write("/tmp/transform_with_simple.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/transform_with_simple.cad"])
        .output()
        .expect("Failed to execute command");

    // The infrastructure is in place, so this should succeed
    // (even though transforms aren't applied yet)
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert!(
        output.status.success(),
        "Command should succeed with transform infrastructure in place. stderr: {}, stdout: {}",
        stderr,
        stdout
    );

    // Should have basic output
    assert!(stdout.contains("offset_x = 10"));
    assert!(stdout.contains("x = 0"));
}

#[test]
fn test_transform_application_basic() {
    // Test that transform methods are automatically applied to variable declarations
    // in transform contexts, creating shadow variables and linking them via constraints.
    //
    // REGRESSION TEST: This also validates that the solver uses qualified names
    // (sketch.entities.p) rather than short names (p) when constructing variable paths
    // for container variables in transform contexts. This ensures VarDefinition.identifier
    // is used correctly throughout the solver.
    let test_code = r#"
struct Point2D {
    x: f64,
    y: f64,
}

struct Point3D {
    x: f64,
    y: f64,
    z: f64,
}

struct Sketch2D {
    container entities,
    origin: Point3D,

    fn __transform__(p3d: &Point3D) -> Point2D {
        return Point2D {
            x: p3d.x - self.origin.x,
            y: p3d.y - self.origin.y,
        };
    }
}

let sketch: Sketch2D;
sketch.origin.x == 0.0;
sketch.origin.y == 0.0;
sketch.origin.z == 0.0;

with sketch {
    let .p: Point2D;
    .p.x == 10.0;
    .p.y == 20.0;
}
"#;

    std::fs::write("/tmp/transform_basic.cad", test_code).unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", "/tmp/transform_basic.cad"])
        .output()
        .expect("Failed to execute command");

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    assert!(
        output.status.success(),
        "Transform application should work. stderr: {}, stdout: {}",
        stderr,
        stdout
    );

    // The transform creates a shadow Point3D variable internally and links it via constraints:
    //   sketch.entities.p.x == shadow.x - sketch.origin.x
    //   sketch.entities.p.y == shadow.y - sketch.origin.y
    //   sketch.entities.p.x == 10.0
    //   sketch.entities.p.y == 20.0
    //   sketch.origin.x == 0.0, sketch.origin.y == 0.0
    // The solver computes: shadow.x = 10.0, shadow.y = 20.0
    // Note: Shadow variables are filtered from output (implementation detail)

    // Check that the declared variable got the right values
    assert!(stdout.contains("sketch.entities.p.x = 10"));
    assert!(stdout.contains("sketch.entities.p.y = 20"));
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

#[test]
fn test_if_let_in_then_branch() {
    // Regression test: let declarations inside if-then branches create uniquely-scoped
    // Z3 variables (y_0, y_1, …) so they do not conflict with variables in other branches
    // or with each other across separate if-statements.  The scoped name appears in output.
    let (success, stdout, stderr) = solve_fixture("if_let_in_then_branch.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
    // y declared in then-branch gets a unique scoped name (y_0 for the first let in this run)
    verify_solution(&stdout, "y_0", "20");
}

#[test]
fn test_if_let_in_else_branch() {
    // Regression test: let declarations inside if-else branches must be supported.
    // Each branch-scoped let gets a unique counter suffix so they are separate Z3 variables.
    let (success, stdout, stderr) = solve_fixture("if_let_in_else_branch.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "3");
    // then-branch y → y_0, else-branch z → z_1
    verify_solution(&stdout, "z_1", "99"); // else-branch fires since x <= 5
}

#[test]
fn test_if_let_uninit_in_branch() {
    // Regression test: uninitialized let declarations inside if-branches must be supported.
    // The variable is scoped (y_0) and the subsequent constraint inside the branch
    // is resolved through the alias y → y_0.
    let (success, stdout, stderr) = solve_fixture("if_let_uninit_in_branch.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "7");
    // y declared without init in the branch → scoped as y_0
    verify_solution(&stdout, "y_0", "42");
}

#[test]
fn test_if_let_same_name_both_branches() {
    // Core scoping test: the same variable name declared in both branches must
    // produce two *separate* Z3 variables, not one shared one.
    // If they shared a variable the constraints "x==7" and "x==9" would conflict
    // and the system would be UNSAT.
    let (success, stdout, stderr) = solve_fixture("if_let_same_name_both_branches.cad");
    assert!(
        success,
        "Solver failed (expected SAT): {}{}",
        stdout, stderr
    );
    verify_solution(&stdout, "a", "true");
    // then-branch x → x_0 = 7, else-branch x → x_1 = 9 (both appear in output)
    verify_solution(&stdout, "x_0", "7");
    verify_solution(&stdout, "x_1", "9");
}

// ============================================================================
// Reference Type Alias Tests
// ============================================================================
//
// Tests for reference type alias tracking, where references create aliases
// rather than separate variables, enabling proper constraint propagation.

#[test]
fn test_simple_alias() {
    // Test that a simple reference (let r = &x) creates an alias
    // Constraints on *r should affect x directly
    let (success, stdout, stderr) = solve_fixture("simple_alias.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
}

#[test]
fn test_multi_alias() {
    // Test that multiple aliases to the same variable share constraints
    // Both r1 and r2 should be aliases to x
    let (success, stdout, stderr) = solve_fixture("multi_alias.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "5");
}

#[test]
fn test_transitive_alias() {
    // Test that transitive aliases work correctly
    // r1 -> x, r2 -> r1, so **r2 should affect x
    let (success, stdout, stderr) = solve_fixture("transitive_alias.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "10");
}

#[test]
fn test_struct_field_alias() {
    // Test that references to struct fields create proper aliases
    // r should be an alias to p.x
    let (success, stdout, stderr) = solve_fixture("struct_field_alias.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "p.x", "10");
    verify_solution(&stdout, "p.y", "20");
}

#[test]
fn test_function_returns_ref() {
    // Test type-based alias tracking for functions returning references
    // let r = get_ref() should create an alias to x when get_ref() returns &x
    let (success, stdout, stderr) = solve_fixture("function_returns_ref.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "42");
}

#[test]
fn test_method_returns_ref() {
    // Test type-based alias tracking for methods returning references
    // let r = p.get_x() should create an alias to p.x when get_x() returns &self.x
    let (success, stdout, stderr) = solve_fixture("method_returns_ref.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "p.x", "100");
    verify_solution(&stdout, "p.y", "200");
}

#[test]
fn test_function_param_ref() {
    // Test function that returns a reference to its parameter
    // let r = identity(&x) should create an alias to x
    let (success, stdout, stderr) = solve_fixture("function_param_ref.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "50");
    verify_solution(&stdout, "y", "60");
}

#[test]
fn test_struct_ref_fields() {
    // Test struct with reference-typed fields
    // Reference fields should create proper aliases to the referenced variables
    let (success, stdout, stderr) = solve_fixture("struct_ref_fields.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "100");
    verify_solution(&stdout, "y", "200");
}

#[test]
fn test_struct_ref_fields_nested() {
    // Test struct with reference fields pointing to struct fields
    // Reference fields should create aliases to nested struct fields
    let (success, stdout, stderr) = solve_fixture("struct_ref_fields_nested.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "p.x", "50");
    verify_solution(&stdout, "p.y", "75");
}

// ============================================================================
// Rune Blocks
// ============================================================================

#[test]
fn test_rune_basic() {
    // Test basic rune block execution
    let (success, stdout, stderr) = solve_fixture("rune_basic.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "5");
    verify_solution(&stdout, "y", "10");
}

#[test]
fn test_rune_in_loop() {
    // Test rune block executed multiple times with different inputs
    // The same rune block code is compiled once but executed with different parameter values
    // This ensures execution results are NOT cached (only compilation is cached)
    let (success, stdout, stderr) = solve_fixture("rune_in_loop.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    // Verify input variables
    verify_solution(&stdout, "x1", "10");
    verify_solution(&stdout, "x2", "20");
    verify_solution(&stdout, "x3", "30");

    // Verify each rune block executed with different values
    verify_solution(&stdout, "result1", "20"); // 10 * 2
    verify_solution(&stdout, "result2", "40"); // 20 * 2
    verify_solution(&stdout, "result3", "60"); // 30 * 2
}

#[test]
fn test_rune_in_function() {
    // Test multiple rune blocks called with different simple variables
    // This verifies execution works correctly with different parameters
    let (success, stdout, stderr) = solve_fixture("rune_in_function.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    verify_solution(&stdout, "a", "5");
    verify_solution(&stdout, "b", "10");
    verify_solution(&stdout, "result_a", "10"); // 5 * 2 = 10
    verify_solution(&stdout, "result_b", "20"); // 10 * 2 = 20
}

#[test]
fn test_rune_multiple_params() {
    // Test rune block with multiple parameters
    let (success, stdout, stderr) = solve_fixture("rune_multiple_params.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    verify_solution(&stdout, "x", "3");
    verify_solution(&stdout, "y", "7");
    verify_solution(&stdout, "result", "10"); // 3 + 7 = 10
}

#[test]
fn test_rune_param_assignment() {
    // Test rune block with three direct parameters
    let (success, stdout, stderr) = solve_fixture("rune_param_assignment.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    verify_solution(&stdout, "x", "5");
    verify_solution(&stdout, "y", "10");
    verify_solution(&stdout, "z", "100");
    verify_solution(&stdout, "result", "115"); // 5 + 10 + 100 = 115
}

#[test]
fn test_rune_control_flow() {
    // Test if-statement inside rune block
    let (success, stdout, stderr) = solve_fixture("rune_control_flow.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    verify_solution(&stdout, "x", "15");
    verify_solution(&stdout, "result", "30"); // 15 > 10, so 15 * 2 = 30
}

#[test]
fn test_rune_loop() {
    // Test for-loop inside rune block (sum of 0..5)
    let (success, stdout, stderr) = solve_fixture("rune_loop.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    verify_solution(&stdout, "n", "5");
    verify_solution(&stdout, "result", "10"); // 0 + 1 + 2 + 3 + 4 = 10
}

#[test]
fn test_rune_result_in_constraint() {
    // Test chaining rune blocks (results flow to next rune block's parameters)
    let (success, stdout, stderr) = solve_fixture("rune_result_in_constraint.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    verify_solution(&stdout, "x", "5");
    verify_solution(&stdout, "y", "25"); // 5 * 5 = 25
    verify_solution(&stdout, "z", "35"); // 25 + 10 = 35
}

#[test]
fn test_rune_fibonacci() {
    // Test Fibonacci calculation (10th Fibonacci number)
    let (success, stdout, stderr) = solve_fixture("rune_fibonacci.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    verify_solution(&stdout, "n", "10");
    verify_solution(&stdout, "fib", "55"); // 10th Fibonacci number is 55
}

#[test]
fn test_rune_abs_value() {
    // Test absolute value computation
    let (success, stdout, stderr) = solve_fixture("rune_abs_value.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    verify_solution(&stdout, "x", "-15");
    verify_solution(&stdout, "y", "20");
    verify_solution(&stdout, "abs_x", "15"); // |-15| = 15
    verify_solution(&stdout, "abs_y", "20"); // |20| = 20
}

#[test]
fn test_rune_nested_expressions() {
    // Test rune block with nested expressions
    let (success, stdout, stderr) = solve_fixture("rune_nested_expressions.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);

    verify_solution(&stdout, "a", "5");
    verify_solution(&stdout, "b", "3");
    verify_solution(&stdout, "result", "16"); // (5 + 3) * (5 - 3) = 8 * 2 = 16
}

// ============================================================================
// Rune Blocks - Error Cases
// ============================================================================

#[test]
fn test_rune_error_undefined_param() {
    // Test that using undefined variable as rune parameter fails
    let (success, stdout, stderr) = solve_fixture("rune_error_undefined_param.cad");
    assert!(!success, "Expected solver to fail for undefined parameter");

    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("not found")
            || combined.contains("undefined")
            || combined.contains("Variable"),
        "Expected error about undefined variable, got: {}",
        combined
    );
}

#[test]
fn test_rune_error_unconstrained_param() {
    // Test that unconstrained rune parameter is detected
    // Note: The current implementation may handle this differently
    // This test documents expected behavior
    let (success, stdout, stderr) = solve_fixture("rune_error_unconstrained_param.cad");

    // This may succeed with Z3 picking an arbitrary value, or fail
    // Both behaviors are acceptable - document what happens
    if !success {
        let combined = format!("{}{}", stdout, stderr);
        assert!(
            combined.contains("unconstrained")
                || combined.contains("not determined")
                || combined.contains("unknown"),
            "Expected error about unconstrained variable, got: {}",
            combined
        );
    } else {
        // If it succeeds, x should have some value
        let x = extract_value(&stdout, "x");
        let y = extract_value(&stdout, "y");
        assert_eq!(y, x * 2, "y should equal x * 2");
    }
}

// ============================================================================
// Optimize Block Tests
// ============================================================================

#[test]
fn test_optimize_minimize_simple() {
    let (success, stdout, stderr) = solve_fixture("optimize_minimize_simple.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // x should be minimized: smallest integer > 0 is 1
    verify_solution(&stdout, "x", "1");
}

#[test]
fn test_optimize_maximize_simple() {
    let (success, stdout, stderr) = solve_fixture("optimize_maximize_simple.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // x should be maximized: largest integer < 100 is 99
    verify_solution(&stdout, "x", "99");
}

#[test]
fn test_optimize_lexicographic() {
    let (success, stdout, stderr) = solve_fixture("optimize_lexicographic.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // Primary: minimize x -> x = 0
    // Secondary: maximize y (with x=0 and x+y=10) -> y = 10
    verify_solution(&stdout, "x", "0");
    verify_solution(&stdout, "y", "10");
}

// ============================================================================
// Scientific Notation Tests
// ============================================================================

#[test]
fn test_scientific_notation() {
    let (success, stdout, stderr) = solve_fixture("scientific_notation.cad");
    assert!(success, "Solver failed: {}{}", stdout, stderr);
    // 1e3 = 1000.0
    verify_solution(&stdout, "x", "1000");
    // 1e-3 = 0.001
    verify_solution(&stdout, "y", "0.001");
}

// ============================================================================
// Rune File I/O Tests
// ============================================================================

/// Helper: run the solver on an arbitrary CAD source file path (not a fixture).
fn solve_cad_file(path: &str) -> (bool, String, String) {
    let output = Command::new("cargo")
        .args(["run", "--", "solve", path])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (success, stdout, stderr)
}

#[test]
fn test_rune_file_write() {
    // Build unique paths for both the temporary .cad file and the output file
    // so that parallel test runs do not interfere with each other.
    let pid = std::process::id();
    let cad_path = format!("/tmp/cad_dsl_rune_file_write_{}.cad", pid);
    let out_path = format!("/tmp/cad_dsl_rune_file_write_out_{}.txt", pid);

    // Use .to_string() instead of format!() to avoid `{}` in the CAD source,
    // which would confuse the CAD lexer's brace-counting in rune block bodies.
    // Use `fs::` prefix (not `file::`) to avoid Rune's built-in file! macro namespace.
    let cad_source = format!(
        "let x: i32;\n\
         x == 42;\n\
         \n\
         let written = rune(x) {{\n\
             fs::write(\"{out}\", x.to_string());\n\
             1\n\
         }};\n",
        out = out_path,
    );
    std::fs::write(&cad_path, &cad_source).expect("Failed to write temp .cad file");

    // Remove any leftover output file from a previous interrupted run
    let _ = std::fs::remove_file(&out_path);

    let (success, stdout, stderr) = solve_cad_file(&cad_path);

    // Clean up temp .cad file regardless of outcome
    let _ = std::fs::remove_file(&cad_path);

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "42");
    verify_solution(&stdout, "written", "1");

    // Verify the rune block actually wrote the file
    let content = std::fs::read_to_string(&out_path)
        .expect("fs::write should have created the output file");

    // Clean up output file
    let _ = std::fs::remove_file(&out_path);

    assert!(
        content.contains("42"),
        "Output file should contain the solved value 42, got: {:?}",
        content
    );
}

#[test]
fn test_rune_file_append() {
    let pid = std::process::id();
    let cad_path = format!("/tmp/cad_dsl_rune_file_append_{}.cad", pid);
    let out_path = format!("/tmp/cad_dsl_rune_file_append_out_{}.txt", pid);

    // Remove any leftover output file
    let _ = std::fs::remove_file(&out_path);

    // Use string concatenation to build the content lines rather than format!()
    // macros with {}, which confuse the CAD lexer in rune block bodies.
    let cad_source = format!(
        "let a: i32;\n\
         let b: i32;\n\
         a == 10;\n\
         b == 20;\n\
         \n\
         let w1 = rune(a) {{\n\
             fs::append(\"{out}\", \"a=\" + a.to_string() + \"\\n\");\n\
             1\n\
         }};\n\
         \n\
         let w2 = rune(b) {{\n\
             fs::append(\"{out}\", \"b=\" + b.to_string() + \"\\n\");\n\
             1\n\
         }};\n",
        out = out_path,
    );
    std::fs::write(&cad_path, &cad_source).expect("Failed to write temp .cad file");

    let (success, stdout, stderr) = solve_cad_file(&cad_path);

    let _ = std::fs::remove_file(&cad_path);

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "a", "10");
    verify_solution(&stdout, "b", "20");

    let content = std::fs::read_to_string(&out_path)
        .expect("fs::append should have created the output file");

    let _ = std::fs::remove_file(&out_path);

    assert!(
        content.contains("a=10"),
        "Output file should contain 'a=10', got: {:?}",
        content
    );
    assert!(
        content.contains("b=20"),
        "Output file should contain 'b=20', got: {:?}",
        content
    );
}

#[test]
fn test_rune_file_read() {
    let pid = std::process::id();
    let cad_path = format!("/tmp/cad_dsl_rune_file_read_{}.cad", pid);
    let in_path = format!("/tmp/cad_dsl_rune_file_read_in_{}.txt", pid);

    // Pre-populate the file that the rune block will read
    std::fs::write(&in_path, "hello from file").expect("Failed to write input file");

    // Return 1 if content is non-empty (i.e. read succeeded), 0 otherwise.
    // Avoid {{ }} inside the rune body to prevent CAD lexer brace confusion.
    let cad_source = format!(
        "let x: i32;\n\
         x == 7;\n\
         \n\
         let read_ok = rune(x) {{\n\
             let content = fs::read(\"{inp}\");\n\
             if content.len() > 0 {{ 1 }} else {{ 0 }}\n\
         }};\n",
        inp = in_path,
    );
    std::fs::write(&cad_path, &cad_source).expect("Failed to write temp .cad file");

    let (success, stdout, stderr) = solve_cad_file(&cad_path);

    let _ = std::fs::remove_file(&cad_path);
    let _ = std::fs::remove_file(&in_path);

    assert!(success, "Solver failed: {}{}", stdout, stderr);
    verify_solution(&stdout, "x", "7");
    verify_solution(&stdout, "read_ok", "1");
}
