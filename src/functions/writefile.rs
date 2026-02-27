/// `writefile` — write (or overwrite) a file with the given content.
///
/// The first argument is the file path; all remaining arguments are
/// concatenated and written as the file content.
/// The return value is the content that was written.
///
/// ```bucl
/// {ok} writefile "out.txt" "Hello, World!"
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

    pub struct WriteFile;

    impl BuclFunction for WriteFile {
        fn call(
            &self,
            evaluator: &mut Evaluator,
            _target: Option<&str>,
            args: Vec<String>,
            _block: Option<&[Statement]>,
            _continuation: Option<&Statement>,
        ) -> Result<Option<String>> {
            // Named params: {path} = "out.txt"; {content} = "Hello"
            //               writefile {path} {content}
            let path = evaluator
                .named_arg("path")
                .cloned()
                .or_else(|| args.first().cloned())
                .ok_or_else(|| {
                    BuclError::RuntimeError("writefile: requires a path and content".into())
                })?;
            let content = evaluator
                .named_arg("content")
                .cloned()
                .unwrap_or_else(|| {
                    if args.len() > 1 { args[1..].join("") } else { String::new() }
                });
            fs::write(path, &content)?;
            Ok(Some(content))
        }
    }

    pub fn register(eval: &mut Evaluator) {
        eval.register("writefile", WriteFile);
    }
}

pub fn register(eval: &mut Evaluator) {
    #[cfg(not(target_arch = "wasm32"))]
    native::register(eval);
    let _ = eval; // suppress unused warning on wasm32
}
