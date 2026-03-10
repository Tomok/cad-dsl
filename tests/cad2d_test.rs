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

// ============================================================================
// cad2d stdlib: constraint helpers
// ============================================================================

/// `Line2D::horizontal()` forces end.y == start.y.
#[test]
fn test_cad2d_horizontal() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_horizontal.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("l.end.y = 5"),
        "Expected l.end.y = 5 in:\n{}",
        stdout
    );
}

/// `Line2D::vertical()` forces end.x == start.x.
#[test]
fn test_cad2d_vertical() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_vertical.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("l.end.x = 3"),
        "Expected l.end.x = 3 in:\n{}",
        stdout
    );
}

/// `Line2D::parallel_to()` constrains l2 to have the same slope as l1.
#[test]
fn test_cad2d_parallel() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_parallel.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("l2.end.y = 5"),
        "Expected l2.end.y = 5 in:\n{}",
        stdout
    );
}

/// `Line2D::perpendicular_to()` forces l2 to be orthogonal to l1.
#[test]
fn test_cad2d_perpendicular() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_perpendicular.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("l2.end.x = 5"),
        "Expected l2.end.x = 5 in:\n{}",
        stdout
    );
}

/// `Line2D::point_on()` constrains a point to lie on the (infinite) line.
#[test]
fn test_cad2d_point_on_line() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_point_on_line.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("p.y = 5"),
        "Expected p.y = 5 in:\n{}",
        stdout
    );
}

/// `Circle2D::point_on()` constrains a point to lie on the circle.
#[test]
fn test_cad2d_point_on_circle() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_point_on_circle.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("p.y = 4"),
        "Expected p.y = 4 in:\n{}",
        stdout
    );
}

/// `Line2D::tangent_to()` is satisfied by the horizontal tangent y=5 to circle r=5.
#[test]
fn test_cad2d_tangent_line_circle() {
    let (success, stdout, stderr) =
        solve_file("tests/fixtures/solve/cad2d_tangent_line_circle.cad");
    assert!(success, "Expected success (SAT), stderr:\n{}", stderr);
}

/// `Circle2D::tangent_to_circle_ext()` finds the radius of the second circle.
#[test]
fn test_cad2d_tangent_circles_ext() {
    let (success, stdout, stderr) =
        solve_file("tests/fixtures/solve/cad2d_tangent_circles_ext.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("c2.radius = 4"),
        "Expected c2.radius = 4 in:\n{}",
        stdout
    );
}

/// `Point2D::coincident()` constrains p2 to the same location as p1.
#[test]
fn test_cad2d_coincident() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_coincident.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("p2.x = 3"),
        "Expected p2.x = 3 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("p2.y = 7"),
        "Expected p2.y = 7 in:\n{}",
        stdout
    );
}

// ============================================================================
// cad2d stdlib: algebraic measurements
// ============================================================================

/// `Point2D::distance_sq_to()` returns squared Euclidean distance.
#[test]
fn test_cad2d_distance_sq() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_distance_sq.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("d_sq = 25"),
        "Expected d_sq = 25 in:\n{}",
        stdout
    );
}

/// `Line2D::midpoint_x()` and `midpoint_y()` return the midpoint coordinates.
#[test]
fn test_cad2d_midpoint() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_midpoint.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(stdout.contains("mx = 3"), "Expected mx = 3 in:\n{}", stdout);
    assert!(stdout.contains("my = 4"), "Expected my = 4 in:\n{}", stdout);
}

/// `Rectangle2D::area()` and `perimeter()` return algebraic measurements.
#[test]
fn test_cad2d_rectangle_area() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_rectangle_area.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(stdout.contains("a = 15"), "Expected a = 15 in:\n{}", stdout);
    assert!(stdout.contains("p = 16"), "Expected p = 16 in:\n{}", stdout);
}

/// `Line2D::length_sq()` returns the squared segment length.
#[test]
fn test_cad2d_length_sq() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_length_sq.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("lsq = 25"),
        "Expected lsq = 25 in:\n{}",
        stdout
    );
}

/// `Line2D::equal_length_to()` constrains two segments to have equal length.
#[test]
fn test_cad2d_equal_length() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_equal_length.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("l2.end.x = 15"),
        "Expected l2.end.x = 15 in:\n{}",
        stdout
    );
}

// ============================================================================
// cad2d stdlib: rune-based measurements (requires solver fix)
// ============================================================================

/// `Line2D::length()` computes segment length via rune block (sqrt).
#[test]
fn test_cad2d_rune_line_length() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_rune_line_length.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(
        stdout.contains("len = 5"),
        "Expected len = 5 in:\n{}",
        stdout
    );
}

/// `Point2D::distance_to()` computes Euclidean distance via rune block (sqrt).
#[test]
fn test_cad2d_rune_distance() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_rune_distance.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(stdout.contains("d = 5"), "Expected d = 5 in:\n{}", stdout);
}

/// `Circle2D::area()` computes area via rune block (PI * r^2); zero radius => area 0.
#[test]
fn test_cad2d_rune_circle_area() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/cad2d_rune_circle_area.cad");
    assert!(success, "Expected success, stderr:\n{}", stderr);
    assert!(stdout.contains("a = 0"), "Expected a = 0 in:\n{}", stdout);
}
