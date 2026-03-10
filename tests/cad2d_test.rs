//! Integration tests for the 2D CAD standard library (`lib/cad2d.cad`).
//!
//! Tests exercise the full pipeline against real fixture files, verifying
//! that the stdlib types (Point2D, Line2D, Circle2D, Sketch2D) parse,
//! type-check, and solve correctly.

use std::process::Command;

// ============================================================================
// Helpers
// ============================================================================

fn solve_file(path: &str) -> (bool, String, String) {
    let output = Command::new("cargo")
        .args(["run", "--", "solve", path])
        .output()
        .expect("Failed to execute cargo run");
    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (success, stdout, stderr)
}

// ============================================================================
// Algebraic<unit> parsing
// ============================================================================

/// `Algebraic<m>` is a valid field type; the solver treats it like f64.
#[test]
fn test_algebraic_with_unit() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/algebraic_with_unit.cad");
    assert!(
        success,
        "Expected success for algebraic_with_unit.cad, stderr:\n{}",
        stderr
    );
    assert!(
        stdout.contains("meas.value = 42"),
        "Expected meas.value = 42 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("meas.scale = 2"),
        "Expected meas.scale = 2 in:\n{}",
        stdout
    );
}

// ============================================================================
// cad2d stdlib: basic types
// ============================================================================

/// Point2D imported from cad2d.cad solves correctly.
#[test]
fn test_cad2d_point() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_point.cad");
    assert!(
        success,
        "Expected success for cad2d_point.cad, stderr:\n{}",
        stderr
    );
    assert!(
        stdout.contains("p.x = 3"),
        "Expected p.x = 3 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("p.y = 4"),
        "Expected p.y = 4 in:\n{}",
        stdout
    );
}

/// Line2D imported from cad2d.cad solves correctly.
#[test]
fn test_cad2d_line() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_line.cad");
    assert!(
        success,
        "Expected success for cad2d_line.cad, stderr:\n{}",
        stderr
    );
    assert!(
        stdout.contains("l.start.x = 0"),
        "Expected l.start.x = 0 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("l.start.y = 0"),
        "Expected l.start.y = 0 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("l.end.x = 10"),
        "Expected l.end.x = 10 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("l.end.y = 5"),
        "Expected l.end.y = 5 in:\n{}",
        stdout
    );
}

/// Sketch2D with Line2D and Circle2D entities solves correctly.
#[test]
fn test_cad2d_sketch() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_sketch.cad");
    assert!(
        success,
        "Expected success for cad2d_sketch.cad, stderr:\n{}",
        stderr
    );
    assert!(
        stdout.contains("sketch.entities.l.start.x = 0"),
        "Expected sketch.entities.l.start.x = 0 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("sketch.entities.l.end.x = 100"),
        "Expected sketch.entities.l.end.x = 100 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("sketch.entities.c.center.x = 50"),
        "Expected sketch.entities.c.center.x = 50 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("sketch.entities.c.radius = 25"),
        "Expected sketch.entities.c.radius = 25 in:\n{}",
        stdout
    );
}
