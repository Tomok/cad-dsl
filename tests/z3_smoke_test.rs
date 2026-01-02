/// Basic smoke test to verify Z3 constraint solver integration works.
///
/// This test creates a simple constraint problem and solves it using Z3,
/// verifying that the solver can create contexts, add assertions, and find solutions.

#[test]
fn z3_basic_functionality() {
    let solver = z3::Solver::new();

    // Create an integer variable 'x'
    let x = z3::ast::Int::new_const("x");

    // Add assertion: x == 42
    let forty_two = z3::ast::Int::from_i64(42);
    solver.assert(&x.eq(&forty_two));

    // Solve and verify it's satisfiable
    assert_eq!(solver.check(), z3::SatResult::Sat);

    // Get the model and verify x = 42
    let model = solver.get_model().expect("Failed to get model");
    let x_value = model.eval(&x, true).expect("Failed to evaluate x");
    assert_eq!(x_value.as_i64().expect("x should be an integer"), 42);
}

#[test]
fn z3_constraint_solving() {
    let solver = z3::Solver::new();

    // Create two integer variables
    let x = z3::ast::Int::new_const("x");
    let y = z3::ast::Int::new_const("y");

    // Add constraints: x + y == 10 and x > y
    let ten = z3::ast::Int::from_i64(10);
    let sum = z3::ast::Int::add(&[&x, &y]);
    solver.assert(&sum.eq(&ten));
    solver.assert(&x.gt(&y));

    // Solve and verify it's satisfiable
    assert_eq!(solver.check(), z3::SatResult::Sat);

    // Get the model and verify the solution
    let model = solver.get_model().expect("Failed to get model");
    let x_value = model
        .eval(&x, true)
        .expect("Failed to evaluate x")
        .as_i64()
        .expect("x should be an integer");
    let y_value = model
        .eval(&y, true)
        .expect("Failed to evaluate y")
        .as_i64()
        .expect("y should be an integer");

    // Verify the constraints are satisfied
    assert_eq!(x_value + y_value, 10);
    assert!(x_value > y_value);
}

#[test]
fn z3_unsatisfiable_constraints() {
    let solver = z3::Solver::new();

    // Create an integer variable
    let x = z3::ast::Int::new_const("x");

    // Add contradictory constraints: x == 5 and x == 10
    let five = z3::ast::Int::from_i64(5);
    let ten = z3::ast::Int::from_i64(10);
    solver.assert(&x.eq(&five));
    solver.assert(&x.eq(&ten));

    // Verify it's unsatisfiable
    assert_eq!(solver.check(), z3::SatResult::Unsat);
}
