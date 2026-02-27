/// `repeat` — execute an indented block a fixed number of times.
///
/// The target variable is populated before iteration begins:
///
/// - `{r}`        — the repeat count.
/// - `{r/count}`  — same as `{r}`.
/// - `{r/length}` — character length of the count string.
///
/// During each iteration one extra sub-variable is updated:
/// - `{r/index}` — 1-based iteration number.
///
/// ```bucl
/// {r} repeat 5
///     {output} = "Iteration {r/index} of {r/count}"
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

        // Named param: {count} = 5; {r} repeat {count}
        let count_str = evaluator
            .named_arg("count")
            .cloned()
            .or_else(|| args.first().cloned())
            .ok_or_else(|| BuclError::RuntimeError("repeat: missing count argument".into()))?;

        let count: usize = count_str.parse().map_err(|_| {
            BuclError::RuntimeError(format!("repeat: '{}' is not a valid count", &count_str))
        })?;

        // Populate the target variable with metadata before iterating so the
        // full structure is available inside the first block execution.
        evaluator.set_var(prefix, count.to_string());
        evaluator
            .variables
            .insert(format!("{}/count", prefix), count.to_string());

        if let Some(block) = block {
            for i in 0..count {
                evaluator
                    .variables
                    .insert(format!("{}/index", prefix), (i + 1).to_string());
                evaluator.evaluate_statements(block)?;
            }
        }

        Ok(None)
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("repeat", Repeat);
}
