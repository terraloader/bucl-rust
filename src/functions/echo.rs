/// `echo` — print one or more values to standard output.
///
/// All arguments are joined with a single space and emitted as one line.
/// On native targets the line is also printed to stdout immediately.
///
/// ```bucl
/// echo "Hello, World!"
/// echo "x =" {x}
/// ```
use crate::ast::Statement;
use crate::error::Result;
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Echo;

impl BuclFunction for Echo {
    fn call(
        &self,
        evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let value = args.join(" ");
        evaluator.output_buffer.push(value.clone());
        #[cfg(not(target_arch = "wasm32"))]
        println!("{}", value);
        Ok(None)
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("echo", Echo);
}
