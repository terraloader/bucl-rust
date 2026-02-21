/// `cmp` — numeric comparison of two values.
///
/// Returns `"1"` when a > b, `"-1"` when a < b, `"0"` when a == b.
/// This enables greater-than / less-than branching inside BUCL scripts
/// because `if` only supports equality (`=`).
///
/// ```bucl
/// {r} cmp "5" "3"    # {r} = "1"   (5 > 3)
/// {r} cmp "2" "9"    # {r} = "-1"  (2 < 9)
/// {r} cmp "4" "4"    # {r} = "0"   (equal)
/// ```
use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Cmp;

impl BuclFunction for Cmp {
    fn call(
        &self,
        _evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        if args.len() < 2 {
            return Err(BuclError::RuntimeError(
                "cmp: requires two arguments".into(),
            ));
        }

        let parse = |s: &str| -> f64 { s.parse().unwrap_or(0.0) };
        let a = parse(&args[0]);
        let b = parse(&args[1]);

        let result = if a > b { "1" } else if a < b { "-1" } else { "0" };
        Ok(Some(result.to_string()))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("cmp", Cmp);
}
