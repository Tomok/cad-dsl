#![allow(dead_code)] // Error types for future type checker phases

use crate::lexer::Span;
use std::fmt;

/// Type checking errors for the CAD-DSL language
#[derive(Debug, Clone)]
pub enum TypeCheckError {
    /// Type mismatch between expected and found types
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    /// Cannot infer the type of an expression
    CannotInferType { expr_kind: String, span: Span },

    /// Incompatible types for a binary operation
    IncompatibleTypes {
        lhs_type: String,
        rhs_type: String,
        operation: String,
        span: Span,
    },

    /// Function argument type does not match parameter type
    ArgumentTypeMismatch {
        param_name: String,
        expected: String,
        found: String,
        span: Span,
    },

    /// Wrong number of arguments provided to a function
    WrongNumberOfArguments {
        expected: usize,
        found: usize,
        span: Span,
    },

    /// Non-numeric operand used with numeric operator
    NonNumericOperand {
        operator: String,
        operand_type: String,
        span: Span,
    },

    /// Non-boolean condition used where boolean is required
    NonBooleanCondition { found_type: String, span: Span },

    /// Cannot index into a non-array type
    CannotIndex { array_type: String, span: Span },

    /// Rune type checking error
    Rune { message: String, span: Span },
}

impl fmt::Display for TypeCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeCheckError::TypeMismatch {
                expected,
                found,
                span,
            } => {
                write!(
                    f,
                    "Type mismatch at line {}, column {}: expected '{}', found '{}'",
                    span.start.line, span.start.column, expected, found
                )
            }
            TypeCheckError::CannotInferType { expr_kind, span } => {
                write!(
                    f,
                    "Cannot infer type of {} at line {}, column {}",
                    expr_kind, span.start.line, span.start.column
                )
            }
            TypeCheckError::IncompatibleTypes {
                lhs_type,
                rhs_type,
                operation,
                span,
            } => {
                write!(
                    f,
                    "Incompatible types for {} operation at line {}, column {}: '{}' and '{}'",
                    operation, span.start.line, span.start.column, lhs_type, rhs_type
                )
            }
            TypeCheckError::ArgumentTypeMismatch {
                param_name,
                expected,
                found,
                span,
            } => {
                write!(
                    f,
                    "Argument type mismatch for parameter '{}' at line {}, column {}: expected '{}', found '{}'",
                    param_name, span.start.line, span.start.column, expected, found
                )
            }
            TypeCheckError::WrongNumberOfArguments {
                expected,
                found,
                span,
            } => {
                write!(
                    f,
                    "Wrong number of arguments at line {}, column {}: expected {}, found {}",
                    span.start.line, span.start.column, expected, found
                )
            }
            TypeCheckError::NonNumericOperand {
                operator,
                operand_type,
                span,
            } => {
                write!(
                    f,
                    "Non-numeric operand for operator '{}' at line {}, column {}: found '{}'",
                    operator, span.start.line, span.start.column, operand_type
                )
            }
            TypeCheckError::NonBooleanCondition { found_type, span } => {
                write!(
                    f,
                    "Non-boolean condition at line {}, column {}: expected 'bool', found '{}'",
                    span.start.line, span.start.column, found_type
                )
            }
            TypeCheckError::CannotIndex { array_type, span } => {
                write!(
                    f,
                    "Cannot index into non-array type '{}' at line {}, column {}",
                    array_type, span.start.line, span.start.column
                )
            }
            TypeCheckError::Rune { message, span } => {
                write!(
                    f,
                    "Rune type checking error at line {}, column {}: {}",
                    span.start.line, span.start.column, message
                )
            }
        }
    }
}

