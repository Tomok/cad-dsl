#[cfg(test)]
mod tests {
    use crate::{IdentArena, parse, tokenize};

    #[test]
    fn test_simple_sketch_parsing() {
        let source = r#"
            sketch test {
                let x: Length = 10mm;
            }
        "#;

        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
        let (ast, errors) = parse(tokens, &idents);

        assert!(
            errors.is_empty(),
            "Parsing should not have errors: {:?}",
            errors
        );
        assert!(ast.is_some(), "AST should be parsed successfully");

        let ast = ast.unwrap();
        assert_eq!(ast.sketches.len(), 1, "Should have exactly one sketch");
        assert_eq!(
            ast.sketches[0].body.len(),
            1,
            "Sketch should have one statement"
        );
    }

    #[test]
    fn test_empty_sketch_parsing() {
        let source = r#"
            sketch empty {
            }
        "#;

        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
        let (ast, errors) = parse(tokens, &idents);

        assert!(
            errors.is_empty(),
            "Parsing should not have errors: {:?}",
            errors
        );
        assert!(ast.is_some(), "AST should be parsed successfully");

        let ast = ast.unwrap();
        assert_eq!(ast.sketches.len(), 1, "Should have exactly one sketch");
        assert_eq!(ast.sketches[0].body.len(), 0, "Sketch should be empty");
    }

    #[test]
    fn test_struct_parsing() {
        let source = r#"
            struct Circle {
                center: Point,
                radius: Length,
            }
        "#;

        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
        let (ast, errors) = parse(tokens, &idents);

        assert!(
            errors.is_empty(),
            "Parsing should not have errors: {:?}",
            errors
        );
        assert!(ast.is_some(), "AST should be parsed successfully");

        let ast = ast.unwrap();
        assert_eq!(ast.structs.len(), 1, "Should have exactly one struct");
        assert_eq!(
            ast.structs[0].fields.len(),
            2,
            "Struct should have two fields"
        );
    }

    #[test]
    fn test_struct_with_methods() {
        let source = r#"
            struct Circle {
                center: Point,
                radius: Length,
                
                fn diameter() -> Length {
                    self.radius * 2.0
                }
                
                fn area() -> Area {
                    PI * self.radius * self.radius
                }
            }
        "#;

        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
        let (ast, errors) = parse(tokens, &idents);

        assert!(
            errors.is_empty(),
            "Parsing should not have errors: {:?}",
            errors
        );
        assert!(ast.is_some(), "AST should be parsed successfully");

        let ast = ast.unwrap();
        assert_eq!(ast.structs.len(), 1, "Should have exactly one struct");
        assert_eq!(
            ast.structs[0].fields.len(),
            2,
            "Struct should have two fields"
        );
        assert_eq!(
            ast.structs[0].methods.len(),
            2,
            "Struct should have two methods"
        );
    }

    #[test]
    fn test_control_flow() {
        let source = r#"
            sketch control_test {
                for i in 0..5 {
                    points[i] = point(i * 10mm, 0mm);
                }
                
                with view1 {
                    let p: Point = point(10mm, 20mm);
                }
            }
        "#;

        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
        let (ast, errors) = parse(tokens, &idents);

        assert!(
            errors.is_empty(),
            "Parsing should not have errors: {:?}",
            errors
        );
        assert!(ast.is_some(), "AST should be parsed successfully");
    }

    #[test]
    fn test_complex_expressions() {
        let source = r#"
            sketch expr_test {
                let result: Length = (10mm + 5mm) * 2.0;
                let points: [Point; 3] = [p1, p2, p3];
            }
        "#;

        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
        let (ast, errors) = parse(tokens, &idents);

        assert!(
            errors.is_empty(),
            "Parsing should not have errors: {:?}",
            errors
        );
        assert!(ast.is_some(), "AST should be parsed successfully");
    }

    #[test]
    fn test_array_types() {
        let source = r#"
            struct Polygon {
                vertices: [Point; 6],
                edges: [&Line; 6],
            }
        "#;

        let mut idents = IdentArena::new();
        let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
        let (ast, errors) = parse(tokens, &idents);

        assert!(
            errors.is_empty(),
            "Parsing should not have errors: {:?}",
            errors
        );
        assert!(ast.is_some(), "AST should be parsed successfully");
    }
}
