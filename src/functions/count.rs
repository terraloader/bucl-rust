/// `count` — return the number of arguments passed.
///
/// ```bucl
/// {n} count "one" "two" "three"   # {n} = "3"
/// ```
use crate::ast::Statement;
use crate::error::Result;
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Count;

impl BuclFunction for Count {
    fn call(
        &self,
        _evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        Ok(Some(args.len().to_string()))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("count", Count);
}
