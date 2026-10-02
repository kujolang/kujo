// File: src/type_checker.rs
//
// Type checker for the Kujo programming language.
// Performs type inference and type checking on the AST before interpretation.
//
// Features:
// - Type inference for expressions and variables
// - Type checking for assignments, function calls, and return statements
// - Symbol table for tracking variable and function types
// - Support for gradual typing (mixed typed/untyped code)
//
// The type checker uses a two-pass approach:
// 1. First pass: Collect function signatures
// 2. Second pass: Check statements and infer types

use crate::ast::{Expr, Pattern, Stmt, TypeAnnotation};
use crate::errors::{ErrorKind, KujoError, SourceLocation};
use crate::lexer::tokenize_with_file;
use crate::parser::Parser;
use crate::path_security;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Represents a function signature with parameter and return types
#[derive(Debug, Clone)]
struct FunctionSignature {
    param_types: Vec<Option<TypeAnnotation>>,
    return_type: Option<TypeAnnotation>,
}

#[derive(Debug, Clone, Default)]
struct StructShape {
    fields: HashMap<String, Option<TypeAnnotation>>,
    methods: HashMap<String, FunctionSignature>,
}

/// Type checker maintains symbol tables for variables and functions
pub struct TypeChecker {
    /// Symbol table mapping variable names to their types
    variables: HashMap<String, Option<TypeAnnotation>>,
    /// Function signatures mapping function names to their types
    functions: HashMap<String, FunctionSignature>,
    /// The polymorphic builtin must not override a user/imported function.
    builtin_contains_active: bool,
    /// Stack of scopes for nested blocks
    scope_stack: Vec<HashMap<String, Option<TypeAnnotation>>>,
    annotated_variables: HashSet<String>,
    annotation_scopes: Vec<HashSet<String>>,
    /// Current function return type (for checking return statements)
    current_function_return: Option<TypeAnnotation>,
    /// Collect errors instead of failing immediately
    errors: Vec<KujoError>,
    /// Recursion depth counter to prevent infinite loops
    recursion_depth: usize,
    /// Search roots used to resolve module imports for static signature inference.
    module_search_paths: Vec<PathBuf>,
    /// Cache of parsed module export signatures keyed by canonical module path.
    module_export_signatures: HashMap<PathBuf, HashMap<String, FunctionSignature>>,
    module_export_structs: HashMap<PathBuf, HashMap<String, StructShape>>,
    module_export_values: HashMap<PathBuf, HashMap<String, TypeAnnotation>>,
    module_ast_cache: HashMap<PathBuf, Arc<Vec<Stmt>>>,
    #[cfg(test)]
    module_parse_count: usize,
    structs: HashMap<String, StructShape>,
    validate_module_existence: bool,
    inferred_return_stack: Vec<Option<TypeAnnotation>>,
    current_struct: Option<String>,
}

/// Maximum recursion depth for type checking to prevent infinite loops
const MAX_RECURSION_DEPTH: usize = 1000;

impl TypeChecker {
    /// Creates a new type checker with empty symbol tables
    pub fn new() -> Self {
        let mut checker = TypeChecker {
            variables: HashMap::new(),
            functions: HashMap::new(),
            builtin_contains_active: true,
            scope_stack: Vec::new(),
            annotated_variables: HashSet::new(),
            annotation_scopes: Vec::new(),
            current_function_return: None,
            errors: Vec::new(),
            recursion_depth: 0,
            module_search_paths: {
                let mut paths = crate::module::initial_module_search_paths();
                paths.extend(crate::module::automatic_kennel_package_search_paths(
                    &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                ));
                paths
            },
            module_export_signatures: HashMap::new(),
            module_export_structs: HashMap::new(),
            module_export_values: HashMap::new(),
            module_ast_cache: HashMap::new(),
            #[cfg(test)]
            module_parse_count: 0,
            structs: HashMap::new(),
            validate_module_existence: false,
            inferred_return_stack: Vec::new(),
            current_struct: None,
        };

        // Register built-in functions
        checker.register_builtins();
        // Runtime registration is authoritative for builtin availability. Unknown
        // signatures remain gradual rather than being reported as undefined.
        for name in crate::interpreter::Interpreter::get_builtin_names() {
            let canonical = crate::interpreter::Interpreter::canonical_native_function_name(name);
            let signature = checker
                .functions
                .get(canonical)
                .cloned()
                .unwrap_or(FunctionSignature { param_types: Vec::new(), return_type: None });
            checker.functions.entry(name.to_string()).or_insert(signature);
        }

        checker
    }

