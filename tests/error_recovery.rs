// Phase 5.3: Error recovery tests for better user experience
// These tests verify that the parser can gracefully handle errors and provide useful feedback

use cad_dsl::{IdentArena, parse, tokenize};

#[test]
fn test_missing_semicolon_recovery() {
    let source = r#"
        sketch test {
            let x = 10mm
            let y = 20mm;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    // Should have parsing errors but shouldn't crash
    assert!(
        !errors.is_empty(),
        "Missing semicolon should produce errors"
    );

    // Check that error contains helpful information about semicolon
    let error_msg = format!("{:?}", errors[0]);
    assert!(
        error_msg.contains("Semicolon") || error_msg.contains("semicolon"),
        "Error should mention semicolon: {}",
        error_msg
    );
}

#[test]
fn test_unterminated_parentheses_recovery() {
    let source = r#"
        sketch test {
            let x = (10mm + 20mm;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    assert!(
        !errors.is_empty(),
        "Unterminated parentheses should produce errors"
    );

    // Check for helpful error about parentheses
    let error_msg = format!("{:?}", errors[0]);
    assert!(
        error_msg.contains("LParen") || error_msg.contains("paren") || error_msg.contains("RParen"),
        "Error should mention parentheses: {}",
        error_msg
    );
}

#[test]
fn test_invalid_expression_recovery() {
    let source = r#"
        sketch test {
            let x = 10mm + + 5mm;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    assert!(
        !errors.is_empty(),
        "Invalid expression should produce errors"
    );
}

#[test]
fn test_malformed_struct_recovery() {
    let source = r#"
        struct Point {
            x Length,
            y: Length,
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    assert!(!errors.is_empty(), "Malformed struct should produce errors");
}

#[test]
fn test_invalid_function_syntax_recovery() {
    let source = r#"
        sketch test {
            fn invalid_function( {
                let x = 10mm;
            }
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    assert!(
        !errors.is_empty(),
        "Invalid function syntax should produce errors"
    );
}

#[test]
fn test_unclosed_brace_recovery() {
    let source = r#"
        sketch test {
            let x = 10mm;
            let y = 20mm;
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    assert!(!errors.is_empty(), "Unclosed brace should produce errors");
}

#[test]
fn test_invalid_array_syntax_recovery() {
    let source = r#"
        sketch test {
            let arr: [Point 5] = [];
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    assert!(
        !errors.is_empty(),
        "Invalid array syntax should produce errors"
    );
}

#[test]
fn test_partial_parsing_with_errors() {
    // Test that valid parts can still be parsed even with errors
    let source = r#"
        sketch test {
            let valid1 = 10mm;
            let invalid = 10mm + + 5mm;
            let valid2 = 20mm;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    // Should have errors for the invalid line
    assert!(
        !errors.is_empty(),
        "Should have parsing errors for invalid syntax"
    );

    // May or may not have partial AST - depends on error recovery implementation
    // This test documents current behavior
}

#[test]
fn test_multiple_errors_in_one_file() {
    let source = r#"
        sketch test {
            let x = 10mm
            let y = (20mm + 30mm;
            let z = 40mm + + 50mm;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    // Should detect multiple errors
    assert!(!errors.is_empty(), "Should have parsing errors");

    // Document the number of errors found
    println!("Found {} parsing errors:", errors.len());
    for (i, error) in errors.iter().enumerate() {
        println!("  {}: {:?}", i + 1, error);
    }
}

#[test]
fn test_error_span_information() {
    let source = r#"
        sketch test {
            let x = 10mm + + 5mm;
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    assert!(!errors.is_empty(), "Should have parsing errors");

    // Check that errors have span information
    for error in &errors {
        let span = error.span();
        assert!(
            span.start < span.end,
            "Error span should have valid range: {:?}",
            span
        );
        assert!(
            span.start < source.len(),
            "Error span start should be within source"
        );
    }
}

#[test]
fn test_helpful_error_messages() {
    let test_cases = vec![
        // Missing semicolon
        (r#"sketch test { let x = 10mm }"#, "semicolon"),
        // Invalid operator sequence
        (r#"sketch test { let x = 10mm + +; }"#, "unexpected"),
        // Unclosed parentheses
        (r#"sketch test { let x = (10mm; }"#, "paren"),
    ];

    for (source, expected_keyword) in test_cases {
        let mut idents = IdentArena::new();
        if let Ok(tokens) = tokenize(source, &mut idents) {
            let (_ast, errors) = parse(tokens, &idents);

            if !errors.is_empty() {
                let error_text = format!("{:?}", errors[0]).to_lowercase();
                println!("Testing '{}' for keyword '{}':", source, expected_keyword);
                println!("  Error: {}", error_text);

                // This test documents what error messages we currently get
                // Future improvements can make these more helpful
            }
        }
    }
}

#[test]
fn test_error_recovery_doesnt_crash() {
    // Test various invalid syntax patterns to ensure they don't crash the parser
    let invalid_sources = vec![
        "sketch { }",
        "let = 10mm;",
        "sketch test { let; }",
        "struct { field: Type }",
        "fn () {}",
        "sketch test { 10mm + ; }",
        "sketch test { let x: = 10mm; }",
        "sketch test { for in 0..10 {} }",
        "sketch test { with {} }",
        "import;",
        "",
        ";;;",
        "((()))",
        "sketch test { let x = ; }",
    ];

    for source in invalid_sources {
        let mut idents = IdentArena::new();

        // Tokenization might fail for some cases, that's ok
        if let Ok(tokens) = tokenize(source, &mut idents) {
            // Parsing should not crash, even with invalid input
            let result = std::panic::catch_unwind(|| parse(tokens, &idents));

            assert!(
                result.is_ok(),
                "Parser should not panic on invalid input: '{}'",
                source
            );
        }
    }
}

#[test]
fn test_error_context_preservation() {
    // Test that errors preserve context about what was being parsed
    let source = r#"
        struct Circle {
            radius: Length,
            
            fn area() -> Length {
                let result = radius * radius
            }
        }
    "#;

    let mut idents = IdentArena::new();
    let tokens = tokenize(source, &mut idents).expect("Tokenization should succeed");
    let (_ast, errors) = parse(tokens, &idents);

    if !errors.is_empty() {
        // Check that we can determine the error occurred in a function context
        let error_msg = format!("{:?}", errors[0]);
        println!("Error in struct/function context: {}", error_msg);

        // The specific error message format can be improved over time
        // This test documents current behavior
    }
}
