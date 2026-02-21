/// `writefile` — write (or overwrite) a file with the given content.
///
/// The first argument is the file path; all remaining arguments are
/// concatenated and written as the file content.
/// The return value is the content that was written.
///
/// ```bucl
/// {ok} writefile "out.txt" "Hello, World!"
/// ```
use std::fs;

use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct WriteFile;

impl BuclFunction for WriteFile {
    fn call(
        &self,
        _evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        if args.len() < 2 {
            return Err(BuclError::RuntimeError(
                "writefile: requires a path and content".into(),
            ));
        }
        let path = &args[0];
        let content = args[1..].join("");
        fs::write(path, &content)?;
        Ok(Some(content))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("writefile", WriteFile);
}
