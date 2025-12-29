use crate::lexer::Span;
use crate::type_checker_errors::TypeCheckError;
use bumpalo::Bump;

/// A unique identifier for a type variable in the type inference system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub struct TypeId(usize);

#[allow(dead_code)]
impl TypeId {
    /// Create a new TypeId from a usize
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Get the underlying usize value
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

/// Type constraints collected during type checking
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum TypeConstraint {
    /// Two types must be equal
    Equal {
        lhs: TypeId,
        rhs: TypeId,
        span: Span,
    },
    /// Types must be compatible (e.g., for implicit conversions)
    Compatible {
        lhs: TypeId,
        rhs: TypeId,
        span: Span,
    },
}

/// Context for type checking, including arena allocator, source code, and error collection
#[allow(dead_code)]
pub struct TypeCheckContext<'src, 'arena> {
    /// Arena allocator for type checking data structures
    arena: &'arena Bump,
    /// Source code being type checked
    source: &'src str,
    /// Collected type constraints
    type_constraints: Vec<TypeConstraint>,
    /// Collected type checking errors
    errors: Vec<TypeCheckError>,
}

#[allow(dead_code)]
impl<'src, 'arena> TypeCheckContext<'src, 'arena> {
    /// Create a new type checking context
    pub fn new(arena: &'arena Bump, source: &'src str) -> Self {
        Self {
            arena,
            source,
            type_constraints: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add a type constraint to the context
    pub fn add_constraint(&mut self, constraint: TypeConstraint) {
        self.type_constraints.push(constraint);
    }

    /// Add a type checking error to the context
    pub fn add_error(&mut self, error: TypeCheckError) {
        self.errors.push(error);
    }

    /// Check if any errors have been collected
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Take all collected errors, leaving the error list empty
    pub fn take_errors(&mut self) -> Vec<TypeCheckError> {
        std::mem::take(&mut self.errors)
    }
}

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
    fn test_type_id_creation() {
        let id1 = TypeId::new(0);
        let id2 = TypeId::new(1);
        let id3 = TypeId::new(0);

        assert_eq!(id1.as_usize(), 0);
        assert_eq!(id2.as_usize(), 1);
        assert_eq!(id1, id3);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_type_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();

        let id1 = TypeId::new(1);
        let id2 = TypeId::new(2);
        let id3 = TypeId::new(1);

        set.insert(id1);
        set.insert(id2);
        set.insert(id3);

        // id1 and id3 are equal, so only 2 unique elements
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_type_id_debug() {
        let id = TypeId::new(42);
        let debug_str = format!("{:?}", id);
        assert!(debug_str.contains("TypeId"));
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn test_type_constraint_equal() {
        let id1 = TypeId::new(0);
        let id2 = TypeId::new(1);
        let span = make_span(10, 5);

        let constraint = TypeConstraint::Equal {
            lhs: id1,
            rhs: id2,
            span,
        };

        match constraint {
            TypeConstraint::Equal { lhs, rhs, span: s } => {
                assert_eq!(lhs, id1);
                assert_eq!(rhs, id2);
                assert_eq!(s, span);
            }
            _ => panic!("Expected Equal constraint"),
        }
    }

    #[test]
    fn test_type_constraint_compatible() {
        let id1 = TypeId::new(5);
        let id2 = TypeId::new(7);
        let span = make_span(20, 15);

        let constraint = TypeConstraint::Compatible {
            lhs: id1,
            rhs: id2,
            span,
        };

        match constraint {
            TypeConstraint::Compatible { lhs, rhs, span: s } => {
                assert_eq!(lhs, id1);
                assert_eq!(rhs, id2);
                assert_eq!(s, span);
            }
            _ => panic!("Expected Compatible constraint"),
        }
    }

    #[test]
    fn test_type_constraint_clone() {
        let constraint1 = TypeConstraint::Equal {
            lhs: TypeId::new(0),
            rhs: TypeId::new(1),
            span: make_span(1, 1),
        };
        let constraint2 = constraint1.clone();

        match (constraint1, constraint2) {
            (
                TypeConstraint::Equal {
                    lhs: lhs1,
                    rhs: rhs1,
                    ..
                },
                TypeConstraint::Equal {
                    lhs: lhs2,
                    rhs: rhs2,
                    ..
                },
            ) => {
                assert_eq!(lhs1, lhs2);
                assert_eq!(rhs1, rhs2);
            }
            _ => panic!("Expected Equal constraints"),
        }
    }

    #[test]
    fn test_context_creation() {
        let arena = Bump::new();
        let source = "let x = 5;";
        let ctx = TypeCheckContext::new(&arena, source);

        assert!(!ctx.has_errors());
        assert_eq!(ctx.type_constraints.len(), 0);
        assert_eq!(ctx.errors.len(), 0);
    }

    #[test]
    fn test_add_constraint() {
        let arena = Bump::new();
        let source = "let x = 5;";
        let mut ctx = TypeCheckContext::new(&arena, source);

        assert_eq!(ctx.type_constraints.len(), 0);

        let constraint = TypeConstraint::Equal {
            lhs: TypeId::new(0),
            rhs: TypeId::new(1),
            span: make_span(1, 1),
        };
        ctx.add_constraint(constraint);

        assert_eq!(ctx.type_constraints.len(), 1);
    }

    #[test]
    fn test_add_multiple_constraints() {
        let arena = Bump::new();
        let source = "let x = 5;";
        let mut ctx = TypeCheckContext::new(&arena, source);

        ctx.add_constraint(TypeConstraint::Equal {
            lhs: TypeId::new(0),
            rhs: TypeId::new(1),
            span: make_span(1, 1),
        });

        ctx.add_constraint(TypeConstraint::Compatible {
            lhs: TypeId::new(2),
            rhs: TypeId::new(3),
            span: make_span(2, 2),
        });

        ctx.add_constraint(TypeConstraint::Equal {
            lhs: TypeId::new(4),
            rhs: TypeId::new(5),
            span: make_span(3, 3),
        });

        assert_eq!(ctx.type_constraints.len(), 3);
    }

    #[test]
    fn test_add_error() {
        let arena = Bump::new();
        let source = "let x = 5;";
        let mut ctx = TypeCheckContext::new(&arena, source);

        assert!(!ctx.has_errors());

        let error = TypeCheckError::TypeMismatch {
            expected: "f64".to_string(),
            found: "i32".to_string(),
            span: make_span(10, 5),
        };
        ctx.add_error(error);

        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 1);
    }

    #[test]
    fn test_add_multiple_errors() {
        let arena = Bump::new();
        let source = "let x = 5;";
        let mut ctx = TypeCheckContext::new(&arena, source);

        ctx.add_error(TypeCheckError::TypeMismatch {
            expected: "f64".to_string(),
            found: "i32".to_string(),
            span: make_span(1, 1),
        });

        ctx.add_error(TypeCheckError::CannotInferType {
            expr_kind: "array".to_string(),
            span: make_span(2, 2),
        });

        assert!(ctx.has_errors());
        assert_eq!(ctx.errors.len(), 2);
    }

    #[test]
    fn test_has_errors_empty() {
        let arena = Bump::new();
        let source = "let x = 5;";
        let ctx = TypeCheckContext::new(&arena, source);

        assert!(!ctx.has_errors());
    }

    #[test]
    fn test_has_errors_with_errors() {
        let arena = Bump::new();
        let source = "let x = 5;";
        let mut ctx = TypeCheckContext::new(&arena, source);

        ctx.add_error(TypeCheckError::NonBooleanCondition {
            found_type: "i32".to_string(),
            span: make_span(1, 1),
        });

        assert!(ctx.has_errors());
    }

    #[test]
    fn test_take_errors() {
        let arena = Bump::new();
        let source = "let x = 5;";
        let mut ctx = TypeCheckContext::new(&arena, source);

        let error1 = TypeCheckError::TypeMismatch {
            expected: "f64".to_string(),
            found: "i32".to_string(),
            span: make_span(1, 1),
        };
        let error2 = TypeCheckError::CannotInferType {
            expr_kind: "array".to_string(),
            span: make_span(2, 2),
        };

        ctx.add_error(error1);
        ctx.add_error(error2);

        assert_eq!(ctx.errors.len(), 2);

        let errors = ctx.take_errors();

        assert_eq!(errors.len(), 2);
        assert!(!ctx.has_errors());
        assert_eq!(ctx.errors.len(), 0);
    }

    #[test]
    fn test_take_errors_empties_list() {
        let arena = Bump::new();
        let source = "let x = 5;";
        let mut ctx = TypeCheckContext::new(&arena, source);

        ctx.add_error(TypeCheckError::TypeMismatch {
            expected: "bool".to_string(),
            found: "String".to_string(),
            span: make_span(5, 10),
        });

        assert!(ctx.has_errors());

        let _errors = ctx.take_errors();

        // After taking errors, the list should be empty
        assert!(!ctx.has_errors());
        assert_eq!(ctx.errors.len(), 0);

        // Taking again should return an empty vector
        let errors2 = ctx.take_errors();
        assert_eq!(errors2.len(), 0);
    }

    #[test]
    fn test_context_lifetimes() {
        let arena = Bump::new();
        let source = String::from("let x = 5;");
        let ctx = TypeCheckContext::new(&arena, &source);

        // Verify that we can access the source through the context
        assert_eq!(ctx.source, "let x = 5;");
    }

    #[test]
    fn test_constraint_and_error_together() {
        let arena = Bump::new();
        let source = "let x = 5;";
        let mut ctx = TypeCheckContext::new(&arena, source);

        // Add both constraints and errors
        ctx.add_constraint(TypeConstraint::Equal {
            lhs: TypeId::new(0),
            rhs: TypeId::new(1),
            span: make_span(1, 1),
        });

        ctx.add_error(TypeCheckError::TypeMismatch {
            expected: "f64".to_string(),
            found: "i32".to_string(),
            span: make_span(1, 5),
        });

        ctx.add_constraint(TypeConstraint::Compatible {
            lhs: TypeId::new(2),
            rhs: TypeId::new(3),
            span: make_span(2, 1),
        });

        assert_eq!(ctx.type_constraints.len(), 2);
        assert_eq!(ctx.errors.len(), 1);
        assert!(ctx.has_errors());
    }
}
