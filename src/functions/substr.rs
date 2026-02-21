/// `substr` — extract a substring.
///
/// Arguments: `start` (0-based char index), `length` (number of chars), `string`.
///
/// ```bucl
/// {res} substr 0 3 "AAAaaa"   # {res} = "AAA"
/// ```
use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Substr;

impl BuclFunction for Substr {
    fn call(
        &self,
        _evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        if args.len() < 3 {
            return Err(BuclError::RuntimeError(
                "substr: requires start, length, and string arguments".into(),
            ));
        }

        let start: usize = args[0].parse().map_err(|_| {
            BuclError::RuntimeError(format!("substr: '{}' is not a valid start index", args[0]))
        })?;
        let length: usize = args[1].parse().map_err(|_| {
            BuclError::RuntimeError(format!("substr: '{}' is not a valid length", args[1]))
        })?;

        let chars: Vec<char> = args[2].chars().collect();
        let start = start.min(chars.len());
        let end = (start + length).min(chars.len());

        Ok(Some(chars[start..end].iter().collect()))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("substr", Substr);
}
