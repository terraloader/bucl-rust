/// `length` — return the total character count across all arguments.
///
/// ```bucl
/// {l} length "hal" "lo"   # {l} = "5"
/// ```
use crate::ast::Statement;
use crate::error::Result;
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Length;

impl BuclFunction for Length {
    fn call(
        &self,
        _evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let total: usize = args.iter().map(|s| s.chars().count()).sum();
        Ok(Some(total.to_string()))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("length", Length);
}
