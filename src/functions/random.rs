/// `random` — generate a random integer.
///
/// ```bucl
/// {r} random           # 0 .. i64::MAX
/// {r} random 10        # 0 .. 10  (inclusive)
/// {r} random 1 6       # 1 .. 6   (inclusive, like a die)
/// ```
use rand::Rng;

use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Random;

impl BuclFunction for Random {
    fn call(
        &self,
        _evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let parse = |s: &str| -> Result<i64> {
            s.parse().map_err(|_| {
                BuclError::RuntimeError(format!("random: '{}' is not a valid integer", s))
            })
        };

        let (min, max) = match args.as_slice() {
            [] => (0, i64::MAX),
            [max_s] => (0, parse(max_s)?),
            [min_s, max_s, ..] => (parse(min_s)?, parse(max_s)?),
        };

        if min > max {
            return Err(BuclError::RuntimeError(format!(
                "random: min ({}) is greater than max ({})",
                min, max
            )));
        }

        let value = rand::thread_rng().gen_range(min..=max);
        Ok(Some(value.to_string()))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("random", Random);
}
