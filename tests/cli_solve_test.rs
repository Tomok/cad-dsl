/// Integration tests for the solve CLI command
use std::process::Command;

#[test]
fn test_solve_simple_constraint() {
    let output = Command::new("cargo")
        .args(["run", "--", "solve", "tests/simple_constraint.cad"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("x = 10"));
    assert!(stdout.contains("y = 10"));
}

#[test]
fn test_solve_multiple_constraints() {
    let output = Command::new("cargo")
        .args(["run", "--", "solve", "tests/multiple_constraints.cad"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("x = 10"));
    assert!(stdout.contains("y = 10"));
}

#[test]
fn test_solve_unsat_constraint() {
    let output = Command::new("cargo")
        .args(["run", "--", "solve", "tests/unsat_constraint.cad"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("UNSAT") || combined.contains("cannot be satisfied"));
}

#[test]
fn test_solve_file_not_found() {
    let output = Command::new("cargo")
        .args(["run", "--", "solve", "tests/nonexistent.cad"])
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to read file") || stderr.contains("No such file"));
}
