/// `if` / `elseif` / `else` — conditional execution.
///
/// ```bucl
/// if {x} = "hello"
///     {output} = "Got hello"
/// elseif {x} = "world"
///     {output} = "Got world"
/// else
///     {output} = "Got something else"
/// ```
///
/// The condition is always `<lhs> = <rhs>` (string equality).
/// `elseif` shares the same implementation as `if`.
/// `else` simply runs its block unconditionally.
use crate::ast::Statement;
use crate::error::Result;
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

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
        // Expected args layout: [lhs, "=", rhs]
        let condition = matches!(args.as_slice(), [lhs, op, rhs] if op == "=" && lhs == rhs);

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
