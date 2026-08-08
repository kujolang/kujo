use crate::ast::{Expr, Pattern, Stmt};
use crate::errors::{ErrorKind, KujoError};
use crate::interpreter::{Environment, Interpreter, Value};
use crate::lexer::tokenize_with_file;
use crate::parser::Parser;
use crate::path_security;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Represents a loaded module with its exported symbols.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub exports: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ModuleCacheKey {
    package_root: PathBuf,
    module_path: PathBuf,
}

#[derive(Debug, Clone)]
struct CachedModule {
    module: Module,
    source_state: ModuleSourceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModuleSourceState {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone)]
struct LoadingModule {
    module_name: String,
    cache_key: ModuleCacheKey,
}

#[derive(Debug, Clone)]
struct ResolvedModulePath {
    module_path: PathBuf,
    cache_key: ModuleCacheKey,
}

impl ModuleSourceState {
    fn from_path(path: &Path) -> Result<Self, Box<KujoError>> {
        let metadata = fs::metadata(path).map_err(|error| {
            ModuleLoader::runtime_error(format!(
                "Failed to read module metadata '{}': {}",
                path.display(),
                error
            ))
        })?;

        let modified = metadata.modified().ok();

        Ok(Self { modified, len: metadata.len() })
    }
}

/// Manages module loading, caching, and resolution.
pub struct ModuleLoader {
    /// Cache of loaded modules to avoid re-parsing.
    loaded_modules: HashMap<ModuleCacheKey, CachedModule>,
    /// Stack of modules currently being loaded (for circular import detection).
    loading_stack: Vec<LoadingModule>,
    /// O(1) index for modules currently being loaded.
    loading_stack_index: HashMap<ModuleCacheKey, usize>,
    /// Search paths for module resolution.
    search_paths: Vec<PathBuf>,
}

impl ModuleLoader {
    fn runtime_error(message: String) -> Box<KujoError> {
        Box::new(KujoError::new(
            ErrorKind::RuntimeError,
            message,
            crate::errors::SourceLocation::unknown(),
        ))
    }

    fn runtime_error_with_help(message: String, help: String) -> Box<KujoError> {
        Box::new(
            KujoError::new(
                ErrorKind::RuntimeError,
                message,
                crate::errors::SourceLocation::unknown(),
            )
            .with_help(help),
        )
    }

    fn collect_pattern_bindings(pattern: &Pattern, names: &mut Vec<String>) {
        match pattern {
            Pattern::Identifier(name) => names.push(name.clone()),
            Pattern::Array { elements, rest } => {
                for element in elements {
                    Self::collect_pattern_bindings(element, names);
                }
                if let Some(rest_name) = rest {
                    names.push(rest_name.clone());
                }
            }
            Pattern::Dict { keys, rest } => {
                names.extend(keys.iter().cloned());
                if let Some(rest_name) = rest {
                    names.push(rest_name.clone());
                }
            }
            Pattern::Ignore => {}
        }
    }

    fn collect_export_bindings(stmt: &Stmt, names: &mut Vec<String>) {
        match stmt {
            Stmt::Let { pattern, .. } => Self::collect_pattern_bindings(pattern, names),
            Stmt::Const { name, .. }
            | Stmt::FuncDef { name, .. }
            | Stmt::StructDef { name, .. } => names.push(name.clone()),
            Stmt::EnumDef { name, variants } => {
                for variant in variants {
                    names.push(format!("{}::{}", name, variant));
                }
            }
            Stmt::Assign { target, .. } => {
                if let Expr::Identifier(name) = target {
                    names.push(name.clone());
                }
            }
            Stmt::ExprStmt(Expr::Identifier(name)) => names.push(name.clone()),
            Stmt::Block(stmts) => {
                for nested_stmt in stmts {
                    Self::collect_export_bindings(nested_stmt, names);
                }
            }
            Stmt::Export { stmt } => Self::collect_export_bindings(stmt, names),
            _ => {}
        }
    }

