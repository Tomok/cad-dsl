//! Struct and Array Field Flattening for Z3 Variable Mapping
//!
//! This module flattens struct and array types into their constituent primitive fields,
//! generating qualified variable names for each primitive field.
//!
//! # Purpose
//!
//! The constraint solver represents struct and array variables as multiple Z3 variables,
//! one for each primitive field. Nested structs and arrays are recursively flattened:
//!
//! ```text
//! struct Point { x: i32, y: i32 }
//! struct Line { start: Point, end: Point }
//!
//! let line: Line;
//!
//! // Flattened to 4 Z3 variables:
//! // "line.start.x": Int
//! // "line.start.y": Int
//! // "line.end.x": Int
//! // "line.end.y": Int
//!
//! let points: [Point; 3];
//!
//! // Flattened to 6 Z3 variables:
//! // "points[0].x": Int
//! // "points[0].y": Int
//! // "points[1].x": Int
//! // "points[1].y": Int
//! // "points[2].x": Int
//! // "points[2].y": Int
//! ```
//!
//! # Algorithm
//!
//! The flattening process recursively walks the type structure:
//! 1. Start with a variable name and type
//! 2. If type is primitive → yield (name, type)
//! 3. If type is struct → recurse on each field with name "prefix.field_name"
//! 4. If type is array → recurse on each element with name "prefix[index]"
//! 5. If type is reference → unwrap and continue
//!
//! # Reference Types
//!
//! Reference types (`&T`) are transparent for flattening - we look through
//! the reference to the underlying type. This matches the solver's semantics
//! where references are dereferenced for constraint solving.

use crate::hir::types::ResolvedType;
use crate::lexer::Span;

// ============================================================================
// Flattened Field Representation
// ============================================================================

/// A flattened primitive field from a (potentially nested) struct
#[derive(Debug, Clone, PartialEq)]
pub struct FlattenedField<'src, 'arena> {
    /// Fully qualified field name (e.g., "line.start.x")
    pub full_name: String,

    /// The primitive type of this field
    pub primitive_type: ResolvedType<'src, 'arena>,

    /// Span of the original type declaration (for error reporting)
    pub span: Span,
}

impl<'src, 'arena> FlattenedField<'src, 'arena> {
    /// Create a new flattened field
    pub fn new(full_name: String, primitive_type: ResolvedType<'src, 'arena>, span: Span) -> Self {
        Self {
            full_name,
            primitive_type,
            span,
        }
    }
}

// ============================================================================
// Flattening Functions
// ============================================================================

/// Flatten a type into its primitive fields with qualified names
///
/// Returns a vector of flattened fields, each with a fully qualified name
/// and primitive type.
///
/// # Arguments
///
/// * `name_prefix` - The prefix for field names (e.g., "point" or "line.start")
/// * `resolved_type` - The type to flatten
///
/// # Examples
///
/// ```text
/// // Simple struct:
/// flatten_type("p", Point { x: i32, y: i32 })
/// // Returns: [("p.x", i32), ("p.y", i32)]
///
/// // Nested struct:
/// flatten_type("line", Line { start: Point, end: Point })
/// // Returns: [("line.start.x", i32), ("line.start.y", i32),
/// //           ("line.end.x", i32), ("line.end.y", i32)]
/// ```
pub fn flatten_type<'src, 'arena>(
    name_prefix: &str,
    resolved_type: ResolvedType<'src, 'arena>,
) -> Vec<FlattenedField<'src, 'arena>> {
    let mut fields = Vec::new();
    flatten_type_recursive(name_prefix, resolved_type, &mut fields);
    fields
}

