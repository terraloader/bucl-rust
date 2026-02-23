use crate::ast::Statement;
use crate::error::Result;
use crate::evaluator::Evaluator;

// ---------------------------------------------------------------------------
// Core trait
// ---------------------------------------------------------------------------

/// Implement this trait to add a new built-in BUCL function.
///
/// For higher-level functions that don't need OS access or Rust internals,
/// prefer writing a `functions/<name>.bucl` file instead — no recompilation
/// needed.
///
/// # Calling convention (mirrors the .bucl convention)
/// - `target`  — the `{var}` written before the function name, if any.
/// - `args`    — already-evaluated string arguments.
/// - `block`   — indented block owned by this call (`if`, `repeat`, `each`, …).
/// - `continuation` — `elseif`/`else` chained after an `if`.
///
/// Return `Ok(Some(value))` to store `value` in the target variable.
/// Return `Ok(None)` to leave the target variable unchanged.
pub trait BuclFunction: Send + Sync {
    fn call(
        &self,
        evaluator: &mut Evaluator,
        target: Option<&str>,
        args: Vec<String>,
        block: Option<&[Statement]>,
        continuation: Option<&Statement>,
    ) -> Result<Option<String>>;
}

// ---------------------------------------------------------------------------
// Core built-in modules
// These are compiled into the binary because they need Rust-level access
// (control flow, OS I/O, arithmetic, or character-level string operations).
// ---------------------------------------------------------------------------

pub mod assign;    // =
pub mod count;     // count
pub mod each;      // each
pub mod getvar;    // getvar — read a variable by computed name
pub mod if_fn;     // if / elseif / else
pub mod length;    // length
pub mod math;      // math
pub mod random;    // random
pub mod readfile;  // readfile
pub mod repeat;    // repeat
pub mod setvar;    // setvar — write a variable by computed name
pub mod strpos;    // strpos — find substring position
pub mod substr;    // substr — extract substring by index + length
pub mod writefile; // writefile

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register every core built-in with the evaluator.
///
/// Higher-level functions (`explode`, `implode`, `reverse`, `maxlength`,
/// `slice`, …) live in `functions/*.bucl` and are loaded automatically at
/// runtime — no registration needed here.
pub fn register_all(eval: &mut Evaluator) {
    assign::register(eval);
    count::register(eval);
    each::register(eval);
    getvar::register(eval);
    if_fn::register(eval);
    length::register(eval);
    math::register(eval);
    random::register(eval);
    readfile::register(eval);
    repeat::register(eval);
    setvar::register(eval);
    strpos::register(eval);
    substr::register(eval);
    writefile::register(eval);
}