    fn collect_exported_symbol_names(program: &[Stmt]) -> Vec<String> {
        let mut names = Vec::new();
        for stmt in program {
            if let Stmt::Export { stmt } = stmt {
                Self::collect_export_bindings(stmt, &mut names);
            }
        }

        let mut deduped = Vec::new();
        let mut seen = HashSet::new();
        for name in names {
            if seen.insert(name.clone()) {
                deduped.push(name);
            }
        }

        deduped
    }

    fn bind_export_value_to_module_env(
        value: Value,
        module_env: &Arc<Mutex<Environment>>,
    ) -> Value {
        match value {
            Value::Function(params, body, captured_env) => {
                let bound_env = captured_env.or_else(|| Some(Arc::clone(module_env)));
                Value::Function(params, body, bound_env)
            }
            Value::AsyncFunction(params, body, captured_env) => {
                let bound_env = captured_env.or_else(|| Some(Arc::clone(module_env)));
                Value::AsyncFunction(params, body, bound_env)
            }
            Value::StructDef { name, field_names, methods } => {
                let mut bound_methods = HashMap::new();
                for (method_name, method_value) in methods {
                    bound_methods.insert(
                        method_name,
                        Self::bind_export_value_to_module_env(method_value, module_env),
                    );
                }
                Value::StructDef { name, field_names, methods: bound_methods }
            }
            other => other,
        }
    }

    /// Creates a new module loader with default search paths.
    pub fn new() -> Self {
        let mut search_paths = vec![PathBuf::from("."), PathBuf::from("./modules")];
        search_paths.extend(configured_module_search_paths());
        ModuleLoader {
            loaded_modules: HashMap::new(),
            loading_stack: Vec::new(),
            loading_stack_index: HashMap::new(),
            search_paths,
        }
    }

