/// `each` — execute an indented block once for every argument.
///
/// The target variable is used as a namespace prefix for the current item.
/// Inside the block, `{<target>/value}` holds the current item.
///
/// ```bucl
/// {e} each "Alice" "Bob" "Charlie"
///     {output} = "Hello, {e/value}!"
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

        if let Some(block) = block {
            for item in args {
                evaluator.set_var(&format!("{}/value", prefix), item);
                evaluator.evaluate_statements(block)?;
            }
        }

        Ok(None)
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("each", Each);
}
