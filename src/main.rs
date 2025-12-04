use cad_dsl::{IdentArena, tokenize};

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
        }
        Err(errors) => {
            println!("Tokenization failed with {} errors:", errors.len());
            for error in errors {
                println!("  Error: {} at {:?}", error.error, error.span);
            }
        }
    }
}
