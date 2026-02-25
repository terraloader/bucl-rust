// `echo` — print one or more values to standard output.
//
// All arguments are joined with a single space and emitted as one line.
// On native: buffered in output_buffer and printed to stdout immediately.
// On WASM:   each line is sent to the JS host via js_print, which streams
//            it to the main thread in real time via postMessage.

// WASM: imported from the Web Worker (see docs/demo/wasm/worker.js).
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn js_print(ptr: *const u8, len: usize);
}

use crate::ast::Statement;
use crate::error::Result;
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Echo;

impl BuclFunction for Echo {
    #[cfg_attr(target_arch = "wasm32", allow(unused_variables))]
    fn call(
        &self,
        evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let value = args.join(" ");
        #[cfg(target_arch = "wasm32")]
        unsafe {
            js_print(value.as_ptr(), value.len());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            evaluator.output_buffer.push(value.clone());
            println!("{}", value);
        }
        Ok(None)
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("echo", Echo);
}
