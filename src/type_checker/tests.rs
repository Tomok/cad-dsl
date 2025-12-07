#[cfg(test)]
mod tests {
    use crate::ast::resolved::*;
    use crate::ast::typed::*;
    use crate::ast::unresolved::{AngleUnit, LengthUnit, LiteralKind};
    use crate::ident::IdentArena;
    use crate::span::Span;
    use crate::type_checker::{TypeChecker, TypeError, TypeErrorKind};

    #[test]
    fn test_literal_type_inference() {
        let ident_arena = IdentArena::new();
        let symbol_table = SymbolTable::new();

        let checker = TypeChecker::new(&symbol_table, &ident_arena);

        // Test integer literal
        assert_eq!(checker.infer_literal_type(&LiteralKind::Int(42)), Type::I32);

        // Test float literal
        assert_eq!(
            checker.infer_literal_type(&LiteralKind::Float(3.14)),
            Type::F64
        );

        // Test boolean literal
        assert_eq!(
            checker.infer_literal_type(&LiteralKind::Bool(true)),
            Type::Bool
        );

        // Test length literal
        assert_eq!(
            checker.infer_literal_type(&LiteralKind::Length {
                value: 10.0,
                unit: LengthUnit::Millimeter
            }),
            Type::Length
        );

        // Test angle literal
        assert_eq!(
            checker.infer_literal_type(&LiteralKind::Angle {
                value: 45.0,
                unit: AngleUnit::Degree
            }),
            Type::Angle
        );
    }

    #[test]
    fn test_type_compatibility() {
        let ident_arena = IdentArena::new();
        let symbol_table = SymbolTable::new();

        let checker = TypeChecker::new(&symbol_table, &ident_arena);
        let _span = Span::new(0, 10);

        // Compatible types
        assert!(checker.types_compatible(&Type::Length, &Type::Length));
        assert!(checker.types_compatible(&Type::Unknown, &Type::Length));
        assert!(checker.types_compatible(&Type::Length, &Type::Unknown));
        assert!(checker.types_compatible(&Type::Error, &Type::Length));

        // Incompatible types
        assert!(!checker.types_compatible(&Type::Length, &Type::Angle));
        assert!(!checker.types_compatible(&Type::Bool, &Type::I32));
    }

    #[test]
    fn test_binary_operation_types() {
        let ident_arena = IdentArena::new();
        let symbol_table = SymbolTable::new();

        let mut checker = TypeChecker::new(&symbol_table, &ident_arena);
        let span = Span::new(0, 10);

        // Length + Length = Length
        let result = checker.check_binary_op_types("+", &Type::Length, &Type::Length, span);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::Length);