impl std::error::Error for TypeCheckError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::LineColumn;

    fn make_span(line: usize, column: usize) -> Span {
        Span {
            start: LineColumn { line, column },
            lines: 0,
            end_column: column + 5,
        }
    }

    #[test]
    fn test_type_mismatch_display() {
        let error = TypeCheckError::TypeMismatch {
            expected: "f64".to_string(),
            found: "i32".to_string(),
            span: make_span(10, 5),
        };
        let display = error.to_string();
        assert!(display.contains("Type mismatch"));
        assert!(display.contains("expected 'f64'"));
        assert!(display.contains("found 'i32'"));
        assert!(display.contains("line 10"));
        assert!(display.contains("column 5"));
    }

    #[test]
    fn test_cannot_infer_type_display() {
        let error = TypeCheckError::CannotInferType {
            expr_kind: "empty array literal".to_string(),
            span: make_span(7, 12),
        };
        let display = error.to_string();
        assert!(display.contains("Cannot infer type"));
        assert!(display.contains("empty array literal"));
        assert!(display.contains("line 7"));
        assert!(display.contains("column 12"));
    }

    #[test]
    fn test_incompatible_types_display() {
        let error = TypeCheckError::IncompatibleTypes {
            lhs_type: "f64".to_string(),
            rhs_type: "bool".to_string(),
            operation: "addition".to_string(),
            span: make_span(15, 8),
        };
        let display = error.to_string();
        assert!(display.contains("Incompatible types"));
        assert!(display.contains("addition"));
        assert!(display.contains("'f64'"));
        assert!(display.contains("'bool'"));
        assert!(display.contains("line 15"));
        assert!(display.contains("column 8"));
    }

    #[test]
    fn test_argument_type_mismatch_display() {
        let error = TypeCheckError::ArgumentTypeMismatch {
            param_name: "radius".to_string(),
            expected: "f64".to_string(),
            found: "String".to_string(),
            span: make_span(20, 15),
        };
        let display = error.to_string();
        assert!(display.contains("Argument type mismatch"));
        assert!(display.contains("parameter 'radius'"));
        assert!(display.contains("expected 'f64'"));
        assert!(display.contains("found 'String'"));
        assert!(display.contains("line 20"));
        assert!(display.contains("column 15"));
    }

    #[test]
    fn test_wrong_number_of_arguments_display() {
        let error = TypeCheckError::WrongNumberOfArguments {
            expected: 3,
            found: 2,
            span: make_span(5, 10),
        };
        let display = error.to_string();
        assert!(display.contains("Wrong number of arguments"));
        assert!(display.contains("expected 3"));
        assert!(display.contains("found 2"));
        assert!(display.contains("line 5"));
        assert!(display.contains("column 10"));
    }

    #[test]
    fn test_non_numeric_operand_display() {
        let error = TypeCheckError::NonNumericOperand {
            operator: "+".to_string(),
            operand_type: "String".to_string(),
            span: make_span(12, 7),
        };
        let display = error.to_string();
        assert!(display.contains("Non-numeric operand"));
        assert!(display.contains("operator '+'"));
        assert!(display.contains("found 'String'"));
        assert!(display.contains("line 12"));
        assert!(display.contains("column 7"));
    }

    #[test]
    fn test_non_boolean_condition_display() {
        let error = TypeCheckError::NonBooleanCondition {
            found_type: "i32".to_string(),
            span: make_span(8, 3),
        };
        let display = error.to_string();
        assert!(display.contains("Non-boolean condition"));
        assert!(display.contains("expected 'bool'"));
        assert!(display.contains("found 'i32'"));
        assert!(display.contains("line 8"));
        assert!(display.contains("column 3"));
    }

    #[test]
    fn test_cannot_index_display() {
        let error = TypeCheckError::CannotIndex {
            array_type: "String".to_string(),
            span: make_span(18, 22),
        };
        let display = error.to_string();
        assert!(display.contains("Cannot index into non-array type"));
        assert!(display.contains("'String'"));
        assert!(display.contains("line 18"));
        assert!(display.contains("column 22"));
    }

    #[test]
    fn test_error_trait_implementation() {
        let error = TypeCheckError::TypeMismatch {
            expected: "f64".to_string(),
            found: "i32".to_string(),
            span: make_span(1, 1),
        };
        // Ensure it can be used as a std::error::Error
        let _error_ref: &dyn std::error::Error = &error;
    }

    #[test]
    fn test_debug_trait_derived() {
        let error = TypeCheckError::IncompatibleTypes {
            lhs_type: "f64".to_string(),
            rhs_type: "bool".to_string(),
            operation: "multiplication".to_string(),
            span: make_span(5, 5),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("IncompatibleTypes"));
        assert!(debug_str.contains("f64"));
        assert!(debug_str.contains("bool"));
        assert!(debug_str.contains("multiplication"));
    }

    #[test]
    fn test_clone_trait() {
        let error1 = TypeCheckError::NonBooleanCondition {
            found_type: "String".to_string(),
            span: make_span(1, 1),
        };
        let error2 = error1.clone();
        // Verify both are identical by checking their string representations
        assert_eq!(error1.to_string(), error2.to_string());
    }
}
