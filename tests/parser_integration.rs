// Phase 5.2: Integration tests for complete parsing pipeline
// These tests verify end-to-end parsing with real TextCAD code examples

use cad_dsl::{IdentArena, parse, tokenize};

#[test]
#[ignore = "Requires implicit return expressions in functions - will be implemented in Phase 4 (Type Checking)"]
fn test_complete_geometric_sketch() {
    let source = r#"
        import geometry;
        
        struct Circle {
            center: Point,
            radius: Length,
            
            fn area() -> Area {
                let result = radius * radius;
            }
            
            fn circumference() -> Length {
                let result = radius * 2.0;
            }
        }
        
        sketch geometric_design {
            let origin: Point = point(0mm, 0mm);
            let circle_radius: Length = 50mm;
            let main_circle = Circle { center: origin, radius: circle_radius };
            
            let outer_radius: Length = circle_radius + 10mm;
            let inner_radius: Length = circle_radius - 10mm;
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
#[ignore = "Requires reference expression parsing (&variable syntax) - will be implemented in Phase 2.1 (Parser Extensions)"]
fn test_nested_structures_integration() {
    let source = r#"
        struct Point2D {
            x: Length,
            y: Length,
            
            fn distance_to(other: &Point2D) -> Length {
                let dx = x - other.x;
                let dy = y - other.y;
                let result = sqrt(dx * dx);
            }
        }
        
        struct Line {
            start: Point2D,
            end: Point2D,
            
            fn length() -> Length {
                let result = start.distance_to(&end);
            }
            
            fn midpoint() -> Point2D {
                let result = Point2D {
                    x: start.x + end.x,
                    y: start.y + end.y
                };
            }
        }
        
        struct Polygon {
            vertices: [Point2D; 6],
            edges: [Line; 6],
            
            fn area() -> Area {
                let total: Area = 0.0;
                for i in 0..6 {
                    let j = 1;
                    total = total + vertices[i].x * vertices[j].y;
                }
                let result = total / 2.0;
            }
        }
        
        sketch polygon_design {
            let hex_vertices: [Point2D; 6] = [
                Point2D { x: 10mm, y: 0mm },
                Point2D { x: 5mm, y: 8.66mm },
                Point2D { x: -5mm, y: 8.66mm },
                Point2D { x: -10mm, y: 0mm },
                Point2D { x: -5mm, y: -8.66mm },
                Point2D { x: 5mm, y: -8.66mm }
            ];
            
            let hex_polygon = Polygon {
                vertices: hex_vertices,
                edges: []
            };
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
#[ignore = "Requires advanced literal and function expression parsing - will be implemented in Phase 4 (Type Checking)"]
fn test_units_and_types_integration() {
    let source = r#"
        sketch units_demo {
            // Various unit types
            let length_mm: Length = 25.5mm;
            let length_inches: Length = 2.0in;
            let angle_degrees: Angle = 45.0deg;
            let angle_radians: Angle = 1.57rad;
            
            // Reference types
            let point_ref: &Point = &origin;
            let line_ref: &Line = &main_line;
            
            // Array types with expressions
            let measurements: [Length; 3] = [10mm, 20mm, 30mm];
            let angles: [Angle; 4] = [0deg, 90deg, 180deg, 270deg];
            
            // Complex type expressions
            let area_calc: Area = length_mm * length_mm;
            let circumference: Length = 2.0 * 3.14159 * radius;
            
            // Type conversion and calculations
            let total_length: Length = length_mm + length_inches;
            let total_angle: Angle = angle_degrees + angle_radians;
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
#[ignore = "Requires function definitions in sketch context - will be implemented in Phase 2.1 (Parser Extensions)"]
fn test_function_definitions_integration() {
    let source = r#"
        sketch function_examples {
            fn calculate_distance(p1: Point, p2: Point) -> Length {
                let dx = p1.x - p2.x;
                let dy = p1.y - p2.y;
                let result = sqrt(dx * dx);
            }
            
            fn create_circle(center: Point, radius: Length) -> Circle {
                let result = Circle {
                    center: center,
                    radius: radius
                };
            }
            
            fn process_points(points: [Point; 10]) -> [Point; 10] {
                let processed: [Point; 10] = [];
                for i in 0..10 {
                    processed[i] = transform(points[i]);
                }
                let result = processed;
            }
            
            // Function calls in expressions
            let p1 = point(0mm, 0mm);
            let p2 = point(10mm, 10mm);
            let dist = calculate_distance(p1, p2);
            let circle = create_circle(p1, dist);
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
#[ignore = "Requires multiple advanced features - will be implemented across Phase 2.1 (Parser Extensions) and Phase 4 (Type Checking)"]
fn test_real_world_cad_example() {
    let source = r#"
        import geometry;
        import constraints;
        
        struct Gear {
            center: Point,
            radius: Length,
            teeth: i32,
            
            fn tooth_pitch() -> Angle {
                let result = 360deg / teeth;
            }
            
            fn circumference() -> Length {
                let result = 2.0 * 3.14159 * radius;
            }
        }
        
        sketch gear_system {
            // Main gear specifications
            let main_gear_center = point(0mm, 0mm);
            let main_gear_radius: Length = 25mm;
            let main_gear_teeth: i32 = 20;
            
            let main_gear = Gear {
                center: main_gear_center,
                radius: main_gear_radius,
                teeth: main_gear_teeth
            };
            
            // Secondary gear positioned relative to main gear
            let gear_distance: Length = main_gear_radius + 15mm;
            let secondary_gear_center = point(gear_distance, 0mm);
            let secondary_gear_radius: Length = 15mm;
            let secondary_gear_teeth: i32 = 12;
            
            let secondary_gear = Gear {
                center: secondary_gear_center,
                radius: secondary_gear_radius,
                teeth: secondary_gear_teeth
            };
            
            // Calculate gear ratio
            let gear_ratio = main_gear_teeth / 2;
            
            // Verify gear mesh (centers should be separated by sum of radii)
            let actual_distance = distance(main_gear_center, secondary_gear_center);
            let expected_distance = main_gear_radius + secondary_gear_radius;
            let mesh_is_correct = actual_distance == expected_distance;
            
            // Create tooth points for main gear
            let main_gear_teeth_points: [Point; 20] = [];
            for i in 0..20 {
                let angle: Angle = i * 18deg;
                let tooth_x = main_gear_center.x + main_gear_radius;
                let tooth_y = main_gear_center.y + main_gear_radius;
                main_gear_teeth_points[i] = point(tooth_x, tooth_y);
            }
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
