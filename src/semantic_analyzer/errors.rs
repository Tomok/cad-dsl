#![allow(dead_code)] // Error types for semantic analysis

use crate::lexer::Span;
use std::fmt;

/// Semantic analysis errors for the CAD-DSL language
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticError {
    /// Reference to an undefined variable
    UndefinedVariable { name: String, span: Span },

    /// Reference to an undefined function
    UndefinedFunction { name: String, span: Span },

    /// Reference to an undefined type
    UndefinedType { name: String, span: Span },

    /// Reference to an undefined field on a struct
    UndefinedField {
        struct_name: String,
        field_name: String,
        span: Span,
    },

    /// Reference to an undefined method on a struct
    UndefinedMethod {
        struct_name: String,
        method_name: String,
        span: Span,
    },

    /// Method call on non-struct type
    MethodCallOnNonStruct { method_name: String, span: Span },

    /// Duplicate definition of a name (variable, function, type, etc.)
    DuplicateDefinition {
        name: String,
        first_span: Span,
        second_span: Span,
    },

    /// Type mismatch between expected and found types
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    /// Dot-prefixed field access used outside of a with block
    NotInWithContext { span: Span },

    /// Attempting to use container field syntax on a non-container struct
    NoContainerField { struct_name: String, span: Span },

    /// Invalid use of dot prefix in this context
    InvalidDotPrefix { span: Span },
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemanticError::UndefinedVariable { name, span } => {
                write!(
                    f,
                    "Undefined variable '{}' at line {}, column {}",
                    name, span.start.line, span.start.column
                )
            }
            SemanticError::UndefinedFunction { name, span } => {
                write!(
                    f,
                    "Undefined function '{}' at line {}, column {}",
                    name, span.start.line, span.start.column
                )
            }
            SemanticError::UndefinedType { name, span } => {
                write!(
                    f,
                    "Undefined type '{}' at line {}, column {}",
                    name, span.start.line, span.start.column
                )
            }
            SemanticError::UndefinedField {
                struct_name,
                field_name,
                span,
            } => {
                write!(
                    f,
                    "Struct '{}' has no field named '{}' at line {}, column {}",
                    struct_name, field_name, span.start.line, span.start.column
                )
            }
            SemanticError::UndefinedMethod {
                struct_name,
                method_name,
                span,
            } => {
                write!(
                    f,
                    "Struct '{}' has no method named '{}' at line {}, column {}",
                    struct_name, method_name, span.start.line, span.start.column
                )
            }
            SemanticError::MethodCallOnNonStruct { method_name, span } => {
                write!(
                    f,
                    "Cannot call method '{}' on non-struct type at line {}, column {}",
                    method_name, span.start.line, span.start.column
                )
            }
            SemanticError::DuplicateDefinition {
                name,
                first_span,
                second_span,
            } => {
                write!(
                    f,
                    "Duplicate definition of '{}': first defined at line {}, column {}, \
                     redefined at line {}, column {}",
                    name,
                    first_span.start.line,
                    first_span.start.column,
                    second_span.start.line,
                    second_span.start.column
                )
            }
            SemanticError::TypeMismatch {
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
            SemanticError::NotInWithContext { span } => {
                write!(
                    f,
                    "Dot-prefixed field access '.field' can only be used inside a 'with' block \
                     at line {}, column {}",
                    span.start.line, span.start.column
                )
            }
            SemanticError::NoContainerField { struct_name, span } => {
                write!(
                    f,
                    "Struct '{}' is not declared as a container and has no implicit container field \
                     at line {}, column {}",
                    struct_name, span.start.line, span.start.column
                )
            }
            SemanticError::InvalidDotPrefix { span } => {
                write!(
                    f,
                    "Invalid dot prefix usage at line {}, column {}",
                    span.start.line, span.start.column
                )
            }
        }
    }
}

