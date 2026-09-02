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

/// Represents a function signature with parameter and return types
#[derive(Debug, Clone)]
struct FunctionSignature {
    param_types: Vec<Option<TypeAnnotation>>,
    return_type: Option<TypeAnnotation>,
}

/// Type checker maintains symbol tables for variables and functions
pub struct TypeChecker {
    /// Symbol table mapping variable names to their types
    variables: HashMap<String, Option<TypeAnnotation>>,
    /// Function signatures mapping function names to their types
    functions: HashMap<String, FunctionSignature>,
    /// Stack of scopes for nested blocks
    scope_stack: Vec<HashMap<String, Option<TypeAnnotation>>>,
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
}

/// Maximum recursion depth for type checking to prevent infinite loops
const MAX_RECURSION_DEPTH: usize = 1000;

impl TypeChecker {
    /// Creates a new type checker with empty symbol tables
    pub fn new() -> Self {
        let mut checker = TypeChecker {
            variables: HashMap::new(),
            functions: HashMap::new(),
            scope_stack: Vec::new(),
            current_function_return: None,
            errors: Vec::new(),
            recursion_depth: 0,
            module_search_paths: {
                let mut paths = vec![PathBuf::from("."), PathBuf::from("./modules")];
                paths.extend(crate::module::configured_module_search_paths());
                paths.extend(crate::module::automatic_kennel_package_search_paths(
                    &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                ));
                paths
            },
            module_export_signatures: HashMap::new(),
        };

        // Register built-in functions
        checker.register_builtins();

        checker
    }

