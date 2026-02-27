/// `random` — generate a random integer.
///
/// ```bucl
/// {r} random           # 0 .. i64::MAX
/// {r} random 10        # 0 .. 10  (inclusive)
/// {r} random 1 6       # 1 .. 6   (inclusive, like a die)
/// ```
///
/// On native targets this uses `rand::thread_rng`.
/// On WASM targets it imports `js_math_random` from the host (provided by the
/// demo's JS glue as `() => Math.random()`).

// Native: pull in the rand crate.
#[cfg(not(target_arch = "wasm32"))]
use rand::Rng;

// WASM: import Math.random() from the JavaScript host.
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn js_math_random() -> f64;
}

fn random_in_range(min: i64, max: i64) -> i64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        rand::thread_rng().gen_range(min..=max)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let f = unsafe { js_math_random() };
        // Map [0, 1) float to [min, max] integer.
        let range = (max - min).saturating_add(1) as f64;
        min + (f * range) as i64
    }
}

use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Random;

impl BuclFunction for Random {
    fn call(
        &self,
        evaluator: &mut Evaluator,
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

        // Named params: {min} = 1; {max} = 6; {r} random {min} {max}
        let named_min = evaluator.named_arg("min").cloned();
        let named_max = evaluator.named_arg("max").cloned();

        let (min, max) = match (named_min, named_max) {
            (Some(min_s), Some(max_s)) => (parse(&min_s)?, parse(&max_s)?),
            (None, Some(max_s)) => (0, parse(&max_s)?),
            _ => match args.as_slice() {
                [] => (0, i64::MAX),
                [max_s] => (0, parse(max_s)?),
                [min_s, max_s, ..] => (parse(min_s)?, parse(max_s)?),
            },
        };

        if min > max {
            return Err(BuclError::RuntimeError(format!(
                "random: min ({}) is greater than max ({})",
                min, max
            )));
        }

        Ok(Some(random_in_range(min, max).to_string()))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("random", Random);
}
