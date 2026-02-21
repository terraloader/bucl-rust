use crate::ast::{Param, Statement};
use crate::error::{BuclError, Result};
use crate::lexer::{self, Line, Token};

/// Parse a full BUCL source string into a list of top-level statements.
pub fn parse(source: &str) -> Result<Vec<Statement>> {
    let lines = lexer::tokenize(source)?;
    let mut p = Parser { lines, cursor: 0 };
    p.parse_block(0)
}

// ---------------------------------------------------------------------------
// Internal parser state
// ---------------------------------------------------------------------------

struct Parser {
    lines: Vec<Line>,
    cursor: usize,
}

impl Parser {
    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn current_indent(&self) -> Option<usize> {
        self.lines.get(self.cursor).map(|l| l.indent)
    }

    /// Returns true when the line at `idx` is `elseif` or `else`.
    /// These are handled as continuations of an `if`/`elseif` statement and
    /// must never be consumed as standalone top-level statements.
    fn is_continuation_at(&self, idx: usize) -> bool {
        if let Some(line) = self.lines.get(idx) {
            if let Some(Token::Bare(name)) = line.tokens.first() {
                return name == "elseif" || name == "else";
            }
        }
        false
    }

    // -----------------------------------------------------------------------
    // Block parser
    // -----------------------------------------------------------------------

    /// Parse all consecutive statements at exactly `expected_indent`.
    /// Stops (without consuming) when indentation drops below `expected_indent`
    /// or when an `elseif`/`else` keyword is seen (handled by parent).
    fn parse_block(&mut self, expected_indent: usize) -> Result<Vec<Statement>> {
        let mut stmts = Vec::new();

        loop {
            match self.current_indent() {
                None => break,
                Some(i) if i < expected_indent => break,
                Some(i) if i > expected_indent => {
                    return Err(BuclError::ParseError(format!(
                        "unexpected indentation: expected {} spaces/tabs, got {}",
                        expected_indent, i
                    )));
                }
                _ => {}
            }

            // Leave elseif/else for the parent if/elseif to consume.
            if self.is_continuation_at(self.cursor) {
                break;
            }

            let stmt = self.parse_statement(expected_indent)?;
            stmts.push(stmt);
        }

        Ok(stmts)
    }

    // -----------------------------------------------------------------------
    // Statement parser
    // -----------------------------------------------------------------------

    fn parse_statement(&mut self, current_indent: usize) -> Result<Statement> {
        let line = self.lines[self.cursor].clone();
        self.cursor += 1;

        let (target, function, args) = extract_parts(&line.tokens)?;

        // Collect a deeper-indented block that belongs to this statement.
        let block = match self.current_indent() {
            Some(i) if i > current_indent => {
                let block_indent = i;
                Some(self.parse_block(block_indent)?)
            }
            _ => None,
        };

        // Collect elseif / else as a continuation (only for if / elseif).
        let continuation = if function == "if" || function == "elseif" {
            if self.is_continuation_at(self.cursor)
                && self.current_indent() == Some(current_indent)
            {
                Some(Box::new(self.parse_statement(current_indent)?))
            } else {
                None
            }
        } else {
            None
        };

        Ok(Statement {
            target,
            function,
            args,
            block,
            continuation,
        })
    }
}

// ---------------------------------------------------------------------------
// Token-level helpers
// ---------------------------------------------------------------------------

/// Decompose a tokenised line into `(target, function_name, args)`.
///
/// Grammar:
/// ```text
/// line = ( '{' IDENT '}' ) BARE param*
///      | BARE param*
/// param = '{' IDENT '}' | '"' … '"' | BARE
/// ```
fn extract_parts(tokens: &[Token]) -> Result<(Option<String>, String, Vec<Param>)> {
    if tokens.is_empty() {
        return Err(BuclError::ParseError("empty line".to_string()));
    }

    let mut iter = tokens.iter();
    let first = iter.next().unwrap();

    // Determine target and function name.
    let (target, function) = match first {
        Token::Variable(name) => {
            // The variable is the result target; next token must be the function.
            match iter.next() {
                Some(Token::Bare(f)) => (Some(name.clone()), f.clone()),
                Some(other) => {
                    return Err(BuclError::ParseError(format!(
                        "expected function name after '{{{}}}', got {:?}",
                        name, other
                    )));
                }
                None => {
                    return Err(BuclError::ParseError(format!(
                        "expected function name after '{{{}}}'",
                        name
                    )));
                }
            }
        }
        Token::Bare(name) => (None, name.clone()),
        Token::Quoted(s) => {
            return Err(BuclError::ParseError(format!(
                "a line cannot start with a string literal: \"{}\"",
                s
            )));
        }
    };

    // Remaining tokens are arguments.
    let args = iter
        .map(|t| match t {
            Token::Quoted(s) => Param::Quoted(s.clone()),
            Token::Variable(n) => Param::Variable(n.clone()),
            Token::Bare(s) => Param::Bare(s.clone()),
        })
        .collect();

    Ok((target, function, args))
}
