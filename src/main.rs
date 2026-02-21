mod ast;
mod error;
mod evaluator;
mod functions;
mod lexer;
mod parser;

use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = env::args().collect();

    let (source, base_dir) = if args.len() > 1 {
        let path = PathBuf::from(&args[1]);
        let source = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading '{}': {}", path.display(), e);
                std::process::exit(1);
            }
        };
        // Resolve the script's parent directory so the evaluator can find
        // functions/ relative to the script.
        let base = path
            .canonicalize()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()));
        (source, base)
    } else {
        let mut buf = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buf) {
            eprintln!("Error reading stdin: {}", e);
            std::process::exit(1);
        }
        (buf, None)
    };

    let mut eval = evaluator::Evaluator::new();
    eval.base_dir = base_dir;
    functions::register_all(&mut eval);

    let stmts = match parser::parse(&source) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = eval.evaluate_statements(&stmts) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
