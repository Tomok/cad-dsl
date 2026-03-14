//! Performance profiling tests for the constraint solver
//!
//! These tests measure solver performance on various problem sizes and complexities.
//! They are designed to track performance regressions and identify bottlenecks.

use std::io::Write;
use std::process::Command;
use std::time::Instant;

/// Helper to run the solve command and measure time
fn solve_with_timing(test_code: &str) -> (bool, String, String, u128) {
    let mut temp_file = tempfile::Builder::new()
        .suffix(".cad")
        .tempfile()
        .expect("Failed to create temp file");
    temp_file
        .write_all(test_code.as_bytes())
        .expect("Failed to write temp file");
    let path = temp_file.path().to_owned();

    let start = Instant::now();
    let output = Command::new("cargo")
        .args(["run", "--", "solve", path.to_str().unwrap()])
        .output()
        .expect("Failed to execute command");
    let duration = start.elapsed().as_millis();

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // temp_file is dropped here, automatically deleting the file
    (success, stdout, stderr, duration)
}

// ============================================================================
// Small Problem Tests (Baseline)
// ============================================================================

#[test]
fn perf_small_linear_system() {
    // 3 variables, 3 constraints - should be very fast
    let test_code = r#"
let x: i32;
let y: i32;
let z: i32;

x + y == 10;
y + z == 15;
x + z == 13;
"#;

    let (success, _stdout, stderr, duration) = solve_with_timing(test_code);
    assert!(success, "Solver failed: {}", stderr);
    println!("Small linear system (3 vars): {}ms", duration);

    // Note: Time includes compilation overhead from 'cargo run'
    // Actual solver time is much faster
    // Should complete in reasonable time (under 90 seconds with compilation)
    assert!(
        duration < 90000,
        "Small problem took too long: {}ms",
        duration
    );
}

// ============================================================================
// Medium Problem Tests
// ============================================================================

#[test]
fn perf_medium_struct_system() {
    // 10 points (20 variables), interconnected constraints
    let test_code = r#"
struct Point {
    x: i32,
    y: i32,
}

let p0: Point;
let p1: Point;
let p2: Point;
let p3: Point;
let p4: Point;
let p5: Point;
let p6: Point;
let p7: Point;
let p8: Point;
let p9: Point;

// Set initial point
p0.x == 0;
p0.y == 0;

// Chain constraints
p1.x == p0.x + 10;
p1.y == p0.y + 5;

p2.x == p1.x + 10;
p2.y == p1.y + 5;

p3.x == p2.x + 10;
p3.y == p2.y + 5;

p4.x == p3.x + 10;
p4.y == p3.y + 5;

p5.x == p4.x + 10;
p5.y == p4.y + 5;

p6.x == p5.x + 10;
p6.y == p5.y + 5;

p7.x == p6.x + 10;
p7.y == p6.y + 5;

p8.x == p7.x + 10;
p8.y == p7.y + 5;

p9.x == p8.x + 10;
p9.y == p8.y + 5;
"#;

    let (success, _stdout, stderr, duration) = solve_with_timing(test_code);
    assert!(success, "Solver failed: {}", stderr);
    println!("Medium struct system (10 points, 20 vars): {}ms", duration);

    // Note: Time includes compilation overhead
    // Should complete in reasonable time (under 100 seconds with compilation)
    assert!(
        duration < 100000,
        "Medium problem took too long: {}ms",
        duration
    );
}

#[test]
fn perf_array_system() {
    // Array of 20 integers with constraints
    let test_code = r#"
let arr: [i32; 20];

arr[0] == 1;
arr[1] == arr[0] + 1;
arr[2] == arr[1] + 1;
arr[3] == arr[2] + 1;
arr[4] == arr[3] + 1;
arr[5] == arr[4] + 1;
arr[6] == arr[5] + 1;
arr[7] == arr[6] + 1;
arr[8] == arr[7] + 1;
arr[9] == arr[8] + 1;
arr[10] == arr[9] + 1;
arr[11] == arr[10] + 1;
arr[12] == arr[11] + 1;
arr[13] == arr[12] + 1;
arr[14] == arr[13] + 1;
arr[15] == arr[14] + 1;
arr[16] == arr[15] + 1;
arr[17] == arr[16] + 1;
arr[18] == arr[17] + 1;
arr[19] == arr[18] + 1;
"#;

    let (success, _stdout, stderr, duration) = solve_with_timing(test_code);
    assert!(success, "Solver failed: {}", stderr);
    println!("Array system (20 elements): {}ms", duration);

    // Note: Time includes compilation overhead
    assert!(
        duration < 100000,
        "Array problem took too long: {}ms",
        duration
    );
}

// ============================================================================
// Large Problem Tests
// ============================================================================