/// Recursively flatten a type, accumulating results in the fields vector
fn flatten_type_recursive<'src, 'arena>(
    name_prefix: &str,
    resolved_type: ResolvedType<'src, 'arena>,
    fields: &mut Vec<FlattenedField<'src, 'arena>>,
) {
    match resolved_type {
        // Primitive types - base case, add to results
        ResolvedType::Bool { span }
        | ResolvedType::I32 { span }
        | ResolvedType::F64 { span }
        | ResolvedType::Real { span }
        | ResolvedType::Algebraic { span } => {
            fields.push(FlattenedField::new(
                name_prefix.to_string(),
                resolved_type,
                span,
            ));
        }

        // Reference type - look through to inner type
        ResolvedType::Reference { inner, .. } => {
            flatten_type_recursive(name_prefix, *inner, fields);
        }

        // User-defined type - recursively flatten fields
        ResolvedType::UserDefined { definition, .. } => {
            for &field in &definition.fields {
                let qualified_name = if name_prefix.is_empty() {
                    field.name.to_string()
                } else {
                    format!("{}.{}", name_prefix, field.name)
                };

                flatten_type_recursive(&qualified_name, field.field_type, fields);
            }
        }

        // Array type - flatten each element with index notation
        // Example: points[0], points[1], points[2], etc.
        ResolvedType::Array {
            element_type,
            size,
            span: _,
        } => {
            for i in 0..size {
                let indexed_name = format!("{}[{}]", name_prefix, i);
                flatten_type_recursive(&indexed_name, *element_type, fields);
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::definitions::{FieldDefinition, StructDefinition};
    use crate::lexer::{LineColumn, Span};
    use bumpalo::Bump;

    fn test_span() -> Span {
        Span {
            start: LineColumn { line: 1, column: 1 },
            lines: 0,
            end_column: 10,
        }
    }

    #[test]
    fn test_flatten_primitive_type() {
        // Flattening a primitive type returns single field with the same name
        let result = flatten_type("x", ResolvedType::I32 { span: test_span() });

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "x");
        assert!(matches!(result[0].primitive_type, ResolvedType::I32 { .. }));
    }

    #[test]
    fn test_flatten_simple_struct() {
        let arena = Bump::new();

        // struct Point { x: i32, y: i32 }
        let point = arena.alloc(StructDefinition::new(
            "Point",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new(
                    "x",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "y",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
            ],
            vec![],
            None,
            test_span(),
        ));

        let result = flatten_type(
            "p",
            ResolvedType::UserDefined {
                name: "Point",
                definition: point,
                span: test_span(),
            },
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].full_name, "p.x");
        assert!(matches!(result[0].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[1].full_name, "p.y");
        assert!(matches!(result[1].primitive_type, ResolvedType::I32 { .. }));
    }

    #[test]
    fn test_flatten_nested_struct() {
        let arena = Bump::new();

        // struct Point { x: i32, y: i32 }
        let point = arena.alloc(StructDefinition::new(
            "Point",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new(
                    "x",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "y",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
            ],
            vec![],
            None,
            test_span(),
        ));

        // struct Line { start: Point, end: Point }
        let line = StructDefinition::new(
            "Line",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new(
                    "start",
                    test_span(),
                    ResolvedType::UserDefined {
                        name: "Point",
                        definition: point,
                        span: test_span(),
                    },
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "end",
                    test_span(),
                    ResolvedType::UserDefined {
                        name: "Point",
                        definition: point,
                        span: test_span(),
                    },
                    test_span(),
                )),
            ],
            vec![],
            None,
            test_span(),
        );

        let result = flatten_type(
            "line",
            ResolvedType::UserDefined {
                name: "Line",
                definition: &line,
                span: test_span(),
            },
        );

        assert_eq!(result.len(), 4);
        assert_eq!(result[0].full_name, "line.start.x");
        assert!(matches!(result[0].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[1].full_name, "line.start.y");
        assert!(matches!(result[1].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[2].full_name, "line.end.x");
        assert!(matches!(result[2].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[3].full_name, "line.end.y");
        assert!(matches!(result[3].primitive_type, ResolvedType::I32 { .. }));
    }

    #[test]
    fn test_flatten_mixed_types() {
        let arena = Bump::new();

        // struct Mixed { a: bool, b: i32, c: f64 }
        let mixed = StructDefinition::new(
            "Mixed",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new(
                    "a",
                    test_span(),
                    ResolvedType::Bool { span: test_span() },
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "b",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "c",
                    test_span(),
                    ResolvedType::F64 { span: test_span() },
                    test_span(),
                )),
            ],
            vec![],
            None,
            test_span(),
        );

        let result = flatten_type(
            "m",
            ResolvedType::UserDefined {
                name: "Mixed",
                definition: &mixed,
                span: test_span(),
            },
        );

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].full_name, "m.a");
        assert!(matches!(
            result[0].primitive_type,
            ResolvedType::Bool { .. }
        ));
        assert_eq!(result[1].full_name, "m.b");
        assert!(matches!(result[1].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[2].full_name, "m.c");
        assert!(matches!(result[2].primitive_type, ResolvedType::F64 { .. }));
    }

    #[test]
    fn test_flatten_reference_type() {
        let arena = Bump::new();

        // &i32 should flatten to same as i32
        let inner_type = arena.alloc(ResolvedType::I32 { span: test_span() });
        let ref_type = ResolvedType::Reference {
            inner: inner_type,
            span: test_span(),
        };

        let result = flatten_type("r", ref_type);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "r");
        assert!(matches!(result[0].primitive_type, ResolvedType::I32 { .. }));
    }

    #[test]
    fn test_flatten_reference_to_struct() {
        let arena = Bump::new();

        // struct Point { x: i32, y: i32 }
        let point = arena.alloc(StructDefinition::new(
            "Point",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new(
                    "x",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "y",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
            ],
            vec![],
            None,
            test_span(),
        ));

        // &Point
        let point_type = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: point,
            span: test_span(),
        });
        let ref_type = ResolvedType::Reference {
            inner: point_type,
            span: test_span(),
        };

        let result = flatten_type("p", ref_type);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].full_name, "p.x");
        assert!(matches!(result[0].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[1].full_name, "p.y");
        assert!(matches!(result[1].primitive_type, ResolvedType::I32 { .. }));
    }

    #[test]
    fn test_flatten_deeply_nested() {
        let arena = Bump::new();

        // struct A { value: i32 }
        let a = arena.alloc(StructDefinition::new(
            "A",
            test_span(),
            vec![arena.alloc(FieldDefinition::new(
                "value",
                test_span(),
                ResolvedType::I32 { span: test_span() },
                test_span(),
            ))],
            vec![],
            None,
            test_span(),
        ));

        // struct B { a: A }
        let b = arena.alloc(StructDefinition::new(
            "B",
            test_span(),
            vec![arena.alloc(FieldDefinition::new(
                "a",
                test_span(),
                ResolvedType::UserDefined {
                    name: "A",
                    definition: a,
                    span: test_span(),
                },
                test_span(),
            ))],
            vec![],
            None,
            test_span(),
        ));

        // struct C { b: B }
        let c = StructDefinition::new(
            "C",
            test_span(),
            vec![arena.alloc(FieldDefinition::new(
                "b",
                test_span(),
                ResolvedType::UserDefined {
                    name: "B",
                    definition: b,
                    span: test_span(),
                },
                test_span(),
            ))],
            vec![],
            None,
            test_span(),
        );

        let result = flatten_type(
            "c",
            ResolvedType::UserDefined {
                name: "C",
                definition: &c,
                span: test_span(),
            },
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].full_name, "c.b.a.value");
        assert!(matches!(result[0].primitive_type, ResolvedType::I32 { .. }));
    }

    #[test]
    fn test_flatten_empty_prefix() {
        // Test flattening with empty prefix (for struct literals)
        let arena = Bump::new();

        let point = arena.alloc(StructDefinition::new(
            "Point",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new(
                    "x",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "y",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
            ],
            vec![],
            None,
            test_span(),
        ));

        let result = flatten_type(
            "",
            ResolvedType::UserDefined {
                name: "Point",
                definition: point,
                span: test_span(),
            },
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].full_name, "x");
        assert_eq!(result[1].full_name, "y");
    }

    #[test]
    fn test_flatten_array_of_primitives() {
        let arena = Bump::new();

        // [i32; 3]
        let element_type = arena.alloc(ResolvedType::I32 { span: test_span() });
        let array_type = ResolvedType::Array {
            element_type,
            size: 3,
            span: test_span(),
        };

        let result = flatten_type("nums", array_type);

        assert_eq!(result.len(), 3);
        assert_eq!(result[0].full_name, "nums[0]");
        assert!(matches!(result[0].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[1].full_name, "nums[1]");
        assert!(matches!(result[1].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[2].full_name, "nums[2]");
        assert!(matches!(result[2].primitive_type, ResolvedType::I32 { .. }));
    }

    #[test]
    fn test_flatten_array_of_structs() {
        let arena = Bump::new();

        // struct Point { x: i32, y: i32 }
        let point = arena.alloc(StructDefinition::new(
            "Point",
            test_span(),
            vec![
                arena.alloc(FieldDefinition::new(
                    "x",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
                arena.alloc(FieldDefinition::new(
                    "y",
                    test_span(),
                    ResolvedType::I32 { span: test_span() },
                    test_span(),
                )),
            ],
            vec![],
            None,
            test_span(),
        ));

        // [Point; 2]
        let point_type = arena.alloc(ResolvedType::UserDefined {
            name: "Point",
            definition: point,
            span: test_span(),
        });
        let array_type = ResolvedType::Array {
            element_type: point_type,
            size: 2,
            span: test_span(),
        };

        let result = flatten_type("points", array_type);

        assert_eq!(result.len(), 4);
        assert_eq!(result[0].full_name, "points[0].x");
        assert!(matches!(result[0].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[1].full_name, "points[0].y");
        assert!(matches!(result[1].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[2].full_name, "points[1].x");
        assert!(matches!(result[2].primitive_type, ResolvedType::I32 { .. }));
        assert_eq!(result[3].full_name, "points[1].y");
        assert!(matches!(result[3].primitive_type, ResolvedType::I32 { .. }));
    }
}
