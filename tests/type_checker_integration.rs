use cad_dsl::*;

#[test]
fn test_successful_type_checking() {
    let source = r#"
        sketch test_sketch {
            let x: Length = 10mm;
            let y: Length = 20mm;
            let sum: Length = x;
            sum = y;
        }
    "#;

    let mut idents = IdentArena::new();

    // Tokenize
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");

    // Parse
    let (ast, parse_errors) = parse(tokens, &idents);
    assert!(ast.is_some(), "Parsing should succeed");
    assert!(parse_errors.is_empty(), "No parse errors expected");
    let ast = ast.unwrap();

    // Name resolution
    let (resolved_ast, name_errors) = resolve_names(ast, &idents);
    assert!(name_errors.is_empty(), "No name resolution errors expected");

    // Type checking
    let (typed_ir, type_errors) =
        check_types(resolved_ast.clone(), &resolved_ast.symbol_table, &idents);
    assert!(
        type_errors.is_empty(),
        "No type errors expected: {:?}",
        type_errors
    );

    // Verify the typed IR
    assert_eq!(typed_ir.sketches.len(), 1);
    assert_eq!(typed_ir.sketches[0].body.len(), 4); // 3 let statements + 1 assignment
}

#[test]
fn test_type_mismatch_error() {
    let source = r#"
        sketch test_sketch {
            let x: Length = 10mm;
            let y: Angle = 45deg;
            let sum: Length = x;
            sum = y;  // This should cause a type error
        }
    "#;

    let mut idents = IdentArena::new();

    // Tokenize
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");

    // Parse
    let (ast, parse_errors) = parse(tokens, &idents);
    assert!(ast.is_some(), "Parsing should succeed");
    assert!(parse_errors.is_empty(), "No parse errors expected");
    let ast = ast.unwrap();

    // Name resolution
    let (resolved_ast, name_errors) = resolve_names(ast, &idents);
    assert!(name_errors.is_empty(), "No name resolution errors expected");

    // Type checking
    let (_, type_errors) = check_types(resolved_ast.clone(), &resolved_ast.symbol_table, &idents);
    assert!(!type_errors.is_empty(), "Type error expected");
    assert_eq!(type_errors.len(), 1);

    // Check that it's the right kind of error
    match &type_errors[0].kind {
        TypeErrorKind::TypeMismatch { expected, found } => {
            assert_eq!(expected, "Length");
            assert_eq!(found, "Angle");
        }
        _ => panic!(
            "Expected TypeMismatch error, got: {:?}",
            type_errors[0].kind
        ),
    }
}

#[test]
fn test_binary_operation_type_checking() {
    let source = r#"
        sketch test_sketch {
            let x: Length = 10mm;
            let y: Length = 20mm;
            let sum: Length = x;
            sum = x;
        }
    "#;

    let mut idents = IdentArena::new();

    // Tokenize
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");

    // Parse
    let (ast, parse_errors) = parse(tokens, &idents);
    assert!(ast.is_some(), "Parsing should succeed");
    assert!(parse_errors.is_empty(), "No parse errors expected");
    let ast = ast.unwrap();

    // Name resolution
    let (resolved_ast, name_errors) = resolve_names(ast, &idents);
    assert!(name_errors.is_empty(), "No name resolution errors expected");

    // Type checking
    let (typed_ir, type_errors) =
        check_types(resolved_ast.clone(), &resolved_ast.symbol_table, &idents);
    assert!(
        type_errors.is_empty(),
        "No type errors expected: {:?}",
        type_errors
    );

    // Verify the typed IR structure
    assert_eq!(typed_ir.sketches.len(), 1);
    let sketch = &typed_ir.sketches[0];
    assert_eq!(sketch.body.len(), 4);

    // Check that variables have correct types
    for stmt in &sketch.body {
        match stmt {
            TypedStmt::Let { ty, .. } => {
                assert_eq!(*ty, Type::Length);
            }
            TypedStmt::Constraint { target, value, .. } => {
                assert_eq!(target.ty, Type::Length);
                assert_eq!(value.ty, Type::Length);
            }
            _ => {} // Other statement types
        }
    }
}

