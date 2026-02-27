use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::ast::{Param, ResolvedArg, Statement};
use crate::error::{BuclError, Result};
use crate::functions::BuclFunction;

// ---------------------------------------------------------------------------
// Helpers (free functions)
// ---------------------------------------------------------------------------

/// Derive a parameter name from a variable path.
///
/// - `"port"` → `Some("port")`
/// - `"db/port"` → `Some("port")` (last segment)
/// - `"0"` / `"argc"` / `"args"` / … → `None` (reserved or numeric)
fn extract_param_name(var_name: &str) -> Option<String> {
    let base = match var_name.rfind('/') {
        Some(pos) => &var_name[pos + 1..],
        None => var_name,
    };
    if base.is_empty() {
        return None;
    }
    // Numeric names would collide with positional {0}, {1}, …
    if base.parse::<usize>().is_ok() {
        return None;
    }
    // Reserved variable names used by the calling convention.
    const RESERVED: &[&str] = &["argc", "args", "target", "return", "count", "length"];
    if RESERVED.contains(&base) {
        return None;
    }
    Some(base.to_string())
}

/// Check for duplicate named parameters and return an error if found.
fn check_duplicate_names(resolved: &[ResolvedArg]) -> Result<()> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for (i, arg) in resolved.iter().enumerate() {
        if let Some(ref name) = arg.name {
            if let Some(prev_i) = seen.insert(name.as_str(), i) {
                return Err(BuclError::RuntimeError(format!(
                    "duplicate named parameter '{}' (args {} and {})",
                    name, prev_i, i
                )));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------

/// The runtime environment: variable store + function registry.
pub struct Evaluator {
    pub(crate) variables: HashMap<String, String>,
    functions: HashMap<String, Arc<dyn BuclFunction>>,
    /// Directory to resolve `functions/<name>.bucl` lookups against.
    /// Typically the directory containing the script being run.
    pub base_dir: Option<PathBuf>,
    /// Captured output lines.  Every call to `echo` appends here.
    /// On native targets the line is also printed to stdout immediately.
    pub output_buffer: Vec<String>,
    /// Pre-loaded BUCL function sources keyed by function name (no `.bucl`
    /// extension).  Checked before the filesystem so WASM builds can embed
    /// the standard library with `include_str!`.
    pub embedded_functions: HashMap<String, String>,
    /// Named arguments for the current function call.
    ///
    /// Set before each function dispatch, cleared afterward.  Built-in Rust
    /// functions can read these via [`named_arg`](Evaluator::named_arg).
    pub call_named_args: HashMap<String, String>,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            base_dir: None,
            output_buffer: Vec::new(),
            embedded_functions: HashMap::new(),
            call_named_args: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Function registry
    // -----------------------------------------------------------------------

    pub fn register<F: BuclFunction + 'static>(&mut self, name: &str, func: F) {
        self.functions.insert(name.to_string(), Arc::new(func));
    }

    // -----------------------------------------------------------------------
    // Named argument access (for built-in functions)
    // -----------------------------------------------------------------------

    /// Look up a named argument for the current function call.
    ///
    /// Built-in Rust functions can call this to check whether the caller
    /// passed a variable whose name (or last path segment) matches `name`.
    pub fn named_arg(&self, name: &str) -> Option<&String> {
        self.call_named_args.get(name)
    }

    // -----------------------------------------------------------------------
    // Variable access
    // -----------------------------------------------------------------------

    /// Store a value.
    ///
    /// For **root variables** (no `/` in the name) two metadata sub-variables
    /// are maintained automatically:
    /// - `{name/count}`  — number of string arguments that were assigned
    ///   (always `"1"` here; `assign` overrides this for multi-arg calls).
    /// - `{name/length}` — total character length of the stored value.
    ///
    /// Sub-variables (names that contain `/`) are stored as-is with no
    /// automatic metadata so that internal slots like `{r/index}` stay clean.
    pub fn set_var(&mut self, name: &str, value: String) {
        // Auto-maintain metadata only for root variables.
        if !name.contains('/') {
            let length = value.chars().count();
            self.variables.insert(format!("{}/length", name), length.to_string());
            self.variables.insert(format!("{}/count", name), "1".to_string());
        }
        self.variables.insert(name.to_string(), value);
    }

    /// Resolve a variable name, with automatic index-based fallback.
    ///
    /// Lookup order for `"var/N"` (where N is a non-negative integer):
    ///
    /// 1. **Direct lookup** — returns the value if `{var/N}` is explicitly set.
    ///    This covers `{r/index}`, `{e/value}`, `{parts/0}`, `{myvar/mysub}`, …
    ///
    /// 2. **Count-gated index fallback** — reads `{var/count}` to decide the
    ///    indexing mode:
    ///    - `count == 1` → **character indexing**: returns the Nth character of
    ///      `{var}`.  This is the single-string case (`{word} = "hello"`).
    ///    - `count > 1` → the indexed strings are already stored explicitly;
    ///      if the direct lookup above failed the index is out of range →
    ///      returns `""`.
    ///
    /// For non-numeric suffixes (e.g. `{r/index}`, `{myvar/label}`) step 2 is
    /// skipped and the result is `""` when the direct lookup misses.
    pub fn resolve_var(&self, name: &str) -> String {
        // 0. If the name itself contains nested variable refs (e.g. "var/{key}"),
        //    resolve them first via interpolation, then look up the resulting name.
        if name.contains('{') {
            let resolved = self.interpolate(name);
            return self.resolve_var(&resolved);
        }

        // 1. Direct lookup.
        if let Some(v) = self.variables.get(name) {
            return v.clone();
        }

        // 2. Index fallback — only for numeric suffixes after the first '/'.
        if let Some(slash) = name.find('/') {
            let parent = &name[..slash];
            let index_str = &name[slash + 1..];
            if let Ok(idx) = index_str.parse::<usize>() {
                let count: usize = self
                    .variables
                    .get(&format!("{}/count", parent))
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                if count == 1 {
                    // Single-string variable: return the character at position idx.
                    if let Some(value) = self.variables.get(parent) {
                        if let Some(ch) = value.chars().nth(idx) {
                            return ch.to_string();
                        }
                    }
                }
                // count > 1: strings were stored explicitly; missing index → "".
                // count == 0: variable not set → "".
            }
        }

        String::new()
    }

    // -----------------------------------------------------------------------
    // String interpolation
    // -----------------------------------------------------------------------

    pub fn interpolate(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c != '{' {
                result.push(c);
                continue;
            }
            let mut var_name = String::new();
            let mut closed = false;
            let mut depth = 1usize;
            for ch in chars.by_ref() {
                match ch {
                    '{' => { depth += 1; var_name.push('{'); }
                    '}' => {
                        depth -= 1;
                        if depth == 0 { closed = true; break; }
                        var_name.push('}');
                    }
                    _ => var_name.push(ch),
                }
            }
            if closed {
                result.push_str(&self.resolve_var_for_interpolation(&var_name));
            } else {
                result.push('{');
                result.push_str(&var_name);
            }
        }

        result
    }

    /// Resolve a variable reference that appears **inside a quoted string**.
    ///
    /// For root-level variables (no `/` after inner resolution) that hold
    /// **multiple strings** (`{var/count} > 1`), the elements are joined with
    /// a single space and returned as one string — matching the "auto-implode
    /// in string context" rule.
    ///
    /// For everything else (single-string variables, sub-variable paths,
    /// nested references that resolve to a sub-path) the call falls through
    /// to the normal [`resolve_var`] logic.
    fn resolve_var_for_interpolation(&self, name: &str) -> String {
        // First resolve any nested variable refs inside the name itself
        // (e.g. "parts/{i}" → "parts/2").
        let resolved_name = if name.contains('{') {
            self.interpolate(name)
        } else {
            name.to_string()
        };

        // Only apply auto-implode for root-level variable names (no '/').
        if !resolved_name.contains('/') {
            let count: usize = self
                .variables
                .get(&format!("{}/count", resolved_name))
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            if count > 1 {
                let parts: Vec<String> = (0..count)
                    .map(|i| {
                        self.variables
                            .get(&format!("{}/{}", resolved_name, i))
                            .cloned()
                            .unwrap_or_default()
                    })
                    .collect();
                return parts.join(" ");
            }
        }

        self.resolve_var(&resolved_name)
    }

    // -----------------------------------------------------------------------
    // Parameter evaluation
    // -----------------------------------------------------------------------

    pub fn eval_param(&self, param: &Param) -> String {
        match param {
            Param::Quoted(s) => self.interpolate(s),
            Param::Variable(name) => self.resolve_var(name),
            Param::Bare(s) => s.clone(),
        }
    }

    /// Evaluate a list of parameters into a flat `Vec<String>`.
    ///
    /// **Array expansion rule** — when a bare variable reference (`{var}`, not
    /// inside a quoted string) resolves to a multi-arg variable
    /// (`{var/count} > 1`), it is automatically **expanded** into as many
    /// separate arguments as there are stored elements.  This mirrors the
    /// behaviour of shell word splitting: the elements are *individually*
    /// passed to the callee rather than concatenated.
    ///
    /// ```bucl
    /// {colors} = "red" "green" "blue"
    /// # Direct use → three separate arguments:
    /// {joined} implode " / " {colors}   # same as implode " / " "red" "green" "blue"
    /// # Inside a string → single space-joined value (handled by interpolate):
    /// echo "colors: {colors}"           # prints: colors: red green blue
    /// ```
    pub fn eval_params(&self, params: &[Param]) -> Vec<String> {
        self.eval_params_with_names(params)
            .into_iter()
            .map(|ra| ra.value)
            .collect()
    }

    /// Find non-numeric, non-metadata sub-variables of `parent`.
    ///
    /// Used for **struct expansion**: when `{db}` is passed as an argument and
    /// `db/port`, `db/host` exist, those sub-variables are expanded as named
    /// parameters.
    fn find_named_sub_vars(&self, parent: &str) -> Vec<(String, String)> {
        let prefix = format!("{}/", parent);
        let mut result = Vec::new();
        for (key, value) in &self.variables {
            if let Some(suffix) = key.strip_prefix(&prefix) {
                // Skip nested sub-variables (e.g. "db/config/x" when parent is "db").
                if suffix.contains('/') {
                    continue;
                }
                // Skip metadata.
                if suffix == "count" || suffix == "length" {
                    continue;
                }
                // Skip numeric indices (from array assignment).
                if suffix.parse::<usize>().is_ok() {
                    continue;
                }
                result.push((suffix.to_string(), value.clone()));
            }
        }
        // Sort alphabetically for deterministic ordering.
        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    /// Evaluate parameters while preserving variable-name metadata.
    ///
    /// This is the name-aware version of [`eval_params`].  Each returned
    /// [`ResolvedArg`] carries an optional `name` derived from the source
    /// variable (last path segment).
    ///
    /// **Struct expansion** — if a root-level variable `{db}` has non-numeric,
    /// non-metadata sub-variables (e.g. `db/port`, `db/host`), passing `{db}`
    /// as an argument expands it into multiple named arguments:
    ///
    /// ```bucl
    /// {db/port} = "3308"
    /// {db/host} = "myserver"
    /// {r} connect {db}          # expands to connect host:"myserver" port:"3308"
    /// ```
    pub fn eval_params_with_names(&self, params: &[Param]) -> Vec<ResolvedArg> {
        let mut result = Vec::new();
        for p in params {
            match p {
                Param::Variable(name) => {
                    // Resolve any nested refs inside the name first.
                    let resolved_name = if name.contains('{') {
                        self.interpolate(name)
                    } else {
                        name.clone()
                    };

                    // Only expand root-level variable names (no '/').
                    if !resolved_name.contains('/') {
                        // Check for struct expansion first: named sub-variables.
                        let named_subs = self.find_named_sub_vars(&resolved_name);
                        if !named_subs.is_empty() {
                            for (suffix, value) in named_subs {
                                result.push(ResolvedArg {
                                    name: Some(suffix),
                                    value,
                                });
                            }
                            continue;
                        }

                        // Array expansion: count > 1 → expand numerically, no names.
                        let count: usize = self
                            .variables
                            .get(&format!("{}/count", resolved_name))
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);

                        if count > 1 {
                            for i in 0..count {
                                result.push(ResolvedArg {
                                    name: None,
                                    value: self
                                        .variables
                                        .get(&format!("{}/{}", resolved_name, i))
                                        .cloned()
                                        .unwrap_or_default(),
                                });
                            }
                            continue;
                        }
                    }

                    // Single value — carry the variable name.
                    result.push(ResolvedArg {
                        name: extract_param_name(&resolved_name),
                        value: self.resolve_var(name),
                    });
                }
                _ => {
                    result.push(ResolvedArg {
                        name: None,
                        value: self.eval_param(p),
                    });
                }
            }
        }
        result
    }

    // -----------------------------------------------------------------------
    // Execution
    // -----------------------------------------------------------------------

    pub fn evaluate_statements(&mut self, stmts: &[Statement]) -> Result<()> {
        for stmt in stmts {
            self.evaluate_statement(stmt)?;
        }
        Ok(())
    }

    pub fn evaluate_statement(&mut self, stmt: &Statement) -> Result<()> {
        // Resolve args with names preserved.
        let resolved = self.eval_params_with_names(&stmt.args);

        // Check for duplicate named parameters.
        check_duplicate_names(&resolved)?;

        // Extract flat values for built-in functions.
        let values: Vec<String> = resolved.iter().map(|a| a.value.clone()).collect();

        // Build named-args map and set on evaluator so built-in functions
        // can access them via `self.named_arg("name")`.
        let named: HashMap<String, String> = resolved
            .iter()
            .filter_map(|a| a.name.as_ref().map(|n| (n.clone(), a.value.clone())))
            .collect();
        self.call_named_args = named;

        // Resolve target name — supports nested variable refs like {var/{key}}.
        let resolved_target: Option<String> = stmt.target.as_ref().map(|t| {
            if t.contains('{') { self.interpolate(t) } else { t.clone() }
        });

        // 1. Try built-in Rust functions first.
        if let Some(func) = self.functions.get(&stmt.function).cloned() {
            let result = func.call(
                self,
                resolved_target.as_deref(),
                values,
                stmt.block.as_deref(),
                stmt.continuation.as_deref(),
            )?;
            self.call_named_args.clear();
            if let (Some(target), Some(value)) = (&resolved_target, result) {
                self.set_var(target, value);
            }
            return Ok(());
        }

        // 2. Fall back to a dynamically loaded .bucl function file.
        self.call_named_args.clear();
        let result = self.call_bucl_function(
            &stmt.function.clone(),
            resolved_target.as_deref(),
            resolved,
        )?;
        if let (Some(target), Some(value)) = (&resolved_target, result) {
            self.set_var(target, value);
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Dynamic .bucl function loading
    // -----------------------------------------------------------------------

    /// Search for a `.bucl` function by name.
    ///
    /// Lookup order:
    /// 1. `embedded_functions` map (used by WASM builds and for stdlib).
    /// 2. Filesystem: `functions/<name>.bucl` relative to `base_dir`, then CWD.
    ///    (skipped when targeting `wasm32`).
    fn find_bucl_function(&self, name: &str) -> Option<String> {
        // 1. Embedded (in-memory) registry — always checked first.
        if let Some(src) = self.embedded_functions.get(name) {
            return Some(src.clone());
        }

        // 2. Filesystem lookup — not available on WASM targets.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let filename = format!("{}.bucl", name);
            let mut candidates: Vec<PathBuf> = Vec::new();
            if let Some(base) = &self.base_dir {
                candidates.push(base.join("functions").join(&filename));
            }
            candidates.push(Path::new("functions").join(&filename));
            for path in candidates {
                if let Ok(source) = std::fs::read_to_string(&path) {
                    return Some(source);
                }
            }
        }

        None
    }

    /// Execute a `.bucl` function in an isolated child scope.
    ///
    /// ## Calling convention
    /// - Arguments are available as `{0}`, `{1}`, … inside the function.
    /// - Named arguments (derived from the caller's variable names) are also
    ///   injected: e.g. passing `{port}` makes `{port}` available by name.
    /// - `{argc}` holds the number of arguments.
    /// - `{target}` holds the caller's target variable name (if any).
    ///
    /// ## Return convention
    /// - Set `{return}` to return a single value.
    /// - Set `{return/0}`, `{return/1}`, … to return indexed sub-values;
    ///   these are copied to `{target/0}`, `{target/1}`, … in the caller's
    ///   scope automatically.
    fn call_bucl_function(
        &mut self,
        name: &str,
        target: Option<&str>,
        resolved_args: Vec<ResolvedArg>,
    ) -> Result<Option<String>> {
        let source = self
            .find_bucl_function(name)
            .ok_or_else(|| BuclError::UnknownFunction(name.to_string()))?;

        let stmts = crate::parser::parse(&source)?;

        // Build an isolated child evaluator that shares the function registry,
        // base_dir, and embedded_functions but has its own variable scope.
        let mut child = Evaluator::new();
        child.base_dir = self.base_dir.clone();
        child.embedded_functions = self.embedded_functions.clone();
        crate::functions::register_all(&mut child);

        // Extract string values for positional injection.
        let values: Vec<String> = resolved_args.iter().map(|a| a.value.clone()).collect();

        // Inject call arguments — bypass set_var to avoid spurious output.
        let argc = values.len();
        child.variables.insert("argc".to_string(), argc.to_string());
        for (i, val) in values.iter().enumerate() {
            child.variables.insert(i.to_string(), val.clone());
        }
        // Also expose arguments as a structured {args} variable so that BUCL
        // functions can use {args/{i}} for dynamic positional access without
        // needing the `getvar` built-in.
        child.variables.insert("args".to_string(), values.join(""));
        child
            .variables
            .insert("args/count".to_string(), argc.to_string());
        let args_length: usize = values.iter().map(|s| s.chars().count()).sum();
        child
            .variables
            .insert("args/length".to_string(), args_length.to_string());
        for (i, val) in values.iter().enumerate() {
            child.variables.insert(format!("args/{}", i), val.clone());
        }

        // Inject named parameters as variables in the child scope.
        for ra in &resolved_args {
            if let Some(ref param_name) = ra.name {
                child.variables.insert(param_name.clone(), ra.value.clone());
            }
        }

        if let Some(t) = target {
            child.variables.insert("target".to_string(), t.to_string());
        }

        child.evaluate_statements(&stmts)?;

        // Propagate any output the child produced into the parent buffer.
        self.output_buffer.append(&mut child.output_buffer);

        // Extract the primary return value.
        let return_val = child.variables.get("return").cloned();

        // Copy return value and indexed sub-variables to the caller's scope.
        //
        // Order matters: call set_var FIRST (which auto-sets count=1), then
        // copy sub-variables so that {return/count} etc. can override the
        // auto-metadata.  This allows BUCL functions to return arrays by
        // setting {return}, {return/count}, and {return/0}, {return/1}, …
        if let Some(prefix) = target {
            if let Some(ref val) = return_val {
                self.set_var(prefix, val.clone());
            }

            let sub_vars: Vec<(String, String)> = child
                .variables
                .iter()
                .filter(|(k, _)| k.starts_with("return/"))
                .map(|(k, v)| {
                    let suffix = &k["return/".len()..];
                    (format!("{}/{}", prefix, suffix), v.clone())
                })
                .collect();
            for (key, val) in sub_vars {
                self.variables.insert(key, val);
            }

            // We handled set_var ourselves; return None so evaluate_statement
            // does not call set_var again.
            Ok(None)
        } else {
            Ok(return_val)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_param_name_root() {
        assert_eq!(extract_param_name("port"), Some("port".to_string()));
        assert_eq!(extract_param_name("host"), Some("host".to_string()));
    }

    #[test]
    fn test_extract_param_name_sub_variable() {
        assert_eq!(extract_param_name("db/port"), Some("port".to_string()));
        assert_eq!(extract_param_name("config/db/host"), Some("host".to_string()));
    }

    #[test]
    fn test_extract_param_name_numeric() {
        assert_eq!(extract_param_name("0"), None);
        assert_eq!(extract_param_name("42"), None);
        assert_eq!(extract_param_name("args/0"), None);
    }

    #[test]
    fn test_extract_param_name_reserved() {
        assert_eq!(extract_param_name("argc"), None);
        assert_eq!(extract_param_name("args"), None);
        assert_eq!(extract_param_name("target"), None);
        assert_eq!(extract_param_name("return"), None);
        assert_eq!(extract_param_name("count"), None);
        assert_eq!(extract_param_name("length"), None);
    }

    #[test]
    fn test_extract_param_name_empty() {
        assert_eq!(extract_param_name(""), None);
    }

    #[test]
    fn test_find_named_sub_vars() {
        let mut eval = Evaluator::new();
        eval.variables.insert("db/port".to_string(), "3308".to_string());
        eval.variables.insert("db/host".to_string(), "myserver".to_string());
        eval.variables.insert("db/count".to_string(), "1".to_string());
        eval.variables.insert("db/length".to_string(), "5".to_string());
        eval.variables.insert("db/0".to_string(), "zero".to_string());
        eval.variables.insert("db/nested/deep".to_string(), "skip".to_string());

        let subs = eval.find_named_sub_vars("db");
        assert_eq!(subs, vec![
            ("host".to_string(), "myserver".to_string()),
            ("port".to_string(), "3308".to_string()),
        ]);
    }

    #[test]
    fn test_check_duplicate_names_ok() {
        let args = vec![
            ResolvedArg { name: Some("host".to_string()), value: "a".to_string() },
            ResolvedArg { name: Some("port".to_string()), value: "b".to_string() },
            ResolvedArg { name: None, value: "c".to_string() },
        ];
        assert!(check_duplicate_names(&args).is_ok());
    }

    #[test]
    fn test_check_duplicate_names_error() {
        let args = vec![
            ResolvedArg { name: Some("port".to_string()), value: "a".to_string() },
            ResolvedArg { name: Some("port".to_string()), value: "b".to_string() },
        ];
        assert!(check_duplicate_names(&args).is_err());
    }
}
