/// `setvar` — write a variable whose name is computed at runtime.
///
/// The complement of `getvar`.  Primarily used inside BUCL library functions
/// to write indexed return values (`return/0`, `return/1`, …) that the
/// evaluator then copies to the caller's target namespace.
///
/// ```bucl
/// {key} = "return/0"
/// setvar {key} "first item"   # sets {return/0} = "first item"
/// ```
use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct SetVar;

impl BuclFunction for SetVar {
    fn call(
        &self,
        evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        if args.len() < 2 {
            return Err(BuclError::RuntimeError(
                "setvar: requires a variable name and a value".into(),
            ));
        }
        evaluator.set_var(&args[0], args[1].clone());
        Ok(None)
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("setvar", SetVar);
}
