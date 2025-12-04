#[cfg(test)]
mod tests {
    use crate::{IdentArena, tokenize, parse};

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
        
        assert!(errors.is_empty(), "Parsing should not have errors: {:?}", errors);
        assert!(ast.is_some(), "AST should be parsed successfully");
        
        let ast = ast.unwrap();
        assert_eq!(ast.sketches.len(), 1, "Should have exactly one sketch");
        assert_eq!(ast.sketches[0].body.len(), 1, "Sketch should have one statement");
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
        
        assert!(errors.is_empty(), "Parsing should not have errors: {:?}", errors);
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
        
        assert!(errors.is_empty(), "Parsing should not have errors: {:?}", errors);
        assert!(ast.is_some(), "AST should be parsed successfully");
        
        let ast = ast.unwrap();
        assert_eq!(ast.structs.len(), 1, "Should have exactly one struct");
        assert_eq!(ast.structs[0].fields.len(), 2, "Struct should have two fields");
    }
}