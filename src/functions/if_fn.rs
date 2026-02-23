/// `if` / `elseif` / `else` — conditional execution.
///
/// Condition syntax: `<lhs> <op> <rhs>`
///
/// Supported operators:
/// - `=`  — string equality
/// - `!=` — string inequality
/// - `>`  — greater than
/// - `<`  — less than
/// - `>=` — greater than or equal
/// - `<=` — less than or equal
///
/// For `>`, `<`, `>=`, `<=`: if both sides parse as numbers the comparison is
/// numeric (integer or decimal); otherwise it falls back to lexicographic
/// string comparison.
///
/// ```bucl
/// if {x} = "hello"
///     {output} = "Got hello"
/// elseif {x} > "10"
///     {output} = "Got a number bigger than 10"
/// else
///     {output} = "Got something else"
/// ```
///
/// `elseif` shares the same implementation as `if`.
/// `else` simply runs its block unconditionally.
use crate::ast::Statement;
use crate::error::Result;
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

// ---------------------------------------------------------------------------
// Condition evaluation
// ---------------------------------------------------------------------------

fn evaluate_condition(lhs: &str, op: &str, rhs: &str) -> bool {
    match op {
        "=" => lhs == rhs,
        "!=" => lhs != rhs,
        ">" | "<" | ">=" | "<=" => {
            // Prefer numeric comparison; fall back to lexicographic.
            if let (Ok(l), Ok(r)) = (lhs.parse::<f64>(), rhs.parse::<f64>()) {
                match op {
                    ">"  => l > r,
                    "<"  => l < r,
                    ">=" => l >= r,
                    "<=" => l <= r,
                    _    => unreachable!(),
                }
            } else {
                match op {
                    ">"  => lhs > rhs,
                    "<"  => lhs < rhs,
                    ">=" => lhs >= rhs,
                    "<=" => lhs <= rhs,
                    _    => unreachable!(),
                }
            }
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// if / elseif
// ---------------------------------------------------------------------------

pub struct IfFn;

impl BuclFunction for IfFn {
    fn call(
        &self,
        evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        block: Option<&[Statement]>,
        continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let condition = match args.as_slice() {
            [lhs, op, rhs] => evaluate_condition(lhs, op, rhs),
            _ => false,
        };

        if condition {
            if let Some(block) = block {
                evaluator.evaluate_statements(block)?;
            }
        } else if let Some(cont) = continuation {
            evaluator.evaluate_statement(cont)?;
        }

        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// else
// ---------------------------------------------------------------------------

pub struct ElseFn;

impl BuclFunction for ElseFn {
    fn call(
        &self,
        evaluator: &mut Evaluator,
        _target: Option<&str>,
        _args: Vec<String>,
        block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        if let Some(block) = block {
            evaluator.evaluate_statements(block)?;
        }
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

pub fn register(eval: &mut Evaluator) {
    eval.register("if", IfFn);
    eval.register("elseif", IfFn); // identical logic
    eval.register("else", ElseFn);
}
