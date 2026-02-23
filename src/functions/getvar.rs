/// `getvar` — read a variable whose name is computed at runtime.
///
/// This allows BUCL scripts to access dynamically named variables such as
/// positional arguments `{0}`, `{1}`, … or indexed sub-variables.
///
/// ```bucl
/// {n} = "greeting"
/// {greeting} = "hello"
/// {val} getvar {n}       # {val} = "hello"
///
/// # Access a positional argument by index stored in another variable:
/// {i} = "2"
/// {item} getvar {i}      # same as {item} = {2}
/// ```
use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct GetVar;

impl BuclFunction for GetVar {
    fn call(
        &self,
        evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let name = args
            .first()
            .ok_or_else(|| BuclError::RuntimeError("getvar: requires a variable name".into()))?;
        Ok(Some(evaluator.resolve_var(name)))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("getvar", GetVar);
}
