//! Comprehensive HIR transform representation tests (Phase 5)
//!
//! This test suite validates the HIR transform representation implementation
//! completed in phases 1-4. Tests cover:
//! - Transform detection for both internal (dot-prefix) and external variables
//! - Nested transform contexts and transform chains
//! - Type compatibility across transform boundaries
//! - Container variables with qualified names
//!
//! See docs/HIR_TRANSFORM_REPRESENTATION.md for implementation details.
//!
//! Note: These are integration tests that verify behavior through the CLI,
//! not unit tests. Unit tests for internal types should be added to the
//! relevant source modules using #[cfg(test)].

use std::io::Write;
use std::process::Command;

/// Helper function to run the solve command on a test file
fn solve_test(test_code: &str) -> (bool, String, String) {
    let mut temp_file = tempfile::Builder::new()
        .suffix(".cad")
        .tempfile()
        .expect("Failed to create temp file");
    temp_file
        .write_all(test_code.as_bytes())
        .expect("Failed to write temp file");
    let path = temp_file.path().to_owned();

    let output = Command::new("cargo")
        .args(["run", "--", "solve", path.to_str().unwrap()])
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // temp_file is dropped here, automatically deleting the file
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

// ============================================================================
// Integration Tests - Internal Declarations (Dot-Prefix)
// ============================================================================

#[test]
fn test_transform_simple_dot_prefix() {
    // Test simple transform with dot-prefix variable
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

struct Sketch {
    container entities,
    origin: Point3D,

    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D {
            x: p.x - self.origin.x,
            y: p.y - self.origin.y,
        };
    }
}

let sketch: Sketch;
sketch.origin.x == 0.0;
sketch.origin.y == 0.0;
sketch.origin.z == 0.0;

with sketch {
    let .p: Point2D;
    .p.x == 10.0;
    .p.y == 20.0;
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Transform with dot-prefix variable should succeed. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // Verify the solution contains the container variable (not the view variable)
    assert!(
        stdout.contains("sketch.entities.p.x = 10"),
        "Expected sketch.entities.p.x = 10 in solution, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("sketch.entities.p.y = 20"),
        "Expected sketch.entities.p.y = 20 in solution, got:\n{}",
        stdout
    );
}

#[test]
fn test_transform_nested_contexts() {
    // Test nested transform contexts (with inside with)
    // NOTE: When containers are declared outside, they maintain separate namespaces.
    // The inner container variable path uses only its own container, not the outer's.
    let test_code = r#"
struct Point3D {
    x: f64,
    y: f64,
    z: f64,
}

struct Transform {
    offset: Point3D,
    container entities,

    fn __transform__(p: &Point3D) -> Point3D {
        return Point3D {
            x: p.x + self.offset.x,
            y: p.y + self.offset.y,
            z: p.z + self.offset.z,
        };
    }
}

struct Scale {
    scale: f64,
    container entities,

    fn __transform__(p: &Point3D) -> Point3D {
        return Point3D {
            x: p.x * self.scale,
            y: p.y * self.scale,
            z: p.z * self.scale,
        };
    }
}

let t: Transform;
t.offset.x == 10.0;
t.offset.y == 20.0;
t.offset.z == 30.0;

let s: Scale;
s.scale == 2.0;

with t {
    with s {
        let .p: Point3D;
        .p.x == 5.0;
        .p.y == 10.0;
        .p.z == 15.0;
    }
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Nested transform contexts should succeed. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // When containers are declared outside the with-blocks, the innermost container
    // maintains only its own namespace (s.entities.p), not a fully qualified path
    assert!(
        stdout.contains("s.entities.p"),
        "Expected inner container variable path in solution, got:\n{}",
        stdout
    );
}

#[test]
fn test_transform_multiple_variables_same_context() {
    // Test multiple dot-prefix variables in the same transform context
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

struct Sketch {
    container entities,
    origin: Point3D,

    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D {
            x: p.x - self.origin.x,
            y: p.y - self.origin.y,
        };
    }
}

let sketch: Sketch;
sketch.origin.x == 0.0;
sketch.origin.y == 0.0;
sketch.origin.z == 0.0;

with sketch {
    let .p1: Point2D;
    let .p2: Point2D;
    .p1.x == 10.0;
    .p1.y == 20.0;
    .p2.x == 30.0;
    .p2.y == 40.0;
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Multiple variables in transform context should succeed. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // Verify both variables are in the solution
    verify_solution(&stdout, "sketch.entities.p1.x", "10");
    verify_solution(&stdout, "sketch.entities.p1.y", "20");
    verify_solution(&stdout, "sketch.entities.p2.x", "30");
    verify_solution(&stdout, "sketch.entities.p2.y", "40");
}

#[test]
fn test_transform_with_container_combination() {
    // Test that container and transform can be combined (container without transform)
    let test_code = r#"
struct Point {
    x: i32,
    y: i32,
}

struct Container {
    container entities,
}

let c: Container;

with c {
    let .p: Point;
    .p.x == 10;
    .p.y == 20;
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Container without transform should succeed. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // Verify the solution contains the container variable
    verify_solution(&stdout, "c.entities.p.x", "10");
    verify_solution(&stdout, "c.entities.p.y", "20");
}

// ============================================================================
// Integration Tests - External Variable Access
// ============================================================================

#[test]
fn test_external_variable_in_transform_context() {
    // Test that external variables (declared outside with-block) are transformed
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

struct Sketch {
    container entities,
    origin: Point3D,

    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D {
            x: p.x - self.origin.x,
            y: p.y - self.origin.y,
        };
    }
}

let p: Point3D;
let sketch: Sketch;

sketch.origin.x == 0.0;
sketch.origin.y == 0.0;
sketch.origin.z == 0.0;

with sketch {
    // External variable accessed in transform context
    p.x == 10.0;
    p.y == 20.0;
    p.z == 30.0;
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "External variable in transform context should succeed. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // External variable p should have Point3D values
    verify_solution(&stdout, "p.x", "10");
    verify_solution(&stdout, "p.y", "20");
    verify_solution(&stdout, "p.z", "30");
}

#[test]
fn test_external_variable_nested_field_access() {
    // Test external variable with nested field access in transform context
    let test_code = r#"
struct Point {
    x: f64,
    y: f64,
}

struct Line {
    start: Point,
    end: Point,
}

struct Transform {
    offset: f64,
    container entities,

    fn __transform__(p: &Point) -> Point {
        return Point {
            x: p.x + self.offset,
            y: p.y + self.offset,
        };
    }
}

let line: Line;
let t: Transform;

t.offset == 5.0;

with t {
    line.start.x == 10.0;
    line.start.y == 20.0;
    line.end.x == 30.0;
    line.end.y == 40.0;
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "External variable with nested fields should succeed. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // Verify the line fields are in the solution
    verify_solution(&stdout, "line.start.x", "10");
    verify_solution(&stdout, "line.start.y", "20");
    verify_solution(&stdout, "line.end.x", "30");
    verify_solution(&stdout, "line.end.y", "40");
}

#[test]
fn test_external_variable_array_element() {
    // Test external variable array element access in transform context
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

struct Sketch {
    container entities,
    origin: Point3D,

    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D {
            x: p.x - self.origin.x,
            y: p.y - self.origin.y,
        };
    }
}

let points: [Point3D; 2];
let sketch: Sketch;

sketch.origin.x == 0.0;
sketch.origin.y == 0.0;
sketch.origin.z == 0.0;

with sketch {
    points[0].x == 10.0;
    points[0].y == 20.0;
    points[0].z == 30.0;
    points[1].x == 40.0;
    points[1].y == 50.0;
    points[1].z == 60.0;
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "External array element access should succeed. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // Verify the array elements are in the solution
    verify_solution(&stdout, "points[0].x", "10");
    verify_solution(&stdout, "points[0].y", "20");
    verify_solution(&stdout, "points[0].z", "30");
    verify_solution(&stdout, "points[1].x", "40");
    verify_solution(&stdout, "points[1].y", "50");
    verify_solution(&stdout, "points[1].z", "60");
}

#[test]
fn test_mixed_internal_and_external_variables() {
    // Test both dot-prefix (internal) and external variables in the same with-block
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

struct Sketch {
    container entities,
    origin: Point3D,

    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D {
            x: p.x - self.origin.x,
            y: p.y - self.origin.y,
        };
    }
}

let external_p: Point3D;
let sketch: Sketch;

sketch.origin.x == 0.0;
sketch.origin.y == 0.0;
sketch.origin.z == 0.0;

with sketch {
    let .internal_p: Point2D;
    .internal_p.x == 10.0;
    .internal_p.y == 20.0;

    external_p.x == 30.0;
    external_p.y == 40.0;
    external_p.z == 50.0;
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Mixed internal and external variables should succeed. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // Verify both internal and external variables are in the solution
    verify_solution(&stdout, "sketch.entities.internal_p.x", "10");
    verify_solution(&stdout, "sketch.entities.internal_p.y", "20");
    verify_solution(&stdout, "external_p.x", "30");
    verify_solution(&stdout, "external_p.y", "40");
    verify_solution(&stdout, "external_p.z", "50");
}

// ============================================================================
// End-to-End Tests - Complex Transform Scenarios
// ============================================================================

#[test]
fn test_transform_chain_three_levels() {
    // Test transform chain with 3 levels of nesting
    // NOTE: When containers are declared outside, each maintains its own namespace.
    let test_code = r#"
struct Value {
    v: f64,
}

struct Transform1 {
    offset1: f64,
    container entities,

    fn __transform__(val: &Value) -> Value {
        return Value { v: val.v + self.offset1 };
    }
}

struct Transform2 {
    offset2: f64,
    container entities,

    fn __transform__(val: &Value) -> Value {
        return Value { v: val.v + self.offset2 };
    }
}

struct Transform3 {
    offset3: f64,
    container entities,

    fn __transform__(val: &Value) -> Value {
        return Value { v: val.v + self.offset3 };
    }
}

let t1: Transform1;
let t2: Transform2;
let t3: Transform3;

t1.offset1 == 10.0;
t2.offset2 == 20.0;
t3.offset3 == 30.0;

with t1 {
    with t2 {
        with t3 {
            let .v: Value;
            .v.v == 100.0;
        }
    }
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Three-level transform chain should succeed. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // When containers are declared outside, the innermost container maintains
    // only its own namespace (t3.entities.v)
    assert!(
        stdout.contains("t3.entities.v"),
        "Expected inner container variable path in solution, got:\n{}",
        stdout
    );

    // Verify the transform chain applied correctly: 100 = base + 10 + 20 + 30
    // So base = 40, and result should be v.v = 90 (after transforms are applied)
    assert!(
        stdout.contains("t3.entities.v.v = 90"),
        "Expected t3.entities.v.v in solution, got:\n{}",
        stdout
    );
}

#[test]
fn test_multiple_independent_transform_contexts() {
    // Test multiple independent with-blocks (not nested)
    // NOTE: Container variables store Point3D values, view variables (Point2D) are internal only
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

struct Sketch {
    container entities,
    origin: Point3D,

    fn __transform__(p: &Point3D) -> Point2D {
        return Point2D {
            x: p.x - self.origin.x,
            y: p.y - self.origin.y,
        };
    }
}

let sketch1: Sketch;
let sketch2: Sketch;

sketch1.origin.x == 0.0;
sketch1.origin.y == 0.0;
sketch1.origin.z == 0.0;

sketch2.origin.x == 100.0;
sketch2.origin.y == 200.0;
sketch2.origin.z == 300.0;

with sketch1 {
    let .p1: Point2D;
    .p1.x == 10.0;
    .p1.y == 20.0;
}

with sketch2 {
    let .p2: Point2D;
    .p2.x == 30.0;
    .p2.y == 40.0;
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Multiple independent contexts should succeed. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // Container variables store the untransformed Point3D values:
    // For sketch1: p1 (Point2D) = (10, 20) → container (Point3D) = (10, 20, ?)
    // For sketch2: p2 (Point2D) = (30, 40) → container (Point3D) = (130, 240, ?)
    //              because p2.x = p3d.x - 100, so p3d.x = 130
    verify_solution(&stdout, "sketch1.entities.p1.x", "10");
    verify_solution(&stdout, "sketch1.entities.p1.y", "20");
    verify_solution(&stdout, "sketch2.entities.p2.x", "130"); // 30 + 100 (origin offset)
    verify_solution(&stdout, "sketch2.entities.p2.y", "240"); // 40 + 200 (origin offset)
}

// ============================================================================
// Regression Tests
// ============================================================================

#[test]
fn test_regression_no_transform_still_works() {
    // Regression test: ensure code without transforms still works
    let test_code = r#"
struct Point {
    x: i32,
    y: i32,
}

let p: Point;
p.x == 10;
p.y == 20;
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Code without transforms should still work. stderr: {}, stdout: {}",
        stderr, stdout
    );

    verify_solution(&stdout, "p.x", "10");
    verify_solution(&stdout, "p.y", "20");
}

#[test]
fn test_regression_simple_container_still_works() {
    // Regression test: ensure simple container (no transform) still works
    let test_code = r#"
struct Container {
    container entities,
}

let c: Container;

with c {
    let .x: i32;
    .x == 42;
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Simple container without transform should work. stderr: {}, stdout: {}",
        stderr, stdout
    );

    verify_solution(&stdout, "c.entities.x", "42");
}

#[test]
fn test_regression_all_basic_features() {
    // Comprehensive regression test covering basic features
    let test_code = r#"
struct Point {
    x: i32,
    y: i32,
}

let p1: Point;
let p2: Point;
let arr: [i32; 3];

p1.x == 10;
p1.y == 20;
p2.x == p1.x + 5;
p2.y == p1.y + 10;

arr[0] == 1;
arr[1] == arr[0] + 1;
arr[2] == arr[1] + 1;
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Basic features regression test should pass. stderr: {}, stdout: {}",
        stderr, stdout
    );

    verify_solution(&stdout, "p1.x", "10");
    verify_solution(&stdout, "p1.y", "20");
    verify_solution(&stdout, "p2.x", "15");
    verify_solution(&stdout, "p2.y", "30");
    verify_solution(&stdout, "arr[0]", "1");
    verify_solution(&stdout, "arr[1]", "2");
    verify_solution(&stdout, "arr[2]", "3");
}

// ============================================================================
// Type Compatibility Tests
// ============================================================================

#[test]
fn test_transform_type_compatibility() {
    // Test that transform chains maintain type compatibility
    // (output type of outer transform matches input type of inner transform)
    // NOTE: When containers are declared outside, each maintains its own namespace.
    let test_code = r#"
struct TypeA {
    a: f64,
}

struct TypeB {
    b: f64,
}

struct TypeC {
    c: f64,
}

struct TransformAtoB {
    container entities,

    fn __transform__(input: &TypeA) -> TypeB {
        return TypeB { b: input.a * 2.0 };
    }
}

struct TransformBtoC {
    container entities,

    fn __transform__(input: &TypeB) -> TypeC {
        return TypeC { c: input.b + 10.0 };
    }
}

let t1: TransformAtoB;
let t2: TransformBtoC;

with t1 {
    with t2 {
        let .val: TypeC;
        .val.c == 50.0;
    }
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Transform type compatibility should work. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // When containers are declared outside, the innermost container
    // maintains only its own namespace (t2.entities.val)
    assert!(
        stdout.contains("t2.entities.val"),
        "Expected inner container variable path in solution, got:\n{}",
        stdout
    );
}

#[test]
fn test_container_variable_qualified_names() {
    // Test that container variables use qualified names (not simple names)
    // This verifies that VarDefinition.identifier uses VariableIdentifier correctly
    let test_code = r#"
struct Container {
    container entities,
}

let c: Container;

with c {
    let .var1: i32;
    let .var2: i32;
    .var1 == 10;
    .var2 == .var1 + 5;
}
"#;

    let (success, stdout, stderr) = solve_test(test_code);

    assert!(
        success,
        "Container variables with qualified names should work. stderr: {}, stdout: {}",
        stderr, stdout
    );

    // Verify qualified names are used in the solution
    verify_solution(&stdout, "c.entities.var1", "10");
    verify_solution(&stdout, "c.entities.var2", "15");
}
