// `sleep` — pause execution for the given number of seconds.
//
// The argument is a floating-point number so fractional seconds are supported.
//
// On native targets this uses std::thread::sleep.
// On WASM targets a synchronous busy-wait is performed via js_sleep(ms),
// provided by the JavaScript host (see docs/demo/wasm/index.html).
// The host implements it as a Date.now() spin-loop so that the synchronous
// evaluator can block without requiring an async runtime.

// WASM: import a host-provided busy-wait from JavaScript.
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn js_sleep(ms: f64);
}

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

        #[cfg(target_arch = "wasm32")]
        unsafe {
            js_sleep(secs * 1000.0);
        }

        Ok(None)
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("sleep", Sleep);
}