    /// Adds a search path for module resolution.
    #[allow(dead_code)]
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.search_paths.push(path.as_ref().to_path_buf());
    }

    fn module_search_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();

        if let Some(active_module) = self.loading_stack.last() {
            roots.push(active_module.cache_key.package_root.clone());
        }

        roots.extend(self.search_paths.iter().cloned());
        roots
    }

    fn module_resolution_candidates(
        &self,
        module_name: &str,
    ) -> Result<Vec<PathBuf>, Box<KujoError>> {
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();

        let flat_filename = format!("{}.kujo", module_name);
        let normalized_flat = path_security::sanitize_relative_path(
            &flat_filename,
            "module import",
        )
        .map_err(|error| {
            Self::runtime_error(format!("Unsafe module import '{}': {}", module_name, error))
        })?;
        if seen.insert(normalized_flat.clone()) {
            candidates.push(normalized_flat);
        }

        if module_name.contains('.') {
            let segments: Vec<&str> = module_name.split('.').collect();
            if segments.iter().any(|segment| segment.is_empty()) {
                return Err(Self::runtime_error(format!(
                    "Unsafe module import '{}': dotted module path contains an empty segment",
                    module_name
                )));
            }

            let nested_filename = format!("{}.kujo", segments.join("/"));
            let normalized_nested = path_security::sanitize_relative_path(
                &nested_filename,
                "module import",
            )
            .map_err(|error| {
                Self::runtime_error(format!("Unsafe module import '{}': {}", module_name, error))
            })?;

            if seen.insert(normalized_nested.clone()) {
                candidates.push(normalized_nested);
            }
        }

        Ok(candidates)
    }

    fn missing_module_help(module_name: &str) -> String {
        format!(
            "Check that '{}' exists as a flat <module>.kujo file or a nested src/... path under the package root, and confirm the import name matches the on-disk layout.",
            module_name
        )
    }

    /// Resolves a module name to a file path.
    fn resolve_module_path(
        &self,
        module_name: &str,
    ) -> Result<Option<ResolvedModulePath>, Box<KujoError>> {
        let resolution_candidates = self.module_resolution_candidates(module_name)?;

        let mut visited_roots = HashSet::new();

        for search_path in self.module_search_roots() {
            let canonical_search_root =
                match path_security::canonicalize_root(&search_path, "module search path") {
                    Ok(path) => path,
                    Err(_) => continue,
                };

            if !visited_roots.insert(canonical_search_root.clone()) {
                continue;
            }

            for normalized_filename in &resolution_candidates {
                let full_path = canonical_search_root.join(normalized_filename);
                if full_path.exists() {
                    let canonical_module_path = fs::canonicalize(&full_path).map_err(|error| {
                        Self::runtime_error(format!(
                            "Failed to resolve module '{}' path '{}': {}",
                            module_name,
                            full_path.display(),
                            error
                        ))
                    })?;

                    if path_security::ensure_path_within_root(
                        &canonical_module_path,
                        &canonical_search_root,
                        "module import path",
                    )
                    .is_err()
                    {
                        return Err(Self::runtime_error(format!(
                            "Unsafe module import '{}': resolved path '{}' escapes module search root '{}'",
                            module_name,
                            canonical_module_path.display(),
                            canonical_search_root.display()
                        )));
                    }

                    return Ok(Some(ResolvedModulePath {
                        module_path: canonical_module_path.clone(),
                        cache_key: ModuleCacheKey {
                            package_root: canonical_search_root.clone(),
                            module_path: canonical_module_path,
                        },
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Loads a module by name, returning cached version if available.
    pub fn load_module(&mut self, module_name: &str) -> Result<Module, Box<KujoError>> {
        let resolved_module = self.resolve_module_path(module_name)?.ok_or_else(|| {
            let help = Self::missing_module_help(module_name);
            Self::runtime_error_with_help(
                format!("Module not found: {}; {}", module_name, help),
                help,
            )
        })?;
        let cache_key = resolved_module.cache_key.clone();

        if let Some(cycle_start) = self.loading_stack_index.get(&cache_key).copied() {
            let mut import_chain: Vec<String> = self.loading_stack[cycle_start..]
                .iter()
                .map(|entry| entry.module_name.clone())
                .collect();
            import_chain.push(module_name.to_string());
            return Err(Self::runtime_error(format!(
                "Circular import detected: {}",
                import_chain.join(" -> ")
            )));
        }

        let current_source_state = ModuleSourceState::from_path(&resolved_module.module_path)?;

        if let Some(cached_module) = self.loaded_modules.get(&cache_key) {
            if cached_module.source_state == current_source_state {
                return Ok(cached_module.module.clone());
            }
        }

        self.loaded_modules.remove(&cache_key);

        let loading_stack_position = self.loading_stack.len();
        self.loading_stack.push(LoadingModule {
            module_name: module_name.to_string(),
            cache_key: cache_key.clone(),
        });
        self.loading_stack_index.insert(cache_key.clone(), loading_stack_position);

        let load_result = (|| {
            let module_path = resolved_module.module_path.clone();
            let source = fs::read_to_string(&module_path).map_err(|e| {
                Self::runtime_error(format!("Failed to read module '{}': {}", module_name, e))
            })?;

            let tokens = tokenize_with_file(&source, Some(&module_path.to_string_lossy()))
                .map_err(|diagnostics| {
                    let first = diagnostics
                        .first()
                        .map(|diagnostic| {
                            format!(
                                "{}:{}: {}",
                                diagnostic.line, diagnostic.column, diagnostic.message
                            )
                        })
                        .unwrap_or_else(|| "unknown lexer error".to_string());
                    Self::runtime_error(format!(
                        "Failed to tokenize module '{}': {}",
                        module_name, first
                    ))
                })?;
            let mut parser = Parser::new(tokens);
            let parse_output = parser.parse_with_diagnostics();
            if !parse_output.diagnostics.is_empty() {
                let first = parse_output
                    .diagnostics
                    .first()
                    .map(|diagnostic| {
                        format!("{}:{}: {}", diagnostic.line, diagnostic.column, diagnostic.message)
                    })
                    .unwrap_or_else(|| "unknown parser error".to_string());
                return Err(Self::runtime_error(format!(
                    "Failed to parse module '{}': {}",
                    module_name, first
                )));
            }
            let program = parse_output.stmts;
            let export_names = Self::collect_exported_symbol_names(&program);

            let mut interpreter = Interpreter::new();
            let mut active_loader = std::mem::take(self);
            if let Some(parent) = module_path.parent() {
                active_loader.add_search_path(parent);
            }
            interpreter.module_loader = active_loader;
            interpreter.source_file = Some(module_path.to_string_lossy().to_string());
            interpreter.eval_stmts(&program);

            *self = std::mem::take(&mut interpreter.module_loader);

            if let Some(return_value) = interpreter.return_value.take() {
                match return_value {
                    Value::Error(message) => {
                        return Err(Self::runtime_error(format!(
                            "Failed to evaluate module '{}': {}",
                            module_name, message
                        )));
                    }
                    Value::ErrorObject { message, .. } => {
                        return Err(Self::runtime_error(format!(
                            "Failed to evaluate module '{}': {}",
                            module_name, message
                        )));
                    }
                    _ => {}
                }
            }

            let module_env = Arc::new(Mutex::new(interpreter.env.clone()));

            let mut exports = HashMap::new();
            for export_name in export_names {
                if let Some(value) = interpreter.env.get(&export_name) {
                    let bound_value = Self::bind_export_value_to_module_env(value, &module_env);
                    exports.insert(export_name, bound_value);
                } else {
                    return Err(Self::runtime_error(format!(
                        "Exported symbol '{}' was not defined in module '{}'",
                        export_name, module_name
                    )));
                }
            }

            Ok(Module { name: module_name.to_string(), path: module_path.clone(), exports })
        })();

        if let Some(loading_module) = self.loading_stack.pop() {
            self.loading_stack_index.remove(&loading_module.cache_key);
        }

        let module = load_result?;
        let source_state = ModuleSourceState::from_path(&module.path)?;
        self.loaded_modules
            .insert(cache_key, CachedModule { module: module.clone(), source_state });

        Ok(module)
    }

    /// Gets a specific symbol from a module.
    pub fn get_symbol(
        &mut self,
        module_name: &str,
        symbol_name: &str,
    ) -> Result<Value, Box<KujoError>> {
        let module = self.load_module(module_name)?;

        module.exports.get(symbol_name).cloned().ok_or_else(|| {
            Box::new(KujoError::new(
                ErrorKind::RuntimeError,
                format!("Symbol '{}' not found in module '{}'", symbol_name, module_name),
                crate::errors::SourceLocation::unknown(),
            ))
        })
    }

    /// Gets all exports from a module.
    pub fn get_all_exports(
        &mut self,
        module_name: &str,
    ) -> Result<HashMap<String, Value>, Box<KujoError>> {
        let module = self.load_module(module_name)?;
        Ok(module.exports.clone())
    }
}

/// Returns additional module roots configured for the current process.
///
/// The path-list environment variable lets an installed Kujo executable use
/// first-party source modules without requiring every consumer repository to
/// vendor a copy. Local project roots remain ahead of these shared roots.
pub fn configured_module_search_paths() -> Vec<PathBuf> {
    std::env::var_os("KUJO_MODULE_PATH")
        .map(|paths| std::env::split_paths(&paths).collect())
        .unwrap_or_default()
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;

    fn unique_name(prefix: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        format!("{}_{}_{}", prefix, std::process::id(), nanos)
    }

    #[test]
    fn load_module_collects_explicit_exports_only() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_exports"));
        fs::create_dir_all(&temp_root).expect("failed to create temp module dir");

        let module_name = unique_name("math_mod");
        let module_path = temp_root.join(format!("{}.kujo", module_name));
        fs::write(&module_path, "helper := 10\nexport visible := helper + 5\nhidden := 99\n")
            .expect("failed to write module source");

        loader.add_search_path(&temp_root);

        let exports =
            loader.get_all_exports(&module_name).expect("module should load and return exports");

        assert!(matches!(exports.get("visible"), Some(Value::Int(15))));
        assert!(exports.get("helper").is_none());
        assert!(exports.get("hidden").is_none());

        fs::remove_file(&module_path).expect("failed to clean up module file");
        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[test]
    fn load_module_uses_configured_environment_search_path() {
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_env_path"));
        fs::create_dir_all(&temp_root).expect("failed to create module root");
        let module_name = unique_name("shared_cli");
        fs::write(temp_root.join(format!("{}.kujo", module_name)), "export answer := 42\n")
            .expect("failed to write shared module");

        std::env::set_var("KUJO_MODULE_PATH", &temp_root);
        let result = ModuleLoader::new()
            .get_symbol(&module_name, "answer")
            .expect("module should resolve from KUJO_MODULE_PATH");
        std::env::remove_var("KUJO_MODULE_PATH");

        assert!(matches!(result, Value::Int(42)));
        fs::remove_dir_all(&temp_root).expect("failed to clean up module root");
    }

    #[test]
    fn get_symbol_reports_missing_symbol_deterministically() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_missing_symbol"));
        fs::create_dir_all(&temp_root).expect("failed to create temp module dir");

        let module_name = unique_name("missing_symbol_mod");
        let module_path = temp_root.join(format!("{}.kujo", module_name));
        fs::write(&module_path, "export present := 1\n").expect("failed to write module source");

        loader.add_search_path(&temp_root);

        let err =
            loader.get_symbol(&module_name, "absent").expect_err("expected missing symbol error");

        assert!(err
            .message
            .contains(&format!("Symbol 'absent' not found in module '{}'", module_name)));

        fs::remove_file(&module_path).expect("failed to clean up module file");
        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[test]
    fn load_module_detects_circular_imports() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_circular"));
        fs::create_dir_all(&temp_root).expect("failed to create temp module dir");

        let module_a = unique_name("module_a");
        let module_b = unique_name("module_b");
        let module_a_path = temp_root.join(format!("{}.kujo", module_a));
        let module_b_path = temp_root.join(format!("{}.kujo", module_b));

        fs::write(&module_a_path, format!("import {}\nexport a := 1\n", module_b))
            .expect("failed to write module A");
        fs::write(&module_b_path, format!("import {}\nexport b := 2\n", module_a))
            .expect("failed to write module B");

        loader.add_search_path(&temp_root);

        let err = loader.get_all_exports(&module_a).expect_err("expected circular import failure");

        assert!(
            err.message.contains(&format!(
                "Circular import detected: {} -> {} -> {}",
                module_a, module_b, module_a
            )),
            "expected circular import chain, got: {}",
            err.message
        );
        assert!(loader.loading_stack.is_empty(), "loading stack should be cleared after errors");
        assert!(
            loader.loading_stack_index.is_empty(),
            "loading stack index should be cleared after errors"
        );

        fs::remove_file(&module_a_path).expect("failed to clean up module A");
        fs::remove_file(&module_b_path).expect("failed to clean up module B");
        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[test]
    fn load_module_deep_chain_completes_and_clears_loading_index() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_deep_chain"));
        fs::create_dir_all(&temp_root).expect("failed to create temp module dir");

        let module_count = 12usize;
        let mut names = Vec::with_capacity(module_count);
        for index in 0..module_count {
            names.push(unique_name(&format!("chain_{index}")));
        }

        for index in 0..module_count {
            let module_path = temp_root.join(format!("{}.kujo", names[index]));
            let body = if index == 0 {
                "export depth := 0\n".to_string()
            } else {
                format!("from {} import depth\nexport depth := depth + 1\n", names[index - 1])
            };
            fs::write(&module_path, body).expect("failed to write chain module");
        }

        loader.add_search_path(&temp_root);

        let value = loader
            .get_symbol(names.last().expect("deep chain should have a final module"), "depth")
            .expect("expected deep chain module to load");

        assert!(
            matches!(value, Value::Int(v) if v == (module_count as i64 - 1)),
            "expected deep chain export depth {}, got {:?}",
            module_count - 1,
            value
        );
        assert!(
            loader.loading_stack.is_empty(),
            "loading stack should be empty after successful deep-chain load"
        );
        assert!(
            loader.loading_stack_index.is_empty(),
            "loading stack index should be empty after successful deep-chain load"
        );

        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[test]
    fn load_module_rejects_parent_traversal_module_name() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_traversal_reject"));
        fs::create_dir_all(&temp_root).expect("failed to create temp module dir");
        loader.add_search_path(&temp_root);

        let err = loader
            .get_all_exports("../outside")
            .expect_err("expected traversal module name to fail");

        assert!(
            err.message.contains("Unsafe module import '../outside'"),
            "expected unsafe traversal error, got: {}",
            err.message
        );

        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[test]
    fn load_module_prefers_importer_package_root_for_relative_imports() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_relative_root"));
        let package_a = temp_root.join("package_a");
        let package_b = temp_root.join("package_b");
        fs::create_dir_all(&package_a).expect("failed to create package A");
        fs::create_dir_all(&package_b).expect("failed to create package B");

        let module_shared = unique_name("shared");
        let module_entry_a = unique_name("entry_a");
        let module_entry_b = unique_name("entry_b");

        fs::write(package_a.join(format!("{}.kujo", module_shared)), "export shared_value := 11\n")
            .expect("failed to write package A shared module");
        fs::write(package_b.join(format!("{}.kujo", module_shared)), "export shared_value := 22\n")
            .expect("failed to write package B shared module");

        fs::write(
            package_a.join(format!("{}.kujo", module_entry_a)),
            format!(
                "from {} import shared_value\nexport package_value := shared_value\n",
                module_shared
            ),
        )
        .expect("failed to write package A entry module");
        fs::write(
            package_b.join(format!("{}.kujo", module_entry_b)),
            format!(
                "from {} import shared_value\nexport package_value := shared_value\n",
                module_shared
            ),
        )
        .expect("failed to write package B entry module");

        loader.add_search_path(&package_a);
        loader.add_search_path(&package_b);

        let package_a_value = loader
            .get_symbol(&module_entry_a, "package_value")
            .expect("expected package A import to succeed");
        assert!(
            matches!(package_a_value, Value::Int(11)),
            "expected package A shared value 11, got: {:?}",
            package_a_value
        );

        let package_b_value = loader
            .get_symbol(&module_entry_b, "package_value")
            .expect("expected package B import to succeed");
        assert!(
            matches!(package_b_value, Value::Int(22)),
            "expected package B shared value 22, got: {:?}",
            package_b_value
        );

        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[test]
    fn load_module_reloads_when_module_source_changes() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_cache_invalidation"));
        fs::create_dir_all(&temp_root).expect("failed to create temp module dir");

        let module_name = unique_name("cache_target");
        let module_path = temp_root.join(format!("{}.kujo", module_name));
        fs::write(&module_path, "export current := 1\n")
            .expect("failed to write initial module source");

        loader.add_search_path(&temp_root);

        let initial_value =
            loader.get_symbol(&module_name, "current").expect("expected initial module export");
        assert!(
            matches!(initial_value, Value::Int(1)),
            "expected initial export value 1, got: {:?}",
            initial_value
        );

        fs::write(&module_path, "export current := 222\n").expect("failed to update module source");

        let updated_value = loader
            .get_symbol(&module_name, "current")
            .expect("expected module export after source update");
        assert!(
            matches!(updated_value, Value::Int(222)),
            "expected updated export value 222 after cache invalidation, got: {:?}",
            updated_value
        );

        fs::remove_file(&module_path).expect("failed to clean up module file");
        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[test]
    fn load_module_resolves_dotted_from_import_name_to_nested_module_path() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_dotted_nested"));
        let nested_dir = temp_root.join("src").join("core");
        fs::create_dir_all(&nested_dir).expect("failed to create nested module dir");

        let module_path = nested_dir.join("math.kujo");
        fs::write(&module_path, "export answer := 42\n").expect("failed to write nested module");

        loader.add_search_path(&temp_root);

        let answer = loader
            .get_symbol("src.core.math", "answer")
            .expect("expected dotted module name to resolve to nested module");
        assert!(matches!(answer, Value::Int(42)), "expected dotted import export value 42");

        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[test]
    fn load_module_dotted_name_resolution_prefers_legacy_flat_filename_before_nested_path() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_dotted_precedence"));
        let nested_dir = temp_root.join("src").join("core");
        fs::create_dir_all(&nested_dir).expect("failed to create nested module dir");

        let dotted_module_name = "src.core.math";
        let flat_module_path = temp_root.join(format!("{}.kujo", dotted_module_name));
        let nested_module_path = nested_dir.join("math.kujo");
        fs::write(&flat_module_path, "export source := \"flat\"\n")
            .expect("failed to write flat dotted module file");
        fs::write(&nested_module_path, "export source := \"nested\"\n")
            .expect("failed to write nested dotted module file");

        loader.add_search_path(&temp_root);

        let source = loader
            .get_symbol(dotted_module_name, "source")
            .expect("expected dotted module resolution to return a source marker");
        assert!(
            matches!(source, Value::Str(ref value) if value.as_ref() == "flat"),
            "expected flat dotted module path to win precedence, got: {:?}",
            source
        );

        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[test]
    fn load_module_reuses_cached_nested_dotted_module_without_duplicate_cache_entries() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_dotted_cache_reuse"));
        let nested_dir = temp_root.join("src").join("core");
        fs::create_dir_all(&nested_dir).expect("failed to create nested module dir");

        let module_path = nested_dir.join("math.kujo");
        fs::write(&module_path, "export answer := 42\n").expect("failed to write nested module");

        loader.add_search_path(&temp_root);

        let first = loader
            .get_symbol("src.core.math", "answer")
            .expect("expected first dotted lookup to succeed");
        assert!(matches!(first, Value::Int(42)), "expected first lookup to return 42");
        assert_eq!(
            loader.loaded_modules.len(),
            1,
            "expected one cached module entry after first dotted lookup"
        );

        let second = loader
            .get_symbol("src.core.math", "answer")
            .expect("expected second dotted lookup to succeed");
        assert!(matches!(second, Value::Int(42)), "expected second lookup to return 42");
        assert_eq!(
            loader.loaded_modules.len(),
            1,
            "expected repeated dotted lookups to reuse cache without duplicate module entries"
        );

        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[test]
    fn load_module_rejects_dotted_module_names_with_empty_segments() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_dotted_invalid"));
        fs::create_dir_all(&temp_root).expect("failed to create temp module dir");
        loader.add_search_path(&temp_root);

        let err = loader
            .get_all_exports("src..core")
            .expect_err("expected invalid dotted module name to fail");
        assert!(
            err.message.contains("dotted module path contains an empty segment"),
            "expected invalid dotted module segment error, got: {}",
            err.message
        );

        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    #[cfg(unix)]
    #[test]
    fn load_module_rejects_symlink_escape_outside_search_root() {
        let mut loader = ModuleLoader::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_module_symlink_root"));
        let outside_root = std::env::temp_dir().join(unique_name("kujo_module_symlink_outside"));
        fs::create_dir_all(&temp_root).expect("failed to create temp module dir");
        fs::create_dir_all(&outside_root).expect("failed to create outside module dir");

        let module_name = unique_name("escaped_module");
        let outside_module_path = outside_root.join(format!("{}.kujo", module_name));
        fs::write(&outside_module_path, "export escaped := 99\n")
            .expect("failed to write outside module source");

        let symlink_module_path = temp_root.join(format!("{}.kujo", module_name));
        unix_fs::symlink(&outside_module_path, &symlink_module_path)
            .expect("failed to create symlink module path");

        loader.add_search_path(&temp_root);

        let err = loader
            .get_all_exports(&module_name)
            .expect_err("expected symlink escape import to fail");

        assert!(
            err.message.contains("escapes module search root"),
            "expected symlink-escape rejection error, got: {}",
            err.message
        );

        fs::remove_file(&symlink_module_path).expect("failed to remove symlink module path");
        fs::remove_file(&outside_module_path).expect("failed to remove outside module file");
        fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
        fs::remove_dir_all(&outside_root).expect("failed to clean up outside module dir");
    }
}