    /// Adds a module search path used to resolve imported Kujo files during static analysis.
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();
        self.module_search_paths.push(path.clone());
        self.module_search_paths
            .extend(crate::module::automatic_kennel_package_search_paths(&path));
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
                param_types: vec![Some(TypeAnnotation::String)],
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
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::Bool),
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
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
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

        // JSON functions
        self.functions.insert(
            "parse_json".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: None, // Returns any type (dict, array, etc.)
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
            "parse_date".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String), Some(TypeAnnotation::String)],
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

        // File I/O functions
        self.functions.insert(
            "read_file".to_string(),
            FunctionSignature {
                param_types: vec![Some(TypeAnnotation::String)],
                return_type: Some(TypeAnnotation::String),
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
            "db_begin".to_string(),
            FunctionSignature {
                param_types: vec![None], // Database connection
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
            "encode_uri_component".to_string(),
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
                    Some(TypeAnnotation::Int),    // count
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
                    Some(TypeAnnotation::Int),    // count
                ],
                return_type: Some(TypeAnnotation::Bool),
            },
        );

        // Concurrency functions
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
                param_types: vec![None, None, Some(TypeAnnotation::Int)],
                return_type: None,
            },
        );

        self.functions.insert(
            "par_map".to_string(),
            FunctionSignature {
                param_types: vec![None, None, Some(TypeAnnotation::Int)], // array, mapper, optional limit
                return_type: None,
            },
        );

        self.functions.insert(
            "par_each".to_string(),
            FunctionSignature {
                param_types: vec![None, None, Some(TypeAnnotation::Int)], // array, mapper, optional limit
                return_type: None,
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
    ) -> FunctionSignature {
        FunctionSignature {
            param_types: param_types
                .iter()
                .cloned()
                .chain(std::iter::repeat(None))
                .take(params.len())
                .collect(),
            return_type: return_type.clone(),
        }
    }

    fn signature_from_imported_value(
        value: &Expr,
        known_functions: &HashMap<String, FunctionSignature>,
    ) -> Option<FunctionSignature> {
        match value {
            Expr::Identifier(name) => known_functions.get(name).cloned(),
            Expr::Function { params, param_types, return_type, .. } => {
                Some(Self::function_signature_from_params(params, param_types, return_type))
            }
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
            Stmt::FuncDef { name, params, param_types, return_type, .. } => {
                binding_signatures.insert(
                    name.clone(),
                    Self::function_signature_from_params(params, param_types, return_type),
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

        let source = fs::read_to_string(&module_path).ok()?;
        let tokens = tokenize_with_file(&source, Some(&module_path.to_string_lossy())).ok()?;
        let mut parser = Parser::new(tokens);
        let parse_output = parser.parse_with_diagnostics();
        if !parse_output.diagnostics.is_empty() {
            return None;
        }

        active_modules.push(module_path.clone());

        let mut binding_signatures = HashMap::new();
        let mut export_signatures = HashMap::new();
        for stmt in &parse_output.stmts {
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

    fn collect_export_signatures_from_stmt(
        &mut self,
        stmt: &Stmt,
        binding_signatures: &HashMap<String, FunctionSignature>,
        export_signatures: &mut HashMap<String, FunctionSignature>,
        active_modules: &mut Vec<PathBuf>,
    ) {
        match stmt {
            Stmt::FuncDef { name, params, param_types, return_type, .. } => {
                export_signatures.insert(
                    name.clone(),
                    Self::function_signature_from_params(params, param_types, return_type),
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
            if let Stmt::FuncDef { name, param_types, return_type, .. } = stmt {
                self.functions.insert(
                    name.clone(),
                    FunctionSignature {
                        param_types: param_types.clone(),
                        return_type: return_type.clone(),
                    },
                );
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
                        self.variables.insert(name.clone(), inferred_type);
                    }
                }
                // For destructuring patterns, we skip type checking for now
                // TODO: Implement proper type checking for destructuring patterns
            }

            Stmt::Const { name, value, type_annotation } => {
                let inferred_type = self.infer_expr(value);

                // If type annotation is provided, check compatibility
                if let Some(annotated_type) = type_annotation {
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
                    self.variables.insert(name.clone(), inferred_type);
                }
            }

            Stmt::FuncDef {
                name: _,
                params,
                param_types,
                return_type,
                body,
                is_generator: _,
                is_async: _,
            } => {
                // Enter function scope
                let saved_return_type = self.current_function_return.clone();
                self.current_function_return = return_type.clone();
                self.push_scope();

                // Add parameters to scope
                for (i, param) in params.iter().enumerate() {
                    let param_type = param_types.get(i).and_then(|t| t.clone());
                    self.variables.insert(param.clone(), param_type);
                }

                // Check function body
                for stmt in body {
                    self.check_stmt(stmt);
                }

                // Exit function scope
                self.pop_scope();
                self.current_function_return = saved_return_type;
            }

            Stmt::Return(expr) => {
                let return_type = expr.as_ref().and_then(|e| self.infer_expr(e));

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
                self.infer_expr(iterable);
                self.push_scope();
                self.variables.insert(var.clone(), None); // Iterator type unknown
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
                for (_, case_body) in cases {
                    for s in case_body {
                        self.check_stmt(s);
                    }
                }
                if let Some(default_body) = default {
                    for s in default_body {
                        self.check_stmt(s);
                    }
                }
            }

            Stmt::TryExcept { try_block, except_var: _, except_block } => {
                for s in try_block {
                    self.check_stmt(s);
                }
                for s in except_block {
                    self.check_stmt(s);
                }
            }

            Stmt::ExprStmt(expr) => {
                self.infer_expr(expr);
            }

            Stmt::Assign { target, value } => {
                let inferred_type = self.infer_expr(value);

                // Check based on assignment target
                match target {
                    Expr::Identifier(name) => {
                        // Check if variable exists and types are compatible
                        if let Some(Some(expected)) = self.variables.get(name) {
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

                if let Some(symbols) = symbols {
                    for symbol in symbols {
                        let signature = module_signatures
                            .as_ref()
                            .and_then(|signatures| signatures.get(symbol))
                            .cloned();
                        self.register_imported_symbol(
                            symbol,
                            signature,
                            module_signatures.is_none(),
                        );
                    }
                } else {
                    self.variables.insert(module.clone(), Some(TypeAnnotation::Any));
                }
            }

            Stmt::Export { stmt } => {
                // Type check the exported statement
                self.check_stmt(stmt);
            }

            Stmt::StructDef { name: _, fields: _, methods } => {
                // Type check methods
                for method in methods {
                    self.check_stmt(method);
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

        let result = match expr {
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
                self.variables.get(name).cloned().flatten().or_else(|| {
                    self.functions.get(name).map(Self::function_signature_to_type_annotation)
                })
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
                    // Clone the signature to avoid borrow conflicts
                    let sig = self.functions.get(func_name).cloned();

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
                }
                None
            }

            Expr::Tag(_, _) => None, // Enum types not yet supported

            Expr::StructInstance { name: _, fields } => {
                // Type check struct field initializers
                for (_field_name, field_expr) in fields {
                    self.infer_expr(field_expr);
                }
                None // TODO: Return struct type when struct types are implemented
            }

            Expr::FieldAccess { object, field: _ } => {
                // Type check the object expression
                self.infer_expr(object);
                None // TODO: Look up field type from struct definition
            }

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
                params: _,
                param_types,
                return_type,
                body,
                is_generator: _,
                is_async: _,
            } => {
                // Type check function expression (anonymous function)
                // Enter function scope
                self.push_scope();

                // Add parameters to scope
                for t in param_types.iter().flatten() {
                    // We would need the param name here, but it's not available in this context
                    // For now, just validate the function body
                    let _ = t;
                }

                // Check function body
                for stmt in body {
                    self.check_stmt(stmt);
                }

                // Exit function scope
                self.pop_scope();

                // Return function type annotation if available
                // For now, just return None since we don't have full function types yet
                let _ = return_type;
                None
            }

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
                for arg in args {
                    self.infer_expr(arg);
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
                // Type check the promise expression
                self.infer_expr(promise_expr);
                // TODO: If we know it's a Promise<T>, return T
                // For now, return Any
                Some(TypeAnnotation::Any)
            }
        };

        self.recursion_depth -= 1;
        result
    }

    /// Push a new scope onto the scope stack
    fn push_scope(&mut self) {
        self.scope_stack.push(self.variables.clone());
    }

    /// Pop a scope from the scope stack
    fn pop_scope(&mut self) {
        if let Some(prev_scope) = self.scope_stack.pop() {
            self.variables = prev_scope;
        }
    }

    fn merge_inferred_types(
        current: Option<TypeAnnotation>,
        next: Option<TypeAnnotation>,
    ) -> Option<TypeAnnotation> {
        match (current, next) {
            (None, None) => None,
            (Some(existing), None) => Some(existing),
            (None, Some(next)) => Some(next),
            (Some(existing), Some(next)) => {
                if existing.matches(&next) || next.matches(&existing) {
                    if matches!(existing, TypeAnnotation::Float)
                        || matches!(next, TypeAnnotation::Float)
                    {
                        Some(TypeAnnotation::Float)
                    } else {
                        Some(existing)
                    }
                } else {
                    Some(TypeAnnotation::Any)
                }
            }
        }
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
}
