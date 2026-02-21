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
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            functions: HashMap::new(),
            base_dir: None,
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
    pub fn set_var(&mut self, name: &str, value: String) {
        if name == "output" {
            println!("{}", value);
        }
        self.variables.insert(name.to_string(), value);
    }

    /// Retrieve a value; returns `""` for undefined variables.
    pub fn get_var(&self, name: &str) -> &str {
        self.variables.get(name).map(String::as_str).unwrap_or("")
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
            for ch in chars.by_ref() {
                if ch == '}' {
                    closed = true;
                    break;
                }
                var_name.push(ch);
            }
            if closed {
                result.push_str(self.get_var(&var_name));
            } else {
                result.push('{');
                result.push_str(&var_name);
            }
        }

        result
    }

    // -----------------------------------------------------------------------
    // Parameter evaluation
    // -----------------------------------------------------------------------

    pub fn eval_param(&self, param: &Param) -> String {
        match param {
            Param::Quoted(s) => self.interpolate(s),
            Param::Variable(name) => self.get_var(name).to_string(),
            Param::Bare(s) => s.clone(),
        }
    }

    pub fn eval_params(&self, params: &[Param]) -> Vec<String> {
        params.iter().map(|p| self.eval_param(p)).collect()
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

        // 1. Try built-in Rust functions first.
        if let Some(func) = self.functions.get(&stmt.function).cloned() {
            let result = func.call(
                self,
                stmt.target.as_deref(),
                args,
                stmt.block.as_deref(),
                stmt.continuation.as_deref(),
            )?;
            if let (Some(target), Some(value)) = (&stmt.target, result) {
                self.set_var(target, value);
            }
            return Ok(());
        }

        // 2. Fall back to a dynamically loaded .bucl function file.
        let result = self.call_bucl_function(
            &stmt.function.clone(),
            stmt.target.as_deref(),
            args,
        )?;
        if let (Some(target), Some(value)) = (&stmt.target, result) {
            self.set_var(target, value);
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Dynamic .bucl function loading
    // -----------------------------------------------------------------------

    /// Search for `functions/<name>.bucl` relative to `base_dir` (the script's
    /// directory) and then relative to the current working directory.
    fn find_bucl_function(&self, name: &str) -> Option<String> {
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

        // Build an isolated child evaluator that shares the function registry
        // and base_dir but has its own variable scope.
        let mut child = Evaluator::new();
        child.base_dir = self.base_dir.clone();
        crate::functions::register_all(&mut child);

        // Inject call arguments — bypass set_var to avoid spurious output.
        child.variables.insert("argc".to_string(), args.len().to_string());
        for (i, arg) in args.iter().enumerate() {
            child.variables.insert(i.to_string(), arg.clone());
        }
        if let Some(t) = target {
            child.variables.insert("target".to_string(), t.to_string());
        }

        child.evaluate_statements(&stmts)?;

        // Extract the primary return value.
        let return_val = child.variables.get("return").cloned();

        // Copy indexed sub-variables {return/N} → {target/N} in caller scope.
        if let Some(prefix) = target {
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
        }

        Ok(return_val)
    }
}
