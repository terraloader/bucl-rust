/// `readfile` — read the entire contents of a file into a variable.
///
/// ```bucl
/// {contents} readfile "hello.txt"
/// ```
use std::fs;

use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct ReadFile;

impl BuclFunction for ReadFile {
    fn call(
        &self,
        _evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let path = args
            .first()
            .ok_or_else(|| BuclError::RuntimeError("readfile: missing path argument".into()))?;
        let contents = fs::read_to_string(path)?;
        Ok(Some(contents))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("readfile", ReadFile);
}