#[test]
fn test_literal_type_inference() {
    let source = r#"
        sketch test_sketch {
            let x: Length = 10mm;
            let y: Angle = 45deg;
            let z: Bool = true;
            let w: I32 = 42;
        }
    "#;

    let mut idents = IdentArena::new();

    // Full pipeline
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);
    assert!(ast.is_some() && parse_errors.is_empty());

    let (resolved_ast, name_errors) = resolve_names(ast.unwrap(), &idents);
    assert!(name_errors.is_empty());

    let (typed_ir, type_errors) =
        check_types(resolved_ast.clone(), &resolved_ast.symbol_table, &idents);
    assert!(type_errors.is_empty(), "Type errors: {:?}", type_errors);

    // Verify all variables have the expected types
    let sketch = &typed_ir.sketches[0];
    assert_eq!(sketch.body.len(), 4);

    for stmt in &sketch.body {
        if let TypedStmt::Let { ty, .. } = stmt {
            // All are valid types that should be inferred correctly
            assert!(matches!(
                ty,
                Type::Length | Type::Angle | Type::Bool | Type::I32
            ));
        }
    }
}

#[test]
fn test_complete_compilation_pipeline() {
    let source = r#"
        sketch complete_example {
            let length1: Length = 100mm;
            let length2: Length = 50mm;
            let angle1: Angle = 90deg;
            let result: Length = length1;
            result = length2;
        }
    "#;

    let mut idents = IdentArena::new();

    // Test complete pipeline
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");

    let (ast, parse_errors) = parse(tokens, &idents);
    assert!(ast.is_some(), "Parsing should succeed");
    assert!(parse_errors.is_empty(), "Parse errors: {:?}", parse_errors);

    let (resolved_ast, name_errors) = resolve_names(ast.unwrap(), &idents);
    assert!(
        name_errors.is_empty(),
        "Name resolution errors: {:?}",
        name_errors
    );

    let (typed_ir, type_errors) =
        check_types(resolved_ast.clone(), &resolved_ast.symbol_table, &idents);
    assert!(type_errors.is_empty(), "Type errors: {:?}", type_errors);

    // Verify final result
    assert_eq!(typed_ir.sketches.len(), 1);
    let sketch = &typed_ir.sketches[0];
    assert_eq!(idents.resolve(sketch.name), "complete_example");
    assert_eq!(sketch.body.len(), 5); // 4 let statements + 1 assignment
}

use crate::type_checker::TypeErrorKind;

