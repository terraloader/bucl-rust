/// `=` — store (and concatenate) one or more text values into a variable.
///
/// ```bucl
/// {greeting} = "Hello, " "World!"
/// ```
use crate::ast::Statement;
use crate::error::Result;
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Assign;

impl BuclFunction for Assign {
    fn call(
        &self,
        _evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        Ok(Some(args.join("")))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("=", Assign);
}
