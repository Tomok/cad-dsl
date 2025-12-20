mod lexer;
mod parser;

use clap::{Parser, Subcommand};
use logos::Logos;
use std::fs;

#[derive(Parser)]
#[command(name = "cad-dsl")]
#[command(about = "A DSL for CAD operations")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Lex { file: String },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Lex { file } => {
            let content = fs::read_to_string(file).expect("Failed to read file");
            let tokens = lexer::Token::lexer(&content);

            for token_result in tokens {
                match token_result {
                    Ok(token) => println!("{:?}", token),
                    Err(error) => eprintln!("Lexing error: {:?}", error),
                }
            }
        }
    }
}