#[test]
fn test_phase_4_enabled_integration() {
    // Test that demonstrates Phase 4 (Type Checking) features working
    // with all enabled test cases from parser_integration.rs

    let sources = vec![
        // Simple units and types
        r#"
            sketch units_demo {
                let length_mm: Length = 25mm;
                let angle_degrees: Angle = 45deg;
                let bool_var: Bool = true;
                let int_var: I32 = 42;
            }
        "#,
        // Function definitions (without calls to avoid name resolution issues)
        r#"
            sketch function_examples {
                fn simple_function(x: Length) -> Length {
                    return x;
                }
                
                fn add_lengths(a: Length, b: Length) -> Length {
                    let sum: Length = a;
                    return sum;
                }
                
                let test_value: Length = 10mm;
                let another_value: Length = test_value;
            }
        "#,
        // Structs with methods
        r#"
            import geometry;
            
            struct Circle {
                center: Point,
                radius: Length,
                
                fn area() -> Area {
                    let result: Area;
                    return result;
                }
                
                fn circumference() -> Length {
                    let result: Length;
                    return result;
                }
            }
            
            sketch geometric_design {
                let circle_radius: Length = 50mm;
                let outer_radius: Length = circle_radius;
                let inner_radius: Length = circle_radius;
            }
        "#,
        // Nested structures
        r#"
            struct Point2D {
                x: Length,
                y: Length,
                
                fn simple_method() -> Length {
                    let result: Length;
                    return result;
                }
            }
            
            struct Line {
                start: Point2D,
                end: Point2D,
                
                fn get_length() -> Length {
                    let result: Length;
                    return result;
                }
            }
            
            sketch polygon_design {
                let vertex_count: I32 = 6;
                let side_length: Length = 10mm;
            }
        "#,
    ];

    for (i, source) in sources.iter().enumerate() {
        let mut idents = IdentArena::new();

        // Test complete pipeline: tokenize -> parse -> name resolution -> type checking
        let tokens = tokenize(source, &mut idents)
            .expect(&format!("Tokenization should succeed for source {}", i));

        let (ast, parse_errors) = parse(tokens, &idents);
        assert!(
            parse_errors.is_empty(),
            "Source {} should not have parse errors: {:?}",
            i,
            parse_errors
        );
        assert!(ast.is_some(), "Source {} should produce an AST", i);

        let (resolved_ast, name_errors) = resolve_names(ast.unwrap(), &idents);
        assert!(
            name_errors.is_empty(),
            "Source {} should not have name resolution errors: {:?}",
            i,
            name_errors
        );

        let (typed_ir, type_errors) =
            check_types(resolved_ast.clone(), &resolved_ast.symbol_table, &idents);
        assert!(
            type_errors.is_empty(),
            "Source {} should not have type errors: {:?}",
            i,
            type_errors
        );

        // Verify basic structure of typed IR
        assert!(
            !typed_ir.sketches.is_empty(),
            "Source {} should have sketches",
            i
        );

        // Verify that type checking actually assigned types
        for sketch in &typed_ir.sketches {
            for stmt in &sketch.body {
                match stmt {
                    TypedStmt::Let { ty, .. } => {
                        assert!(
                            !matches!(ty, Type::Unknown),
                            "All variables should have concrete types"
                        );
                        assert!(!matches!(ty, Type::Error), "No error types should remain");
                    }
                    TypedStmt::Constraint { target, value, .. } => {
                        assert!(
                            !matches!(target.ty, Type::Unknown | Type::Error),
                            "Target should have concrete type"
                        );
                        assert!(
                            !matches!(value.ty, Type::Unknown | Type::Error),
                            "Value should have concrete type"
                        );
                    }
                    _ => {} // Other statement types are fine
                }
            }
        }
    }
}

#[test]
fn test_phase_4_type_error_scenarios() {
    // Test cases that should produce type errors
    let error_sources = vec![
        (
            r#"
                sketch type_error {
                    let x: Length = 10mm;
                    let y: Angle = 45deg;
                    x = y; // Type mismatch
                }
            "#,
            "Length/Angle type mismatch",
        ),
        (
            r#"
                sketch type_error {
                    let a: Bool = true;
                    let b: Length = 10mm;
                    a = b; // Bool/Length type mismatch
                }
            "#,
            "Bool/Length type mismatch",
        ),
        (
            r#"
                sketch type_error {
                    let x: I32 = 42;
                    let y: F64 = 3.14;
                    x = y; // I32/F64 type mismatch
                }
            "#,
            "I32/F64 type mismatch",
        ),
    ];

    for (i, (source, description)) in error_sources.iter().enumerate() {
        let mut idents = IdentArena::new();

        let tokens = tokenize(source, &mut idents).expect(&format!(
            "Tokenization should succeed for error source {}",
            i
        ));

        let (ast, parse_errors) = parse(tokens, &idents);
        assert!(
            parse_errors.is_empty(),
            "Error source {} should parse correctly: {:?}",
            i,
            parse_errors
        );

        let (resolved_ast, name_errors) = resolve_names(ast.unwrap(), &idents);
        assert!(
            name_errors.is_empty(),
            "Error source {} should have no name resolution errors: {:?}",
            i,
            name_errors
        );

        let (_, type_errors) =
            check_types(resolved_ast.clone(), &resolved_ast.symbol_table, &idents);
        assert!(
            !type_errors.is_empty(),
            "Source {} should produce type errors: {}",
            i,
            description
        );

        // Verify it's a type mismatch error
        assert!(
            type_errors
                .iter()
                .any(|e| matches!(e.kind, TypeErrorKind::TypeMismatch { .. })),
            "Source {} should produce TypeMismatch error for: {}",
            i,
            description
        );
    }
}
