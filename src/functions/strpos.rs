/// `strpos` — find the first occurrence of a needle inside a string.
///
/// Returns the 0-based character index, or `"-1"` when not found.
/// This is a core primitive that enables string-splitting in BUCL scripts.
///
/// ```bucl
/// {p} strpos "hello world" "world"   # {p} = "6"
/// {p} strpos "hello" "xyz"           # {p} = "-1"
/// ```
use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct StrPos;

impl BuclFunction for StrPos {
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
                "strpos: requires text and needle arguments".into(),
            ));
        }
        let text = &args[0];
        let needle = &args[1];

        let result = text
            .find(needle.as_str())
            // Convert byte offset → char offset (Unicode-safe).
            .map(|byte_pos| text[..byte_pos].chars().count() as i64)
            .unwrap_or(-1);

        Ok(Some(result.to_string()))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("strpos", StrPos);
}