    /// Adds a module search path used to resolve imported Kujo files during static analysis.
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();
        self.module_search_paths.push(path.clone());
        self.module_search_paths
            .extend(crate::module::automatic_kennel_package_search_paths(&path));
        self.validate_module_existence = true;
    }

    /// Registers all built-in function signatures
    fn register_builtins(&mut self) {
        // Math constants
        self.variables.insert("PI".to_string(), Some(TypeAnnotation::Float));
        self.variables.insert("E".to_string(), Some(TypeAnnotation::Float));

        // Math functions - single arg
        for name in &["abs", "sqrt", "floor", "ceil", "round", "sin", "cos", "tan", "log", "exp"] {
            self.functions.insert(
                name.to_string(),
                FunctionSignature {
                    param_types: vec![Some(TypeAnnotation::Float)],
                    return_type: Some(TypeAnnotation::Float),
                },
            );
        }

        // Math functions - two args
        for name in &["pow", "min", "max"] {
            self.functions.insert(
                name.to_string(),
                FunctionSignature {
                    param_types: vec![Some(TypeAnnotation::Float), Some(TypeAnnotation::Float)],
                    return_type: Some(TypeAnnotation::Float),
                },
            );
        }

        // String functions
        self.functions.insert(
            "len".to_string(),
            FunctionSignature {
                param_types: vec![None], // Strings, arrays, bytes, dictionaries and other sized values.
                return_type: Some(TypeAnnotation::Int),
            },
        );

        self.functions.insert(
            "bytes_is_ascii".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Any)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "byte_length".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Any)],
                return_type: Some(TypeAnnotation::Int),
            },
        );

        self.functions.insert(
            "to_upper".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "upper".to_string(), // Alias
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "to_lower".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "lower".to_string(), // Alias
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "capitalize".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "trim".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "trim_start".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "trim_end".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "char_at".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::Int)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "is_empty".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "count_chars".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Int),
            },
        );

        self.functions.insert(
            "contains".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Any), Some(TypeAnnotation::Any)],
                return_type: Some(TypeAnnotation::Any),
            },
        );

        self.functions.insert(
            "substring".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "replace_str".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                ],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "replace".to_string(), // Alias
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                ],
                return_type: Some(TypeAnnotation::String),
            },
        );

        // New string functions
        self.functions.insert(
            "starts_with".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "ends_with".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "index_of".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Any), Some(TypeAnnotation::Any)],
                return_type: Some(TypeAnnotation::Int),
            },
        );

        self.functions.insert(
            "repeat".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::Float)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "split".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: None, // Returns array, but we don't have array type annotation yet
            },
        );

        self.functions.insert(
            "join".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String)], // First param is array
                return_type: Some(TypeAnnotation::String),
            },
        );

        // Advanced string methods
        for name in ["pad_left", "pad_start", "pad_right", "pad_end"] {
            self.functions.insert(
                name.to_string(),
                FunctionSignature {
                    param_types: vec![
                        Some(TypeAnnotation::String),
                        Some(TypeAnnotation::Int),
                        Some(TypeAnnotation::String),
                    ],
                    return_type: Some(TypeAnnotation::String),
                },
            );
        }

        self.functions.insert(
            "lines".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns array of strings
            },
        );

        self.functions.insert(
            "words".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns array of strings
            },
        );

        self.functions.insert(
            "str_reverse".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "slugify".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "truncate".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::String),
                ],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "to_camel_case".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "to_snake_case".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "to_kebab_case".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        // Array mutation methods
        self.functions.insert(
            "push".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Array and item
                return_type: None,             // Returns modified array
            },
        );

        self.functions.insert(
            "append".to_string(), // Alias
            FunctionSignature {
                param_types: vec![None, None], // Array and item
                return_type: None,             // Returns modified array
            },
        );

        self.functions.insert(
            "pop".to_string(),
            FunctionSignature {
                param_types: vec![None], // Array
                return_type: None,       // Returns [array, popped_item]
            },
        );

        self.functions.insert(
            "insert".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::Int), None], // Array, index, item
                return_type: None,                                        // Returns modified array
            },
        );

        self.functions.insert(
            "remove_at".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::Int)], // Array, index
                return_type: None,                                  // Returns [array, removed_item]
            },
        );

        self.functions.insert(
            "clear".to_string(),
            FunctionSignature {
                param_types: vec![None], // Array
                return_type: None,       // Returns empty array
            },
        );

        self.functions.insert(
            "slice".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::Int), Some(TypeAnnotation::Int)], // Array, start, end
                return_type: None, // Returns sub-array
            },
        );

        self.functions.insert(
            "concat".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Two arrays
                return_type: None,             // Returns combined array
            },
        );

        // Array higher-order functions
        self.functions.insert(
            "map".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Array and function
                return_type: None,             // Returns array
            },
        );

        self.functions.insert(
            "filter".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Array and function
                return_type: None,             // Returns array
            },
        );

        self.functions.insert(
            "reduce".to_string(),
            FunctionSignature {
                param_types: vec![None, None, None], // Array, initial value, and function
                return_type: None,                   // Returns value of initial type
            },
        );

        self.functions.insert(
            "find".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Array and function
                return_type: None,             // Returns element or 0
            },
        );

        // Array utility functions
        self.functions.insert(
            "sort".to_string(),
            FunctionSignature {
                param_types: vec![None], // Array
                return_type: None,       // Returns sorted array
            },
        );

        self.functions.insert(
            "reverse".to_string(),
            FunctionSignature {
                param_types: vec![None], // Array
                return_type: None,       // Returns reversed array
            },
        );

        self.functions.insert(
            "unique".to_string(),
            FunctionSignature {
                param_types: vec![None], // Array
                return_type: None,       // Returns array with unique elements
            },
        );

        self.functions.insert(
            "sum".to_string(),
            FunctionSignature {
                param_types: vec![None], // Array
                return_type: None,       // Returns Int or Float
            },
        );

        self.functions.insert(
            "any".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Array and function
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "all".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Array and function
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        // Advanced array methods
        self.functions.insert(
            "chunk".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::Int)], // Array and size
                return_type: None,                                  // Returns array of arrays
            },
        );

        self.functions.insert(
            "flatten".to_string(),
            FunctionSignature {
                param_types: vec![None], // Array
                return_type: None,       // Returns flattened array
            },
        );

        self.functions.insert(
            "zip".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Two arrays
                return_type: None,             // Returns array of pairs
            },
        );

        self.functions.insert(
            "enumerate".to_string(),
            FunctionSignature {
                param_types: vec![None], // Array
                return_type: None,       // Returns array of [index, value] pairs
            },
        );

        self.functions.insert(
            "take".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::Int)], // Array and count
                return_type: None,                                  // Returns sub-array
            },
        );

        self.functions.insert(
            "skip".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::Int)], // Array and count
                return_type: None,                                  // Returns sub-array
            },
        );

        self.functions.insert(
            "windows".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::Int)], // Array and window size
                return_type: None,                                  // Returns array of arrays
            },
        );

        // Dict/Map methods
        self.functions.insert(
            "keys".to_string(),
            FunctionSignature {
                param_types: vec![None], // Dict
                return_type: None,       // Returns array of strings (keys)
            },
        );

        self.functions.insert(
            "values".to_string(),
            FunctionSignature {
                param_types: vec![None], // Dict
                return_type: None,       // Returns array of values
            },
        );

        self.functions.insert(
            "items".to_string(),
            FunctionSignature {
                param_types: vec![None], // Dict
                return_type: None,       // Returns array of [key, value] pairs
            },
        );

        self.functions.insert(
            "has_key".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String)], // Dict, key
                return_type: Some(TypeAnnotation::Int),
            },
        );

        self.functions.insert(
            "get".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String), None], // Dict, key, default (optional)
                return_type: None, // Returns value or default
            },
        );

        self.functions.insert(
            "merge".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Two dicts
                return_type: None,             // Returns merged dict
            },
        );

        // Advanced dict methods
        self.functions.insert(
            "invert".to_string(),
            FunctionSignature {
                param_types: vec![None], // Dict
                return_type: None,       // Returns inverted dict
            },
        );

        self.functions.insert(
            "update".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Two dicts
                return_type: None,             // Returns updated dict
            },
        );

        self.functions.insert(
            "get_default".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String), None], // Dict, key, default value
                return_type: None, // Returns value or default
            },
        );

        // Array generation functions
        self.functions.insert(
            "range".to_string(),
            FunctionSignature {
                param_types: vec![], // Variadic: 1, 2, or 3 numeric arguments
                return_type: None,   // Returns array of integers
            },
        );

        // String formatting functions
        self.functions.insert(
            "format".to_string(),
            FunctionSignature {
                param_types: vec![], // Variadic: template + args
                return_type: Some(TypeAnnotation::String),
            },
        );

        for (name, n) in crate::interpreter::native_functions::web_data::BUILTINS
            .iter()
            .chain(crate::interpreter::native_functions::platform::BUILTINS)
        {
            self.functions.insert(
                name.to_string(),
                FunctionSignature { param_types: vec![None; *n], return_type: None },
            );
        }
        for (name, n) in [
            ("bytes", 1),
            ("bit_and", 2),
            ("bit_or", 2),
            ("bit_xor", 2),
            ("bit_not", 1),
            ("bit_shl", 2),
            ("bit_shr", 2),
        ] {
            self.functions.insert(
                name.to_string(),
                FunctionSignature { param_types: vec![None; n], return_type: None },
            );
        }
        self.functions.insert(
            "http_request".to_string(),
            FunctionSignature { param_types: vec![], return_type: None },
        );
        // JSON functions
        self.functions.insert(
            "parse_json".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns any type (dict, array, etc.)
            },
        );

        self.functions.insert(
            "parse_xml_bounded".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None],
                return_type: None, // Returns a deterministic document dictionary.
            },
        );

        self.functions.insert(
            "to_json".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts any value
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "to_json_pretty".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts any value
                return_type: Some(TypeAnnotation::String),
            },
        );

        // TOML functions
        self.functions.insert(
            "parse_toml".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns any type (dict, array, etc.)
            },
        );

        self.functions.insert(
            "to_toml".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts any value
                return_type: Some(TypeAnnotation::String),
            },
        );

        // YAML functions
        self.functions.insert(
            "parse_yaml".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns any type (dict, array, etc.)
            },
        );

        self.functions.insert(
            "to_yaml".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts any value
                return_type: Some(TypeAnnotation::String),
            },
        );

        // CSV functions
        self.functions.insert(
            "parse_csv".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns array of dicts
            },
        );

        self.functions.insert(
            "to_csv".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts array of dicts
                return_type: Some(TypeAnnotation::String),
            },
        );

        // Type conversion functions
        self.functions.insert(
            "to_int".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts any type
                return_type: Some(TypeAnnotation::Int),
            },
        );

        self.functions.insert(
            "to_float".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts any type
                return_type: Some(TypeAnnotation::Float),
            },
        );

        self.functions.insert(
            "to_string".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts any type
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "secret".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None,
            },
        );

        self.functions.insert(
            "reveal".to_string(),
            FunctionSignature {
                param_types: vec![None],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "to_bool".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts any type
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        // Type introspection functions
        self.functions.insert(
            "type".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts any type
                return_type: Some(TypeAnnotation::String),
            },
        );

        for name in &[
            "is_int",
            "is_float",
            "is_string",
            "is_secret",
            "is_array",
            "is_dict",
            "is_bool",
            "is_null",
            "is_function",
        ] {
            self.functions.insert(
                name.to_string(),
                FunctionSignature {
                    param_types: vec![None], // Accepts any type
                    return_type: Some(TypeAnnotation::Bool),
                },
            );
        }

        // Assert & Debug functions
        self.functions.insert(
            "assert".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // condition (any type), optional message (string)
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "assert_equal".to_string(),
            FunctionSignature {
                param_types: vec![None, None],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "assert_true".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Bool)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "assert_false".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Bool)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "assert_contains".to_string(),
            FunctionSignature {
                param_types: vec![None, None],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "debug".to_string(),
            FunctionSignature {
                param_types: vec![], // Variadic - accepts any number of any type (empty vec = no validation)
                return_type: None,   // Returns null
            },
        );

        // Random functions
        self.functions.insert(
            "random".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::Float) },
        );

        self.functions.insert(
            "random_int".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Float), Some(TypeAnnotation::Float)],
                return_type: Some(TypeAnnotation::Float),
            },
        );

        self.functions.insert(
            "random_choice".to_string(),
            FunctionSignature {
                param_types: vec![None], // Array
                return_type: None,       // Returns element from array
            },
        );

        self.functions.insert(
            "uuid_v4".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::String) },
        );

        self.functions.insert(
            "random_id".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Int)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "secure_random_token".to_string(),
            FunctionSignature { param_types: vec![Some(TypeAnnotation::Int)], return_type: None },
        );

        self.functions.insert(
            "pdf_render_html".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None, None],
                return_type: None,
            },
        );

        self.functions.insert(
            "pdf_render_html_to_file".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    None,
                    None,
                    Some(TypeAnnotation::String),
                ],
                return_type: None,
            },
        );

        // Random seed control (for deterministic testing)
        self.functions.insert(
            "set_random_seed".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Int)], // Seed value
                return_type: None,
            },
        );

        self.functions.insert(
            "clear_random_seed".to_string(),
            FunctionSignature { param_types: vec![], return_type: None },
        );

        // Date/Time functions
        self.functions.insert(
            "now".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::Float) },
        );

        self.functions.insert(
            "now_utc".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::String) },
        );

        self.functions.insert(
            "now_unix".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::Int) },
        );

        self.functions.insert(
            "now_utc_seconds".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::Int) },
        );

        self.functions.insert(
            "current_timestamp".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::Float) },
        );

        self.functions.insert(
            "time".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::Float) },
        );

        self.functions.insert(
            "performance_now".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::Float) },
        );

        self.functions.insert(
            "time_us".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::Float) },
        );

        self.functions.insert(
            "time_ns".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::Float) },
        );

        self.functions.insert(
            "format_duration".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Float)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "elapsed".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Float), Some(TypeAnnotation::Float)],
                return_type: Some(TypeAnnotation::Float),
            },
        );

        self.functions.insert(
            "format_date".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Float), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "format_date_tz".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::Float),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                ],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "parse_date".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Float),
            },
        );
        self.functions.insert(
            "parse_datetime".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Float),
            },
        );

        // System operation functions
        self.functions.insert(
            "env".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "kv_get".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "kv_set".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "args".to_string(),
            FunctionSignature {
                param_types: vec![],
                return_type: None, // Returns array of strings
            },
        );

        self.functions.insert(
            "exit".to_string(),
            FunctionSignature { param_types: vec![Some(TypeAnnotation::Float)], return_type: None },
        );

        self.functions.insert(
            "sleep".to_string(),
            FunctionSignature { param_types: vec![Some(TypeAnnotation::Float)], return_type: None },
        );

        self.functions.insert(
            "execute".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "execute_status".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None],
                return_type: None,
            },
        );

        // Path operation functions
        self.functions.insert(
            "join_path".to_string(),
            FunctionSignature {
                param_types: vec![], // Variadic string arguments
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "dirname".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "basename".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "path_exists".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "path_is_absolute".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        // File I/O functions
        self.functions.insert(
            "read_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "list_dir_beneath".into(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::Any),
            },
        );

        self.functions.insert(
            "copy_file_beneath".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::Any),
            },
        );

        self.functions.insert(
            "sha256_file_beneath".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::Any),
            },
        );

        self.functions.insert(
            "read_file_beneath".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "read_binary_file_beneath".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::Any),
            },
        );

        self.functions.insert(
            "digest_file_beneath".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::Any),
            },
        );

        self.functions.insert(
            "read_binary_prefix_beneath".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::Any),
            },
        );

        self.functions.insert(
            "write_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String), None],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "sync_directory_beneath".into(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "write_file_atomic_beneath".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    None,
                    None,
                ],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "write_file_atomic".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None, None],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "io_write_private_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None, Some(TypeAnnotation::Int)],
                return_type: None,
            },
        );

        self.functions.insert(
            "io_private_spool_open".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::Int),
                ],
                return_type: None,
            },
        );

        self.functions.insert(
            "io_private_spool_write".to_string(),
            FunctionSignature { param_types: vec![None, None], return_type: None },
        );

        self.functions.insert(
            "io_private_spool_write_file_range".to_string(),
            FunctionSignature {
                param_types: vec![
                    None,
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::String),
                ],
                return_type: None,
            },
        );

        for function_name in ["io_private_spool_finish", "io_private_spool_abort"] {
            self.functions.insert(
                function_name.to_string(),
                FunctionSignature { param_types: vec![None], return_type: None },
            );
        }

        self.functions.insert(
            "aes_encrypt_file_stream".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                ],
                return_type: None,
            },
        );

        self.functions.insert(
            "aes_decrypt_file_stream".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                ],
                return_type: None,
            },
        );

        self.functions.insert(
            "append_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "file_exists".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "read_lines".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns array of strings
            },
        );

        self.functions.insert(
            "jsonl_query".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None],
                return_type: None,
            },
        );

        self.functions.insert(
            "list_dir_page".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::String),
                ],
                return_type: None,
            },
        );

        self.functions.insert(
            "list_dir".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns array of strings
            },
        );

        self.functions.insert(
            "create_dir".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "file_size".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Int),
            },
        );

        self.functions.insert(
            "delete_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "rename_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "publish_file_noreplace".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: None,
            },
        );

        self.functions.insert(
            "copy_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        // Regular expression functions
        self.functions.insert(
            "regex_match".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "regex_find_all".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: None, // Returns array of strings
            },
        );

        self.functions.insert(
            "regex_replace".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                ],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "regex_split".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: None, // Returns array of strings
            },
        );

        // HTTP client functions
        self.functions.insert(
            "http_get".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns Result<dict, string>
            },
        );

        self.functions.insert(
            "http_post".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: None, // Returns Result<dict, string>
            },
        );

        self.functions.insert(
            "http_put".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: None, // Returns Result<dict, string>
            },
        );

        self.functions.insert(
            "http_delete".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns Result<dict, string>
            },
        );

        // HTTP server functions
        self.functions.insert(
            "http_server".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Int)],
                return_type: None, // Returns HttpServer object
            },
        );

        self.functions.insert(
            "http_listen".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::Int)],
                return_type: None, // Returns HttpServer object
            },
        );

        self.functions.insert(
            "http_response".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Int), Some(TypeAnnotation::String)],
                return_type: None, // Returns HttpResponse object
            },
        );

        self.functions.insert(
            "json_response".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Int), None], // Status code and any data
                return_type: None,                                  // Returns HttpResponse object
            },
        );

        self.functions.insert(
            "redirect_response".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None], // URL to redirect to, optional headers dict
                return_type: None, // Returns HttpResponse object
            },
        );

        self.functions.insert(
            "set_header".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String), Some(TypeAnnotation::String)], // Response, key, value
                return_type: None, // Returns HttpResponse object
            },
        );

        self.functions.insert(
            "set_headers".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Response, headers dict
                return_type: None,             // Returns HttpResponse object
            },
        );

        // Database functions
        self.functions.insert(
            "db_connect".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)], // db_type, connection_string
                return_type: None, // Returns Database object
            },
        );

        self.functions.insert(
            "db_connect_readonly".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: None,
            },
        );

        self.functions.insert(
            "db_connect_postgres_tls".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: None,
            },
        );

        self.functions.insert(
            "db_execute".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String), None], // db, sql, params (optional array)
                return_type: None, // Returns number (rows affected) or Error
            },
        );

        self.functions.insert(
            "db_query".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String), None], // db, sql, params (optional array)
                return_type: None, // Returns array of dicts
            },
        );

        self.functions.insert(
            "db_close".to_string(),
            FunctionSignature {
                param_types: vec![None], // Database connection
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "db_pool".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String), None], // db_type, connection_string, options
                return_type: None, // Returns DatabasePool object
            },
        );

        self.functions.insert(
            "db_pool_postgres_tls".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String), None],
                return_type: None,
            },
        );

        for name in ["db_pool_acquire", "db_pool_stats", "db_pool_close"] {
            self.functions.insert(
                name.to_string(),
                FunctionSignature { param_types: vec![None], return_type: None },
            );
        }
        self.functions.insert(
            "db_pool_release".to_string(),
            FunctionSignature {
                param_types: vec![None, None],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "db_begin".to_string(),
            FunctionSignature {
                param_types: vec![None], // Database connection
                return_type: None,
            },
        );

        self.functions.insert(
            "db_begin_immediate".to_string(),
            FunctionSignature {
                param_types: vec![None], // SQLite database connection
                return_type: None,
            },
        );

        self.functions.insert(
            "db_commit".to_string(),
            FunctionSignature {
                param_types: vec![None], // Database connection
                return_type: None,
            },
        );

        self.functions.insert(
            "db_last_insert_id".to_string(),
            FunctionSignature { param_types: vec![None], return_type: Some(TypeAnnotation::Int) },
        );

        self.functions.insert(
            "db_rollback".to_string(),
            FunctionSignature {
                param_types: vec![None], // Database connection
                return_type: None,
            },
        );

        // File I/O functions
        self.functions.insert(
            "create_dir".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // Directory path
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "write_binary_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None, None], // Path, bytes, optional overwrite
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        // HTTP streaming functions
        self.functions.insert(
            "http_get_stream".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // URL
                return_type: None,                               // Returns bytes array
            },
        );

        self.functions.insert(
            "http_download_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String), None],
                return_type: None,
            },
        );

        self.functions.insert(
            "http_upload_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String), None],
                return_type: None,
            },
        );

        // v0.6.0 Authentication & Streaming functions
        self.functions.insert(
            "jwt_encode".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String)], // Payload dict and secret
                return_type: Some(TypeAnnotation::String),             // Returns JWT token string
            },
        );

        self.functions.insert(
            "jwt_decode".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)], // Token and secret
                return_type: None, // Returns dict or error
            },
        );

        self.functions.insert(
            "jwt_verify".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "oauth2_auth_url".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String), // client_id
                    Some(TypeAnnotation::String), // redirect_uri
                    Some(TypeAnnotation::String), // auth_url
                    Some(TypeAnnotation::String), // scope
                ],
                return_type: Some(TypeAnnotation::String), // Returns authorization URL
            },
        );

        self.functions.insert(
            "oauth2_get_token".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String), // code
                    Some(TypeAnnotation::String), // client_id
                    Some(TypeAnnotation::String), // client_secret
                    Some(TypeAnnotation::String), // token_url
                    Some(TypeAnnotation::String), // redirect_uri
                ],
                return_type: None, // Returns dict with token data
            },
        );

        self.functions.insert(
            "html_response".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Int), Some(TypeAnnotation::String)], // Status code and HTML
                return_type: None, // Returns HttpResponse object
            },
        );

        // Collection constructors and methods
        // Set operations
        self.functions.insert(
            "Set".to_string(),
            FunctionSignature {
                param_types: vec![None], // Array
                return_type: None,       // Returns Set
            },
        );
        self.functions.insert(
            "set_add".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Set and item
                return_type: None,             // Returns modified Set
            },
        );
        self.functions.insert(
            "set_has".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Set and item
                return_type: Some(TypeAnnotation::Bool),
            },
        );
        self.functions.insert(
            "set_remove".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Set and item
                return_type: None,             // Returns modified Set
            },
        );
        self.functions.insert(
            "set_union".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Two Sets
                return_type: None,             // Returns new Set
            },
        );
        self.functions.insert(
            "set_intersect".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Two Sets
                return_type: None,             // Returns new Set
            },
        );
        self.functions.insert(
            "set_difference".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Two Sets
                return_type: None,             // Returns new Set
            },
        );
        self.functions.insert(
            "set_to_array".to_string(),
            FunctionSignature {
                param_types: vec![None], // Set
                return_type: None,       // Returns Array
            },
        );

        // Queue operations
        self.functions.insert(
            "Queue".to_string(),
            FunctionSignature {
                param_types: vec![None], // Optional array
                return_type: None,       // Returns Queue
            },
        );
        self.functions.insert(
            "queue_enqueue".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Queue and item
                return_type: None,             // Returns modified Queue
            },
        );
        self.functions.insert(
            "queue_dequeue".to_string(),
            FunctionSignature {
                param_types: vec![None], // Queue
                return_type: None,       // Returns [modified Queue, item]
            },
        );
        self.functions.insert(
            "queue_peek".to_string(),
            FunctionSignature {
                param_types: vec![None], // Queue
                return_type: None,       // Returns item or null
            },
        );
        self.functions.insert(
            "queue_is_empty".to_string(),
            FunctionSignature {
                param_types: vec![None], // Queue
                return_type: Some(TypeAnnotation::Bool),
            },
        );
        self.functions.insert(
            "queue_to_array".to_string(),
            FunctionSignature {
                param_types: vec![None], // Queue
                return_type: None,       // Returns Array
            },
        );

        // Stack operations
        self.functions.insert(
            "Stack".to_string(),
            FunctionSignature {
                param_types: vec![None], // Optional array
                return_type: None,       // Returns Stack
            },
        );
        self.functions.insert(
            "stack_push".to_string(),
            FunctionSignature {
                param_types: vec![None, None], // Stack and item
                return_type: None,             // Returns modified Stack
            },
        );
        self.functions.insert(
            "stack_pop".to_string(),
            FunctionSignature {
                param_types: vec![None], // Stack
                return_type: None,       // Returns [modified Stack, item]
            },
        );
        self.functions.insert(
            "stack_peek".to_string(),
            FunctionSignature {
                param_types: vec![None], // Stack
                return_type: None,       // Returns item or null
            },
        );
        self.functions.insert(
            "stack_is_empty".to_string(),
            FunctionSignature {
                param_types: vec![None], // Stack
                return_type: Some(TypeAnnotation::Bool),
            },
        );
        self.functions.insert(
            "stack_to_array".to_string(),
            FunctionSignature {
                param_types: vec![None], // Stack
                return_type: None,       // Returns Array
            },
        );

        // Image processing functions
        self.functions.insert(
            "load_image".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // Image path
                return_type: None,                               // Returns Image object
            },
        );
        self.functions.insert(
            "gif_to_webp".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    None,
                    None,
                    None,
                ],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        // I/O functions (CRITICAL - these were missing!)
        self.functions.insert(
            "print".to_string(),
            FunctionSignature {
                param_types: vec![], // Variadic - accepts any number of arguments
                return_type: None,   // Returns null
            },
        );

        self.functions.insert(
            "input".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // Optional prompt
                return_type: Some(TypeAnnotation::String),       // Returns user input
            },
        );

        self.functions.insert(
            "read_stdin".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Int)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        // Additional commonly used functions
        self.functions.insert(
            "parse_int".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Int),
            },
        );

        self.functions.insert(
            "parse_float".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Float),
            },
        );

        self.functions.insert(
            "throw".to_string(),
            FunctionSignature {
                param_types: vec![None], // Accepts any error value
                return_type: None,       // Never returns (throws)
            },
        );

        // Environment variable helpers (v0.8.0)
        self.functions.insert(
            "env_or".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)], // key, default
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "env_int".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // key
                return_type: Some(TypeAnnotation::Int),
            },
        );

        self.functions.insert(
            "env_float".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // key
                return_type: Some(TypeAnnotation::Float),
            },
        );

        self.functions.insert(
            "env_bool".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // key
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "env_required".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // key
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "env_set".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)], // key, value
                return_type: None,
            },
        );

        self.functions.insert(
            "env_list".to_string(),
            FunctionSignature {
                param_types: vec![],
                return_type: None, // Returns dict of all env vars
            },
        );

        // Argument parser function
        self.functions.insert(
            "arg_parser".to_string(),
            FunctionSignature {
                param_types: vec![],
                return_type: None, // Returns ArgParser object
            },
        );

        // Compression & hashing functions (v0.8.0)
        self.functions.insert(
            "zip_create".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // zip file path
                return_type: None,                               // Returns ZipArchive object
            },
        );

        self.functions.insert(
            "zip_add_file".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String)], // archive, file path
                return_type: None,
            },
        );

        self.functions.insert(
            "zip_add_dir".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String)], // archive, dir path
                return_type: None,
            },
        );

        self.functions.insert(
            "zip_close".to_string(),
            FunctionSignature {
                param_types: vec![None], // archive
                return_type: None,
            },
        );

        self.functions.insert(
            "unzip".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)], // zip path, dest dir
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "gzip_compress".to_string(),
            FunctionSignature { param_types: vec![None], return_type: None },
        );

        self.functions.insert(
            "gzip_decompress".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::Int)],
                return_type: None,
            },
        );

        self.functions.insert(
            "zip_single_file_read".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::Int)],
                return_type: None,
            },
        );

        self.functions.insert(
            "sha256".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // data
                return_type: Some(TypeAnnotation::String),       // hash
            },
        );

        self.functions.insert(
            "hmac_sha256".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "hmac_sha256_verify".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                ],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "md5".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // data
                return_type: Some(TypeAnnotation::String),       // hash
            },
        );

        self.functions.insert(
            "md5_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // file path
                return_type: Some(TypeAnnotation::String),       // hash
            },
        );

        self.functions.insert(
            "sha256_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // file path
                return_type: Some(TypeAnnotation::String),       // hash
            },
        );

        self.functions.insert(
            "sha256_file_range".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "decode_file_range_info".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::Any),
            },
        );

        self.functions.insert(
            "decode_text_file_range_info".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::Int),
                ],
                return_type: Some(TypeAnnotation::Any),
            },
        );

        self.functions.insert(
            "sha256_canonical_text_file_range".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::Int),
                    Some(TypeAnnotation::String),
                ],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "hash_password".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // password
                return_type: Some(TypeAnnotation::String),       // hashed
            },
        );

        self.functions.insert(
            "verify_password".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)], // password, hash
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        // Process management functions
        self.functions.insert(
            "spawn_process".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Any), None], // command argv + optional options
                return_type: None, // Returns dict with stdout, stderr, status
            },
        );

        self.functions.insert(
            "pipe_commands".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Any), None], // command arrays + optional options
                return_type: None, // Returns dict with stdout, stderr, status
            },
        );

        // OS & Path module functions
        self.functions.insert(
            "os_getcwd".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::String) },
        );

        self.functions.insert(
            "os_chdir".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // path
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "os_rmdir".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // path
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "os_environ".to_string(),
            FunctionSignature {
                param_types: vec![],
                return_type: None, // Returns dict of env vars
            },
        );

        self.functions.insert(
            "path_join".to_string(),
            FunctionSignature {
                param_types: vec![], // Variadic string arguments
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "path_absolute".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // path
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "path_is_dir".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // path
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "path_is_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // path
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "path_is_symlink".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // path
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "path_extension".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // path
                return_type: Some(TypeAnnotation::String),
            },
        );

        // Binary/Base64 functions
        self.functions.insert(
            "encode_base64".to_string(),
            FunctionSignature {
                param_types: vec![None], // bytes
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "decode_base64".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // base64 string
                return_type: None,                               // bytes
            },
        );

        self.functions.insert(
            "decode_base64_utf8".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "decode_charset".to_string(),
            FunctionSignature {
                param_types: vec![None, Some(TypeAnnotation::String), Some(TypeAnnotation::Int)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "encode_uri_component".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "decode_uri_component".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
            },
        );

        self.functions.insert(
            "read_binary_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // file path
                return_type: None,                               // bytes
            },
        );

        self.functions.insert(
            "http_get_binary".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // URL
                return_type: None,                               // bytes
            },
        );

        // Advanced I/O functions (v0.8.0)
        self.functions.insert(
            "io_read_bytes".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::Int)], // path, count
                return_type: None,                                                          // bytes
            },
        );

        self.functions.insert(
            "io_write_bytes".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None], // path, bytes
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "io_append_bytes".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), None], // path, bytes
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "io_read_at".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String), // path
                    Some(TypeAnnotation::Int),    // offset
                    Some(TypeAnnotation::Int),    // count
                ],
                return_type: None, // bytes
            },
        );

        self.functions.insert(
            "io_write_at".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String), // path
                    Some(TypeAnnotation::Int),    // offset
                    None,                         // bytes
                ],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "io_seek_read".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String), // path
                    Some(TypeAnnotation::Int),    // position
                ],
                return_type: None, // bytes
            },
        );

        self.functions.insert(
            "io_file_metadata".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)], // path
                return_type: None,                               // dict with metadata
            },
        );

        self.functions.insert(
            "io_set_permissions".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::Int)],
                return_type: None, // verified permission result dict
            },
        );

        self.functions.insert(
            "io_truncate".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::Int)], // path, size
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        self.functions.insert(
            "io_copy_range".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::String), // source path
                    Some(TypeAnnotation::String), // dest path
                    Some(TypeAnnotation::Int),    // offset
                    None,                         // optional count
                ],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        // Concurrency functions
        self.functions.insert(
            "async_sleep".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Int)],
                return_type: Some(TypeAnnotation::Promise(Box::new(TypeAnnotation::Any))),
            },
        );

        for name in ["await_task"] {
            self.functions.insert(
                name.to_string(),
                FunctionSignature {
                    param_types: vec![Some(TypeAnnotation::Any)],
                    return_type: Some(TypeAnnotation::Promise(Box::new(TypeAnnotation::Any))),
                },
            );
        }

        for name in ["promise_all", "await_all"] {
            self.functions.insert(
                name.to_string(),
                FunctionSignature {
                    param_types: vec![
                        Some(TypeAnnotation::Array(Box::new(TypeAnnotation::Any))),
                        None,
                    ],
                    return_type: Some(TypeAnnotation::Promise(Box::new(TypeAnnotation::Array(
                        Box::new(TypeAnnotation::Any),
                    )))),
                },
            );
        }

        self.functions.insert(
            "channel".to_string(),
            FunctionSignature {
                param_types: vec![],
                return_type: None, // Returns Channel object
            },
        );

        self.functions.insert(
            "parallel_http".to_string(),
            FunctionSignature {
                param_types: vec![None], // array of URLs
                return_type: None,       // Returns array of responses
            },
        );

        self.functions.insert(
            "promise_wait".to_string(),
            FunctionSignature { param_types: vec![None], return_type: None },
        );

        self.functions.insert(
            "parallel_map".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::Array(Box::new(TypeAnnotation::Any))),
                    Some(TypeAnnotation::Any),
                    None,
                ],
                return_type: Some(TypeAnnotation::Promise(Box::new(TypeAnnotation::Array(
                    Box::new(TypeAnnotation::Any),
                )))),
            },
        );

        self.functions.insert(
            "par_map".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::Array(Box::new(TypeAnnotation::Any))),
                    Some(TypeAnnotation::Any),
                    None,
                ], // array, mapper, optional limit
                return_type: Some(TypeAnnotation::Promise(Box::new(TypeAnnotation::Array(
                    Box::new(TypeAnnotation::Any),
                )))),
            },
        );

        self.functions.insert(
            "par_each".to_string(),
            FunctionSignature {
                param_types: vec![
                    Some(TypeAnnotation::Array(Box::new(TypeAnnotation::Any))),
                    Some(TypeAnnotation::Any),
                    None,
                ], // array, mapper, optional limit
                return_type: Some(TypeAnnotation::Promise(Box::new(TypeAnnotation::Any))),
            },
        );

        self.functions.insert(
            "set_task_pool_size".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::Int)], // size
                return_type: Some(TypeAnnotation::Int),       // previous size
            },
        );

        self.functions.insert(
            "get_task_pool_size".to_string(),
            FunctionSignature { param_types: vec![], return_type: Some(TypeAnnotation::Int) },
        );
    }

    fn function_signature_to_type_annotation(signature: &FunctionSignature) -> TypeAnnotation {
        TypeAnnotation::Function {
            params: signature
                .param_types
                .iter()
                .map(|param_type| param_type.clone().unwrap_or(TypeAnnotation::Any))
                .collect(),
            return_type: Box::new(signature.return_type.clone().unwrap_or(TypeAnnotation::Any)),
        }
    }

    fn register_imported_symbol(
        &mut self,
        name: &str,
        signature: Option<FunctionSignature>,
        allow_callable_fallback: bool,
    ) {
        if name == "contains" {
            self.builtin_contains_active = false;
        }
        // Imported Kujo values should be visible to the checker even when the
        // module export cannot be resolved statically. If we do know the
        // exported function signature, keep it precise so later calls type-check
        // instead of falling back to `Any`.
        match signature {
            Some(signature) => {
                self.variables.insert(
                    name.to_string(),
                    Some(Self::function_signature_to_type_annotation(&signature)),
                );
                self.functions.insert(name.to_string(), signature);
            }
            None => {
                self.variables.insert(name.to_string(), Some(TypeAnnotation::Any));
                if allow_callable_fallback {
                    self.functions.insert(
                        name.to_string(),
                        FunctionSignature {
                            param_types: vec![],
                            return_type: Some(TypeAnnotation::Any),
                        },
                    );
                }
            }
        }
    }

    fn module_resolution_candidates(module_name: &str) -> Result<Vec<PathBuf>, String> {
        let mut candidates = Vec::new();
        let mut seen = HashSet::new();

        let flat_filename = format!("{}.kujo", module_name);
        let normalized_flat =
            path_security::sanitize_relative_path(&flat_filename, "module import")
                .map_err(|error| format!("Unsafe module import '{}': {}", module_name, error))?;
        if seen.insert(normalized_flat.clone()) {
            candidates.push(normalized_flat);
        }

        if module_name.contains('.') {
            let segments: Vec<&str> = module_name.split('.').collect();
            if segments.iter().any(|segment| segment.is_empty()) {
                return Err(format!(
                    "Unsafe module import '{}': dotted module path contains an empty segment",
                    module_name
                ));
            }

            let nested_filename = format!("{}.kujo", segments.join("/"));
            let normalized_nested =
                path_security::sanitize_relative_path(&nested_filename, "module import").map_err(
                    |error| format!("Unsafe module import '{}': {}", module_name, error),
                )?;

            if seen.insert(normalized_nested.clone()) {
                candidates.push(normalized_nested);
            }
        }

        Ok(candidates)
    }

    fn resolve_module_import_path(&self, module_name: &str) -> Option<PathBuf> {
        let resolution_candidates = Self::module_resolution_candidates(module_name).ok()?;
        let mut visited_roots = HashSet::new();

        for search_path in &self.module_search_paths {
            let canonical_search_root =
                match path_security::canonicalize_root(search_path, "module search path") {
                    Ok(path) => path,
                    Err(_) => continue,
                };

            if !visited_roots.insert(canonical_search_root.clone()) {
                continue;
            }

            for normalized_filename in &resolution_candidates {
                let full_path = canonical_search_root.join(normalized_filename);
                if full_path.exists() {
                    let canonical_module_path = match fs::canonicalize(&full_path) {
                        Ok(path) => path,
                        Err(_) => continue,
                    };

                    if path_security::ensure_path_within_root(
                        &canonical_module_path,
                        &canonical_search_root,
                        "module import path",
                    )
                    .is_err()
                    {
                        continue;
                    }

                    return Some(canonical_module_path);
                }
            }
        }

        None
    }

    fn function_signature_from_params(
        params: &[String],
        param_types: &[Option<TypeAnnotation>],
        return_type: &Option<TypeAnnotation>,
        is_async: bool,
    ) -> FunctionSignature {
        let return_type = if is_async {
            Some(TypeAnnotation::Promise(Box::new(
                return_type.clone().unwrap_or(TypeAnnotation::Any),
            )))
        } else {
            return_type.clone()
        };
        FunctionSignature {
            param_types: param_types
                .iter()
                .cloned()
                .chain(std::iter::repeat(None))
                .take(params.len())
                .map(|param_type| Some(param_type.unwrap_or(TypeAnnotation::Any)))
                .collect(),
            return_type,
        }
    }

    fn signature_from_imported_value(
        value: &Expr,
        known_functions: &HashMap<String, FunctionSignature>,
    ) -> Option<FunctionSignature> {
        match value {
            Expr::Identifier(name) => known_functions.get(name).cloned(),
            Expr::Function { params, param_types, return_type, is_async, .. } => Some(
                Self::function_signature_from_params(params, param_types, return_type, *is_async),
            ),
            _ => None,
        }
    }

    fn collect_binding_signatures_from_stmt(
        &mut self,
        stmt: &Stmt,
        binding_signatures: &mut HashMap<String, FunctionSignature>,
        active_modules: &mut Vec<PathBuf>,
    ) {
        match stmt {
            Stmt::FuncDef { name, params, param_types, return_type, is_async, .. } => {
                binding_signatures.insert(
                    name.clone(),
                    Self::function_signature_from_params(
                        params,
                        param_types,
                        return_type,
                        *is_async,
                    ),
                );
            }
            Stmt::Import { module, symbols: Some(symbols) } => {
                if let Some(module_signatures) =
                    self.module_export_signatures(module, active_modules)
                {
                    for symbol in symbols {
                        if let Some(signature) = module_signatures.get(symbol) {
                            binding_signatures.insert(symbol.clone(), signature.clone());
                        }
                    }
                }
            }
            Stmt::Const { name, value, .. } => {
                if let Some(signature) =
                    Self::signature_from_imported_value(value, binding_signatures)
                {
                    binding_signatures.insert(name.clone(), signature);
                }
            }
            Stmt::Let { pattern: Pattern::Identifier(name), value, .. } => {
                if let Some(signature) =
                    Self::signature_from_imported_value(value, binding_signatures)
                {
                    binding_signatures.insert(name.clone(), signature);
                }
            }
            Stmt::Assign { target: Expr::Identifier(name), value } => {
                if let Some(signature) =
                    Self::signature_from_imported_value(value, binding_signatures)
                {
                    binding_signatures.insert(name.clone(), signature);
                }
            }
            Stmt::Export { stmt } => {
                self.collect_binding_signatures_from_stmt(stmt, binding_signatures, active_modules);
            }
            Stmt::Block(stmts) => {
                let mut nested_bindings = binding_signatures.clone();
                for nested_stmt in stmts {
                    self.collect_binding_signatures_from_stmt(
                        nested_stmt,
                        &mut nested_bindings,
                        active_modules,
                    );
                }
            }
            _ => {}
        }
    }

    fn parsed_module(&mut self, module_name: &str) -> Option<(PathBuf, Arc<Vec<Stmt>>)> {
        let module_path = self.resolve_module_import_path(module_name)?;
        if let Some(cached) = self.module_ast_cache.get(&module_path) {
            return Some((module_path, cached.clone()));
        }
        let source = fs::read_to_string(&module_path).ok()?;
        let tokens = tokenize_with_file(&source, Some(&module_path.to_string_lossy())).ok()?;
        let mut parser = Parser::new(tokens);
        let parsed = parser.parse_with_diagnostics();
        if !parsed.diagnostics.is_empty() {
            return None;
        }
        #[cfg(test)]
        {
            self.module_parse_count += 1;
        }
        let stmts = Arc::new(parsed.stmts);
        self.module_ast_cache.insert(module_path.clone(), Arc::clone(&stmts));
        Some((module_path, stmts))
    }

    fn module_export_signatures(
        &mut self,
        module_name: &str,
        active_modules: &mut Vec<PathBuf>,
    ) -> Option<HashMap<String, FunctionSignature>> {
        let module_path = self.resolve_module_import_path(module_name)?;

        if let Some(cached) = self.module_export_signatures.get(&module_path) {
            return Some(cached.clone());
        }

        if active_modules.contains(&module_path) {
            return None;
        }
        let (_, module_stmts) = self.parsed_module(module_name)?;

        active_modules.push(module_path.clone());

        let mut binding_signatures = HashMap::new();
        let mut export_signatures = HashMap::new();
        for stmt in module_stmts.iter() {
            self.collect_binding_signatures_from_stmt(
                stmt,
                &mut binding_signatures,
                active_modules,
            );

            if let Stmt::Export { stmt } = stmt {
                self.collect_export_signatures_from_stmt(
                    stmt,
                    &binding_signatures,
                    &mut export_signatures,
                    active_modules,
                );
            }
        }

        active_modules.pop();
        self.module_export_signatures.insert(module_path, export_signatures.clone());
        Some(export_signatures)
    }

    fn struct_shape(fields: &[(String, Option<TypeAnnotation>)], methods: &[Stmt]) -> StructShape {
        let fields = fields.iter().cloned().collect();
        let mut method_signatures = HashMap::new();
        for method in methods {
            if let Stmt::FuncDef { name, params, param_types, return_type, is_async, .. } = method {
                let mut signature = Self::function_signature_from_params(
                    params,
                    param_types,
                    return_type,
                    *is_async,
                );
                if params.first().is_some_and(|param| param == "self")
                    && !signature.param_types.is_empty()
                {
                    signature.param_types.remove(0);
                }
                method_signatures.insert(name.clone(), signature);
            }
        }
        StructShape { fields, methods: method_signatures }
    }

    fn module_export_structs(&mut self, module_name: &str) -> Option<HashMap<String, StructShape>> {
        let module_path = self.resolve_module_import_path(module_name)?;
        if let Some(cached) = self.module_export_structs.get(&module_path) {
            return Some(cached.clone());
        }
        let (_, module_stmts) = self.parsed_module(module_name)?;
        let mut structs = HashMap::new();
        for stmt in module_stmts.iter() {
            let Stmt::Export { stmt } = stmt else { continue };
            if let Stmt::StructDef { name, fields, methods } = stmt.as_ref() {
                structs.insert(name.clone(), Self::struct_shape(fields, methods));
            }
        }
        self.module_export_structs.insert(module_path, structs.clone());
        Some(structs)
    }

    fn static_expr_type(expr: &Expr) -> Option<TypeAnnotation> {
        match expr {
            Expr::Int(_) => Some(TypeAnnotation::Int),
            Expr::Float(_) => Some(TypeAnnotation::Float),
            Expr::String(_) | Expr::InterpolatedString(_) => Some(TypeAnnotation::String),
            Expr::Bool(_) => Some(TypeAnnotation::Bool),
            Expr::ArrayLiteral(values) => {
                let mut item = None;
                for value in values {
                    let next = match value {
                        crate::ast::ArrayElement::Single(value) => Self::static_expr_type(value),
                        crate::ast::ArrayElement::Spread(_) => Some(TypeAnnotation::Any),
                    };
                    item = Self::merge_inferred_types(item, next);
                }
                Some(TypeAnnotation::Array(Box::new(item.unwrap_or(TypeAnnotation::Any))))
            }
            Expr::DictLiteral(values) => {
                let mut key = None;
                let mut value_type = None;
                for value in values {
                    match value {
                        crate::ast::DictElement::Pair(entry_key, entry_value) => {
                            key =
                                Self::merge_inferred_types(key, Self::static_expr_type(entry_key));
                            value_type = Self::merge_inferred_types(
                                value_type,
                                Self::static_expr_type(entry_value),
                            );
                        }
                        crate::ast::DictElement::Spread(_) => {
                            key = Some(TypeAnnotation::Any);
                            value_type = Some(TypeAnnotation::Any);
                        }
                    }
                }
                Some(TypeAnnotation::Dict {
                    key: Box::new(key.unwrap_or(TypeAnnotation::Any)),
                    value: Box::new(value_type.unwrap_or(TypeAnnotation::Any)),
                })
            }
            Expr::StructInstance { name, .. } => Some(TypeAnnotation::Struct(name.clone())),
            _ => None,
        }
    }

    fn module_export_values(
        &mut self,
        module_name: &str,
    ) -> Option<HashMap<String, TypeAnnotation>> {
        let module_path = self.resolve_module_import_path(module_name)?;
        if let Some(cached) = self.module_export_values.get(&module_path) {
            return Some(cached.clone());
        }
        let (_, module_stmts) = self.parsed_module(module_name)?;
        let mut values = HashMap::new();
        for stmt in module_stmts.iter() {
            let Stmt::Export { stmt } = stmt else { continue };
            match stmt.as_ref() {
                Stmt::Const { name, value, type_annotation } => {
                    if let Some(value_type) =
                        type_annotation.clone().or_else(|| Self::static_expr_type(value))
                    {
                        values.insert(name.clone(), value_type);
                    }
                }
                Stmt::Let {
                    pattern: Pattern::Identifier(name), value, type_annotation, ..
                } => {
                    if let Some(value_type) =
                        type_annotation.clone().or_else(|| Self::static_expr_type(value))
                    {
                        values.insert(name.clone(), value_type);
                    }
                }
                _ => {}
            }
        }
        self.module_export_values.insert(module_path, values.clone());
        Some(values)
    }

    fn collect_export_signatures_from_stmt(
        &mut self,
        stmt: &Stmt,
        binding_signatures: &HashMap<String, FunctionSignature>,
        export_signatures: &mut HashMap<String, FunctionSignature>,
        active_modules: &mut Vec<PathBuf>,
    ) {
        match stmt {
            Stmt::FuncDef { name, params, param_types, return_type, is_async, .. } => {
                export_signatures.insert(
                    name.clone(),
                    Self::function_signature_from_params(
                        params,
                        param_types,
                        return_type,
                        *is_async,
                    ),
                );
            }
            Stmt::Const { name, value, .. } => {
                if let Some(signature) =
                    Self::signature_from_imported_value(value, binding_signatures)
                {
                    export_signatures.insert(name.clone(), signature);
                }
            }
            Stmt::Let { pattern: Pattern::Identifier(name), value, .. } => {
                if let Some(signature) =
                    Self::signature_from_imported_value(value, binding_signatures)
                {
                    export_signatures.insert(name.clone(), signature);
                }
            }
            Stmt::Assign { target: Expr::Identifier(name), value } => {
                if let Some(signature) =
                    Self::signature_from_imported_value(value, binding_signatures)
                {
                    export_signatures.insert(name.clone(), signature);
                }
            }
            Stmt::ExprStmt(Expr::Identifier(name)) => {
                if let Some(signature) = binding_signatures.get(name) {
                    export_signatures.insert(name.clone(), signature.clone());
                }
            }
            Stmt::Block(stmts) => {
                let mut nested_bindings = binding_signatures.clone();
                for nested_stmt in stmts {
                    self.collect_binding_signatures_from_stmt(
                        nested_stmt,
                        &mut nested_bindings,
                        active_modules,
                    );
                    self.collect_export_signatures_from_stmt(
                        nested_stmt,
                        &nested_bindings,
                        export_signatures,
                        active_modules,
                    );
                }
            }
            Stmt::Export { stmt } => {
                self.collect_export_signatures_from_stmt(
                    stmt,
                    binding_signatures,
                    export_signatures,
                    active_modules,
                );
            }
            _ => {}
        }
    }

    /// Type check a list of statements
    ///
    /// Returns Ok(()) if type checking succeeds, or Err with collected errors
    pub fn check(&mut self, stmts: &[Stmt]) -> Result<(), Vec<KujoError>> {
        // First pass: collect function signatures
        for stmt in stmts {
            let stmt = match stmt {
                Stmt::Export { stmt } => stmt.as_ref(),
                other => other,
            };
            match stmt {
                Stmt::FuncDef { name, params, param_types, return_type, is_async, .. } => {
                    if name == "contains" {
                        self.builtin_contains_active = false;
                    }
                    self.functions.insert(
                        name.clone(),
                        Self::function_signature_from_params(
                            params,
                            param_types,
                            return_type,
                            *is_async,
                        ),
                    );
                }
                Stmt::StructDef { name, fields, methods } => {
                    self.structs.insert(name.clone(), Self::struct_shape(fields, methods));
                }
                _ => {}
            }
        }

        // Second pass: check statements
        for stmt in stmts {
            self.check_stmt(stmt);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Check a single statement
    fn check_stmt(&mut self, stmt: &Stmt) {
        // Check for excessive recursion depth
        if self.recursion_depth >= MAX_RECURSION_DEPTH {
            self.errors.push(KujoError::new(
                ErrorKind::TypeError,
                format!("Type checker recursion depth exceeded (max: {}). Possible infinite loop in type checking.", MAX_RECURSION_DEPTH),
                SourceLocation::unknown(),
            ));
            return;
        }

        self.recursion_depth += 1;

        match stmt {
            Stmt::Let { pattern, value, type_annotation, .. } => {
                let inferred_type = self.infer_expr(value);

                // For simple identifier patterns, check type compatibility
                if let crate::ast::Pattern::Identifier(name) = pattern {
                    // If type annotation is provided, check compatibility
                    if let Some(annotated_type) = type_annotation {
                        self.annotated_variables.insert(name.clone());
                        if let Some(inferred) = &inferred_type {
                            if !annotated_type.matches(inferred) {
                                let error = KujoError::new(
                                    ErrorKind::TypeError,
                                    format!(
                                        "Type mismatch: variable '{}' declared as {:?} but assigned {:?}",
                                        name, annotated_type, inferred
                                    ),
                                    SourceLocation::unknown(),
                                )
                                .with_help("Try removing the type annotation or converting the value to the correct type".to_string());

                                self.errors.push(error);
                            }
                        }
                        // Store the annotated type
                        self.variables.insert(name.clone(), Some(annotated_type.clone()));
                    } else {
                        // Store the inferred type
                        self.annotated_variables.remove(name);
                        self.variables.insert(name.clone(), inferred_type);
                    }
                } else {
                    if let (Some(annotated), Some(inferred)) = (type_annotation, &inferred_type) {
                        if !annotated.matches(inferred) {
                            self.errors.push(
                                KujoError::new(
                                    ErrorKind::TypeError,
                                    format!(
                                        "Destructuring annotation expects {:?} but the value is {:?}",
                                        annotated, inferred
                                    ),
                                    SourceLocation::unknown(),
                                )
                                .with_help(
                                    "Match the annotation to the collection shape or remove it"
                                        .to_string(),
                                ),
                            );
                        }
                    }
                    self.bind_pattern_from_expr(pattern, value, inferred_type);
                }
            }

            Stmt::Const { name, value, type_annotation } => {
                let inferred_type = self.infer_expr(value);

                // If type annotation is provided, check compatibility
                if let Some(annotated_type) = type_annotation {
                    self.annotated_variables.insert(name.clone());
                    if let Some(inferred) = &inferred_type {
                        if !annotated_type.matches(inferred) {
                            let error = KujoError::new(
                                ErrorKind::TypeError,
                                format!(
									"Type mismatch: constant '{}' declared as {:?} but assigned {:?}",
									name, annotated_type, inferred
								),
                                SourceLocation::unknown(),
                            )
                            .with_help("Constants must be initialized with a value matching their declared type".to_string());

                            self.errors.push(error);
                        }
                    }
                    // Store the annotated type
                    self.variables.insert(name.clone(), Some(annotated_type.clone()));
                } else {
                    // Store the inferred type
                    self.annotated_variables.remove(name);
                    self.variables.insert(name.clone(), inferred_type);
                }
            }

            Stmt::FuncDef {
                name,
                params,
                param_types,
                return_type,
                body,
                is_generator: _,
                is_async,
            } => {
                // Nested declarations belong to the current lexical scope.
                self.variables.insert(
                    name.clone(),
                    Some(Self::function_signature_to_type_annotation(
                        &Self::function_signature_from_params(
                            params,
                            param_types,
                            return_type,
                            *is_async,
                        ),
                    )),
                );
                self.annotated_variables.remove(name);
                // Enter function scope
                let saved_return_type = self.current_function_return.clone();
                self.current_function_return = return_type.clone();
                self.push_scope();
                self.inferred_return_stack.push(None);

                // Add parameters to scope
                for (i, param) in params.iter().enumerate() {
                    let param_type = param_types.get(i).and_then(|t| t.clone()).or_else(|| {
                        (param == "self")
                            .then(|| self.current_struct.clone().map(TypeAnnotation::Struct))
                            .flatten()
                    });
                    self.variables.insert(param.clone(), param_type);
                    self.annotated_variables.remove(param);
                }

                // Check function body
                for stmt in body {
                    self.check_stmt(stmt);
                }
                let inferred_return =
                    self.inferred_return_stack.pop().flatten().unwrap_or(TypeAnnotation::Any);

                // Exit function scope
                self.pop_scope();
                self.current_function_return = saved_return_type;
                if return_type.is_none() {
                    let call_result = if *is_async {
                        TypeAnnotation::Promise(Box::new(inferred_return))
                    } else {
                        inferred_return
                    };
                    if let Some(signature) = self.functions.get_mut(name) {
                        signature.return_type = Some(call_result.clone());
                    }
                    self.variables.insert(
                        name.clone(),
                        Some(TypeAnnotation::Function {
                            params: params
                                .iter()
                                .enumerate()
                                .map(|(index, _)| {
                                    param_types
                                        .get(index)
                                        .cloned()
                                        .flatten()
                                        .unwrap_or(TypeAnnotation::Any)
                                })
                                .collect(),
                            return_type: Box::new(call_result),
                        }),
                    );
                }
            }

            Stmt::Return(expr) => {
                let return_type = expr.as_ref().and_then(|e| self.infer_expr(e));
                if let Some(slot) = self.inferred_return_stack.last_mut() {
                    *slot = Self::merge_inferred_types(slot.take(), return_type.clone());
                }

                // Check if return type matches function signature
                if let Some(expected) = &self.current_function_return {
                    if let Some(actual) = &return_type {
                        if !expected.matches(actual) {
                            let error = KujoError::new(
                                ErrorKind::TypeError,
                                format!(
                                    "Return type mismatch: expected {:?} but got {:?}",
                                    expected, actual
                                ),
                                SourceLocation::unknown(),
                            )
                            .with_help("Make sure the return value matches the function's declared return type".to_string())
                            .with_note(format!("Function expects to return {:?}", expected));

                            self.errors.push(error);
                        }
                    }
                }
            }

            Stmt::If { condition, then_branch, else_branch } => {
                self.infer_expr(condition);
                for s in then_branch {
                    self.check_stmt(s);
                }
                if let Some(else_stmts) = else_branch {
                    for s in else_stmts {
                        self.check_stmt(s);
                    }
                }
            }

            Stmt::Loop { condition: _, body } => {
                for s in body {
                    self.check_stmt(s);
                }
            }

            Stmt::While { condition, body } => {
                self.infer_expr(condition);
                for s in body {
                    self.check_stmt(s);
                }
            }

            Stmt::Break => {
                // No type checking needed for break
            }

            Stmt::Continue => {
                // No type checking needed for continue
            }

            Stmt::For { var, iterable, body } => {
                let iterable_type = self.infer_expr(iterable);
                let item_type = match iterable_type {
                    Some(TypeAnnotation::Array(inner)) => Some(*inner),
                    Some(TypeAnnotation::String) => Some(TypeAnnotation::String),
                    Some(TypeAnnotation::Dict { key, .. }) => Some(*key),
                    _ => None,
                };
                self.push_scope();
                self.variables.insert(var.clone(), item_type);
                self.annotated_variables.remove(var);
                for s in body {
                    self.check_stmt(s);
                }
                self.pop_scope();
            }

            Stmt::Spawn { body } => {
                // Check the spawn body in a new scope
                self.push_scope();
                for s in body {
                    self.check_stmt(s);
                }
                self.pop_scope();
            }

            Stmt::Test { body, .. } | Stmt::TestSetup { body } | Stmt::TestTeardown { body } => {
                // Check test body in a new scope
                self.push_scope();
                for s in body {
                    self.check_stmt(s);
                }
                self.pop_scope();
            }

            Stmt::TestGroup { tests, .. } => {
                // Check all test statements in the group
                for s in tests {
                    self.check_stmt(s);
                }
            }

            Stmt::Match { value, cases, default } => {
                self.infer_expr(value);
                for (pattern, case_body) in cases {
                    self.push_scope();
                    if let Some((_, binding)) = pattern.split_once('(') {
                        let binding = binding.trim_end_matches(')').trim();
                        self.variables.insert(binding.to_string(), None);
                        self.annotated_variables.remove(binding);
                    }
                    for s in case_body {
                        self.check_stmt(s);
                    }
                    self.pop_scope();
                }
                if let Some(default_body) = default {
                    for s in default_body {
                        self.check_stmt(s);
                    }
                }
            }

            Stmt::TryExcept { try_block, except_var, except_block } => {
                for s in try_block {
                    self.check_stmt(s);
                }
                self.push_scope();
                self.variables.insert(except_var.clone(), None);
                self.annotated_variables.remove(except_var);
                for s in except_block {
                    self.check_stmt(s);
                }
                self.pop_scope();
            }

            Stmt::ExprStmt(expr) => {
                self.infer_expr(expr);
            }

            Stmt::Assign { target, value } => {
                let inferred_type = self.infer_expr(value);

                // Check based on assignment target
                match target {
                    Expr::Identifier(name) => {
                        // Only explicit annotations constrain reassignment in gradual code.
                        if !self.annotated_variables.contains(name) {
                            self.variables.insert(name.clone(), inferred_type);
                        } else if let Some(Some(expected)) = self.variables.get(name) {
                            if let Some(actual) = &inferred_type {
                                if !expected.matches(actual) {
                                    let error = KujoError::new(
                                        ErrorKind::TypeError,
                                        format!(
                                            "Type mismatch: cannot assign {:?} to variable '{}' of type {:?}",
                                            actual, name, expected
                                        ),
                                        SourceLocation::unknown(),
                                    )
                                    .with_help("Try converting the value with to_int(), to_float(), to_string(), or to_bool()".to_string())
                                    .with_note(format!("Variable '{}' was declared with type {:?}", name, expected));

                                    self.errors.push(error);
                                }
                            }
                        }
                    }
                    Expr::IndexAccess { .. } => {
                        // Type checking for index assignment would need more sophisticated analysis
                        // For now, just type-check the value expression
                    }
                    _ => {
                        // Invalid assignment target - parser should have caught this
                    }
                }
            }

            Stmt::Block(stmts) => {
                for s in stmts {
                    self.check_stmt(s);
                }
            }

            Stmt::EnumDef { .. } => {
                // Enums don't require type checking
            }

            Stmt::Import { module, symbols } => {
                let mut active_modules = Vec::new();
                let module_signatures = self.module_export_signatures(module, &mut active_modules);
                let module_structs = self.module_export_structs(module);
                let module_values = self.module_export_values(module);
                let module_exists = self.resolve_module_import_path(module).is_some();
                if !module_exists && self.validate_module_existence {
                    self.errors.push(
                        KujoError::new(
                            ErrorKind::TypeError,
                            format!("Unknown module '{}'", module),
                            SourceLocation::unknown(),
                        )
                        .with_help(
                            "Check the module name or add its root to the module search path"
                                .to_string(),
                        )
                        .with_note(
                            "Static analysis will keep imported values dynamic so optional typing does not block runtime resolution"
                                .to_string(),
                        ),
                    );
                }

                if let Some(symbols) = symbols {
                    for symbol in symbols {
                        if let Some(shape) =
                            module_structs.as_ref().and_then(|structs| structs.get(symbol)).cloned()
                        {
                            self.structs.insert(symbol.clone(), shape);
                        }
                        if let Some(value_type) =
                            module_values.as_ref().and_then(|values| values.get(symbol)).cloned()
                        {
                            self.variables.insert(symbol.clone(), Some(value_type));
                        }
                        let signature = module_signatures
                            .as_ref()
                            .and_then(|signatures| signatures.get(symbol))
                            .cloned();
                        if signature.is_some() || !self.variables.contains_key(symbol) {
                            self.register_imported_symbol(
                                symbol,
                                signature,
                                module_signatures.is_none(),
                            );
                        }
                    }
                } else {
                    if let Some(signatures) = &module_signatures {
                        for (name, signature) in signatures {
                            self.register_imported_symbol(name, Some(signature.clone()), false);
                        }
                    }
                    if let Some(structs) = &module_structs {
                        self.structs.extend(structs.clone());
                    }
                    if let Some(values) = &module_values {
                        for (name, value_type) in values {
                            self.variables.insert(name.clone(), Some(value_type.clone()));
                        }
                    }
                    let binding = crate::vm::VM::module_binding_name(module);
                    self.variables.insert(binding, Some(TypeAnnotation::Module(module.clone())));
                }
            }

            Stmt::Export { stmt } => {
                // Type check the exported statement
                self.check_stmt(stmt);
            }

            Stmt::StructDef { name, fields, methods } => {
                self.structs.insert(name.clone(), Self::struct_shape(fields, methods));
                // Type check methods
                for method in methods {
                    self.push_scope();
                    let saved_struct = self.current_struct.replace(name.clone());
                    self.variables
                        .insert("self".to_string(), Some(TypeAnnotation::Struct(name.clone())));
                    for (field, field_type) in fields {
                        self.variables.insert(
                            field.clone(),
                            Some(field_type.clone().unwrap_or(TypeAnnotation::Any)),
                        );
                    }
                    self.check_stmt(method);
                    if let Stmt::FuncDef { name: method_name, .. } = method {
                        let inferred_return = match self.variables.get(method_name) {
                            Some(Some(TypeAnnotation::Function { return_type, .. })) => {
                                Some((**return_type).clone())
                            }
                            _ => None,
                        };
                        if let (Some(inferred_return), Some(signature)) = (
                            inferred_return,
                            self.structs
                                .get_mut(name)
                                .and_then(|shape| shape.methods.get_mut(method_name)),
                        ) {
                            signature.return_type = Some(inferred_return);
                        }
                    }
                    self.current_struct = saved_struct;
                    self.pop_scope();
                }
            }
        }

        self.recursion_depth -= 1;
    }

    /// Infer the type of an expression
    fn infer_expr(&mut self, expr: &Expr) -> Option<TypeAnnotation> {
        // Check for excessive recursion depth
        if self.recursion_depth >= MAX_RECURSION_DEPTH {
            self.errors.push(KujoError::new(
                ErrorKind::TypeError,
                format!("Type checker recursion depth exceeded (max: {}). Possible infinite loop in type inference.", MAX_RECURSION_DEPTH),
                SourceLocation::unknown(),
            ));
            return None;
        }

        self.recursion_depth += 1;

        let result = self.infer_expr_inner(expr);
        self.recursion_depth -= 1;
        result
    }

    fn infer_function_expression(
        &mut self,
        params: &[String],
        param_types: &[Option<TypeAnnotation>],
        return_type: &Option<TypeAnnotation>,
        body: &[Stmt],
        is_async: bool,
        contextual_first_param: Option<TypeAnnotation>,
    ) -> TypeAnnotation {
        self.push_scope();
        for (index, param) in params.iter().enumerate() {
            let param_type = param_types
                .get(index)
                .cloned()
                .flatten()
                .or_else(|| (index == 0).then(|| contextual_first_param.clone()).flatten());
            self.variables.insert(param.clone(), param_type);
            self.annotated_variables.remove(param);
        }
        let saved_return_type = self.current_function_return.clone();
        self.current_function_return = return_type.clone();
        self.inferred_return_stack.push(None);
        for stmt in body {
            self.check_stmt(stmt);
        }
        let inferred_return =
            self.inferred_return_stack.pop().flatten().unwrap_or(TypeAnnotation::Any);
        self.pop_scope();
        self.current_function_return = saved_return_type;
        let call_result = return_type.clone().unwrap_or(inferred_return);
        let call_result =
            if is_async { TypeAnnotation::Promise(Box::new(call_result)) } else { call_result };
        TypeAnnotation::Function {
            params: params
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    param_types
                        .get(index)
                        .cloned()
                        .flatten()
                        .or_else(|| (index == 0).then(|| contextual_first_param.clone()).flatten())
                        .unwrap_or(TypeAnnotation::Any)
                })
                .collect(),
            return_type: Box::new(call_result),
        }
    }

    fn overloaded_binary_result(
        &self,
        op: &str,
        left_type: &Option<TypeAnnotation>,
    ) -> Option<TypeAnnotation> {
        let TypeAnnotation::Struct(name) = left_type.as_ref()? else {
            return None;
        };
        let method_name = crate::ast::operator_methods::binary_op_method(op)?;
        let signature = self.structs.get(name)?.methods.get(method_name)?;
        Some(signature.return_type.clone().unwrap_or(TypeAnnotation::Any))
    }

    // Keep early returns in expression inference inside the balanced depth guard.
    fn infer_expr_inner(&mut self, expr: &Expr) -> Option<TypeAnnotation> {
        match expr {
            Expr::Int(_) => Some(TypeAnnotation::Int),
            Expr::Float(_) => Some(TypeAnnotation::Float),

            Expr::String(_) => Some(TypeAnnotation::String),

            Expr::InterpolatedString(parts) => {
                // Type check all embedded expressions
                use crate::ast::InterpolatedStringPart;
                for part in parts {
                    if let InterpolatedStringPart::Expr(expr) = part {
                        self.infer_expr(expr);
                    }
                }
                // Interpolated strings always produce strings
                Some(TypeAnnotation::String)
            }

            Expr::Bool(_) => Some(TypeAnnotation::Bool),

            Expr::Identifier(name) => {
                // Look up variable type in symbol table
                match self.variables.get(name) {
                    Some(value) => value.clone(),
                    None => {
                        self.functions.get(name).map(Self::function_signature_to_type_annotation)
                    }
                }
            }

            Expr::UnaryOp { op, operand } => {
                let operand_type = self.infer_expr(operand);

                match op.as_str() {
                    "-" => {
                        // Unary minus on numbers
                        match operand_type {
                            Some(TypeAnnotation::Int) => Some(TypeAnnotation::Int),
                            Some(TypeAnnotation::Float) => Some(TypeAnnotation::Float),
                            _ => operand_type, // Could be struct with op_neg
                        }
                    }
                    "!" => {
                        // Logical not on booleans
                        match operand_type {
                            Some(TypeAnnotation::Bool) => Some(TypeAnnotation::Bool),
                            _ => operand_type, // Could be struct with op_not
                        }
                    }
                    _ => None,
                }
            }

            Expr::BinaryOp { op, left, right } => {
                let left_type = self.infer_expr(left);
                let right_type = self.infer_expr(right);

                if let Some(result) = self.overloaded_binary_result(op, &left_type) {
                    return if matches!(op.as_str(), "==" | "!=" | "<" | ">" | "<=" | ">=") {
                        Some(TypeAnnotation::Bool)
                    } else {
                        Some(result)
                    };
                }

                match op.as_str() {
                    "==" | "!=" | "<" | ">" | "<=" | ">=" => {
                        // Comparison operations always return bool
                        // Check that operands are comparable
                        if let (Some(l), Some(r)) = (&left_type, &right_type) {
                            if !l.matches(r) && !r.matches(l) {
                                let error = KujoError::new(
                                    ErrorKind::TypeError,
                                    format!(
                                        "Comparison '{}' between incompatible types: {:?} and {:?}",
                                        op, l, r
                                    ),
                                    SourceLocation::unknown(),
                                )
                                .with_help("Convert one value to match the type of the other".to_string())
                                .with_note("Comparison operators require both operands to have compatible types".to_string());

                                self.errors.push(error);
                            }
                        }
                        Some(TypeAnnotation::Bool)
                    }
                    "+" | "-" | "*" | "/" => {
                        // Arithmetic operations with type promotion
                        match (&left_type, &right_type) {
                            // Int op Int => Int
                            (Some(TypeAnnotation::Int), Some(TypeAnnotation::Int)) => {
                                Some(TypeAnnotation::Int)
                            }
                            // Int op Float or Float op Int => Float (type promotion)
                            (Some(TypeAnnotation::Int), Some(TypeAnnotation::Float))
                            | (Some(TypeAnnotation::Float), Some(TypeAnnotation::Int))
                            | (Some(TypeAnnotation::Float), Some(TypeAnnotation::Float)) => {
                                Some(TypeAnnotation::Float)
                            }
                            // String + String => String
                            (Some(TypeAnnotation::String), Some(TypeAnnotation::String))
                                if op == "+" =>
                            {
                                Some(TypeAnnotation::String)
                            }
                            // Gradual typing: Any is intentionally compatible with a
                            // known operand and cannot justify a static mismatch.
                            (Some(TypeAnnotation::Any), _) | (_, Some(TypeAnnotation::Any)) => None,
                            // Incompatible types
                            (Some(l), Some(r)) if l != r => {
                                self.errors.push(KujoError::new(
                                    ErrorKind::TypeError,
                                    format!(
										"Binary operation '{}' with incompatible types: {:?} and {:?}",
										op, l, r
									),
                                    SourceLocation::unknown(),
                                ));
                                None
                            }
                            _ => None, // Unknown types
                        }
                    }
                    _ => None,
                }
            }

            Expr::Call { function, args } => {
                // Look up function signature
                if let Expr::Identifier(func_name) = &**function {
                    if matches!(func_name.as_str(), "parallel_map" | "par_map") && args.len() >= 2 {
                        let collection_type = self.infer_expr(&args[0]);
                        let item_type = match collection_type {
                            Some(TypeAnnotation::Array(item)) => Some(*item),
                            _ => None,
                        };
                        if let (
                            Some(item_type),
                            Expr::Function {
                                params, param_types, return_type, body, is_async, ..
                            },
                        ) = (item_type, &args[1])
                        {
                            let callback_type = self.infer_function_expression(
                                params,
                                param_types,
                                return_type,
                                body,
                                *is_async,
                                Some(item_type),
                            );
                            for argument in args.iter().skip(2) {
                                self.infer_expr(argument);
                            }
                            let callback_result = match callback_type {
                                TypeAnnotation::Function { return_type, .. } => {
                                    match *return_type {
                                        TypeAnnotation::Promise(inner) => *inner,
                                        value => value,
                                    }
                                }
                                _ => TypeAnnotation::Any,
                            };
                            return Some(TypeAnnotation::Promise(Box::new(TypeAnnotation::Array(
                                Box::new(callback_result),
                            ))));
                        }
                    }
                    if func_name == "contains"
                        && self.builtin_contains_active
                        && !self.variables.contains_key(func_name)
                    {
                        return self.infer_builtin_contains(args);
                    }
                    // Lexical values shadow global signatures, including unknown callable results.
                    let sig = if let Some(variable) = self.variables.get(func_name).cloned() {
                        match variable {
                            Some(TypeAnnotation::Function { params, return_type }) => {
                                Some(FunctionSignature {
                                    param_types: params.into_iter().map(Some).collect(),
                                    return_type: Some(*return_type),
                                })
                            }
                            unknown @ (None | Some(TypeAnnotation::Any)) => {
                                for arg in args {
                                    self.infer_expr(arg);
                                }
                                return unknown;
                            }
                            Some(other) => {
                                for arg in args {
                                    self.infer_expr(arg);
                                }
                                self.errors.push(KujoError::new(
                                    ErrorKind::TypeError,
                                    format!(
                                        "Cannot call '{}' with non-callable type {:?}",
                                        func_name, other
                                    ),
                                    SourceLocation::unknown(),
                                ));
                                return None;
                            }
                        }
                    } else {
                        self.functions.get(func_name).cloned()
                    };

                    if let Some(sig) = sig {
                        // Skip type checking for variadic functions (empty param_types means variadic)
                        let is_variadic = sig.param_types.is_empty();

                        if !is_variadic {
                            // Check argument count - allow fewer args than params if trailing params are optional (None)
                            let min_required =
                                sig.param_types.iter().take_while(|p| p.is_some()).count();
                            let max_allowed = sig.param_types.len();

                            if args.len() < min_required || args.len() > max_allowed {
                                self.errors.push(KujoError::new(
                                    ErrorKind::TypeError,
                                    format!(
                                        "Function '{}' expects {}-{} arguments but got {}",
                                        func_name,
                                        min_required,
                                        max_allowed,
                                        args.len()
                                    ),
                                    SourceLocation::unknown(),
                                ));
                            }
                        }

                        // Check argument types
                        for (i, arg) in args.iter().enumerate() {
                            let arg_type = self.infer_expr(arg);
                            if let Some(expected) = sig.param_types.get(i).and_then(|t| t.as_ref())
                            {
                                if let Some(actual) = &arg_type {
                                    if !expected.matches(actual) {
                                        self.errors.push(KujoError::new(
                                            ErrorKind::TypeError,
                                            format!(
												"Function '{}' parameter {} expects {:?} but got {:?}",
												func_name, i + 1, expected, actual
											),
                                            SourceLocation::unknown(),
                                        ));
                                    }
                                }
                            }
                        }

                        // Return the function's return type
                        return sig.return_type.clone();
                    } else {
                        // Function not found - suggest similar functions
                        let available_functions = self.get_available_functions();
                        let suggestion =
                            crate::errors::find_closest_match(func_name, &available_functions);

                        let mut error = KujoError::new(
                            ErrorKind::UndefinedFunction,
                            format!("Undefined function '{}'", func_name),
                            SourceLocation::unknown(),
                        );

                        if let Some(suggested) = suggestion {
                            error = error.with_suggestion(suggested.to_string());
                        }

                        error = error
                            .with_note("Function must be defined before it is called".to_string());

                        self.errors.push(error);
                    }
                } else {
                    let callable_type = self.infer_expr(function);
                    let arg_types: Vec<_> =
                        args.iter().map(|argument| self.infer_expr(argument)).collect();
                    match callable_type {
                        Some(TypeAnnotation::Function { params, return_type }) => {
                            if args.len() != params.len() {
                                self.errors.push(KujoError::new(
                                    ErrorKind::TypeError,
                                    format!(
                                        "Callable expects {} arguments but got {}",
                                        params.len(),
                                        args.len()
                                    ),
                                    SourceLocation::unknown(),
                                ));
                            }
                            for (index, (expected, actual)) in
                                params.iter().zip(arg_types.iter()).enumerate()
                            {
                                if let Some(actual) = actual {
                                    if !expected.matches(actual) {
                                        self.errors.push(KujoError::new(
                                            ErrorKind::TypeError,
                                            format!(
                                                "Callable parameter {} expects {:?} but got {:?}",
                                                index + 1,
                                                expected,
                                                actual
                                            ),
                                            SourceLocation::unknown(),
                                        ));
                                    }
                                }
                            }
                            return Some(*return_type);
                        }
                        None | Some(TypeAnnotation::Any) => return Some(TypeAnnotation::Any),
                        Some(other) => {
                            self.errors.push(
                                KujoError::new(
                                    ErrorKind::TypeError,
                                    format!("Cannot call value with non-callable type {:?}", other),
                                    SourceLocation::unknown(),
                                )
                                .with_help(
                                    "Call a function or keep the value dynamic until runtime"
                                        .to_string(),
                                ),
                            );
                        }
                    }
                }
                None
            }

            Expr::Tag(_, _) => None, // Enum types not yet supported

            Expr::StructInstance { name, fields } => {
                let shape = self.structs.get(name).cloned();
                for (field_name, field_expr) in fields {
                    let actual = self.infer_expr(field_expr);
                    if let Some(shape) = &shape {
                        match shape.fields.get(field_name) {
                            Some(Some(expected)) => {
                                if let Some(actual) = &actual {
                                    if !expected.matches(actual) {
                                        self.errors.push(
                                            KujoError::new(
                                                ErrorKind::TypeError,
                                                format!(
                                                    "Struct '{}.{}' expects {:?} but got {:?}",
                                                    name, field_name, expected, actual
                                                ),
                                                SourceLocation::unknown(),
                                            )
                                            .with_help(
                                                "Convert the field value or correct the struct annotation"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            }
                            Some(None) => {}
                            None => self.errors.push(
                                KujoError::new(
                                    ErrorKind::TypeError,
                                    format!(
                                        "Struct '{}' has no declared field '{}'",
                                        name, field_name
                                    ),
                                    SourceLocation::unknown(),
                                )
                                .with_help(
                                    "Use a declared field name or update the struct definition"
                                        .to_string(),
                                ),
                            ),
                        }
                    }
                }
                if shape.is_none() {
                    self.errors.push(
                        KujoError::new(
                            ErrorKind::TypeError,
                            format!("Unknown struct '{}'", name),
                            SourceLocation::unknown(),
                        )
                        .with_help(
                            "Define or import the struct before constructing it".to_string(),
                        ),
                    );
                    Some(TypeAnnotation::Any)
                } else {
                    Some(TypeAnnotation::Struct(name.clone()))
                }
            }

            Expr::FieldAccess { object, field } => match self.infer_expr(object) {
                Some(TypeAnnotation::Struct(name)) => {
                    let shape = self.structs.get(&name).cloned();
                    if let Some(shape) = shape {
                        if let Some(field_type) = shape.fields.get(field) {
                            return Some(field_type.clone().unwrap_or(TypeAnnotation::Any));
                        }
                        if let Some(signature) = shape.methods.get(field) {
                            return Some(Self::function_signature_to_type_annotation(signature));
                        }
                        self.errors.push(
                            KujoError::new(
                                ErrorKind::TypeError,
                                format!("Struct '{}' has no field or method '{}'", name, field),
                                SourceLocation::unknown(),
                            )
                            .with_help(
                                "Check the field name against the struct definition".to_string(),
                            ),
                        );
                        Some(TypeAnnotation::Any)
                    } else {
                        Some(TypeAnnotation::Any)
                    }
                }
                Some(TypeAnnotation::Module(module)) => {
                    let mut active_modules = Vec::new();
                    let functions = self.module_export_signatures(&module, &mut active_modules);
                    let values = self.module_export_values(&module);
                    let structs = self.module_export_structs(&module);
                    if let Some(signature) =
                        functions.as_ref().and_then(|exports| exports.get(field)).cloned()
                    {
                        Some(Self::function_signature_to_type_annotation(&signature))
                    } else if let Some(value_type) =
                        values.as_ref().and_then(|exports| exports.get(field)).cloned()
                    {
                        Some(value_type)
                    } else if functions.is_some()
                        && values.is_some()
                        && structs.is_some()
                        && !structs.as_ref().is_some_and(|exports| exports.contains_key(field))
                    {
                        self.errors.push(
                            KujoError::new(
                                ErrorKind::TypeError,
                                format!("Module '{}' has no exported member '{}'", module, field),
                                SourceLocation::unknown(),
                            )
                            .with_help("Check the export name in the imported module".to_string()),
                        );
                        Some(TypeAnnotation::Any)
                    } else {
                        Some(TypeAnnotation::Any)
                    }
                }
                None | Some(TypeAnnotation::Any) => Some(TypeAnnotation::Any),
                Some(_) => Some(TypeAnnotation::Any),
            },

            Expr::ArrayLiteral(elements) => {
                use crate::ast::ArrayElement;
                let mut inferred_element_type: Option<TypeAnnotation> = None;

                for elem in elements {
                    let current_type = match elem {
                        ArrayElement::Single(expr) => self.infer_expr(expr),
                        ArrayElement::Spread(expr) => {
                            let spread_type = self.infer_expr(expr);
                            match spread_type {
                                Some(TypeAnnotation::Array(inner_type)) => {
                                    Some((*inner_type).clone())
                                }
                                Some(other) => {
                                    self.errors.push(KujoError::new(
                                        ErrorKind::TypeError,
                                        format!(
                                            "Array spread expects Array value, got {:?}",
                                            other
                                        ),
                                        SourceLocation::unknown(),
                                    ));
                                    Some(TypeAnnotation::Any)
                                }
                                None => Some(TypeAnnotation::Any),
                            }
                        }
                    };

                    inferred_element_type =
                        Self::merge_inferred_types(inferred_element_type, current_type);
                }

                Some(TypeAnnotation::Array(Box::new(
                    inferred_element_type.unwrap_or(TypeAnnotation::Any),
                )))
            }

            Expr::DictLiteral(pairs) => {
                use crate::ast::DictElement;
                let mut inferred_key_type: Option<TypeAnnotation> = None;
                let mut inferred_value_type: Option<TypeAnnotation> = None;

                for elem in pairs {
                    match elem {
                        DictElement::Pair(key, value) => {
                            let key_type = self.infer_expr(key);
                            let value_type = self.infer_expr(value);
                            inferred_key_type =
                                Self::merge_inferred_types(inferred_key_type, key_type);
                            inferred_value_type =
                                Self::merge_inferred_types(inferred_value_type, value_type);
                        }
                        DictElement::Spread(expr) => {
                            let spread_type = self.infer_expr(expr);
                            match spread_type {
                                Some(TypeAnnotation::Dict { key, value }) => {
                                    inferred_key_type = Self::merge_inferred_types(
                                        inferred_key_type,
                                        Some((*key).clone()),
                                    );
                                    inferred_value_type = Self::merge_inferred_types(
                                        inferred_value_type,
                                        Some((*value).clone()),
                                    );
                                }
                                Some(other) => {
                                    self.errors.push(KujoError::new(
                                        ErrorKind::TypeError,
                                        format!("Dict spread expects Dict value, got {:?}", other),
                                        SourceLocation::unknown(),
                                    ));
                                    inferred_key_type = Self::merge_inferred_types(
                                        inferred_key_type,
                                        Some(TypeAnnotation::Any),
                                    );
                                    inferred_value_type = Self::merge_inferred_types(
                                        inferred_value_type,
                                        Some(TypeAnnotation::Any),
                                    );
                                }
                                None => {
                                    inferred_key_type = Self::merge_inferred_types(
                                        inferred_key_type,
                                        Some(TypeAnnotation::Any),
                                    );
                                    inferred_value_type = Self::merge_inferred_types(
                                        inferred_value_type,
                                        Some(TypeAnnotation::Any),
                                    );
                                }
                            }
                        }
                    }
                }

                Some(TypeAnnotation::Dict {
                    key: Box::new(inferred_key_type.unwrap_or(TypeAnnotation::Any)),
                    value: Box::new(inferred_value_type.unwrap_or(TypeAnnotation::Any)),
                })
            }

            Expr::IndexAccess { object, index } => {
                let object_type = self.infer_expr(object);
                let index_type = self.infer_expr(index);

                match object_type {
                    Some(TypeAnnotation::Array(inner_type)) => Some((*inner_type).clone()),
                    Some(TypeAnnotation::Dict { value, .. }) => Some((*value).clone()),
                    Some(TypeAnnotation::String) => {
                        if matches!(
                            index_type,
                            Some(TypeAnnotation::Int) | Some(TypeAnnotation::Any)
                        ) {
                            Some(TypeAnnotation::String)
                        } else {
                            self.errors.push(KujoError::new(
                                ErrorKind::TypeError,
                                "String index access expects integer index".to_string(),
                                SourceLocation::unknown(),
                            ));
                            Some(TypeAnnotation::Any)
                        }
                    }
                    Some(_) | None => Some(TypeAnnotation::Any),
                }
            }

            Expr::Function {
                params,
                param_types,
                return_type,
                body,
                is_generator: _,
                is_async,
            } => Some(self.infer_function_expression(
                params,
                param_types,
                return_type,
                body,
                *is_async,
                None,
            )),

            Expr::Ok(value_expr) => {
                let value_type = self.infer_expr(value_expr);
                // Result<T, Any> - we don't know the error type without more context
                value_type.map(|t| TypeAnnotation::Result {
                    ok_type: Box::new(t),
                    err_type: Box::new(TypeAnnotation::Any),
                })
            }

            Expr::Err(error_expr) => {
                let error_type = self.infer_expr(error_expr);
                // Result<Any, E> - we don't know the ok type without more context
                error_type.map(|t| TypeAnnotation::Result {
                    ok_type: Box::new(TypeAnnotation::Any),
                    err_type: Box::new(t),
                })
            }

            Expr::Some(value_expr) => {
                let value_type = self.infer_expr(value_expr);
                value_type.map(|t| TypeAnnotation::Option { inner_type: Box::new(t) })
            }

            Expr::None => {
                // Option<Any> - we don't know the inner type without more context
                Some(TypeAnnotation::Option { inner_type: Box::new(TypeAnnotation::Any) })
            }

            Expr::Try(expr) => {
                let expr_type = self.infer_expr(expr);
                // Try operator unwraps Result<T, E> to T
                match expr_type {
                    Some(TypeAnnotation::Result { ok_type, .. }) => Some(*ok_type),
                    None | Some(TypeAnnotation::Any) => None,
                    _ => {
                        // Type error: try operator on non-Result value
                        self.errors.push(KujoError::new(
                            ErrorKind::TypeError,
                            "Try operator (?) can only be used on Result values".to_string(),
                            SourceLocation::unknown(),
                        ));
                        None
                    }
                }
            }

            Expr::Yield(value_expr) => {
                // Yield expressions can return any type
                if let Some(expr) = value_expr {
                    self.infer_expr(expr)
                } else {
                    Some(TypeAnnotation::Any)
                }
            }

            Expr::MethodCall { object, method, args } => {
                // Type check the object and arguments
                let object_type = self.infer_expr(object);
                if let (
                    Some(TypeAnnotation::Array(item)),
                    Some(Expr::Function {
                        params, param_types, return_type, body, is_async, ..
                    }),
                ) = (&object_type, args.first())
                {
                    if matches!(method.as_str(), "map" | "filter") {
                        let callback_type = self.infer_function_expression(
                            params,
                            param_types,
                            return_type,
                            body,
                            *is_async,
                            Some((**item).clone()),
                        );
                        let mapped = match (method.as_str(), callback_type) {
                            ("filter", _) => (**item).clone(),
                            (_, TypeAnnotation::Function { return_type, .. }) => match *return_type
                            {
                                TypeAnnotation::Promise(inner) => *inner,
                                value => value,
                            },
                            _ => TypeAnnotation::Any,
                        };
                        for argument in args.iter().skip(1) {
                            self.infer_expr(argument);
                        }
                        return Some(TypeAnnotation::Array(Box::new(mapped)));
                    }
                }
                let arg_types: Vec<_> = args.iter().map(|arg| self.infer_expr(arg)).collect();
                if let Some(TypeAnnotation::Struct(name)) = &object_type {
                    if let Some(signature) =
                        self.structs.get(name).and_then(|shape| shape.methods.get(method)).cloned()
                    {
                        let params = &signature.param_types[..];
                        if !params.is_empty() && args.len() != params.len() {
                            self.errors.push(KujoError::new(
                                ErrorKind::TypeError,
                                format!(
                                    "Method '{}.{}' expects {} arguments but got {}",
                                    name,
                                    method,
                                    params.len(),
                                    args.len()
                                ),
                                SourceLocation::unknown(),
                            ));
                        }
                        for (index, (expected, actual)) in
                            params.iter().zip(arg_types.iter()).enumerate()
                        {
                            if let (Some(expected), Some(actual)) = (expected, actual) {
                                if !expected.matches(actual) {
                                    self.errors.push(KujoError::new(
                                        ErrorKind::TypeError,
                                        format!(
                                            "Method '{}.{}' parameter {} expects {:?} but got {:?}",
                                            name,
                                            method,
                                            index + 1,
                                            expected,
                                            actual
                                        ),
                                        SourceLocation::unknown(),
                                    ));
                                }
                            }
                        }
                        return signature.return_type;
                    }
                }
                Self::infer_known_method_return_type(object_type.as_ref(), method)
                    .or(Some(TypeAnnotation::Any))
            }

            Expr::Spread(expr) => {
                // Type check the spread expression
                self.infer_expr(expr);
                None
            }

            Expr::Await(promise_expr) => {
                match self.infer_expr(promise_expr) {
                    Some(TypeAnnotation::Promise(inner)) => Some(*inner),
                    None | Some(TypeAnnotation::Any) => Some(TypeAnnotation::Any),
                    // Kujo deliberately permits `await` on an already-complete value.
                    known => known,
                }
            }
        }
    }

    fn bind_pattern_from_expr(
        &mut self,
        pattern: &Pattern,
        value: &Expr,
        inferred_type: Option<TypeAnnotation>,
    ) {
        match (pattern, value) {
            (Pattern::Array { elements, rest }, Expr::ArrayLiteral(values)) => {
                for (index, child) in elements.iter().enumerate() {
                    let value = values.get(index);
                    match value {
                        Some(crate::ast::ArrayElement::Single(expr)) => {
                            let child_type = self.infer_expr(expr);
                            self.bind_pattern_from_expr(child, expr, child_type);
                        }
                        _ => self.bind_pattern_type(child, Some(TypeAnnotation::Any)),
                    }
                }
                if let Some(rest) = rest {
                    let mut rest_type = None;
                    for value in values.iter().skip(elements.len()) {
                        let value_type = match value {
                            crate::ast::ArrayElement::Single(expr) => self.infer_expr(expr),
                            crate::ast::ArrayElement::Spread(expr) => match self.infer_expr(expr) {
                                Some(TypeAnnotation::Array(inner)) => Some(*inner),
                                _ => Some(TypeAnnotation::Any),
                            },
                        };
                        rest_type = Self::merge_inferred_types(rest_type, value_type);
                    }
                    self.variables.insert(
                        rest.clone(),
                        Some(TypeAnnotation::Array(Box::new(
                            rest_type.unwrap_or(TypeAnnotation::Any),
                        ))),
                    );
                    self.annotated_variables.remove(rest);
                }
            }
            (Pattern::Dict { keys, rest }, Expr::DictLiteral(values)) => {
                for key in keys {
                    let value = values.iter().find_map(|entry| match entry {
                        crate::ast::DictElement::Pair(Expr::String(candidate), value)
                            if candidate == key =>
                        {
                            Some(value)
                        }
                        _ => None,
                    });
                    let value_type = value.and_then(|value| self.infer_expr(value));
                    self.variables
                        .insert(key.clone(), Some(value_type.unwrap_or(TypeAnnotation::Any)));
                    self.annotated_variables.remove(key);
                }
                if let Some(rest) = rest {
                    let rest_type = match inferred_type {
                        Some(TypeAnnotation::Dict { key, value }) => {
                            TypeAnnotation::Dict { key, value }
                        }
                        _ => TypeAnnotation::Dict {
                            key: Box::new(TypeAnnotation::Any),
                            value: Box::new(TypeAnnotation::Any),
                        },
                    };
                    self.variables.insert(rest.clone(), Some(rest_type));
                    self.annotated_variables.remove(rest);
                }
            }
            _ => self.bind_pattern_type(pattern, inferred_type),
        }
    }

    fn bind_pattern_type(&mut self, pattern: &Pattern, inferred_type: Option<TypeAnnotation>) {
        match pattern {
            Pattern::Identifier(name) => {
                self.variables.insert(name.clone(), inferred_type);
                self.annotated_variables.remove(name);
            }
            Pattern::Ignore => {}
            Pattern::Array { elements, rest } => {
                let element_type = match inferred_type {
                    Some(TypeAnnotation::Array(inner)) => Some(*inner),
                    None | Some(TypeAnnotation::Any) => Some(TypeAnnotation::Any),
                    Some(other) => {
                        self.errors.push(
                            KujoError::new(
                                ErrorKind::TypeError,
                                format!(
                                    "Array destructuring expects an Array value, inferred {:?}",
                                    other
                                ),
                                SourceLocation::unknown(),
                            )
                            .with_help(
                                "Use an array value or bind the value without destructuring"
                                    .to_string(),
                            ),
                        );
                        Some(TypeAnnotation::Any)
                    }
                };
                for child in elements {
                    self.bind_pattern_type(child, element_type.clone());
                }
                if let Some(rest) = rest {
                    self.variables.insert(
                        rest.clone(),
                        Some(TypeAnnotation::Array(Box::new(
                            element_type.unwrap_or(TypeAnnotation::Any),
                        ))),
                    );
                    self.annotated_variables.remove(rest);
                }
            }
            Pattern::Dict { keys, rest } => {
                let (key_type, value_type) = match inferred_type {
                    Some(TypeAnnotation::Dict { key, value }) => (Some(*key), Some(*value)),
                    None | Some(TypeAnnotation::Any) => {
                        (Some(TypeAnnotation::Any), Some(TypeAnnotation::Any))
                    }
                    Some(other) => {
                        self.errors.push(
                            KujoError::new(
                                ErrorKind::TypeError,
                                format!(
                                    "Dictionary destructuring expects a Dict value, inferred {:?}",
                                    other
                                ),
                                SourceLocation::unknown(),
                            )
                            .with_help(
                                "Use a dictionary value or bind the value without destructuring"
                                    .to_string(),
                            ),
                        );
                        (Some(TypeAnnotation::Any), Some(TypeAnnotation::Any))
                    }
                };
                for key in keys {
                    self.variables.insert(key.clone(), value_type.clone());
                    self.annotated_variables.remove(key);
                }
                if let Some(rest) = rest {
                    self.variables.insert(
                        rest.clone(),
                        Some(TypeAnnotation::Dict {
                            key: Box::new(key_type.unwrap_or(TypeAnnotation::Any)),
                            value: Box::new(value_type.unwrap_or(TypeAnnotation::Any)),
                        }),
                    );
                    self.annotated_variables.remove(rest);
                }
            }
        }
    }

    /// Push a new scope onto the scope stack
    fn push_scope(&mut self) {
        self.scope_stack.push(self.variables.clone());
        self.annotation_scopes.push(self.annotated_variables.clone());
    }

    /// Pop a scope from the scope stack
    fn pop_scope(&mut self) {
        if let Some(prev_scope) = self.scope_stack.pop() {
            self.variables = prev_scope;
            self.annotated_variables =
                self.annotation_scopes.pop().expect("balanced checker scopes");
        }
    }

    fn merge_inferred_types(
        current: Option<TypeAnnotation>,
        next: Option<TypeAnnotation>,
    ) -> Option<TypeAnnotation> {
        // `current == None` is an empty accumulator; an unknown next element
        // is a real contribution and must widen the collection to Any.
        let next = next.unwrap_or(TypeAnnotation::Any);
        Some(match current {
            None => next,
            Some(existing) if existing == next => existing,
            Some(TypeAnnotation::Int) if next == TypeAnnotation::Float => TypeAnnotation::Float,
            Some(TypeAnnotation::Float) if next == TypeAnnotation::Int => TypeAnnotation::Float,
            // Assignment compatibility is not a type join: Any matches String,
            // but combining them must not infer String (also for nested types).
            Some(_) => TypeAnnotation::Any,
        })
    }

    fn infer_builtin_contains(&mut self, args: &[Expr]) -> Option<TypeAnnotation> {
        let types: Vec<_> = args.iter().map(|arg| self.infer_expr(arg)).collect();
        if args.len() != 2 {
            self.errors.push(KujoError::new(
                ErrorKind::TypeError,
                format!("Function 'contains' expects 2 arguments but got {}", args.len()),
                SourceLocation::unknown(),
            ));
            return Some(TypeAnnotation::Any);
        }
        let (result, string_needle) = match types[0].as_ref() {
            Some(TypeAnnotation::String) => (TypeAnnotation::Int, true),
            Some(TypeAnnotation::Array(_)) => (TypeAnnotation::Bool, false),
            Some(TypeAnnotation::Dict { .. }) => (TypeAnnotation::Bool, true),
            None | Some(TypeAnnotation::Any) => return Some(TypeAnnotation::Any),
            _ => {
                self.errors.push(KujoError::new(
                    ErrorKind::TypeError,
                    "Function 'contains' first argument must be String, Array or Dict".to_string(),
                    SourceLocation::unknown(),
                ));
                return Some(TypeAnnotation::Any);
            }
        };
        if string_needle
            && !matches!(types[1], None | Some(TypeAnnotation::Any | TypeAnnotation::String))
        {
            self.errors.push(KujoError::new(
                ErrorKind::TypeError,
                "Function 'contains' requires a String needle for String or Dict input".to_string(),
                SourceLocation::unknown(),
            ));
        }
        Some(result)
    }

    fn infer_known_method_return_type(
        object_type: Option<&TypeAnnotation>,
        method: &str,
    ) -> Option<TypeAnnotation> {
        match object_type {
            Some(TypeAnnotation::String) => match method {
                "len" | "count_chars" | "index_of" => Some(TypeAnnotation::Int),
                "to_upper" | "upper" | "to_lower" | "lower" | "capitalize" | "trim"
                | "trim_start" | "trim_end" | "char_at" | "substring" | "replace"
                | "replace_str" => Some(TypeAnnotation::String),
                "starts_with" | "ends_with" | "contains" | "is_empty" => Some(TypeAnnotation::Bool),
                "split" => Some(TypeAnnotation::Array(Box::new(TypeAnnotation::String))),
                _ => None,
            },
            Some(TypeAnnotation::Array(_)) => match method {
                "len" => Some(TypeAnnotation::Int),
                "is_empty" | "contains" => Some(TypeAnnotation::Bool),
                _ => None,
            },
            Some(TypeAnnotation::Dict { .. }) => match method {
                "len" => Some(TypeAnnotation::Int),
                "contains" | "has_key" => Some(TypeAnnotation::Bool),
                _ => None,
            },
            _ => None,
        }
    }

    /// Get all available variable names in current scope
    /// TODO: This will be used when adding "Did you mean?" suggestions to interpreter
    /// runtime errors (currently only used in type checker for undefined function errors)
    #[allow(dead_code)]
    fn get_available_variables(&self) -> Vec<String> {
        self.variables.keys().cloned().collect()
    }

    /// Get all available function names
    fn get_available_functions(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_function_calls_do_not_consume_recursion_budget() {
        for expression in ["contains(\"evidence\", \"e\")", "len(\"evidence\")"] {
            let source = format!("{expression}\n").repeat(MAX_RECURSION_DEPTH + 1);
            let mut parser = Parser::new(crate::lexer::tokenize(&source).unwrap());
            let mut checker = TypeChecker::new();
            assert!(checker.check(&parser.parse()).is_ok(), "{:?}", checker.errors);
            assert_eq!(checker.recursion_depth, 0);
        }
    }

    #[test]
    fn function_inference_still_enforces_nested_depth_and_unwinds_after_errors() {
        let mut checker = TypeChecker::new();
        checker.recursion_depth = MAX_RECURSION_DEPTH - 1;
        checker.infer_expr(&Expr::Call {
            function: Box::new(Expr::Identifier("len".into())),
            args: vec![Expr::String("evidence".into())],
        });
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0].to_string().contains("recursion depth exceeded"));
        assert_eq!(checker.recursion_depth, MAX_RECURSION_DEPTH - 1);
        checker.recursion_depth = 0;
        assert_eq!(checker.infer_expr(&Expr::Int(1)), Some(TypeAnnotation::Int));
        assert_eq!(checker.recursion_depth, 0);
    }

    #[test]
    fn database_last_insert_id_is_a_known_integer_builtin() {
        let mut checker = TypeChecker::new();
        let tokens = crate::lexer::tokenize(
            "let db := db_connect(\"sqlite\", \":memory:\")\nlet id: int := db_last_insert_id(db)",
        )
        .unwrap();
        let statements = crate::parser::Parser::new(tokens).parse();
        assert!(checker.check(&statements).is_ok());
        assert_eq!(checker.variables.get("id"), Some(&Some(TypeAnnotation::Int)));
    }

    #[test]
    fn contains_inference_matches_each_runtime_receiver() {
        let source = r#"
            let text := contains("hello", "h")
            let array := contains([1, 2], 1)
            let dict := contains({"key": 1}, "key")
        "#;
        let mut parser = Parser::new(crate::lexer::tokenize(source).unwrap());
        let statements = parser.parse();
        let mut checker = TypeChecker::new();
        assert!(checker.check(&statements).is_ok(), "{:?}", checker.errors);
        assert_eq!(checker.variables.get("text"), Some(&Some(TypeAnnotation::Int)));
        for name in ["array", "dict"] {
            assert_eq!(checker.variables.get(name), Some(&Some(TypeAnnotation::Bool)));
        }
    }

    #[test]
    fn contains_inference_rejects_invalid_arity_and_types() {
        for source in ["contains()", "contains(1, 2)", "contains(\"text\", 1)", "contains({}, 1)"] {
            let mut parser = Parser::new(crate::lexer::tokenize(source).unwrap());
            assert!(TypeChecker::new().check(&parser.parse()).is_err(), "{source}");
        }
    }

    #[test]
    fn contains_inference_preserves_user_and_imported_signatures() {
        let source =
            "func contains(value, needle) -> bool { return true }\nlet result := contains(1, 2)";
        let mut parser = Parser::new(crate::lexer::tokenize(source).unwrap());
        let mut checker = TypeChecker::new();
        assert!(checker.check(&parser.parse()).is_ok(), "{:?}", checker.errors);
        assert_eq!(checker.variables.get("result"), Some(&Some(TypeAnnotation::Bool)));
        let exported_source = format!("export {source}");
        let mut parser = Parser::new(crate::lexer::tokenize(&exported_source).unwrap());
        let mut exported = TypeChecker::new();
        assert!(exported.check(&parser.parse()).is_ok(), "{:?}", exported.errors);
        assert_eq!(exported.variables.get("result"), Some(&Some(TypeAnnotation::Bool)));
        let mut parser = Parser::new(
            crate::lexer::tokenize("func invoke(contains) { contains(1, 2) }").unwrap(),
        );
        assert!(TypeChecker::new().check(&parser.parse()).is_ok());
        let mut imported = TypeChecker::new();
        imported.register_imported_symbol(
            "contains",
            Some(FunctionSignature {
                param_types: vec![],
                return_type: Some(TypeAnnotation::String),
            }),
            true,
        );
        assert_eq!(
            imported.infer_expr(&Expr::Call {
                function: Box::new(Expr::Identifier("contains".to_string())),
                args: vec![],
            }),
            Some(TypeAnnotation::String)
        );
    }

    #[test]
    fn release_filesystem_builtins_and_gradual_any_are_warning_free() {
        let source = r#"
            path := join_path("root", "nested", "artifact.bin")
            digest := sha256_file(path)
            symlink := path_is_symlink(path)
            unknown := {}["value"]
            label := "artifact-" + unknown
        "#;
        let tokens = crate::lexer::tokenize(source).expect("source should tokenize");
        let mut parser = crate::parser::Parser::new(tokens);
        let statements = parser.parse();
        let mut checker = TypeChecker::new();
        assert!(checker.check(&statements).is_ok(), "unexpected errors: {:?}", checker.errors);
    }

    #[test]
    fn runtime_test_builtins_are_warning_free() {
        let source = r#"
            assert_equal("fixture", "fixture")
            assert_true(true)
            assert_false(false)
            assert_contains(["fixture"], "fixture")
            elapsed_ms := time()
            print(elapsed_ms)
        "#;
        let tokens = crate::lexer::tokenize(source).expect("source should tokenize");
        let mut parser = crate::parser::Parser::new(tokens);
        let statements = parser.parse();
        let mut checker = TypeChecker::new();
        assert!(checker.check(&statements).is_ok(), "unexpected errors: {:?}", checker.errors);
    }

    #[test]
    fn test_simple_type_inference() {
        let mut checker = TypeChecker::new();
        let stmts = vec![Stmt::Let {
            pattern: crate::ast::Pattern::Identifier("x".to_string()),
            value: Expr::Int(42),
            mutable: false,
            type_annotation: Some(TypeAnnotation::Int),
        }];

        assert!(checker.check(&stmts).is_ok());
        assert_eq!(checker.variables.get("x"), Some(&Some(TypeAnnotation::Int)));
    }

    #[test]
    fn test_type_mismatch() {
        let mut checker = TypeChecker::new();
        let stmts = vec![Stmt::Let {
            pattern: crate::ast::Pattern::Identifier("x".to_string()),
            value: Expr::String("hello".to_string()),
            mutable: false,
            type_annotation: Some(TypeAnnotation::Int),
        }];

        assert!(checker.check(&stmts).is_err());
    }

    #[test]
    fn test_int_literal_inference() {
        let mut checker = TypeChecker::new();
        let stmts = vec![Stmt::Let {
            pattern: crate::ast::Pattern::Identifier("count".to_string()),
            value: Expr::Int(42),
            mutable: false,
            type_annotation: None,
        }];

        assert!(checker.check(&stmts).is_ok());
        // Variable should be inferred as Int
        assert_eq!(checker.variables.get("count"), Some(&Some(TypeAnnotation::Int)));
    }

    #[test]
    fn test_float_literal_inference() {
        let mut checker = TypeChecker::new();
        let stmts = vec![Stmt::Let {
            pattern: crate::ast::Pattern::Identifier("pi".to_string()),
            value: Expr::Float(std::f64::consts::PI),
            mutable: false,
            type_annotation: None,
        }];

        assert!(checker.check(&stmts).is_ok());
        // Variable should be inferred as Float
        assert_eq!(checker.variables.get("pi"), Some(&Some(TypeAnnotation::Float)));
    }

    #[test]
    fn test_int_plus_int_equals_int() {
        let mut checker = TypeChecker::new();
        let result = checker.infer_expr(&Expr::BinaryOp {
            op: "+".to_string(),
            left: Box::new(Expr::Int(5)),
            right: Box::new(Expr::Int(10)),
        });

        assert_eq!(result, Some(TypeAnnotation::Int));
    }

    #[test]
    fn test_int_plus_float_equals_float() {
        let mut checker = TypeChecker::new();
        let result = checker.infer_expr(&Expr::BinaryOp {
            op: "+".to_string(),
            left: Box::new(Expr::Int(5)),
            right: Box::new(Expr::Float(10.5)),
        });

        assert_eq!(result, Some(TypeAnnotation::Float));
    }

    #[test]
    fn test_float_plus_int_equals_float() {
        let mut checker = TypeChecker::new();
        let result = checker.infer_expr(&Expr::BinaryOp {
            op: "+".to_string(),
            left: Box::new(Expr::Float(5.5)),
            right: Box::new(Expr::Int(10)),
        });

        assert_eq!(result, Some(TypeAnnotation::Float));
    }

    #[test]
    fn test_float_plus_float_equals_float() {
        let mut checker = TypeChecker::new();
        let result = checker.infer_expr(&Expr::BinaryOp {
            op: "+".to_string(),
            left: Box::new(Expr::Float(5.5)),
            right: Box::new(Expr::Float(10.5)),
        });

        assert_eq!(result, Some(TypeAnnotation::Float));
    }

    #[test]
    fn test_int_to_float_promotion_in_assignment() {
        let mut checker = TypeChecker::new();
        let stmts = vec![Stmt::Let {
            pattern: crate::ast::Pattern::Identifier("x".to_string()),
            value: Expr::Int(42),
            mutable: false,
            type_annotation: Some(TypeAnnotation::Float),
        }];

        // Should succeed due to Int → Float promotion
        assert!(checker.check(&stmts).is_ok());
    }

    #[test]
    fn test_float_to_int_no_promotion() {
        let mut checker = TypeChecker::new();
        let stmts = vec![Stmt::Let {
            pattern: crate::ast::Pattern::Identifier("x".to_string()),
            value: Expr::Float(42.5),
            mutable: false,
            type_annotation: Some(TypeAnnotation::Int),
        }];

        // Should fail - Float cannot be assigned to Int without explicit conversion
        assert!(checker.check(&stmts).is_err());
    }

    #[test]
    fn test_math_function_accepts_int_via_promotion() {
        let mut checker = TypeChecker::new();

        // abs(5) should be accepted (Int promoted to Float)
        let result = checker.infer_expr(&Expr::Call {
            function: Box::new(Expr::Identifier("abs".to_string())),
            args: vec![Expr::Int(5)],
        });

        // Function should accept Int via promotion and return Float
        assert!(checker.errors.is_empty());
        assert_eq!(result, Some(TypeAnnotation::Float));
    }

    #[test]
    fn test_min_with_ints() {
        let mut checker = TypeChecker::new();

        // min(5, 10) should be accepted
        let result = checker.infer_expr(&Expr::Call {
            function: Box::new(Expr::Identifier("min".to_string())),
            args: vec![Expr::Int(5), Expr::Int(10)],
        });

        // Should accept via promotion
        assert!(checker.errors.is_empty());
        assert_eq!(result, Some(TypeAnnotation::Float));
    }

    #[test]
    fn test_min_with_mixed_types() {
        let mut checker = TypeChecker::new();

        // min(5, 10.5) should be accepted
        let result = checker.infer_expr(&Expr::Call {
            function: Box::new(Expr::Identifier("min".to_string())),
            args: vec![Expr::Int(5), Expr::Float(10.5)],
        });

        // Should accept via promotion
        assert!(checker.errors.is_empty());
        assert_eq!(result, Some(TypeAnnotation::Float));
    }

    #[test]
    fn test_arithmetic_type_promotion_all_operators() {
        let mut checker = TypeChecker::new();

        for op in &["+", "-", "*", "/"] {
            let result = checker.infer_expr(&Expr::BinaryOp {
                op: op.to_string(),
                left: Box::new(Expr::Int(10)),
                right: Box::new(Expr::Float(5.0)),
            });

            assert_eq!(
                result,
                Some(TypeAnnotation::Float),
                "Operator {} should promote Int+Float to Float",
                op
            );
        }
    }

    #[test]
    fn test_array_literal_infers_element_type() {
        let mut checker = TypeChecker::new();
        let result = checker.infer_expr(&Expr::ArrayLiteral(vec![
            crate::ast::ArrayElement::Single(Expr::Int(1)),
            crate::ast::ArrayElement::Single(Expr::Int(2)),
        ]));

        assert_eq!(result, Some(TypeAnnotation::Array(Box::new(TypeAnnotation::Int))));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_array_literal_promotes_mixed_numeric_elements_to_float() {
        let mut checker = TypeChecker::new();
        let result = checker.infer_expr(&Expr::ArrayLiteral(vec![
            crate::ast::ArrayElement::Single(Expr::Int(1)),
            crate::ast::ArrayElement::Single(Expr::Float(2.5)),
        ]));

        assert_eq!(result, Some(TypeAnnotation::Array(Box::new(TypeAnnotation::Float))));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_dict_literal_infers_key_and_value_types() {
        let mut checker = TypeChecker::new();
        let result = checker.infer_expr(&Expr::DictLiteral(vec![
            crate::ast::DictElement::Pair(Expr::String("a".to_string()), Expr::Int(1)),
            crate::ast::DictElement::Pair(Expr::String("b".to_string()), Expr::Int(2)),
        ]));

        assert_eq!(
            result,
            Some(TypeAnnotation::Dict {
                key: Box::new(TypeAnnotation::String),
                value: Box::new(TypeAnnotation::Int),
            })
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_index_access_returns_inferred_container_element_type() {
        let mut checker = TypeChecker::new();

        let array_index_result = checker.infer_expr(&Expr::IndexAccess {
            object: Box::new(Expr::ArrayLiteral(vec![
                crate::ast::ArrayElement::Single(Expr::Int(1)),
                crate::ast::ArrayElement::Single(Expr::Int(2)),
            ])),
            index: Box::new(Expr::Int(0)),
        });
        assert_eq!(array_index_result, Some(TypeAnnotation::Int));

        let dict_index_result = checker.infer_expr(&Expr::IndexAccess {
            object: Box::new(Expr::DictLiteral(vec![crate::ast::DictElement::Pair(
                Expr::String("a".to_string()),
                Expr::Bool(true),
            )])),
            index: Box::new(Expr::String("a".to_string())),
        });
        assert_eq!(dict_index_result, Some(TypeAnnotation::Bool));
    }

    #[test]
    fn test_method_call_infers_known_string_method_return_types() {
        let mut checker = TypeChecker::new();

        let len_type = checker.infer_expr(&Expr::MethodCall {
            object: Box::new(Expr::String("hello".to_string())),
            method: "len".to_string(),
            args: vec![],
        });
        assert_eq!(len_type, Some(TypeAnnotation::Int));

        let upper_type = checker.infer_expr(&Expr::MethodCall {
            object: Box::new(Expr::String("hello".to_string())),
            method: "to_upper".to_string(),
            args: vec![],
        });
        assert_eq!(upper_type, Some(TypeAnnotation::String));

        let contains_type = checker.infer_expr(&Expr::MethodCall {
            object: Box::new(Expr::String("hello".to_string())),
            method: "contains".to_string(),
            args: vec![Expr::String("h".to_string())],
        });
        assert_eq!(contains_type, Some(TypeAnnotation::Bool));
    }

    #[test]
    fn test_method_call_unknown_method_falls_back_to_any() {
        let mut checker = TypeChecker::new();
        let inferred = checker.infer_expr(&Expr::MethodCall {
            object: Box::new(Expr::String("hello".to_string())),
            method: "unknown_method".to_string(),
            args: vec![],
        });

        assert_eq!(inferred, Some(TypeAnnotation::Any));
    }

    #[test]
    fn test_selective_import_registers_callable_binding() {
        let mut checker = TypeChecker::new();
        let stmts = vec![
            Stmt::Import {
                module: "math_helper".to_string(),
                symbols: Some(vec!["add_one".to_string()]),
            },
            Stmt::Let {
                pattern: crate::ast::Pattern::Identifier("result".to_string()),
                value: Expr::Call {
                    function: Box::new(Expr::Identifier("add_one".to_string())),
                    args: vec![Expr::Int(41)],
                },
                mutable: false,
                type_annotation: None,
            },
        ];

        assert!(checker.check(&stmts).is_ok());
        assert_eq!(checker.variables.get("add_one"), Some(&Some(TypeAnnotation::Any)));
        assert_eq!(checker.variables.get("result"), Some(&Some(TypeAnnotation::Any)));
    }

    #[test]
    fn test_selective_import_resolves_function_signature_from_module_file() {
        fn unique_name(prefix: &str) -> String {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos();
            format!("{}_{}_{}", prefix, std::process::id(), nanos)
        }

        let mut checker = TypeChecker::new();
        let temp_root = std::env::temp_dir().join(unique_name("kujo_type_checker_import"));
        let src_dir = temp_root.join("src");
        std::fs::create_dir_all(&src_dir).expect("failed to create temp module dir");

        let module_name = unique_name("math_helper");
        let module_path = src_dir.join(format!("{}.kujo", module_name));
        std::fs::write(
            &module_path,
            "export func add_one(value: int) -> int {\n    return value + 1\n}\n",
        )
        .expect("failed to write module source");

        checker.add_search_path(&temp_root);

        let stmts = vec![
            Stmt::Import {
                module: format!("src.{}", module_name),
                symbols: Some(vec!["add_one".to_string()]),
            },
            Stmt::Let {
                pattern: crate::ast::Pattern::Identifier("result".to_string()),
                value: Expr::Call {
                    function: Box::new(Expr::Identifier("add_one".to_string())),
                    args: vec![Expr::Int(41)],
                },
                mutable: false,
                type_annotation: None,
            },
        ];

        assert!(checker.check(&stmts).is_ok());

        let imported_signature =
            checker.functions.get("add_one").expect("imported function should be registered");
        assert_eq!(imported_signature.param_types, vec![Some(TypeAnnotation::Int)]);
        assert_eq!(imported_signature.return_type, Some(TypeAnnotation::Int));
        assert_eq!(
            checker.infer_expr(&Expr::Identifier("add_one".to_string())),
            Some(TypeAnnotation::Function {
                params: vec![TypeAnnotation::Int],
                return_type: Box::new(TypeAnnotation::Int),
            })
        );
        assert_eq!(checker.variables.get("result"), Some(&Some(TypeAnnotation::Int)));

        std::fs::remove_file(&module_path).expect("failed to remove temp module");
        std::fs::remove_dir_all(&temp_root).expect("failed to clean up temp module dir");
    }

    fn check_source(source: &str) -> TypeChecker {
        let mut parser = Parser::new(crate::lexer::tokenize(source).unwrap());
        let parsed = parser.parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let mut checker = TypeChecker::new();
        let _ = checker.check(&parsed.stmts);
        checker
    }

    #[test]
    fn destructuring_infers_literal_positions_nested_values_and_rest() {
        let checker = check_source(
            r#"
                let [count, [label, enabled], ...remaining] := [1, ["ready", true], 2, 3]
                let {name, active, ...metadata} := {"name": "Kujo", "active": true, "score": 10}
                let exact_count: int := count
                let exact_label: string := label
                let exact_enabled: bool := enabled
                let exact_name: string := name
                let exact_active: bool := active
            "#,
        );
        assert!(checker.errors.is_empty(), "{:?}", checker.errors);
        assert_eq!(checker.variables.get("count"), Some(&Some(TypeAnnotation::Int)));
        assert_eq!(checker.variables.get("label"), Some(&Some(TypeAnnotation::String)));
        assert_eq!(checker.variables.get("enabled"), Some(&Some(TypeAnnotation::Bool)));
        assert!(matches!(checker.variables.get("remaining"), Some(Some(TypeAnnotation::Array(_)))));
        assert!(matches!(
            checker.variables.get("metadata"),
            Some(Some(TypeAnnotation::Dict { .. }))
        ));
    }

    #[test]
    fn destructuring_rejects_known_impossible_shapes_but_keeps_unknown_gradual() {
        let wrong = check_source("let [value] := 1");
        assert!(wrong.errors.iter().any(|error| error.to_string().contains("Array destructuring")));

        let dynamic = check_source(
            "func source(value) { return value }\nlet [value] := source([1])\nprint(value)",
        );
        assert!(dynamic.errors.is_empty(), "{:?}", dynamic.errors);
    }

    #[test]
    fn struct_fields_flow_through_returns_and_report_real_field_errors() {
        let good = check_source(
            r#"
                struct Profile { name: string, score: int }
                func make_profile() { return Profile { name: "Ada", score: 42 } }
                let profile := make_profile()
                let name: string := profile.name
                let score: int := profile.score
                let names := await parallel_map([profile], func(value) { return value.name }, 1)
                let first_name: string := names[0]
            "#,
        );
        assert!(good.errors.is_empty(), "{:?}", good.errors);

        let missing = check_source(
            "struct Profile { name: string }\nlet profile := Profile { name: \"Ada\" }\nprint(profile.missing)",
        );
        assert!(missing.errors.iter().any(|error| error.to_string().contains("no field")));

        let mismatch = check_source(
            "struct Profile { score: int }\nlet profile := Profile { score: \"wrong\" }",
        );
        assert!(mismatch.errors.iter().any(|error| error.to_string().contains("expects Int")));
    }

    #[test]
    fn async_calls_aliases_and_reusable_promises_unwrap_precisely() {
        let checker = check_source(
            r#"
                async func answer() { return 42 }
                let alias := answer
                let promise := alias()
                let first: int := await promise
                let second: int := await promise
                let promises := [answer(), alias()]
                let one: int := await promises[0]
                let closure := async func() { return "ready" }
                let status: string := await closure()
            "#,
        );
        assert!(checker.errors.is_empty(), "{:?}", checker.errors);

        let mismatch =
            check_source("async func answer() { return 42 }\nlet wrong: string := await answer()");
        assert!(mismatch.errors.iter().any(|error| error.to_string().contains("Type mismatch")));
    }

    #[test]
    fn callable_aliases_are_checked_while_dynamic_reassignment_falls_back() {
        let checked = check_source(
            r#"
                let callback := func(value: int) -> int { return value + 1 }
                let alias := callback
                let result: int := alias(41)
            "#,
        );
        assert!(checked.errors.is_empty(), "{:?}", checked.errors);

        let wrong = check_source(
            "let callback := func(value: int) -> int { return value }\ncallback(\"wrong\")",
        );
        assert!(wrong.errors.iter().any(|error| error.to_string().contains("expects Int")));

        let dynamic = check_source(
            "func select(value) { return value }\nmut callback := select(func(value) { return value })\ncallback = select(callback)\ncallback(1)",
        );
        assert!(dynamic.errors.is_empty(), "{:?}", dynamic.errors);
    }

    #[test]
    fn unannotated_callable_parameters_are_required_and_builtin_optionals_stay_optional() {
        let missing = check_source("func pair(left, right) { return left }\npair(1)");
        assert!(
            missing
                .errors
                .iter()
                .any(|error| error.to_string().contains("expects 2-2 arguments but got 1")),
            "{:?}",
            missing.errors
        );

        let builtins = check_source(
            r#"
                async func value() { return 1 }
                let values := await promise_all([value()])
                let mapped := await parallel_map([1], func(item) { return item })
            "#,
        );
        assert!(builtins.errors.is_empty(), "{:?}", builtins.errors);

        for source in ["await_task()", "promise_all()", "parallel_map([1])"] {
            let invalid = check_source(source);
            assert!(!invalid.errors.is_empty(), "{source}");
        }
    }

    #[test]
    fn module_namespaces_exported_values_structs_and_missing_modules_are_analyzed() {
        let root = std::env::temp_dir().join(format!(
            "kujo_inference_module_{}_{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("helper.kujo"),
            r#"
                export const label: string := "ready"
                export func add(value: int) -> int { return value + 1 }
                export struct Profile { name: string }
            "#,
        )
        .unwrap();
        let mut parser = Parser::new(
            crate::lexer::tokenize(
                "import helper\nlet n: int := helper.add(41)\nlet label: string := helper.label\nfrom helper import Profile\nlet p := Profile { name: \"Ada\" }\nlet name: string := p.name",
            )
            .unwrap(),
        );
        let parsed = parser.parse_with_diagnostics();
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let mut checker = TypeChecker::new();
        checker.add_search_path(&root);
        assert!(checker.check(&parsed.stmts).is_ok(), "{:?}", checker.errors);
        assert_eq!(checker.module_parse_count, 1, "module AST should be shared by export analyses");

        let mut missing_member_parser =
            Parser::new(crate::lexer::tokenize("import helper\nprint(helper.missing)").unwrap());
        let mut missing_member = TypeChecker::new();
        missing_member.add_search_path(&root);
        assert!(missing_member.check(&missing_member_parser.parse()).is_err());
        assert!(missing_member
            .errors
            .iter()
            .any(|error| error.to_string().contains("no exported member")));

        let mut missing_parser =
            Parser::new(crate::lexer::tokenize("import definitely_missing_module").unwrap());
        let mut missing = TypeChecker::new();
        missing.add_search_path(&root);
        assert!(missing.check(&missing_parser.parse()).is_err());
        assert!(missing.errors.iter().any(|error| error.to_string().contains("Unknown module")));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn struct_operator_overloads_do_not_emit_primitive_type_mismatches() {
        let checked = check_source(
            r#"
                struct Vector {
                    x: float,
                    func op_mul(scale) { return Vector { x: x * scale } },
                    func op_eq(other) { return x == other.x }
                }
                let vector := Vector { x: 3.0 }
                let scaled := vector * 2.0
                let equal: bool := vector == scaled
            "#,
        );
        assert!(checked.errors.is_empty(), "{:?}", checked.errors);
    }
}
