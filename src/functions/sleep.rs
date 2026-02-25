/// `sleep` — pause execution for the given number of seconds.
///
/// The argument is a floating-point number so fractional seconds are supported.
///
/// ```bucl
/// sleep 1        # pause for 1 second
/// sleep 0.5      # pause for half a second
/// sleep 2.75     # pause for 2.75 seconds
/// ```
///
/// On WASM targets this is a no-op (synchronous sleep is not available in a
/// browser environment).
use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Sleep;

impl BuclFunction for Sleep {
    fn call(
        &self,
        _evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let secs_str = args.first().ok_or_else(|| {
            BuclError::RuntimeError("sleep: expected a number of seconds".into())
        })?;

        let secs: f64 = secs_str.parse().map_err(|_| {
            BuclError::RuntimeError(format!(
                "sleep: '{}' is not a valid number of seconds",
                secs_str
            ))
        })?;

        if secs < 0.0 {
            return Err(BuclError::RuntimeError(format!(
                "sleep: duration must not be negative, got {}",
                secs
            )));
        }

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::sleep(std::time::Duration::from_secs_f64(secs));

        Ok(None)
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("sleep", Sleep);
}
