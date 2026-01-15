//! Solution Formatter for Z3 Models
//!
//! This module formats Z3 solver results for display.
//! It extracts variable values from Z3 models and formats them in a human-readable way.
//!
//! # Purpose
//!
//! The solution formatter performs the following tasks:
//! - Checks Z3 solver results (SAT/UNSAT/UNKNOWN)
//! - Extracts variable assignments from Z3 models
//! - Formats values according to their types (i32, f64, bool)
//! - Sorts variables alphabetically for consistent output
//!
//! # Example Output
//!
//! ```text
//! Solution:
//! x = 10
//! y = 20
//! ```
//!
//! Or for unsatisfiable constraints:
//!
//! ```text
//! UNSAT: The constraints cannot be satisfied
//! ```

#![allow(dead_code)] // Public API for future constraint solving implementation

use super::constraint_extractor::Variable;
use super::z3_bridge::Z3Ast;
use std::collections::HashMap;
use std::fmt;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during solution formatting
#[derive(Debug, Clone, PartialEq)]
pub enum SolutionFormatterError {
    /// The constraints are unsatisfiable
    Unsat,

    /// The solver could not determine satisfiability
    Unknown,

    /// A variable was not found in the Z3 model
    VariableNotInModel { var_name: String },

    /// Failed to evaluate a variable in the Z3 model
    EvaluationFailed { var_name: String },

    /// Failed to convert a Z3 value to the expected type
    TypeConversionFailed {
        var_name: String,
        expected_type: String,
    },
}

impl fmt::Display for SolutionFormatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolutionFormatterError::Unsat => {
                write!(f, "UNSAT: The constraints cannot be satisfied")
            }
            SolutionFormatterError::Unknown => {
                write!(f, "UNKNOWN: The solver could not determine satisfiability")
            }
            SolutionFormatterError::VariableNotInModel { var_name } => {
                write!(f, "Variable '{}' not found in model", var_name)
            }
            SolutionFormatterError::EvaluationFailed { var_name } => {
                write!(f, "Failed to evaluate variable '{}' in model", var_name)
            }
            SolutionFormatterError::TypeConversionFailed {
                var_name,
                expected_type,
            } => {
                write!(
                    f,
                    "Failed to convert variable '{}' to type {}",
                    var_name, expected_type
                )
            }
        }
    }
}

impl std::error::Error for SolutionFormatterError {}

// ============================================================================
// Solution Formatter
// ============================================================================

/// Formatter for Z3 solver solutions
///
/// Takes a Z3 solver and extracts the solution in a human-readable format.
pub struct SolutionFormatter<'src, 'arena> {
    /// Map of variable names to their Z3 AST representations
    variables: &'src HashMap<String, Z3Ast>,
    /// List of variables to format
    var_list: Vec<&'arena Variable<'src, 'arena>>,
}

impl<'src, 'arena> SolutionFormatter<'src, 'arena> {
    /// Create a new solution formatter
    pub fn new(
        variables: &'src HashMap<String, Z3Ast>,
        var_list: Vec<&'arena Variable<'src, 'arena>>,
    ) -> Self {
        Self {
            variables,
            var_list,
        }
    }

    /// Format the solver result
    ///
    /// Returns a formatted string with variable assignments sorted alphabetically,
    /// or an error if the constraints are unsatisfiable or the solver is uncertain.
    pub fn format_solution(&self, solver: &z3::Solver) -> Result<String, SolutionFormatterError> {
        // Check solver result
        match solver.check() {
            z3::SatResult::Sat => {
                // Get the model
                let model = solver.get_model().ok_or(SolutionFormatterError::Unknown)?;
                self.format_model(&model)
            }
            z3::SatResult::Unsat => Err(SolutionFormatterError::Unsat),
            z3::SatResult::Unknown => Err(SolutionFormatterError::Unknown),
        }
    }

