// Phase 5.2: Integration tests for complete parsing pipeline
// These tests verify end-to-end parsing with real TextCAD code examples

use cad_dsl::{IdentArena, parse, tokenize};

#[test]
fn test_complete_geometric_sketch() {
    let source = r#"
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
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, errors) = parse(tokens, &idents);

    assert!(
        errors.is_empty(),
        "Integration test should not have parsing errors: {:?}",
        errors
    );
    assert!(ast.is_some(), "AST should be parsed successfully");

    let ast = ast.unwrap();
    assert_eq!(ast.imports.len(), 1, "Should have one import");
    assert_eq!(ast.structs.len(), 1, "Should have one struct definition");
    assert_eq!(ast.sketches.len(), 1, "Should have one sketch");

    // Verify struct has expected structure
    let circle_struct = &ast.structs[0];
    assert_eq!(circle_struct.fields.len(), 2, "Circle should have 2 fields");
    assert_eq!(
        circle_struct.methods.len(),
        2,
        "Circle should have 2 methods"
    );
}

#[test]
fn test_control_flow_with_arrays() {
    let source = r#"
        sketch array_processing {
            let points: [Point; 10] = [];
            
            for i in 0..10 {
                points[i] = point(i * 5mm, 0mm);
            }
            
            let total_distance: Length = 0mm;
            for i in 1..10 {
                let prev_point = points[0];
                let curr_point = points[1];
                total_distance = total_distance + 5mm;
            }
            
            with coordinate_system {
                let transformed_points: [Point; 10] = [];
                for i in 0..10 {
                    transformed_points[i] = transform(points[i]);
                }
            }
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, errors) = parse(tokens, &idents);

    assert!(
        errors.is_empty(),
        "Control flow integration test should not have parsing errors: {:?}",
        errors
    );
    assert!(ast.is_some(), "AST should be parsed successfully");
}

#[test]
fn test_complex_expressions_integration() {
    let source = r#"
        sketch complex_math {
            // Mixed arithmetic with precedence
            let complex_calc: Length = (10mm + 5mm) * 2.0;
            
            // Nested function calls with expressions
            let center_point = point(width / 2.0, height / 2.0);
            
            // Complex boolean expressions
            let is_valid = center_point.x > 0mm && center_point.y > 0mm;
            
            // Array initialization with expressions
            let vertices: [Point; 4] = [
                point(0mm, 0mm),
                point(width, 0mm),
                point(width, height),
                point(0mm, height)
            ];
            
            // Chained method calls and array access
            let distance_to_origin = vertices[0].distance_to(point(0mm, 0mm));
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, errors) = parse(tokens, &idents);

    assert!(
        errors.is_empty(),
        "Complex expressions integration test should not have parsing errors: {:?}",
        errors
    );
    assert!(ast.is_some(), "AST should be parsed successfully");
}

#[test]
fn test_nested_structures_integration() {
    let source = r#"
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
        
        struct Polygon {
            vertex_count: I32,
            
            fn area() -> Area {
                let result: Area;
                return result;
            }
        }
        
        sketch polygon_design {
            let simple_point: Point2D;
            let simple_line: Line;
            let simple_polygon: Polygon;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, errors) = parse(tokens, &idents);

    assert!(
        errors.is_empty(),
        "Nested structures integration test should not have parsing errors: {:?}",
        errors
    );
    assert!(ast.is_some(), "AST should be parsed successfully");

    let ast = ast.unwrap();
    assert_eq!(ast.structs.len(), 3, "Should have three struct definitions");
    assert_eq!(ast.sketches.len(), 1, "Should have one sketch");
}

#[test]
fn test_units_and_types_integration() {
    let source = r#"
        sketch units_demo {
            let length_mm: Length = 25mm;
            let angle_degrees: Angle = 45deg;
            let point_var: Point;
            let area_calc: Area;
            let bool_var: Bool = true;
            let int_var: I32 = 42;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, errors) = parse(tokens, &idents);

    assert!(
        errors.is_empty(),
        "Units and types integration test should not have parsing errors: {:?}",
        errors
    );
    assert!(ast.is_some(), "AST should be parsed successfully");
}

#[test]
fn test_function_definitions_integration() {
    let source = r#"
        sketch function_examples {
            fn simple_function(x: Length) -> Length {
                return x;
            }
            
            fn add_lengths(a: Length, b: Length) -> Length {
                let sum: Length = a;
                return sum;
            }
            
            let test_value: Length = 10mm;
            let result1 = simple_function(test_value);
            let result2 = add_lengths(test_value, test_value);
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, errors) = parse(tokens, &idents);

    assert!(
        errors.is_empty(),
        "Function definitions integration test should not have parsing errors: {:?}",
        errors
    );
    assert!(ast.is_some(), "AST should be parsed successfully");
}

