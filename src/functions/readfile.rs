/// `readfile` — read the entire contents of a file into a variable.
///
/// ```bucl
/// {contents} readfile "hello.txt"
/// ```
///
/// Not available in WASM builds (no filesystem access).
use crate::evaluator::Evaluator;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::fs;

    use crate::ast::Statement;
    use crate::error::{BuclError, Result};
    use crate::evaluator::Evaluator;
    use crate::functions::BuclFunction;

    pub struct ReadFile;

    impl BuclFunction for ReadFile {
        fn call(
            &self,
            evaluator: &mut Evaluator,
            _target: Option<&str>,
            args: Vec<String>,
            _block: Option<&[Statement]>,
            _continuation: Option<&Statement>,
        ) -> Result<Option<String>> {
            // Named param: {path} = "hello.txt"; {c} readfile {path}
            let path = evaluator
                .named_arg("path")
                .cloned()
                .or_else(|| args.first().cloned())
                .ok_or_else(|| {
                    BuclError::RuntimeError("readfile: missing path argument".into())
                })?;
            let contents = fs::read_to_string(&path)?;
            Ok(Some(contents))
        }
    }

    pub fn register(eval: &mut Evaluator) {
        eval.register("readfile", ReadFile);
    }
}

pub fn register(eval: &mut Evaluator) {
    #[cfg(not(target_arch = "wasm32"))]
    native::register(eval);
    let _ = eval; // suppress unused warning on wasm32
}