    /// Format a Z3 model
    fn format_model(&self, model: &z3::Model) -> Result<String, SolutionFormatterError> {
        let mut assignments = Vec::new();

        // Extract values for each variable
        for var in &self.var_list {
            let assignment = self.format_variable(var, model)?;
            assignments.push((var.name.clone(), assignment));
        }

        // Sort alphabetically by variable name
        assignments.sort_by_key(|(name, _)| name.clone());

        // Build the output string
        let mut output = String::new();
        for (name, value) in assignments {
            output.push_str(&format!("{} = {}\n", name, value));
        }

        Ok(output)
    }

    /// Format a single variable assignment
    fn format_variable(
        &self,
        var: &Variable<'src, 'arena>,
        model: &z3::Model,
    ) -> Result<String, SolutionFormatterError> {
        // Get the Z3 AST for this variable
        let z3_var = self.variables.get(&var.name).ok_or_else(|| {
            SolutionFormatterError::VariableNotInModel {
                var_name: var.name.clone(),
            }
        })?;

        // Evaluate the variable in the model
        // Use false to avoid model completion - only get values that are actually constrained
        let value = match z3_var {
            Z3Ast::Int(int_ast) => {
                match model.eval(int_ast, false) {
                    Some(eval_result) => {
                        // Try to convert to concrete value
                        match eval_result.as_i64() {
                            Some(int_value) => format!("{}", int_value),
                            None => {
                                // as_i64() failed - check if it's a numeric value or symbolic expression
                                let z3_str = format!("{}", eval_result);
                                // If the string starts with a digit or '-', it's likely a large number
                                // Otherwise it's a symbolic expression (under-constrained)
                                if z3_str
                                    .chars()
                                    .next()
                                    .is_some_and(|c| c.is_ascii_digit() || c == '-')
                                {
                                    // It's a large number that exceeds i64 range
                                    format!("{} (exceeds i64 range)", z3_str)
                                } else {
                                    // It's a symbolic expression - variable is under-constrained
                                    "<under-constrained>".to_string()
                                }
                            }
                        }
                    }
                    None => {
                        // Variable is not constrained
                        "<under-constrained>".to_string()
                    }
                }
            }
            Z3Ast::Real(real_ast) => {
                match model.eval(real_ast, false) {
                    Some(eval_result) => {
                        // Try to get a real number from Z3
                        match eval_result.as_rational() {
                            Some((numerator, denominator)) => {
                                if denominator == 0 {
                                    // Division by zero in rational - this shouldn't happen normally
                                    format!("{}/{} (invalid rational)", numerator, 0)
                                } else {
                                    // Convert to f64 (handles both positive and negative denominators)
                                    let float_value = numerator as f64 / denominator as f64;
                                    format!("{}", float_value)
                                }
                            }
                            None => {
                                // as_rational() failed - check if it's a numeric value or symbolic expression
                                let z3_str = format!("{}", eval_result);
                                if z3_str
                                    .chars()
                                    .next()
                                    .is_some_and(|c| c.is_ascii_digit() || c == '-')
                                {
                                    // It's a numeric value that cannot be converted to f64
                                    format!("{} (cannot convert to f64)", z3_str)
                                } else {
                                    // It's a symbolic expression - variable is under-constrained
                                    "<under-constrained>".to_string()
                                }
                            }
                        }
                    }
                    None => {
                        // Variable is not constrained
                        "<under-constrained>".to_string()
                    }
                }
            }
            Z3Ast::Bool(bool_ast) => {
                match model.eval(bool_ast, false) {
                    Some(eval_result) => {
                        match eval_result.as_bool() {
                            Some(bool_value) => format!("{}", bool_value),
                            None => {
                                // as_bool() failed - check if it's "true"/"false" or symbolic
                                let z3_str = format!("{}", eval_result);
                                if z3_str == "true" || z3_str == "false" {
                                    // Shouldn't happen, but handle it
                                    format!("{} (unexpected bool value)", z3_str)
                                } else {
                                    // It's a symbolic expression - variable is under-constrained
                                    "<under-constrained>".to_string()
                                }
                            }
                        }
                    }
                    None => {
                        // Variable is not constrained
                        "<under-constrained>".to_string()
                    }
                }
            }
        };

        Ok(value)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::definitions::VarDefinition;
    use crate::hir::expr::{ResolvedExpr, ResolvedExprKind};
    use crate::hir::types::ResolvedType;
    use crate::lexer::{LineColumn, Span};
    use crate::solver_legacy::constraint_extractor::{Constraint, ConstraintProblem, Variable};
    use crate::solver_legacy::z3_bridge::Z3Bridge;
    use assert_matches::assert_matches;
    use bumpalo::Bump;

    fn test_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 10,
        }
    }

    fn make_expr<'arena>(
        arena: &'arena Bump,
        kind: ResolvedExprKind<'static, 'arena>,
        ty: &'arena ResolvedType<'static, 'arena>,
    ) -> &'arena ResolvedExpr<'static, 'arena> {
        arena.alloc(ResolvedExpr {
            span: test_span(),
            kind,
            ty,
        })
    }

    #[test]
    fn test_format_sat_result_single_variable() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create: let x = 10;
        let init_x = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let var_x = Variable::new("x", *int_ty, Some(init_x), test_span());

        // Build constraint problem
        let mut problem = ConstraintProblem::new();
        problem.add_variable(var_x.clone());

        // Translate to Z3
        let mut bridge = Z3Bridge::new();
        bridge.add_problem(&problem).unwrap();

        // Format solution
        let formatter = SolutionFormatter::new(bridge.variables(), vec![&var_x]);
        let result = formatter.format_solution(bridge.solver());

        assert!(result.is_ok());
        let solution = result.unwrap();
        assert_eq!(solution.trim(), "x = 10");
    }

    #[test]
    fn test_format_sat_result_multiple_variables() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // Create: let y = 10;
        let init_y = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let var_y = Variable::new("y", *int_ty, Some(init_y), test_span());

        // Create: let x;
        let var_x = Variable::new("x", *int_ty, None, test_span());

        // Create: x + y == 20
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let var_def_y = arena.alloc(VarDefinition {
            name: "y",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: Some(init_y),
            scope_level: 0,
            span: test_span(),
        });

        let x_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let y_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let sum = make_expr(
            &arena,
            ResolvedExprKind::Add {
                lhs: x_ref,
                rhs: y_ref,
            },
            int_ty,
        );
        let twenty = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: sum,
                rhs: twenty,
            },
            bool_ty,
        );

        // Build constraint problem
        let mut problem = ConstraintProblem::new();
        problem.add_variable(var_y.clone());
        problem.add_variable(var_x.clone());
        problem.add_constraint(Constraint::new(constraint, test_span()));

        // Translate to Z3
        let mut bridge = Z3Bridge::new();
        bridge.add_problem(&problem).unwrap();

        // Format solution (note: variables in alphabetical order)
        let formatter = SolutionFormatter::new(bridge.variables(), vec![&var_x, &var_y]);
        let result = formatter.format_solution(bridge.solver());

        assert!(result.is_ok());
        let solution = result.unwrap();
        // Variables should be sorted alphabetically: x before y
        assert_eq!(solution, "x = 10\ny = 10\n");
    }

    #[test]
    fn test_format_sat_result_alphabetical_sorting() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });

        // Create variables: z, a, m (intentionally out of order)
        let var_z = Variable::new(
            "z",
            *int_ty,
            Some(make_expr(
                &arena,
                ResolvedExprKind::IntLit { value: 3 },
                int_ty,
            )),
            test_span(),
        );
        let var_a = Variable::new(
            "a",
            *int_ty,
            Some(make_expr(
                &arena,
                ResolvedExprKind::IntLit { value: 1 },
                int_ty,
            )),
            test_span(),
        );
        let var_m = Variable::new(
            "m",
            *int_ty,
            Some(make_expr(
                &arena,
                ResolvedExprKind::IntLit { value: 2 },
                int_ty,
            )),
            test_span(),
        );

        // Build constraint problem
        let mut problem = ConstraintProblem::new();
        problem.add_variable(var_z.clone());
        problem.add_variable(var_a.clone());
        problem.add_variable(var_m.clone());

        // Translate to Z3
        let mut bridge = Z3Bridge::new();
        bridge.add_problem(&problem).unwrap();

        // Format solution
        let formatter = SolutionFormatter::new(bridge.variables(), vec![&var_z, &var_a, &var_m]);
        let result = formatter.format_solution(bridge.solver());

        assert!(result.is_ok());
        let solution = result.unwrap();
        // Should be sorted: a, m, z
        assert_eq!(solution, "a = 1\nm = 2\nz = 3\n");
    }

    #[test]
    fn test_format_unsat_result() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // Create: let x;
        let var_x = Variable::new("x", *int_ty, None, test_span());

        // Create: x == 10
        let var_def_x = arena.alloc(VarDefinition {
            name: "x",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: None,
            scope_level: 0,
            span: test_span(),
        });
        let x_ref1 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let ten = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let constraint1 = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: x_ref1,
                rhs: ten,
            },
            bool_ty,
        );

        // Create: x == 20 (conflicting with x == 10)
        let x_ref2 = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "x",
                definition: var_def_x,
            },
            int_ty,
        );
        let twenty = make_expr(&arena, ResolvedExprKind::IntLit { value: 20 }, int_ty);
        let constraint2 = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: x_ref2,
                rhs: twenty,
            },
            bool_ty,
        );

        // Build constraint problem
        let mut problem = ConstraintProblem::new();
        problem.add_variable(var_x.clone());
        problem.add_constraint(Constraint::new(constraint1, test_span()));
        problem.add_constraint(Constraint::new(constraint2, test_span()));

        // Translate to Z3
        let mut bridge = Z3Bridge::new();
        bridge.add_problem(&problem).unwrap();

        // Format solution
        let formatter = SolutionFormatter::new(bridge.variables(), vec![&var_x]);
        let result = formatter.format_solution(bridge.solver());

        assert!(result.is_err());
        assert_matches!(result.unwrap_err(), SolutionFormatterError::Unsat);
    }

    #[test]
    fn test_format_float_variable() {
        let arena = Bump::new();
        let real_ty = arena.alloc(ResolvedType::F64 { span: test_span() });

        // Create: let x = 3.14;
        let init_x = make_expr(&arena, ResolvedExprKind::FloatLit { value: 3.14 }, real_ty);
        let var_x = Variable::new("x", *real_ty, Some(init_x), test_span());

        // Build constraint problem
        let mut problem = ConstraintProblem::new();
        problem.add_variable(var_x.clone());

        // Translate to Z3
        let mut bridge = Z3Bridge::new();
        bridge.add_problem(&problem).unwrap();

        // Format solution
        let formatter = SolutionFormatter::new(bridge.variables(), vec![&var_x]);
        let result = formatter.format_solution(bridge.solver());

        assert!(result.is_ok());
        let solution = result.unwrap();
        // Z3 stores reals as fractions, so we check that it's close to 3.14
        assert!(solution.contains("x = 3.14"));
    }

    #[test]
    fn test_format_bool_variable() {
        let arena = Bump::new();
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // Create: let x = true;
        let init_x = make_expr(&arena, ResolvedExprKind::BoolLit { value: true }, bool_ty);
        let var_x = Variable::new("x", *bool_ty, Some(init_x), test_span());

        // Build constraint problem
        let mut problem = ConstraintProblem::new();
        problem.add_variable(var_x.clone());

        // Translate to Z3
        let mut bridge = Z3Bridge::new();
        bridge.add_problem(&problem).unwrap();

        // Format solution
        let formatter = SolutionFormatter::new(bridge.variables(), vec![&var_x]);
        let result = formatter.format_solution(bridge.solver());

        assert!(result.is_ok());
        let solution = result.unwrap();
        assert_eq!(solution.trim(), "x = true");
    }

    #[test]
    fn test_format_mixed_types() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // Create: let x = 42;
        let var_x = Variable::new(
            "x",
            *int_ty,
            Some(make_expr(
                &arena,
                ResolvedExprKind::IntLit { value: 42 },
                int_ty,
            )),
            test_span(),
        );

        // Create: let flag = false;
        let var_flag = Variable::new(
            "flag",
            *bool_ty,
            Some(make_expr(
                &arena,
                ResolvedExprKind::BoolLit { value: false },
                bool_ty,
            )),
            test_span(),
        );

        // Build constraint problem
        let mut problem = ConstraintProblem::new();
        problem.add_variable(var_x.clone());
        problem.add_variable(var_flag.clone());

        // Translate to Z3
        let mut bridge = Z3Bridge::new();
        bridge.add_problem(&problem).unwrap();

        // Format solution
        let formatter = SolutionFormatter::new(bridge.variables(), vec![&var_x, &var_flag]);
        let result = formatter.format_solution(bridge.solver());

        assert!(result.is_ok());
        let solution = result.unwrap();
        // Should be sorted alphabetically: flag before x
        assert_eq!(solution, "flag = false\nx = 42\n");
    }

    #[test]
    fn test_error_display_unsat() {
        let error = SolutionFormatterError::Unsat;
        let display = format!("{}", error);
        assert!(display.contains("UNSAT"));
        assert!(display.contains("cannot be satisfied"));
    }

    #[test]
    fn test_error_display_unknown() {
        let error = SolutionFormatterError::Unknown;
        let display = format!("{}", error);
        assert!(display.contains("UNKNOWN"));
        assert!(display.contains("could not determine"));
    }

    #[test]
    fn test_error_display_variable_not_in_model() {
        let error = SolutionFormatterError::VariableNotInModel {
            var_name: "x".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("x"));
        assert!(display.contains("not found in model"));
    }

    #[test]
    fn test_error_display_evaluation_failed() {
        let error = SolutionFormatterError::EvaluationFailed {
            var_name: "y".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("y"));
        assert!(display.contains("Failed to evaluate"));
    }

    #[test]
    fn test_error_display_type_conversion_failed() {
        let error = SolutionFormatterError::TypeConversionFailed {
            var_name: "z".to_string(),
            expected_type: "i32".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("z"));
        assert!(display.contains("i32"));
        assert!(display.contains("Failed to convert"));
    }

    #[test]
    fn test_format_unconstrained_variable() {
        let arena = Bump::new();
        let int_ty = arena.alloc(ResolvedType::I32 { span: test_span() });
        let bool_ty = arena.alloc(ResolvedType::Bool { span: test_span() });

        // Create: let x;  (no initializer)
        let var_x = Variable::new("x", *int_ty, None, test_span());

        // Create: let y = 10;
        let init_y = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let var_y = Variable::new("y", *int_ty, Some(init_y), test_span());

        // Create: y == 10  (only constrain y, not x)
        let var_def_y = arena.alloc(VarDefinition {
            name: "y",
            name_span: test_span(),
            var_type: Some(*int_ty),
            init: Some(init_y),
            scope_level: 0,
            span: test_span(),
        });

        let y_ref = make_expr(
            &arena,
            ResolvedExprKind::Var {
                name: "y",
                definition: var_def_y,
            },
            int_ty,
        );
        let ten = make_expr(&arena, ResolvedExprKind::IntLit { value: 10 }, int_ty);
        let constraint = make_expr(
            &arena,
            ResolvedExprKind::Eq {
                lhs: y_ref,
                rhs: ten,
            },
            bool_ty,
        );

        // Build constraint problem
        let mut problem = ConstraintProblem::new();
        problem.add_variable(var_x.clone());
        problem.add_variable(var_y.clone());
        problem.add_constraint(Constraint::new(constraint, test_span()));

        // Translate to Z3
        let mut bridge = Z3Bridge::new();
        bridge.add_problem(&problem).unwrap();

        // Format solution (x has no constraints, y = 10)
        let formatter = SolutionFormatter::new(bridge.variables(), vec![&var_x, &var_y]);
        let result = formatter.format_solution(bridge.solver());

        match &result {
            Ok(solution) => {
                // x should be marked as under-constrained, y should be 10
                assert_eq!(solution, "x = <under-constrained>\ny = 10\n");
            }
            Err(e) => {
                panic!("Formatting failed: {:?}", e);
            }
        }
    }
}
