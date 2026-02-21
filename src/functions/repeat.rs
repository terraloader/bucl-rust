/// `repeat` — execute an indented block a fixed number of times.
///
/// The target variable is used as a namespace prefix for the loop counter.
/// Inside the block, `{<target>/index}` holds the 1-based iteration number.
///
/// ```bucl
/// {r} repeat 5
///     {output} = "Iteration {r/index} of 5"
/// ```
///
/// If no target is given, the prefix defaults to `r`.
use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Repeat;

impl BuclFunction for Repeat {
    fn call(
        &self,
        evaluator: &mut Evaluator,
        target: Option<&str>,
        args: Vec<String>,
        block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let prefix = target.unwrap_or("r");

        let count_str = args
            .first()
            .ok_or_else(|| BuclError::RuntimeError("repeat: missing count argument".into()))?;

        let count: usize = count_str.parse().map_err(|_| {
            BuclError::RuntimeError(format!("repeat: '{}' is not a valid count", count_str))
        })?;

        if let Some(block) = block {
            for i in 0..count {
                evaluator.set_var(&format!("{}/index", prefix), (i + 1).to_string());
                evaluator.evaluate_statements(block)?;
            }
        }

        Ok(None)
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("repeat", Repeat);
}