#[test]
#[ignore] // Run manually for performance analysis
fn perf_large_array_of_structs() {
    // Array of 50 points (100 variables)
    let mut test_code = String::from(
        r#"
struct Point {
    x: i32,
    y: i32,
}

let points: [Point; 50];

points[0].x == 0;
points[0].y == 0;
"#,
    );

    // Generate chain constraints
    for i in 1..50 {
        test_code.push_str(&format!("points[{}].x == points[{}].x + 1;\n", i, i - 1));
        test_code.push_str(&format!("points[{}].y == points[{}].y + 1;\n", i, i - 1));
    }

    let (success, _stdout, stderr, duration) = solve_with_timing(&test_code);
    assert!(success, "Solver failed: {}", stderr);
    println!(
        "Large array of structs (50 points, 100 vars): {}ms",
        duration
    );

    // This is expected to be slower but should still complete (with compilation overhead)
    assert!(
        duration < 120000,
        "Large problem took too long: {}ms",
        duration
    );
}

#[test]
#[ignore] // Run manually for performance analysis
fn perf_complex_nested_structs() {
    // Deeply nested struct system
    let test_code = r#"
struct Point {
    x: i32,
    y: i32,
}

struct Line {
    start: Point,
    end: Point,
}

struct Rectangle {
    top_left: Line,
    bottom_right: Line,
}

let r1: Rectangle;
let r2: Rectangle;
let r3: Rectangle;

// Set initial rectangle
r1.top_left.start.x == 0;
r1.top_left.start.y == 0;
r1.top_left.end.x == 10;
r1.top_left.end.y == 0;
r1.bottom_right.start.x == 0;
r1.bottom_right.start.y == 10;
r1.bottom_right.end.x == 10;
r1.bottom_right.end.y == 10;

// Chain constraints to next rectangles
r2.top_left.start.x == r1.bottom_right.end.x;
r2.top_left.start.y == r1.bottom_right.end.y;
r2.top_left.end.x == r2.top_left.start.x + 10;
r2.top_left.end.y == r2.top_left.start.y;
r2.bottom_right.start.x == r2.top_left.start.x;
r2.bottom_right.start.y == r2.top_left.start.y + 10;
r2.bottom_right.end.x == r2.top_left.start.x + 10;
r2.bottom_right.end.y == r2.top_left.start.y + 10;

r3.top_left.start.x == r2.bottom_right.end.x;
r3.top_left.start.y == r2.bottom_right.end.y;
r3.top_left.end.x == r3.top_left.start.x + 10;
r3.top_left.end.y == r3.top_left.start.y;
r3.bottom_right.start.x == r3.top_left.start.x;
r3.bottom_right.start.y == r3.top_left.start.y + 10;
r3.bottom_right.end.x == r3.top_left.start.x + 10;
r3.bottom_right.end.y == r3.top_left.start.y + 10;
"#;

    let (success, _stdout, stderr, duration) = solve_with_timing(test_code);
    assert!(success, "Solver failed: {}", stderr);
    println!("Complex nested structs (3 rectangles): {}ms", duration);

    // Note: Time includes compilation overhead
    assert!(
        duration < 110000,
        "Nested struct problem took too long: {}ms",
        duration
    );
}

// ============================================================================
// Function Call Performance
// ============================================================================

#[test]
fn perf_function_calls() {
    // Multiple function calls with constraints
    let test_code = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

let x: i32;
let y: i32;
let z: i32;
let result1: i32;
let result2: i32;
let result3: i32;

x == 5;
y == 3;
z == 2;

result1 == add(x, y);
result2 == multiply(x, y);
result3 == add(result1, result2);
"#;

    let (success, _stdout, stderr, duration) = solve_with_timing(test_code);
    assert!(success, "Solver failed: {}", stderr);
    println!("Function call system: {}ms", duration);

    // Note: Time includes compilation overhead
    assert!(
        duration < 100000,
        "Function call problem took too long: {}ms",
        duration
    );
}

// ============================================================================
// For-Loop Performance
// ============================================================================

#[test]
fn perf_for_loop_unrolling() {
    // For-loop that unrolls to 20 iterations
    let test_code = r#"
let arr: [i32; 20];
let n: i32;

n == 20;

for i in 0..n {
    arr[i] == i * i;
}
"#;

    let (success, _stdout, stderr, duration) = solve_with_timing(test_code);
    assert!(success, "Solver failed: {}", stderr);
    println!("For-loop unrolling (20 iterations): {}ms", duration);

    // Note: Time includes compilation overhead
    assert!(
        duration < 110000,
        "For-loop problem took too long: {}ms",
        duration
    );
}

// ============================================================================
// Summary Test
// ============================================================================

#[test]
fn perf_summary() {
    println!("\n=== Solver Performance Summary ===");
    println!("Run individual tests for detailed timings:");
    println!("  cargo test --test solver_performance_test -- --nocapture");
    println!("\nRun large tests (ignored by default):");
    println!("  cargo test --test solver_performance_test -- --ignored --nocapture");
    println!("\n=================================\n");
}
