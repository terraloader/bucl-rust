/// A parameter in a BUCL statement.
#[derive(Debug, Clone)]
pub enum Param {
    /// A double-quoted string literal: `"hello {name}"`.
    /// Variable references inside are interpolated at evaluation time.
    Quoted(String),
    /// A stand-alone variable reference: `{name}`.
    Variable(String),
    /// An unquoted bare word or number: `42`, `=`, `true`.
    Bare(String),
}

/// A single BUCL statement, parsed from one (logical) line.
#[derive(Debug, Clone)]
pub struct Statement {
    /// Optional result variable: `{var}` at the start of a line.
    pub target: Option<String>,
    /// The function / command to invoke (e.g. `=`, `if`, `length`).
    pub function: String,
    /// Arguments passed to the function.
    pub args: Vec<Param>,
    /// Indented block that belongs to this statement (for `if`, `repeat`, `each`, …).
    pub block: Option<Vec<Statement>>,
    /// The `elseif` / `else` continuation attached to an `if` or `elseif`.
    pub continuation: Option<Box<Statement>>,
}
