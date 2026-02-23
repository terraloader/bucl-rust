/// `=` — store one or more text values into a variable.
///
/// Every assignment automatically maintains two metadata sub-variables:
/// - `{var/count}`  — number of arguments (`"1"` for single, `"N"` for multi).
/// - `{var/length}` — total character length of the stored (concatenated) value.
///
/// ## Single string  →  character indexing
/// `{var/N}` returns the Nth character (0-based) of the value.
///
/// ```bucl
/// {word} = "hello"
/// {output} = {word/0}     # h
/// {output} = {word/4}     # o
/// {output} = {word/count} # 1
/// {output} = {word/length}# 5
/// ```
///
/// ## Multiple strings  →  element indexing
/// Each original string is stored as `{var/0}`, `{var/1}`, …
/// `{var}` holds the concatenation.  Out-of-range numeric indices return `""`.
///
/// ```bucl
/// {parts} = "hello" "world"
/// {output} = {parts/0}    # hello
/// {output} = {parts/1}    # world
/// {output} = {parts}      # helloworld
/// {output} = {parts/count}# 2
/// ```
///
/// ## Arbitrary named sub-variables
/// Any `{var/name}` can also be set freely — it is just a normal variable.
/// ```bucl
/// {test/label} = "important"
/// ```
use crate::ast::Statement;
use crate::error::Result;
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Assign;

impl BuclFunction for Assign {
    fn call(
        &self,
        evaluator: &mut Evaluator,
        target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let value = args.join("");

        let Some(prefix) = target else {
            // No target: nothing to store (caller handles the None return).
            return Ok(Some(value));
        };

        // Store the concatenated value.  set_var auto-sets count=1 and length.
        evaluator.set_var(prefix, value);

        if args.len() > 1 {
            // Override count with the actual number of string arguments and
            // store each original string under its 0-based index.
            evaluator
                .variables
                .insert(format!("{}/count", prefix), args.len().to_string());
            for (i, arg) in args.iter().enumerate() {
                evaluator
                    .variables
                    .insert(format!("{}/{}", prefix, i), arg.clone());
            }
        }

        // We handled the store ourselves; tell the evaluator not to call set_var again.
        Ok(None)
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("=", Assign);
}