impl std::error::Error for SemanticError {}

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
    fn test_undefined_variable_display() {
        let error = SemanticError::UndefinedVariable {
            name: "foo".to_string(),
            span: make_span(10, 5),
        };
        let display = error.to_string();
        assert!(display.contains("Undefined variable 'foo'"));
        assert!(display.contains("line 10"));
        assert!(display.contains("column 5"));
    }

    #[test]
    fn test_undefined_function_display() {
        let error = SemanticError::UndefinedFunction {
            name: "calculate".to_string(),
            span: make_span(3, 12),
        };
        let display = error.to_string();
        assert!(display.contains("Undefined function 'calculate'"));
        assert!(display.contains("line 3"));
        assert!(display.contains("column 12"));
    }

    #[test]
    fn test_undefined_type_display() {
        let error = SemanticError::UndefinedType {
            name: "CustomType".to_string(),
            span: make_span(7, 20),
        };
        let display = error.to_string();
        assert!(display.contains("Undefined type 'CustomType'"));
        assert!(display.contains("line 7"));
        assert!(display.contains("column 20"));
    }

    #[test]
    fn test_undefined_field_display() {
        let error = SemanticError::UndefinedField {
            struct_name: "Point".to_string(),
            field_name: "z".to_string(),
            span: make_span(15, 8),
        };
        let display = error.to_string();
        assert!(display.contains("Struct 'Point'"));
        assert!(display.contains("has no field named 'z'"));
        assert!(display.contains("line 15"));
        assert!(display.contains("column 8"));
    }

    #[test]
    fn test_duplicate_definition_display() {
        let error = SemanticError::DuplicateDefinition {
            name: "x".to_string(),
            first_span: make_span(5, 10),
            second_span: make_span(8, 15),
        };
        let display = error.to_string();
        assert!(display.contains("Duplicate definition of 'x'"));
        assert!(display.contains("first defined at line 5, column 10"));
        assert!(display.contains("redefined at line 8, column 15"));
    }

    #[test]
    fn test_type_mismatch_display() {
        let error = SemanticError::TypeMismatch {
            expected: "f64".to_string(),
            found: "i32".to_string(),
            span: make_span(12, 25),
        };
        let display = error.to_string();
        assert!(display.contains("Type mismatch"));
        assert!(display.contains("expected 'f64'"));
        assert!(display.contains("found 'i32'"));
        assert!(display.contains("line 12"));
        assert!(display.contains("column 25"));
    }

    #[test]
    fn test_not_in_with_context_display() {
        let error = SemanticError::NotInWithContext {
            span: make_span(20, 3),
        };
        let display = error.to_string();
        assert!(display.contains("Dot-prefixed field access '.field'"));
        assert!(display.contains("can only be used inside a 'with' block"));
        assert!(display.contains("line 20"));
        assert!(display.contains("column 3"));
    }

    #[test]
    fn test_no_container_field_display() {
        let error = SemanticError::NoContainerField {
            struct_name: "Rectangle".to_string(),
            span: make_span(18, 11),
        };
        let display = error.to_string();
        assert!(display.contains("Struct 'Rectangle'"));
        assert!(display.contains("not declared as a container"));
        assert!(display.contains("has no implicit container field"));
        assert!(display.contains("line 18"));
        assert!(display.contains("column 11"));
    }

    #[test]
    fn test_invalid_dot_prefix_display() {
        let error = SemanticError::InvalidDotPrefix {
            span: make_span(9, 7),
        };
        let display = error.to_string();
        assert!(display.contains("Invalid dot prefix usage"));
        assert!(display.contains("line 9"));
        assert!(display.contains("column 7"));
    }

    #[test]
    fn test_error_trait_implementation() {
        let error = SemanticError::UndefinedVariable {
            name: "test".to_string(),
            span: make_span(1, 1),
        };
        // Ensure it can be used as a std::error::Error
        let _error_ref: &dyn std::error::Error = &error;
    }

    #[test]
    fn test_debug_trait_derived() {
        let error = SemanticError::TypeMismatch {
            expected: "bool".to_string(),
            found: "f64".to_string(),
            span: make_span(5, 5),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("TypeMismatch"));
        assert!(debug_str.contains("bool"));
        assert!(debug_str.contains("f64"));
    }

    #[test]
    fn test_clone_and_partialeq() {
        let error1 = SemanticError::UndefinedVariable {
            name: "var".to_string(),
            span: make_span(1, 1),
        };
        let error2 = error1.clone();
        assert_eq!(error1, error2);
    }
}
