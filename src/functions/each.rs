/// `each` — execute an indented block once for every argument.
///
/// All items are stored into the target variable using the standard indexing
/// system before iteration begins, so the loop variable is a fully-populated
/// structured variable:
///
/// - `{e}`        — number of items (count).
/// - `{e/count}`  — same as `{e}`.
/// - `{e/length}` — total character length across all items.
/// - `{e/0}`, `{e/1}`, … — the original items (0-based).
///
/// During each iteration two extra sub-variables are updated:
/// - `{e/index}` — 0-based index of the current item.
/// - `{e/value}` — value of the current item.
///
/// ```bucl
/// {e} each "Alice" "Bob" "Charlie"
///     {output} = "{e/index}: {e/value}"
/// {output} = "total items: {e/count}"
/// ```
///
/// If no target is given, the prefix defaults to `e`.
use crate::ast::Statement;
use crate::error::Result;
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Each;

impl BuclFunction for Each {
    fn call(
        &self,
        evaluator: &mut Evaluator,
        target: Option<&str>,
        args: Vec<String>,
        block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let prefix = target.unwrap_or("e");
        let count = args.len();

        // Populate the target variable with all items before iterating so the
        // full structure is available even inside the first block execution.
        //
        // set_var handles output-printing and sets count=1 + length for the
        // root variable; we then override count and length with the real values.
        evaluator.set_var(prefix, count.to_string());
        evaluator
            .variables
            .insert(format!("{}/count", prefix), count.to_string());
        let total_len: usize = args.iter().map(|s| s.chars().count()).sum();
        evaluator
            .variables
            .insert(format!("{}/length", prefix), total_len.to_string());
        for (i, item) in args.iter().enumerate() {
            evaluator
                .variables
                .insert(format!("{}/{}", prefix, i), item.clone());
        }

        if let Some(block) = block {
            for (i, item) in args.iter().enumerate() {
                evaluator
                    .variables
                    .insert(format!("{}/index", prefix), i.to_string());
                evaluator
                    .variables
                    .insert(format!("{}/value", prefix), item.clone());
                evaluator.evaluate_statements(block)?;
            }
        }

        Ok(None) // Everything already stored directly.
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("each", Each);
}