#[test]
fn test_phase_2_1_reference_expressions_integration() {
    // Clean integration test for Phase 2.1 reference expression feature
    let source = r#"
        sketch reference_test {
            let value = 10mm;
            let point_ref = &value;
            let nested_ref = &(&value);
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, errors) = parse(tokens, &idents);

    assert!(
        errors.is_empty(),
        "Phase 2.1 reference expressions should not have errors: {:?}",
        errors
    );
    assert!(ast.is_some(), "AST should be parsed successfully");

    // Verify the parsing was successful
    let ast = ast.unwrap();
    assert_eq!(ast.sketches.len(), 1);
    let sketch = &ast.sketches[0];
    assert_eq!(sketch.body.len(), 3); // 3 let statements with reference expressions
}

#[test]
fn test_phase_2_1_sketch_functions_integration() {
    // Clean integration test for Phase 2.1 function definitions in sketch feature
    let source = r#"
        sketch function_test {
            fn helper(x: Length) -> Length {
                return x * 2;
            }
            
            fn calculator(a: Length, b: Length) -> Length {
                let sum = a + b;
                return sum;
            }
            
            let value = 10mm;
            let doubled = helper(value);
            let calculated = calculator(5mm, 15mm);
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, errors) = parse(tokens, &idents);

    assert!(
        errors.is_empty(),
        "Phase 2.1 sketch functions should not have errors: {:?}",
        errors
    );
    assert!(ast.is_some(), "AST should be parsed successfully");

    // Verify the parsing was successful
    let ast = ast.unwrap();
    assert_eq!(ast.sketches.len(), 1);
    let sketch = &ast.sketches[0];
    assert_eq!(sketch.functions.len(), 2); // 2 function definitions
    assert_eq!(sketch.body.len(), 3); // 3 let statements
}

#[test]
fn test_real_world_cad_example() {
    let source = r#"
        import geometry;
        import constraints;
        
        struct Gear {
            center: Point,
            radius: Length,
            teeth: i32,
            
            fn tooth_pitch() -> Angle {
                let result: Angle = 18deg;
                return result;
            }
            
            fn circumference() -> Length {
                let result: Length = 157mm;
                return result;
            }
        }
        
        sketch gear_system {
            // Main gear specifications
            let main_gear_center: Point;
            let main_gear_radius: Length = 25mm;
            let main_gear_teeth: i32 = 20;
            
            let main_gear = Gear {
                center: main_gear_center,
                radius: main_gear_radius,
                teeth: main_gear_teeth
            };
            
            // Secondary gear positioned relative to main gear
            let secondary_gear_center: Point;
            let secondary_gear_radius: Length = 15mm;
            let secondary_gear_teeth: i32 = 12;
            
            let secondary_gear = Gear {
                center: secondary_gear_center,
                radius: secondary_gear_radius,
                teeth: secondary_gear_teeth
            };
            
            // Simple calculations
            let gear_ratio: i32 = 10;
            let total_teeth: i32 = 32;
            let mesh_is_correct: Bool = true;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, errors) = parse(tokens, &idents);

    assert!(
        errors.is_empty(),
        "Real-world CAD example should not have parsing errors: {:?}",
        errors
    );
    assert!(ast.is_some(), "AST should be parsed successfully");

    let ast = ast.unwrap();
    assert_eq!(ast.imports.len(), 2, "Should have two imports");
    assert_eq!(ast.structs.len(), 1, "Should have one struct (Gear)");
    assert_eq!(ast.sketches.len(), 1, "Should have one sketch");
}

#[test]
fn test_error_scenarios_integration() {
    // Test various error scenarios that should be handled gracefully
    let invalid_sources = vec![
        // Missing semicolon
        r#"
            sketch test {
                let x = 10mm
            }
        "#,
        // Invalid syntax in expression
        r#"
            sketch test {
                let x = 10mm + 5mm +;
            }
        "#,
        // Unclosed parentheses
        r#"
            sketch test {
                let x = (10mm + 5mm;
            }
        "#,
        // Invalid function syntax
        r#"
            sketch test {
                fn invalid( {
                }
            }
        "#,
    ];

    for (i, source) in invalid_sources.iter().enumerate() {
        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
        let (ast, errors) = parse(tokens, &idents);

        // These should have errors, but shouldn't crash
        assert!(
            !errors.is_empty() || ast.is_none(),
            "Invalid source {} should produce errors or no AST",
            i
        );
    }
}
