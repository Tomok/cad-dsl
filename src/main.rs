use cad_dsl::{IdentArena, check_types, parse, resolve_names, tokenize};

fn main() {
    let source = r#"
        sketch type_error_example {
            let x: Length = 10mm;
            let y: Angle = 45deg;
            let sum: Length = x;
            sum = y;
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

                    println!("\n=== Name Resolution ===");
                    let (resolved_ast, name_errors) = resolve_names(ast, &idents);
                    {
                        if !name_errors.is_empty() {
                            println!(
                                "Name resolution completed with {} errors:",
                                name_errors.len()
                            );
                            for error in &name_errors {
                                println!("  Error: {}", error);
                            }
                        } else {
                            println!("Name resolution successful!");
                        }

                        if name_errors.is_empty() {
                            println!("\n=== Type Checking ===");
                            let (typed_ir, type_errors) = check_types(
                                resolved_ast.clone(),
                                &resolved_ast.symbol_table,
                                &idents,
                            );

                            if !type_errors.is_empty() {
                                println!(
                                    "Type checking completed with {} errors:",
                                    type_errors.len()
                                );
                                for error in &type_errors {
                                    println!("  Error: {} at {:?}", error.kind, error.span);
                                }
                            } else {
                                println!("Type checking successful!");
                                println!("Typed IR:");
                                println!("  Sketches: {}", typed_ir.sketches.len());
                                println!(
                                    "  Type table entries: {}",
                                    typed_ir.type_table.types.len()
                                );

                                for (i, sketch) in typed_ir.sketches.iter().enumerate() {
                                    println!(
                                        "  Typed Sketch {}: {} with {} statements",
                                        i,
                                        idents.resolve(sketch.name),
                                        sketch.body.len()
                                    );
                                }
                            }
                        } else {
                            println!("Skipping type checking due to name resolution errors.");
                        }
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
