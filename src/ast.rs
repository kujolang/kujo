// File: src/ast.rs
//
// Abstract Syntax Tree (AST) definitions for the Kujo programming language.
// Defines the structure of parsed Kujo programs.
//
// The AST represents the syntactic structure of Kujo code after parsing.
// Expressions (Expr) represent values and computations, while Statements (Stmt)
// represent actions and control flow.

use crate::errors::{SourceLocation, SourceSpan};

/// Shared AST span type used across parser, runtime diagnostics, and LSP diagnostics.
pub type AstSpan = SourceSpan;

/// Special method names for operator overloading
/// These methods can be defined on structs to customize operator behavior
pub mod operator_methods {
    // Arithmetic operators
    pub const ADD: &str = "op_add";
    pub const SUB: &str = "op_sub";
    pub const MUL: &str = "op_mul";
    pub const DIV: &str = "op_div";
    pub const MOD: &str = "op_mod";

    // Comparison operators
    pub const EQ: &str = "op_eq";
    pub const NE: &str = "op_ne";
    pub const LT: &str = "op_lt";
    pub const GT: &str = "op_gt";
    pub const LE: &str = "op_le";
    pub const GE: &str = "op_ge";

    // Unary operators
    pub const NEG: &str = "op_neg";
    pub const NOT: &str = "op_not";

    /// Maps binary operators to their corresponding method names
    pub fn binary_op_method(op: &str) -> Option<&'static str> {
        match op {
            "+" => Some(ADD),
            "-" => Some(SUB),
            "*" => Some(MUL),
            "/" => Some(DIV),
            "%" => Some(MOD),
            "==" => Some(EQ),
            "!=" => Some(NE),
            "<" => Some(LT),
            ">" => Some(GT),
            "<=" => Some(LE),
            ">=" => Some(GE),
            _ => None,
        }
    }

    /// Maps unary operators to their corresponding method names
    pub fn unary_op_method(op: &str) -> Option<&'static str> {
        match op {
            "-" => Some(NEG),
            "!" => Some(NOT),
            _ => None,
        }
    }
}

/// Represents parts of an interpolated string in the AST
#[derive(Debug, Clone)]
pub enum InterpolatedStringPart {
    Text(String),    // Plain text
    Expr(Box<Expr>), // Expression to evaluate
}

/// Destructuring patterns for variable binding
/// Supports array and dict destructuring with rest elements
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Simple identifier: x
    Identifier(String),
    /// Array destructuring: [a, b, ...rest]
    Array {
        elements: Vec<Pattern>,
        rest: Option<String>, // Optional rest element
    },
    /// Dict destructuring: {name, email, ...rest}
    Dict {
        keys: Vec<String>,
        rest: Option<String>, // Optional rest element
    },
    /// Ignore placeholder: _
    Ignore,
}

/// Type annotations for variables and functions
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum TypeAnnotation {
    Int,
    Float,
    String,
    Bool,
    Array(Box<TypeAnnotation>),
    Dict { key: Box<TypeAnnotation>, value: Box<TypeAnnotation> },
    Function { params: Vec<TypeAnnotation>, return_type: Box<TypeAnnotation> },
    Enum(String),
    Union(Vec<TypeAnnotation>),
    Any, // For gradual typing - no type checking
    Result { ok_type: Box<TypeAnnotation>, err_type: Box<TypeAnnotation> }, // Result<T, E>
    Option { inner_type: Box<TypeAnnotation> }, // Option<T>
}

impl TypeAnnotation {
    /// Returns true if this type matches another type (allowing Any to match anything)
    /// Also allows Int to match Float (type promotion)
    pub fn matches(&self, other: &TypeAnnotation) -> bool {
        match (self, other) {
            (TypeAnnotation::Any, _) | (_, TypeAnnotation::Any) => true,
            (TypeAnnotation::Int, TypeAnnotation::Int) => true,
            (TypeAnnotation::Float, TypeAnnotation::Float) => true,
            // Allow Int to be promoted to Float
            (TypeAnnotation::Float, TypeAnnotation::Int) => true,
            (TypeAnnotation::String, TypeAnnotation::String) => true,
            (TypeAnnotation::Bool, TypeAnnotation::Bool) => true,
            (TypeAnnotation::Array(left), TypeAnnotation::Array(right)) => left.matches(right),
            (
                TypeAnnotation::Dict { key: left_key, value: left_value },
                TypeAnnotation::Dict { key: right_key, value: right_value },
            ) => left_key.matches(right_key) && left_value.matches(right_value),
            (TypeAnnotation::Enum(a), TypeAnnotation::Enum(b)) => a == b,
            (TypeAnnotation::Union(types), other) | (other, TypeAnnotation::Union(types)) => {
                types.iter().any(|t| t.matches(other))
            }
            (
                TypeAnnotation::Result { ok_type: ok1, err_type: err1 },
                TypeAnnotation::Result { ok_type: ok2, err_type: err2 },
            ) => ok1.matches(ok2) && err1.matches(err2),
            (
                TypeAnnotation::Option { inner_type: t1 },
                TypeAnnotation::Option { inner_type: t2 },
            ) => t1.matches(t2),
            _ => false,
        }
    }
}

