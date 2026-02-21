/// `math` — evaluate a basic arithmetic expression.
///
/// Supports `+`, `-`, `*`, `/`, `%`, unary `-`, and parentheses.
///
/// ```bucl
/// {m} math "3+3"          # {m} = "6"
/// {m} math "(10-2)*3"     # {m} = "24"
/// ```
use std::iter::Peekable;
use std::str::Chars;

use crate::ast::Statement;
use crate::error::{BuclError, Result};
use crate::evaluator::Evaluator;
use crate::functions::BuclFunction;

pub struct Math;

impl BuclFunction for Math {
    fn call(
        &self,
        _evaluator: &mut Evaluator,
        _target: Option<&str>,
        args: Vec<String>,
        _block: Option<&[Statement]>,
        _continuation: Option<&Statement>,
    ) -> Result<Option<String>> {
        let expr = args.join("");
        let value = eval_expr(&expr)
            .map_err(|e| BuclError::RuntimeError(format!("math: {}", e)))?;

        // Format as integer when there is no fractional part.
        let s = if value.fract() == 0.0 && value.abs() < 1e15 {
            format!("{}", value as i64)
        } else {
            format!("{}", value)
        };

        Ok(Some(s))
    }
}

pub fn register(eval: &mut Evaluator) {
    eval.register("math", Math);
}

// ---------------------------------------------------------------------------
// Recursive-descent expression evaluator
// ---------------------------------------------------------------------------

fn eval_expr(s: &str) -> std::result::Result<f64, String> {
    let mut chars = s.chars().peekable();
    let result = parse_add_sub(&mut chars)?;
    skip_ws(&mut chars);
    if let Some(c) = chars.peek() {
        return Err(format!("unexpected character '{}'", c));
    }
    Ok(result)
}

fn skip_ws(chars: &mut Peekable<Chars>) {
    while chars.peek().map_or(false, |c| c.is_whitespace()) {
        chars.next();
    }
}

fn parse_add_sub(chars: &mut Peekable<Chars>) -> std::result::Result<f64, String> {
    let mut left = parse_mul_div(chars)?;
    loop {
        skip_ws(chars);
        match chars.peek() {
            Some('+') => {
                chars.next();
                left += parse_mul_div(chars)?;
            }
            Some('-') => {
                chars.next();
                left -= parse_mul_div(chars)?;
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_mul_div(chars: &mut Peekable<Chars>) -> std::result::Result<f64, String> {
    let mut left = parse_unary(chars)?;
    loop {
        skip_ws(chars);
        match chars.peek() {
            Some('*') => {
                chars.next();
                left *= parse_unary(chars)?;
            }
            Some('/') => {
                chars.next();
                let right = parse_unary(chars)?;
                if right == 0.0 {
                    return Err("division by zero".to_string());
                }
                left /= right;
            }
            Some('%') => {
                chars.next();
                let right = parse_unary(chars)?;
                if right == 0.0 {
                    return Err("modulo by zero".to_string());
                }
                left %= right;
            }
            _ => break,
        }
    }
    Ok(left)
}

fn parse_unary(chars: &mut Peekable<Chars>) -> std::result::Result<f64, String> {
    skip_ws(chars);
    if chars.peek() == Some(&'-') {
        chars.next();
        return Ok(-parse_primary(chars)?);
    }
    if chars.peek() == Some(&'+') {
        chars.next();
    }
    parse_primary(chars)
}

fn parse_primary(chars: &mut Peekable<Chars>) -> std::result::Result<f64, String> {
    skip_ws(chars);
    if chars.peek() == Some(&'(') {
        chars.next();
        let val = parse_add_sub(chars)?;
        skip_ws(chars);
        match chars.next() {
            Some(')') => return Ok(val),
            other => return Err(format!("expected ')', got {:?}", other)),
        }
    }

    let mut num = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c);
            chars.next();
        } else {
            break;
        }
    }

    if num.is_empty() {
        return Err(match chars.peek() {
            Some(c) => format!("expected number, got '{}'", c),
            None => "expected number, got end of expression".to_string(),
        });
    }

    num.parse()
        .map_err(|_| format!("invalid number literal '{}'", num))
}
