use cad_dsl::{IdentArena, parse, tokenize};

fn main() {
    let source = r#"
        sketch simple_triangle {
            let p1: Point = point(0mm, 0mm);
            let p2: Point = point(30mm, 0mm);
            let p3: Point = point();
            
            distance(&p1, &p3) = 40mm;
            distance(&p2, &p3) = 50mm;
        }
    "#;

    let mut idents = IdentArena::new();

    println!("=== Lexing ===");
    match tokenize(source, &mut idents) {
        Ok(tokens) => {
            println!("Tokenization successful! Found {} tokens:", tokens.len());
            for (i, token) in tokens.iter().enumerate() {
                if let cad_dsl::ProcessedTokenKind::Ident(id) = &token.kind {
                    println!(
                        "  {}: {} at {:?} ({})",
                        i,
                        idents.resolve(*id),
                        token.span,
                        token.kind
                    );
                } else {
                    println!("  {}: {} at {:?}", i, token.kind, token.span);
                }
            }

            println!("\n=== Parsing ===");
            match parse(tokens, &idents) {
                (Some(ast), errors) => {
                    if !errors.is_empty() {
                        println!("Parsing completed with {} warnings:", errors.len());
                        for error in &errors {
                            println!("  Warning: {}", error);
                        }
                    } else {
                        println!("Parsing successful!");
                    }

                    println!("AST:");
                    println!("  Imports: {}", ast.imports.len());
                    println!("  Structs: {}", ast.structs.len());
                    println!("  Sketches: {}", ast.sketches.len());

                    for (i, sketch) in ast.sketches.iter().enumerate() {
                        println!(
                            "  Sketch {}: {} with {} statements",
                            i,
                            idents.resolve(sketch.name),
                            sketch.body.len()
                        );
                    }
                }
                (None, errors) => {
                    println!("Parsing failed with {} errors:", errors.len());
                    for error in &errors {
                        println!("  Error: {}", error);
                    }
                }
            }
        }
        Err(errors) => {
            println!("Tokenization failed with {} errors:", errors.len());
            for error in errors {
                println!("  Error: {} at {:?}", error.error, error.span);
            }
        }
    }
}
