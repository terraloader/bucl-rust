use crate::error::{BuclError, Result};

/// A single token on a BUCL line.
#[derive(Debug, Clone)]
pub enum Token {
    /// `{name}` — a variable reference.
    Variable(String),
    /// `"..."` — a quoted string (escape sequences already resolved).
    Quoted(String),
    /// Any bare word, number, or operator (`=`, `-1`, …).
    Bare(String),
}

/// A successfully tokenized non-empty, non-comment line.
#[derive(Debug, Clone)]
pub struct Line {
    /// Number of leading whitespace characters (used as indent level).
    pub indent: usize,
    pub tokens: Vec<Token>,
}

/// Tokenize one raw source line.
/// Returns `None` for blank lines and pure-comment lines.
pub fn tokenize_line(line: &str) -> Result<Option<Line>> {
    // Measure indent before stripping
    let indent = line.len() - line.trim_start_matches(|c: char| c == ' ' || c == '\t').len();
    let content = line.trim();

    if content.is_empty() || content.starts_with('#') {
        return Ok(None);
    }

    let mut tokens: Vec<Token> = Vec::new();
    let mut chars = content.chars().peekable();

    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }

        if c == '{' {
            chars.next(); // consume opening '{'
            let mut name = String::new();
            let mut depth = 1usize;
            loop {
                match chars.next() {
                    None => break,
                    Some('{') => { depth += 1; name.push('{'); }
                    Some('}') => {
                        depth -= 1;
                        if depth == 0 { break; }
                        name.push('}');
                    }
                    Some(ch) => name.push(ch),
                }
            }
            tokens.push(Token::Variable(name));
        } else if c == '"' {
            chars.next(); // consume opening '"'
            let mut s = String::new();
            loop {
                match chars.next() {
                    None | Some('"') => break,
                    Some('\\') => match chars.next() {
                        Some('"') => s.push('"'),
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('\\') => s.push('\\'),
                        Some(nc) => {
                            s.push('\\');
                            s.push(nc);
                        }
                        None => break,
                    },
                    Some(ch) => s.push(ch),
                }
            }
            tokens.push(Token::Quoted(s));
        } else {
            let mut word = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() {
                    break;
                }
                word.push(ch);
                chars.next();
            }
            tokens.push(Token::Bare(word));
        }
    }

    if tokens.is_empty() {
        return Ok(None);
    }

    Ok(Some(Line { indent, tokens }))
}

/// Tokenize an entire BUCL source string into a sequence of lines.
pub fn tokenize(source: &str) -> Result<Vec<Line>> {
    let mut lines = Vec::new();
    for (lineno, raw) in source.lines().enumerate() {
        match tokenize_line(raw) {
            Ok(Some(line)) => lines.push(line),
            Ok(None) => {}
            Err(BuclError::ParseError(msg)) => {
                return Err(BuclError::ParseError(format!("line {}: {}", lineno + 1, msg)));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(lines)
}
