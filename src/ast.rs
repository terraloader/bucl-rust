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

/// A fully-evaluated function argument, optionally carrying the source variable name.
///
/// When a variable reference like `{port}` or `{db/port}` is passed to a function,
/// the last path segment becomes the parameter name.  This lets functions access
/// arguments by name (e.g. `{port}`) in addition to by position (`{0}`).
#[derive(Debug, Clone)]
pub struct ResolvedArg {
    /// Parameter name derived from the source variable.
    /// - `{port}` → `Some("port")`
    /// - `{db/port}` → `Some("port")` (last path segment)
    /// - `"literal"` or bare `42` → `None`
    pub name: Option<String>,
    /// The resolved string value.
    pub value: String,
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
