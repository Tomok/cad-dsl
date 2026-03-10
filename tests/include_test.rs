//! Integration tests for `include "path";` directive support.
//!
//! These tests exercise the full pipeline (CLI) against real fixture files on
//! disk, since include directives reference other files by relative path.

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
// Tests
// ============================================================================

/// A file that uses a struct defined in an included library should solve
/// correctly.
#[test]
fn test_include_basic() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/include_basic.cad");
    assert!(
        success,
        "Expected success for include_basic.cad, stderr:\n{}",
        stderr
    );
    assert!(
        stdout.contains("p.x = 3"),
        "Expected p.x = 3 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("p.y = 7"),
        "Expected p.y = 7 in:\n{}",
        stdout
    );
}

/// Including the same file twice must not produce a DuplicateDefinition error.
/// The second include should be silently skipped.
#[test]
fn test_include_duplicate_skipped() {
    let (success, stdout, stderr) = solve_file("tests/fixtures/solve/include_duplicate.cad");
    assert!(
        success,
        "Expected success for include_duplicate.cad (duplicate include should be silently skipped), stderr:\n{}",
        stderr
    );
    assert!(
        stdout.contains("p.x = 5"),
        "Expected p.x = 5 in:\n{}",
        stdout
    );
    assert!(
        stdout.contains("p.y = 10"),
        "Expected p.y = 10 in:\n{}",
        stdout
    );
}