/// Represents an expression in Kujo - something that evaluates to a value
#[derive(Debug, Clone)]
pub enum Expr {
    Identifier(String),
    Int(i64),   // Integer literal like 42
    Float(f64), // Float literal like 3.14
    String(String),
    InterpolatedString(Vec<InterpolatedStringPart>), // String with expressions
    Bool(bool),
    Function {
        params: Vec<String>,
        param_types: Vec<Option<TypeAnnotation>>,
        return_type: Option<TypeAnnotation>,
        body: Vec<Stmt>,
        is_generator: bool, // true if func* syntax
        is_async: bool,     // true if async func syntax
    },
    UnaryOp {
        op: String,
        operand: Box<Expr>,
    },
    #[allow(dead_code)]
    BinaryOp {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    #[allow(dead_code)]
    Call {
        function: Box<Expr>,
        args: Vec<Expr>,
    },
    Tag(String, Vec<Expr>), // for enum variant constructors like Result::Ok(...)
    StructInstance {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
    },
    /// Array literal with possible spread elements: [1, 2, ...arr, 3]
    ArrayLiteral(Vec<ArrayElement>),
    /// Dict literal with possible spread elements: {a: 1, ...dict, b: 2}
    DictLiteral(Vec<DictElement>),
    IndexAccess {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    /// Spread expression: ...expr
    /// NOTE: This variant exists in the AST for completeness but is NEVER constructed
    /// as a standalone expression. Spread is only valid within ArrayElement::Spread
    /// and DictElement::Spread contexts. The warning is suppressed because this design
    /// is intentional - spread semantics depend on container context.
    #[allow(dead_code)]
    Spread(Box<Expr>),
    /// Result type constructors
    Ok(Box<Expr>), // Ok(value)
    Err(Box<Expr>), // Err(error)
    /// Option type constructors
    Some(Box<Expr>), // Some(value)
    None,           // None
    /// Try operator for error propagation: expr?
    Try(Box<Expr>),
    /// Yield expression: yield value
    /// Used in generator functions to yield values
    Yield(Option<Box<Expr>>),
    /// Await expression: await promise
    /// Used in async functions to wait for promises
    Await(Box<Expr>),
    /// Method call on expression: expr.method(args)
    /// Used for iterator chaining: range(10).filter(...).map(...)
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
}

/// Array element can be a regular expression or a spread
#[derive(Debug, Clone)]
pub enum ArrayElement {
    Single(Expr),
    Spread(Expr),
}

/// Dict element can be a key-value pair or a spread
#[derive(Debug, Clone)]
pub enum DictElement {
    Pair(Expr, Expr),
    Spread(Expr),
}

impl Expr {
    /// Returns a source span for this expression if available.
    /// The parser still produces unspanned expression nodes in some paths, so unknown is returned
    /// until spans are carried by every AST node.
    #[allow(dead_code)]
    pub fn span(&self) -> AstSpan {
        AstSpan::unknown()
    }

    /// Returns a source location for this expression if available.
    /// Currently returns unknown location - will be enhanced when parser tracks locations.
    #[allow(dead_code)]
    pub fn location(&self) -> SourceLocation {
        self.span().start
    }
}

/// Represents a statement in Kujo - an action or declaration
#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        pattern: Pattern, // Changed from 'name: String' to support destructuring
        value: Expr,
        #[allow(dead_code)]
        mutable: bool,
        type_annotation: Option<TypeAnnotation>,
    },
    Const {
        name: String,
        value: Expr,
        type_annotation: Option<TypeAnnotation>,
    },
    #[allow(dead_code)]
    Assign {
        target: Expr, // Can be Identifier or IndexAccess
        value: Expr,
    },
    FuncDef {
        name: String,
        params: Vec<String>,
        param_types: Vec<Option<TypeAnnotation>>,
        is_async: bool, // true if async func syntax
        return_type: Option<TypeAnnotation>,
        body: Vec<Stmt>,
        is_generator: bool, // true if func* syntax
    },
    EnumDef {
        name: String,
        variants: Vec<String>,
    },
    Match {
        value: Expr,
        cases: Vec<(String, Vec<Stmt>)>,
        default: Option<Vec<Stmt>>,
    },
    #[allow(clippy::enum_variant_names)]
    ExprStmt(Expr),
    Return(Option<Expr>),
    #[allow(dead_code)]
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    Loop {
        condition: Option<Expr>,
        body: Vec<Stmt>,
    },
    For {
        var: String,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    TryExcept {
        try_block: Vec<Stmt>,
        except_var: String,
        except_block: Vec<Stmt>,
    },
    #[allow(dead_code)]
    Block(Vec<Stmt>),
    /// Import statement: import module or from module import symbol1, symbol2
    Import {
        module: String,
        symbols: Option<Vec<String>>, // None means import whole module, Some means specific symbols
    },
    /// Export statement: marks a statement as exported from a module
    Export {
        stmt: Box<Stmt>,
    },
    StructDef {
        name: String,
        fields: Vec<(String, Option<TypeAnnotation>)>,
        methods: Vec<Stmt>, // FuncDef statements
    },
    /// Spawn statement: run a block of code in a background thread
    Spawn {
        body: Vec<Stmt>,
    },
    /// Test statement: define a test case
    Test {
        name: String,
        body: Vec<Stmt>,
    },
    /// Test setup: run before each test
    TestSetup {
        body: Vec<Stmt>,
    },
    /// Test teardown: run after each test
    TestTeardown {
        body: Vec<Stmt>,
    },
    /// Test group: group related tests
    TestGroup {
        #[allow(dead_code)] // Used by test runner for organization (not yet fully implemented)
        name: String,
        tests: Vec<Stmt>, // Should contain Test statements
    },
}

impl Stmt {
    /// Returns a source span for this statement if available.
    /// The parser still produces unspanned statement nodes in some paths, so unknown is returned
    /// until spans are carried by every AST node.
    #[allow(dead_code)]
    pub fn span(&self) -> AstSpan {
        AstSpan::unknown()
    }

    /// Returns a source location for this statement if available.
    /// Currently returns unknown location - will be enhanced when parser tracks locations.
    #[allow(dead_code)]
    pub fn location(&self) -> SourceLocation {
        self.span().start
    }
}
