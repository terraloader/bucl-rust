use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::ast::{Param, Statement};
use crate::error::{BuclError, Result};
use crate::functions::BuclFunction;

/// The runtime environment: variable store + function registry.
pub struct Evaluator {
    pub(crate) variables: HashMap<String, String>,
    functions: HashMap<String, Arc<dyn BuclFunction>>,
    /// Directory to resolve `functions/<name>.bucl` lookups against.
    /// Typically the directory containing the script being run.
    pub base_dir: Option<PathBuf>,
    /// Captured output lines.  Every assignment to `{output}` appends here.
    /// On native targets the line is also printed to stdout.
    pub output_buffer: Vec<String>,
    /// Pre-loaded BUCL function sources keyed by function name (no `.bucl`
    /// extension).  Checked before the filesystem so WASM builds can embed
    /// the standard library with `include_str!`.
    pub embedded_functions: HashMap<String, String>,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            base_dir: None,
            output_buffer: Vec::new(),
            embedded_functions: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Function registry
    // -----------------------------------------------------------------------

    pub fn register<F: BuclFunction + 'static>(&mut self, name: &str, func: F) {
        self.functions.insert(name.to_string(), Arc::new(func));
    }

    // -----------------------------------------------------------------------
    // Variable access
    // -----------------------------------------------------------------------

    /// Store a value.  Writing to `output` also prints to stdout.
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
        if name == "output" {
            self.output_buffer.push(value.clone());
            #[cfg(not(target_arch = "wasm32"))]
            println!("{}", value);
        }
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
    /// {output} = "colors: {colors}"     # prints: colors: red green blue
    /// ```
    pub fn eval_params(&self, params: &[Param]) -> Vec<String> {
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
                        let count: usize = self
                            .variables
                            .get(&format!("{}/count", resolved_name))
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0);

                        if count > 1 {
                            // Expand to individual elements.
                            for i in 0..count {
                                result.push(
                                    self.variables
                                        .get(&format!("{}/{}", resolved_name, i))
                                        .cloned()
                                        .unwrap_or_default(),
                                );
                            }
                            continue;
                        }
                    }

                    result.push(self.resolve_var(name));
                }
                _ => result.push(self.eval_param(p)),
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
        let args = self.eval_params(&stmt.args);

        // Resolve target name — supports nested variable refs like {var/{key}}.
        let resolved_target: Option<String> = stmt.target.as_ref().map(|t| {
            if t.contains('{') { self.interpolate(t) } else { t.clone() }
        });

        // 1. Try built-in Rust functions first.
        if let Some(func) = self.functions.get(&stmt.function).cloned() {
            let result = func.call(
                self,
                resolved_target.as_deref(),
                args,
                stmt.block.as_deref(),
                stmt.continuation.as_deref(),
            )?;
            if let (Some(target), Some(value)) = (&resolved_target, result) {
                self.set_var(target, value);
            }
            return Ok(());
        }

        // 2. Fall back to a dynamically loaded .bucl function file.
        let result = self.call_bucl_function(
            &stmt.function.clone(),
            resolved_target.as_deref(),
            args,
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
    /// - `{argc}` holds the number of arguments.
    /// - `{target}` holds the caller's target variable name (if any).
    ///
    /// ## Return convention
    /// - Set `{return}` to return a single value.
    /// - Set `{return/0}`, `{return/1}`, … (via `setvar`) to return indexed
    ///   sub-values; these are copied to `{target/0}`, `{target/1}`, … in the
    ///   caller's scope automatically.
    fn call_bucl_function(
        &mut self,
        name: &str,
        target: Option<&str>,
        args: Vec<String>,
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

        // Inject call arguments — bypass set_var to avoid spurious output.
        let argc = args.len();
        child.variables.insert("argc".to_string(), argc.to_string());
        for (i, arg) in args.iter().enumerate() {
            child.variables.insert(i.to_string(), arg.clone());
        }
        // Also expose arguments as a structured {args} variable so that BUCL
        // functions can use {args/{i}} for dynamic positional access without
        // needing the `getvar` built-in.
        child.variables.insert("args".to_string(), args.join(""));
        child
            .variables
            .insert("args/count".to_string(), argc.to_string());
        let args_length: usize = args.iter().map(|s| s.chars().count()).sum();
        child
            .variables
            .insert("args/length".to_string(), args_length.to_string());
        for (i, arg) in args.iter().enumerate() {
            child.variables.insert(format!("args/{}", i), arg.clone());
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
                // Bypass set_var: {output/N} shouldn't trigger print.
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
