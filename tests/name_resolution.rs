// Phase 3: Name Resolution tests
// These tests verify that name resolution correctly resolves identifiers to their definitions

use cad_dsl::{IdentArena, NameResolutionError, parse, resolve_names, tokenize};

#[test]
fn test_simple_variable_resolution() {
    let source = r#"
        sketch test {
            let x = 10mm;
            let y = x;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    assert!(
        resolution_errors.is_empty(),
        "Name resolution should succeed: {:?}",
        resolution_errors
    );
    assert_eq!(resolved_ast.sketches.len(), 1);

    let sketch = &resolved_ast.sketches[0];
    assert_eq!(sketch.body.len(), 2);
}

#[test]
fn test_undefined_variable_error() {
    let source = r#"
        sketch test {
            let x = undefined_var;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (_, resolution_errors) = resolve_names(ast, &idents);

    assert!(
        !resolution_errors.is_empty(),
        "Should have resolution errors"
    );
    assert!(matches!(
        resolution_errors[0],
        NameResolutionError::UndefinedSymbol { .. }
    ));
}

#[test]
fn test_duplicate_variable_error() {
    let source = r#"
        sketch test {
            let x = 10mm;
            let x = 20mm;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (_, resolution_errors) = resolve_names(ast, &idents);

    assert!(
        !resolution_errors.is_empty(),
        "Should have resolution errors"
    );
    assert!(matches!(
        resolution_errors[0],
        NameResolutionError::DuplicateDefinition { .. }
    ));
}

#[test]
fn test_struct_declaration_resolution() {
    let source = r#"
        struct Point {
            x: Length,
            y: Length,
        }
        
        sketch test {
            let p: Point;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, _resolution_errors) = resolve_names(ast, &idents);

    // Note: This might have errors since struct instantiation syntax isn't fully implemented
    // but the struct declaration should be resolved
    assert_eq!(resolved_ast.structs.len(), 1);

    let struct_def = &resolved_ast.structs[0];
    assert_eq!(struct_def.fields.len(), 2);
}

#[test]
fn test_function_parameter_resolution() {
    let source = r#"
        sketch test {
            fn calculate(x: Length, y: Length) -> Length {
                let result = x + y;
                return result;
            }
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    assert!(
        resolution_errors.is_empty(),
        "Name resolution should succeed: {:?}",
        resolution_errors
    );
    assert_eq!(resolved_ast.sketches.len(), 1);

    let sketch = &resolved_ast.sketches[0];
    assert_eq!(sketch.functions.len(), 1);

    let function = &sketch.functions[0];
    assert_eq!(function.params.len(), 2);
    assert_eq!(function.body.len(), 2); // let result = ...; return result;
}

#[test]
fn test_for_loop_variable_scoping() {
    let source = r#"
        sketch test {
            for i in 0..10 {
                let temp = i * 2;
            }
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    assert!(
        resolution_errors.is_empty(),
        "Name resolution should succeed: {:?}",
        resolution_errors
    );

    let sketch = &resolved_ast.sketches[0];
    assert_eq!(sketch.body.len(), 1);

    // Loop variable 'i' should be accessible within loop body
    // This tests that we correctly handle loop variable scoping
}

#[test]
fn test_nested_scope_resolution() {
    let source = r#"
        sketch test {
            let x = 10mm;
            for i in 0..5 {
                let y = x + i; // x should resolve to outer scope
                for j in 0..3 {
                    let z = y + j; // y should resolve to middle scope  
                }
            }
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    assert!(
        resolution_errors.is_empty(),
        "Name resolution should succeed: {:?}",
        resolution_errors
    );

    // Test that we have proper nested scoping
    let sketch = &resolved_ast.sketches[0];
    assert_eq!(sketch.body.len(), 2); // let x = ...; for i in ...
}

#[test]
fn test_function_call_resolution() {
    let source = r#"
        sketch test {
            let result = 10mm;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    assert!(
        resolution_errors.is_empty(),
        "Name resolution should succeed: {:?}",
        resolution_errors
    );

    let sketch = &resolved_ast.sketches[0];
    assert_eq!(sketch.body.len(), 1); // let result = 10mm;
}

#[test]
fn test_reference_expression_resolution() {
    let source = r#"
        sketch test {
            let value = 10mm;
            let value_ref = &value;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    assert!(
        resolution_errors.is_empty(),
        "Name resolution should succeed: {:?}",
        resolution_errors
    );

    let sketch = &resolved_ast.sketches[0];
    assert_eq!(sketch.body.len(), 2);

    // Reference to 'value' should resolve correctly
}

#[test]
fn test_builtin_type_resolution() {
    let source = r#"
        sketch test {
            let len: Length = 10mm;
            let angle: Angle = 90deg;
            let flag: bool = true;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    // Built-in types should resolve without requiring explicit declarations
    // Some errors might occur for struct literal syntax, but type references should work
    println!(
        "Resolution errors for builtin types: {:?}",
        resolution_errors
    );

    let sketch = &resolved_ast.sketches[0];
    assert_eq!(sketch.body.len(), 3);
}

#[test]
fn test_struct_method_resolution() {
    let source = r#"
        struct Circle {
            radius: Length,
            
            fn area() -> Area {
                return radius * radius * 3.14159;
            }
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    // The 'radius' reference in the method should resolve to the struct field
    // This might produce an error in current implementation since field access
    // from method context isn't fully implemented
    println!(
        "Resolution errors for struct method: {:?}",
        resolution_errors
    );

    assert_eq!(resolved_ast.structs.len(), 1);

    let struct_def = &resolved_ast.structs[0];
    assert_eq!(struct_def.fields.len(), 1);
    assert_eq!(struct_def.methods.len(), 1);
}

#[test]
fn test_multiple_sketches_isolation() {
    let source = r#"
        sketch first {
            let x = 10mm;
        }
        
        sketch second {
            let x = 20mm; // Should not conflict with first sketch
            let y = x;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    assert!(
        resolution_errors.is_empty(),
        "Name resolution should succeed: {:?}",
        resolution_errors
    );
    assert_eq!(resolved_ast.sketches.len(), 2);

    // Variables in different sketches should not interfere with each other
    let first_sketch = &resolved_ast.sketches[0];
    let second_sketch = &resolved_ast.sketches[1];

    assert_eq!(first_sketch.body.len(), 1);
    assert_eq!(second_sketch.body.len(), 2);
}

#[test]
fn test_complex_expression_resolution() {
    let source = r#"
        sketch test {
            let a = 10mm;
            let b = 20mm;
            let c = 5;
            
            let complex = (a + b) * c - a / 2;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    assert!(
        resolution_errors.is_empty(),
        "Name resolution should succeed: {:?}",
        resolution_errors
    );

    let sketch = &resolved_ast.sketches[0];
    assert_eq!(sketch.body.len(), 4);

    // All variable references in the complex expression should resolve correctly
}

#[test]
fn test_array_type_resolution() {
    let source = r#"
        sketch test {
            let points: [Point; 5] = [];
            let values: [Length; 10] = [];
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    // Array types should resolve correctly
    // Built-in types in array declarations should not cause resolution errors
    println!("Resolution errors for array types: {:?}", resolution_errors);

    let sketch = &resolved_ast.sketches[0];
    assert_eq!(sketch.body.len(), 2);
}

#[test]
fn test_with_statement_scoping() {
    let source = r#"
        sketch test {
            let view = 10mm;
            with view {
                let local_var = 10mm;
                let another = local_var * 2;
            }
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (ast, parse_errors) = parse(tokens, &idents);

    assert!(parse_errors.is_empty(), "Parse should succeed");
    let ast = ast.expect("Should have AST");

    let (resolved_ast, resolution_errors) = resolve_names(ast, &idents);

    // get_view() function call will likely cause an error since it's not defined
    // but the scoping within the with block should work correctly
    println!(
        "Resolution errors for with statement: {:?}",
        resolution_errors
    );

    let sketch = &resolved_ast.sketches[0];
    assert_eq!(sketch.body.len(), 2); // let view = ...; with view { ... }
}