        // Length * Length = Area
        let result = checker.check_binary_op_types("*", &Type::Length, &Type::Length, span);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::Area);

        // Length * F64 = Length
        let result = checker.check_binary_op_types("*", &Type::Length, &Type::F64, span);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Type::Length);

        // Invalid operation: Length + Angle should fail
        let result = checker.check_binary_op_types("+", &Type::Length, &Type::Angle, span);
        assert!(result.is_err());
    }

    #[test]
    fn test_type_to_string() {
        let _ident_arena = IdentArena::new();
        let _symbol_table = SymbolTable::new();

        assert_eq!(TypeChecker::type_to_string(&Type::Point), "Point");
        assert_eq!(TypeChecker::type_to_string(&Type::Length), "Length");
        assert_eq!(TypeChecker::type_to_string(&Type::Angle), "Angle");
        assert_eq!(TypeChecker::type_to_string(&Type::Bool), "Bool");
        assert_eq!(TypeChecker::type_to_string(&Type::Unknown), "Unknown");
        assert_eq!(TypeChecker::type_to_string(&Type::Error), "Error");

        // Test array type
        let array_type = Type::Array {
            element_type: Box::new(Type::Length),
            size: 10,
        };
        assert_eq!(
            TypeChecker::type_to_string(&array_type),
            "Array<Length, 10>"
        );

        // Test reference type
        let ref_type = Type::Reference(Box::new(Type::Point));
        assert_eq!(TypeChecker::type_to_string(&ref_type), "&Point");
    }

    #[test]
    fn test_function_call_validation() {
        let _ident_arena = IdentArena::new();
        let _symbol_table = SymbolTable::new();

        let span = Span::new(0, 10);

        // This would be called during actual function call validation
        // For now, just test the error creation
        let error = TypeError {
            kind: TypeErrorKind::ArgumentCountMismatch {
                expected: 2,
                found: 1,
            },
            span,
        };

        match error.kind {
            TypeErrorKind::ArgumentCountMismatch { expected, found } => {
                assert_eq!(expected, 2);
                assert_eq!(found, 1);
            }
            _ => panic!("Wrong error type"),
        }

        // Test argument type mismatch
        let error = TypeError {
            kind: TypeErrorKind::ArgumentTypeMismatch {
                index: 0,
                expected: "Length".to_string(),
                found: "Angle".to_string(),
            },
            span,
        };

        match error.kind {
            TypeErrorKind::ArgumentTypeMismatch {
                index,
                expected,
                found,
            } => {
                assert_eq!(index, 0);
                assert_eq!(expected, "Length");
                assert_eq!(found, "Angle");
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_resolve_builtin_types() {
        let mut ident_arena = IdentArena::new();

        // Test built-in type resolution
        let point_id = ident_arena.intern("Point");
        let length_id = ident_arena.intern("Length");
        let bool_id = ident_arena.intern("Bool");
        let unknown_id = ident_arena.intern("UnknownType");

        let symbol_table = SymbolTable::new();
        let checker = TypeChecker::new(&symbol_table, &ident_arena);

        let point_ref = ResolvedTypeRef {
            name: point_id,
            symbol_id: None,
            is_reference: false,
            array_size: None,
            span: Span::new(0, 5),
        };

        assert_eq!(checker.resolve_type_ref(&point_ref), Type::Point);

        let length_ref = ResolvedTypeRef {
            name: length_id,
            symbol_id: None,
            is_reference: false,
            array_size: None,
            span: Span::new(0, 6),
        };

        assert_eq!(checker.resolve_type_ref(&length_ref), Type::Length);

        let bool_ref = ResolvedTypeRef {
            name: bool_id,
            symbol_id: None,
            is_reference: false,
            array_size: None,
            span: Span::new(0, 4),
        };

        assert_eq!(checker.resolve_type_ref(&bool_ref), Type::Bool);

        let unknown_ref = ResolvedTypeRef {
            name: unknown_id,
            symbol_id: None,
            is_reference: false,
            array_size: None,
            span: Span::new(0, 11),
        };

        assert_eq!(checker.resolve_type_ref(&unknown_ref), Type::Unknown);
    }

    #[test]
    fn test_reference_types() {
        let mut ident_arena = IdentArena::new();
        let point_id = ident_arena.intern("Point");

        let symbol_table = SymbolTable::new();
        let checker = TypeChecker::new(&symbol_table, &ident_arena);

        // Test reference type resolution
        let ref_point = ResolvedTypeRef {
            name: point_id,
            symbol_id: None,
            is_reference: true,
            array_size: None,
            span: Span::new(0, 6),
        };

        let expected = Type::Reference(Box::new(Type::Point));
        assert_eq!(checker.resolve_type_ref(&ref_point), expected);
    }

    #[test]
    fn test_entity_type_classification() {
        assert!(Type::Point.is_entity_type());
        assert!(Type::Length.is_entity_type());
        assert!(Type::Angle.is_entity_type());
        assert!(Type::Area.is_entity_type());

        assert!(!Type::I32.is_entity_type());
        assert!(!Type::F64.is_entity_type());
        assert!(!Type::Bool.is_entity_type());
        assert!(!Type::Unknown.is_entity_type());
    }

    #[test]
    fn test_numeric_type_classification() {
        assert!(Type::Length.is_numeric());
        assert!(Type::Angle.is_numeric());
        assert!(Type::Area.is_numeric());
        assert!(Type::I32.is_numeric());
        assert!(Type::F64.is_numeric());
        assert!(Type::Real.is_numeric());
        assert!(Type::Algebraic.is_numeric());

        assert!(!Type::Point.is_numeric());
        assert!(!Type::Bool.is_numeric());
        assert!(!Type::View.is_numeric());
        assert!(!Type::Unknown.is_numeric());
    }

    #[test]
    fn test_error_creation() {
        let span = Span::new(0, 10);

        let type_error = TypeError::new(
            TypeErrorKind::TypeMismatch {
                expected: "Length".to_string(),
                found: "Angle".to_string(),
            },
            span,
        );

        assert_eq!(type_error.span, span);
        match type_error.kind {
            TypeErrorKind::TypeMismatch { expected, found } => {
                assert_eq!(expected, "Length");
                assert_eq!(found, "Angle");
            }
            _ => panic!("Expected TypeMismatch error"),
        }
    }
}
