// File: src/interpreter/value.rs
//
// Runtime value types for the Kujo programming language.
// Defines all value types that can be represented and manipulated at runtime.

use crate::ast::Stmt;
#[cfg(feature = "runtime-db")]
use crate::interpreter::async_runtime::AsyncRuntime;
use ahash::AHasher;
#[cfg(feature = "runtime-image")]
use image::DynamicImage;
#[cfg(feature = "runtime-db")]
use mysql_async::Conn as MysqlConn;
use nohash_hasher::NoHashHasher;
#[cfg(feature = "runtime-db")]
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode, SslVersion};
#[cfg(feature = "runtime-db")]
use openssl::x509::X509;
#[cfg(feature = "runtime-db")]
use postgres::config::{Host, SslMode};
#[cfg(feature = "runtime-db")]
use postgres::Client as PostgresClient;
#[cfg(feature = "runtime-db")]
use postgres_openssl::MakeTlsConnector;
#[cfg(feature = "runtime-db")]
use rusqlite::Connection as SqliteConnection;
use std::collections::HashMap;
use std::fs::File;
use std::hash::BuildHasherDefault;
use std::ops::Deref;
#[cfg(feature = "runtime-db")]
use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};
#[cfg(feature = "runtime-archive")]
use zip::ZipWriter;

/// Open, bounded, atomically published private-file spool state.
///
/// The live file is reachable only through a `Value::PrivateSpool` handle. If
/// the handle is dropped or explicitly aborted before publication, the
/// same-directory temporary file is removed.
pub struct PrivateSpoolState {
    pub file: Option<File>,
    pub destination: PathBuf,
    pub temp: PathBuf,
    pub mode: u32,
    pub max_bytes: u64,
    pub bytes_written: u64,
    pub hasher: sha2::Sha256,
}

impl Drop for PrivateSpoolState {
    fn drop(&mut self) {
        self.file.take();
        let _ = std::fs::remove_file(&self.temp);
    }
}

// Forward declaration - Environment is in a sibling module
use super::environment::Environment;

/// Hash map for integer-keyed dictionaries.
pub type IntDictMap = HashMap<i64, Value, BuildHasherDefault<NoHashHasher<i64>>>;

/// Dense vector for integer-keyed dictionaries with non-negative keys.
pub type DenseIntDict = Vec<Value>;

/// Dense vector for integer-keyed dictionaries with optional int values.
pub type DenseIntDictInt = Vec<Option<i64>>;

/// Dense vector for integer-keyed dictionaries with int values only.
pub type DenseIntDictIntFull = Vec<i64>;

/// Hash map for string-keyed dictionaries.
pub type DictMap = HashMap<Arc<str>, Value, BuildHasherDefault<AHasher>>;

/// One-shot parts of an incremental HTTP response.
///
/// The response body is consumed by the routed HTTP server while stream events
/// are dispatched to the Kujo callback on the interpreter/VM thread. Keeping
/// this state behind an `Option` makes a streaming response explicitly
/// single-use even though runtime values are cheaply cloneable.
pub struct HttpResponseStreamParts {
    pub body: Box<dyn std::io::Read + Send>,
    pub events: Receiver<Value>,
    pub cancelled: Arc<std::sync::atomic::AtomicBool>,
    pub content_length: Option<usize>,
}

pub type SharedHttpResponseStream = Arc<Mutex<Option<HttpResponseStreamParts>>>;

/// A routed HTTP upload that authorizes from request metadata before any body
/// bytes are accepted, then streams the body into a bounded private file.
#[derive(Clone)]
pub struct HttpUploadRoute {
    pub method: String,
    pub path: String,
    pub spool_directory: String,
    pub max_body_bytes: u64,
    pub authorize: Value,
    pub handler: Value,
}

/// Stores function bodies and drops deeply nested statement trees iteratively.
///
/// Function bodies can contain very deep nesting through statements like loops,
/// `if`, `match`, and nested function definitions. Rust's default drop is recursive
/// for this shape and can overflow the stack at shutdown or when large AST graphs
/// are released.
///
/// This container keeps existing call sites stable while replacing the old
/// leak-on-drop strategy with a non-recursive drop traversal.
pub struct LeakyFunctionBody {
    id: usize,
}

#[derive(Clone)]
struct FunctionBodyStorage {
    body: Vec<Stmt>,
}

struct StoredFunctionBody {
    refs: usize,
    storage: Arc<FunctionBodyStorage>,
}

static FUNCTION_BODY_STORE: OnceLock<Mutex<HashMap<usize, StoredFunctionBody>>> = OnceLock::new();
static NEXT_FUNCTION_BODY_ID: AtomicUsize = AtomicUsize::new(1);

fn function_body_store() -> &'static Mutex<HashMap<usize, StoredFunctionBody>> {
    FUNCTION_BODY_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

impl Drop for FunctionBodyStorage {
    fn drop(&mut self) {
        let body = std::mem::take(&mut self.body);
        drop_stmt_tree_iterative(body);
    }
}

fn drop_stmt_tree_iterative(root: Vec<Stmt>) {
    let mut stack = root;

    while let Some(mut stmt) = stack.pop() {
        match &mut stmt {
            Stmt::FuncDef { body, .. }
            | Stmt::Loop { body, .. }
            | Stmt::For { body, .. }
            | Stmt::While { body, .. }
            | Stmt::Block(body)
            | Stmt::Spawn { body }
            | Stmt::Test { body, .. }
            | Stmt::TestSetup { body }
            | Stmt::TestTeardown { body } => {
                stack.extend(std::mem::take(body));
            }
            Stmt::Match { cases, default, .. } => {
                for (_, case_body) in std::mem::take(cases) {
                    stack.extend(case_body);
                }
                if let Some(default_body) = default.take() {
                    stack.extend(default_body);
                }
            }
            Stmt::If { then_branch, else_branch, .. } => {
                stack.extend(std::mem::take(then_branch));
                if let Some(else_body) = else_branch.take() {
                    stack.extend(else_body);
                }
            }
            Stmt::TryExcept { try_block, except_block, .. } => {
                stack.extend(std::mem::take(try_block));
                stack.extend(std::mem::take(except_block));
            }
            Stmt::Export { stmt } => {
                let inner_stmt = std::mem::replace(stmt, Box::new(Stmt::Block(Vec::new())));
                stack.push(*inner_stmt);
            }
            Stmt::StructDef { methods, .. } | Stmt::TestGroup { tests: methods, .. } => {
                stack.extend(std::mem::take(methods));
            }
            Stmt::Let { .. }
            | Stmt::Const { .. }
            | Stmt::Assign { .. }
            | Stmt::EnumDef { .. }
            | Stmt::ExprStmt(_)
            | Stmt::Return(_)
            | Stmt::Break
            | Stmt::Continue
            | Stmt::Import { .. } => {}
        }
    }
}

pub struct FunctionBodyRef {
    storage: Arc<FunctionBodyStorage>,
}

impl Deref for FunctionBodyRef {
    type Target = Vec<Stmt>;

    fn deref(&self) -> &Self::Target {
        &self.storage.body
    }
}

impl Clone for LeakyFunctionBody {
    fn clone(&self) -> Self {
        let mut store = function_body_store().lock().unwrap();
        if let Some(entry) = store.get_mut(&self.id) {
            entry.refs += 1;
        }

        LeakyFunctionBody { id: self.id }
    }
}

impl Drop for LeakyFunctionBody {
    fn drop(&mut self) {
        let mut store = function_body_store().lock().unwrap();
        if let Some(entry) = store.get_mut(&self.id) {
            if entry.refs > 1 {
                entry.refs -= 1;
            } else {
                store.remove(&self.id);
            }
        }
    }
}

impl LeakyFunctionBody {
    pub fn new(body: Vec<Stmt>) -> Self {
        let id = NEXT_FUNCTION_BODY_ID.fetch_add(1, Ordering::Relaxed);
        let storage = Arc::new(FunctionBodyStorage { body });

        let mut store = function_body_store().lock().unwrap();
        store.insert(id, StoredFunctionBody { refs: 1, storage });

        LeakyFunctionBody { id }
    }

    pub fn get(&self) -> FunctionBodyRef {
        let store = function_body_store().lock().unwrap();
        if let Some(entry) = store.get(&self.id) {
            FunctionBodyRef { storage: Arc::clone(&entry.storage) }
        } else {
            FunctionBodyRef { storage: Arc::new(FunctionBodyStorage { body: Vec::new() }) }
        }
    }

    pub(crate) fn same_identity(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "runtime-db")]
    use super::ConnectionPool;
    use super::{DictMap, LeakyFunctionBody, Value};
    use crate::ast::Stmt;
    #[cfg(feature = "runtime-db")]
    use std::collections::HashMap;
    use std::sync::Arc;
    #[cfg(feature = "runtime-db")]
    use std::sync::Barrier;

    fn deeply_nested_loop_stmt(depth: usize) -> Stmt {
        let mut current = Stmt::Break;
        for _ in 0..depth {
            current = Stmt::Loop { condition: None, body: vec![current] };
        }
        current
    }

    #[test]
    fn function_body_retains_statements() {
        let body = LeakyFunctionBody::new(vec![Stmt::Break, Stmt::Continue]);
        assert_eq!(2, body.get().len());
    }

    #[test]
    fn cloned_function_body_can_be_dropped_multiple_times() {
        let body = LeakyFunctionBody::new(vec![Stmt::Break]);
        let clone = body.clone();

        drop(clone);
        drop(body);
    }

    #[test]
    fn drops_deeply_nested_function_body_without_recursion_overflow() {
        let body = LeakyFunctionBody::new(vec![deeply_nested_loop_stmt(20000)]);
        drop(body);
    }

    #[test]
    fn cloned_handles_share_same_function_body_storage() {
        let body = LeakyFunctionBody::new(vec![Stmt::Break, Stmt::Continue]);
        let clone = body.clone();

        assert_eq!(body.get().len(), clone.get().len());
    }

    #[cfg(feature = "runtime-db")]
    #[test]
    fn database_pool_reserves_capacity_atomically() {
        let path = std::env::temp_dir().join(format!(
            "kujo_pool_capacity_{}_{}.sqlite",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let mut config = HashMap::new();
        config.insert("min_connections".to_string(), Value::Int(0));
        config.insert("max_connections".to_string(), Value::Int(1));
        config.insert("acquisition_timeout_ms".to_string(), Value::Int(100));
        let pool = Arc::new(
            ConnectionPool::new("sqlite".to_string(), path.to_string_lossy().to_string(), config)
                .expect("pool should initialize"),
        );
        let barrier = Arc::new(Barrier::new(17));
        let mut workers = Vec::new();
        for _ in 0..16 {
            let pool = Arc::clone(&pool);
            let barrier = Arc::clone(&barrier);
            workers.push(std::thread::spawn(move || {
                barrier.wait();
                pool.acquire().ok()
            }));
        }
        barrier.wait();
        let connections = workers
            .into_iter()
            .filter_map(|worker| worker.join().expect("worker should not panic"))
            .collect::<Vec<_>>();
        assert_eq!(connections.len(), 1);
        assert_eq!(pool.stats().get("total"), Some(&1));
        drop(connections);
        pool.close();
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn value_truthiness_semantics_match_runtime_contract() {
        let mut non_empty_dict = DictMap::default();
        non_empty_dict.insert(Arc::<str>::from("k"), Value::Int(1));

        let falsey_values = vec![
            Value::Bool(false),
            Value::Null,
            Value::Int(0),
            Value::Float(0.0),
            Value::Str(Arc::new(String::new())),
            Value::Array(Arc::new(vec![])),
            Value::Dict(Arc::new(DictMap::default())),
        ];
        for value in falsey_values {
            assert!(!value.is_truthy(), "expected falsey value, got {:?}", value);
        }

        let truthy_values = vec![
            Value::Bool(true),
            Value::Int(-1),
            Value::Float(0.5),
            Value::Str(Arc::new("false".to_string())),
            Value::Array(Arc::new(vec![Value::Int(1)])),
            Value::Dict(Arc::new(non_empty_dict)),
        ];
        for value in truthy_values {
            assert!(value.is_truthy(), "expected truthy value, got {:?}", value);
        }
    }

    #[test]
    fn value_equals_supports_numeric_cross_type_and_nested_collections() {
        assert!(Value::equals(&Value::Int(1), &Value::Float(1.0)));
        assert!(!Value::equals(&Value::Int(1), &Value::Float(2.0)));

        let left_nested = Value::Array(Arc::new(vec![
            Value::Int(1),
            Value::Array(Arc::new(vec![Value::Int(2), Value::Int(3)])),
        ]));
        let right_nested = Value::Array(Arc::new(vec![
            Value::Int(1),
            Value::Array(Arc::new(vec![Value::Int(2), Value::Int(3)])),
        ]));
        assert!(Value::equals(&left_nested, &right_nested));

        let mut left_dict = DictMap::default();
        left_dict.insert(Arc::<str>::from("answer"), Value::Int(42));
        let mut right_dict = DictMap::default();
        right_dict.insert(Arc::<str>::from("answer"), Value::Int(42));
        assert!(Value::equals(
            &Value::Dict(Arc::new(left_dict)),
            &Value::Dict(Arc::new(right_dict))
        ));
    }

    #[test]
    fn value_equals_supports_function_and_native_identity() {
        let function_body = LeakyFunctionBody::new(vec![Stmt::Return(None)]);
        let same_function = Value::Function(vec!["x".to_string()], function_body.clone(), None);
        let same_function_alias =
            Value::Function(vec!["x".to_string()], function_body.clone(), None);
        let different_function = Value::Function(
            vec!["x".to_string()],
            LeakyFunctionBody::new(vec![Stmt::Return(None)]),
            None,
        );

        assert!(Value::equals(&same_function, &same_function_alias));
        assert!(!Value::equals(&same_function, &different_function));
        assert!(Value::equals(
            &Value::NativeFunction("print".to_string()),
            &Value::NativeFunction("print".to_string())
        ));
    }

    #[test]
    fn value_compare_order_rejects_unsupported_types() {
        let bool_error =
            Value::compare_order(&Value::Bool(true), "<", &Value::Bool(false)).unwrap_err();
        assert!(bool_error.contains("Invalid binary operation"));

        let mixed_error =
            Value::compare_order(&Value::Int(1), "<", &Value::Str(Arc::new("1".to_string())))
                .unwrap_err();
        assert!(mixed_error.contains("Invalid binary operation"));

        assert_eq!(
            Value::compare_order(
                &Value::Str(Arc::new("a".to_string())),
                "<",
                &Value::Str(Arc::new("b".to_string()))
            )
            .unwrap(),
            true
        );
    }
}

/// Database connection types
/// Infrastructure for database.rs stub module
#[cfg(feature = "runtime-db")]
pub struct RuntimeSafePostgresClient(Option<PostgresClient>);

#[cfg(feature = "runtime-db")]
impl RuntimeSafePostgresClient {
    pub fn new(client: PostgresClient) -> Self {
        Self(Some(client))
    }
}

#[cfg(feature = "runtime-db")]
impl Deref for RuntimeSafePostgresClient {
    type Target = PostgresClient;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("PostgreSQL client is available")
    }
}

#[cfg(feature = "runtime-db")]
impl DerefMut for RuntimeSafePostgresClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("PostgreSQL client is available")
    }
}

#[cfg(feature = "runtime-db")]
impl Drop for RuntimeSafePostgresClient {
    fn drop(&mut self) {
        if let Some(client) = self.0.take() {
            AsyncRuntime::run_runtime_safe_blocking(move || drop(client));
        }
    }
}

#[cfg(feature = "runtime-db")]
#[derive(Clone)]
pub enum DatabaseConnection {
    #[allow(dead_code)]
    Sqlite(Arc<Mutex<SqliteConnection>>),
    #[allow(dead_code)]
    Postgres(Arc<Mutex<RuntimeSafePostgresClient>>),
    #[allow(dead_code)]
    Mysql(Arc<Mutex<MysqlConn>>),
}

#[cfg(feature = "runtime-db")]
#[derive(Clone)]
enum PoolTransport {
    Legacy,
    VerifiedPostgresTls { ca_pem: Arc<String> },
}

#[cfg(feature = "runtime-db")]
struct AvailableConnection {
    connection: DatabaseConnection,
    created_at: std::time::Instant,
    idle_since: std::time::Instant,
}

#[cfg(feature = "runtime-db")]
#[derive(Default)]
struct PoolMetrics {
    acquire_timeout_count: usize,
    connection_error_count: usize,
    eviction_count: usize,
    health_check_failure_count: usize,
    reset_failure_count: usize,
}

#[cfg(feature = "runtime-db")]
#[derive(Default)]
struct PoolState {
    available: std::collections::VecDeque<AvailableConnection>,
    checked_out: HashMap<usize, std::time::Instant>,
    total: usize,
    metrics: PoolMetrics,
}

/// Bounded database connection pool with verified-TLS PostgreSQL support.
#[cfg(feature = "runtime-db")]
#[derive(Clone)]
pub struct ConnectionPool {
    pub(crate) db_type: String,
    pub(crate) connection_string: String,
    pub(crate) min_connections: usize,
    pub(crate) max_connections: usize,
    acquisition_timeout: std::time::Duration,
    connect_timeout: std::time::Duration,
    idle_timeout: std::time::Duration,
    max_lifetime: std::time::Duration,
    statement_timeout_ms: u64,
    health_check: bool,
    transport: PoolTransport,
    state: Arc<Mutex<PoolState>>,
    closed: Arc<AtomicBool>,
}

#[cfg(feature = "runtime-db")]
impl ConnectionPool {
    pub fn new(
        db_type: String,
        connection_string: String,
        config: HashMap<String, Value>,
    ) -> Result<Self, String> {
        Self::from_config(db_type, connection_string, PoolTransport::Legacy, config, false)
    }

    pub fn new_postgres_tls(
        connection_string: String,
        ca_pem: String,
        config: HashMap<String, Value>,
    ) -> Result<Self, String> {
        let pool = Self::from_config(
            "postgres".to_string(),
            connection_string,
            PoolTransport::VerifiedPostgresTls { ca_pem: Arc::new(ca_pem) },
            config,
            true,
        )?;
        pool.warm_minimum()?;
        Ok(pool)
    }

    fn integer_option(
        config: &HashMap<String, Value>,
        name: &str,
        default: u64,
        minimum: u64,
        maximum: u64,
    ) -> Result<u64, String> {
        let value = match config.get(name) {
            None => default,
            Some(Value::Int(value)) if *value >= 0 => *value as u64,
            _ => return Err(format!("pool option '{name}' must be a non-negative integer")),
        };
        if !(minimum..=maximum).contains(&value) {
            return Err(format!("pool option '{name}' must be between {minimum} and {maximum}"));
        }
        Ok(value)
    }

    fn from_config(
        db_type: String,
        connection_string: String,
        transport: PoolTransport,
        config: HashMap<String, Value>,
        strict: bool,
    ) -> Result<Self, String> {
        let allowed = [
            "min_connections",
            "max_connections",
            "connection_timeout",
            "acquisition_timeout_ms",
            "connect_timeout_ms",
            "idle_timeout_seconds",
            "max_lifetime_seconds",
            "statement_timeout_ms",
            "health_check",
        ];
        if strict {
            for key in config.keys() {
                if !allowed.contains(&key.as_str()) {
                    return Err(format!("unknown pool option '{key}'"));
                }
            }
        }
        let min_connections = Self::integer_option(&config, "min_connections", 2, 0, 64)? as usize;
        let max_connections =
            Self::integer_option(&config, "max_connections", 20, 1, 128)? as usize;
        if min_connections > max_connections {
            return Err("min_connections cannot be greater than max_connections".to_string());
        }
        let legacy_timeout_ms =
            Self::integer_option(&config, "connection_timeout", 30, 1, 300)? * 1000;
        let acquisition_timeout_ms = Self::integer_option(
            &config,
            "acquisition_timeout_ms",
            legacy_timeout_ms,
            100,
            300_000,
        )?;
        let connect_timeout_ms =
            Self::integer_option(&config, "connect_timeout_ms", 10_000, 100, 300_000)?;
        let idle_timeout_seconds =
            Self::integer_option(&config, "idle_timeout_seconds", 300, 1, 86_400)?;
        let max_lifetime_seconds =
            Self::integer_option(&config, "max_lifetime_seconds", 1800, 1, 86_400)?;
        let statement_timeout_ms =
            Self::integer_option(&config, "statement_timeout_ms", 30_000, 1, 300_000)?;
        let health_check = match config.get("health_check") {
            None => true,
            Some(Value::Bool(value)) => *value,
            _ => return Err("pool option 'health_check' must be a boolean".to_string()),
        };
        Ok(Self {
            db_type,
            connection_string,
            min_connections,
            max_connections,
            acquisition_timeout: std::time::Duration::from_millis(acquisition_timeout_ms),
            connect_timeout: std::time::Duration::from_millis(connect_timeout_ms),
            idle_timeout: std::time::Duration::from_secs(idle_timeout_seconds),
            max_lifetime: std::time::Duration::from_secs(max_lifetime_seconds),
            statement_timeout_ms,
            health_check,
            transport,
            state: Arc::new(Mutex::new(PoolState::default())),
            closed: Arc::new(AtomicBool::new(false)),
        })
    }

    fn warm_minimum(&self) -> Result<(), String> {
        for _ in 0..self.min_connections {
            let connection = self.create_connection()?;
            let now = std::time::Instant::now();
            let mut state = self.state.lock().map_err(|_| "database pool lock poisoned")?;
            state.total += 1;
            state.available.push_back(AvailableConnection {
                connection,
                created_at: now,
                idle_since: now,
            });
        }
        Ok(())
    }

    pub fn acquire(&self) -> Result<DatabaseConnection, String> {
        let deadline = std::time::Instant::now() + self.acquisition_timeout;

        loop {
            if self.closed.load(Ordering::Acquire) {
                return Err("database pool is closed".to_string());
            }
            let candidate = {
                let mut state = self.state.lock().map_err(|_| "database pool lock poisoned")?;
                state.available.pop_front()
            };
            if let Some(candidate) = candidate {
                let now = std::time::Instant::now();
                if now.duration_since(candidate.created_at) >= self.max_lifetime
                    || now.duration_since(candidate.idle_since) >= self.idle_timeout
                {
                    self.evict_connection();
                    continue;
                }
                if self.health_check && !self.connection_is_healthy(&candidate.connection) {
                    let mut state = self.state.lock().map_err(|_| "database pool lock poisoned")?;
                    state.metrics.health_check_failure_count += 1;
                    drop(state);
                    self.evict_connection();
                    continue;
                }
                return self.checkout(candidate.connection, candidate.created_at);
            }

            let reserved = {
                let mut state = self.state.lock().map_err(|_| "database pool lock poisoned")?;
                if state.total < self.max_connections {
                    state.total += 1;
                    true
                } else {
                    false
                }
            };
            if reserved {
                match self.create_connection() {
                    Ok(connection) => return self.checkout(connection, std::time::Instant::now()),
                    Err(error) => {
                        let mut state =
                            self.state.lock().map_err(|_| "database pool lock poisoned")?;
                        state.total = state.total.saturating_sub(1);
                        state.metrics.connection_error_count += 1;
                        return Err(error);
                    }
                }
            }
            if std::time::Instant::now() >= deadline {
                let mut state = self.state.lock().map_err(|_| "database pool lock poisoned")?;
                state.metrics.acquire_timeout_count += 1;
                return Err("database pool acquisition deadline exceeded".to_string());
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    fn checkout(
        &self,
        connection: DatabaseConnection,
        created_at: std::time::Instant,
    ) -> Result<DatabaseConnection, String> {
        let key = connection_key(&connection);
        let mut state = self.state.lock().map_err(|_| "database pool lock poisoned")?;
        if self.closed.load(Ordering::Acquire) {
            state.total = state.total.saturating_sub(1);
            return Err("database pool is closed".to_string());
        }
        state.checked_out.insert(key, created_at);
        Ok(connection)
    }

    pub fn release(&self, connection: DatabaseConnection) -> Result<(), String> {
        let key = connection_key(&connection);
        let created_at = {
            let mut state = self.state.lock().map_err(|_| "database pool lock poisoned")?;
            state
                .checked_out
                .remove(&key)
                .ok_or_else(|| "connection is not checked out from this pool".to_string())?
        };
        if self.closed.load(Ordering::Acquire) {
            let mut state = self.state.lock().map_err(|_| "database pool lock poisoned")?;
            state.total = state.total.saturating_sub(1);
            return Ok(());
        }
        if let Err(error) = self.reset_connection(&connection) {
            let mut state = self.state.lock().map_err(|_| "database pool lock poisoned")?;
            state.total = state.total.saturating_sub(1);
            state.metrics.reset_failure_count += 1;
            return Err(error);
        }
        let mut state = self.state.lock().map_err(|_| "database pool lock poisoned")?;
        state.available.push_back(AvailableConnection {
            connection,
            created_at,
            idle_since: std::time::Instant::now(),
        });
        Ok(())
    }

    pub fn stats(&self) -> HashMap<String, usize> {
        let state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut stats = HashMap::new();
        stats.insert("available".to_string(), state.available.len());
        stats.insert("in_use".to_string(), state.checked_out.len());
        stats.insert("total".to_string(), state.total);
        stats.insert("min".to_string(), self.min_connections);
        stats.insert("max".to_string(), self.max_connections);
        stats.insert("closed".to_string(), usize::from(self.closed.load(Ordering::Acquire)));
        stats.insert("acquire_timeouts".to_string(), state.metrics.acquire_timeout_count);
        stats.insert("connection_errors".to_string(), state.metrics.connection_error_count);
        stats.insert("evictions".to_string(), state.metrics.eviction_count);
        stats.insert("health_check_failures".to_string(), state.metrics.health_check_failure_count);
        stats.insert("reset_failures".to_string(), state.metrics.reset_failure_count);
        stats
    }

    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let drained = state.available.len();
        state.available.clear();
        state.total = state.total.saturating_sub(drained);
    }

    fn create_connection(&self) -> Result<DatabaseConnection, String> {
        use postgres::NoTls;

        if let PoolTransport::VerifiedPostgresTls { ca_pem } = &self.transport {
            let client = connect_postgres_verified_tls(
                &self.connection_string,
                ca_pem,
                self.connect_timeout,
                self.statement_timeout_ms,
            )?;
            return Ok(DatabaseConnection::Postgres(Arc::new(Mutex::new(
                RuntimeSafePostgresClient::new(client),
            ))));
        }
        match self.db_type.as_str() {
            "sqlite" => SqliteConnection::open(&self.connection_string)
                .map(|conn| DatabaseConnection::Sqlite(Arc::new(Mutex::new(conn))))
                .map_err(|e| format!("Failed to create SQLite connection: {}", e)),
            "postgres" | "postgresql" => AsyncRuntime::run_runtime_safe_blocking(|| {
                PostgresClient::connect(&self.connection_string, NoTls)
            })
            .map(|client| {
                DatabaseConnection::Postgres(Arc::new(Mutex::new(RuntimeSafePostgresClient::new(
                    client,
                ))))
            })
            .map_err(|e| format!("Failed to create PostgreSQL connection: {}", e)),
            "mysql" => {
                let opts = mysql_async::OptsBuilder::from_opts(
                    mysql_async::Opts::from_url(&self.connection_string)
                        .map_err(|e| format!("Invalid MySQL connection string: {}", e))?,
                );

                let runtime = tokio::runtime::Runtime::new()
                    .map_err(|e| format!("Failed to create runtime: {}", e))?;

                runtime.block_on(async {
                    mysql_async::Conn::new(opts)
                        .await
                        .map(|conn| DatabaseConnection::Mysql(Arc::new(Mutex::new(conn))))
                        .map_err(|e| format!("Failed to create MySQL connection: {}", e))
                })
            }
            _ => Err(format!("Unsupported database type: {}", self.db_type)),
        }
    }

    fn connection_is_healthy(&self, connection: &DatabaseConnection) -> bool {
        match connection {
            DatabaseConnection::Postgres(client) => {
                let client = Arc::clone(client);
                AsyncRuntime::run_runtime_safe_blocking(move || {
                    client.lock().map_err(|_| ())?.simple_query("SELECT 1").map_err(|_| ())
                })
                .is_ok()
            }
            _ => true,
        }
    }

    fn reset_connection(&self, connection: &DatabaseConnection) -> Result<(), String> {
        match connection {
            DatabaseConnection::Postgres(client) => {
                let client = Arc::clone(client);
                let statement_timeout_ms = self.statement_timeout_ms;
                AsyncRuntime::run_runtime_safe_blocking(move || {
                    let mut client = client
                        .lock()
                        .map_err(|_| "database pool connection lock poisoned".to_string())?;
                    client
                        .batch_execute("ROLLBACK")
                        .and_then(|_| client.batch_execute("DISCARD ALL"))
                        .and_then(|_| {
                            client.batch_execute(&format!(
                                "SET statement_timeout = {statement_timeout_ms}"
                            ))
                        })
                        .map_err(|_| "database pool could not reset PostgreSQL session".to_string())
                })
            }
            _ => Ok(()),
        }
    }

    fn evict_connection(&self) {
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.total = state.total.saturating_sub(1);
        state.metrics.eviction_count += 1;
    }
}

#[cfg(feature = "runtime-db")]
fn connection_key(connection: &DatabaseConnection) -> usize {
    match connection {
        DatabaseConnection::Sqlite(value) => Arc::as_ptr(value) as usize,
        DatabaseConnection::Postgres(value) => Arc::as_ptr(value) as usize,
        DatabaseConnection::Mysql(value) => Arc::as_ptr(value) as usize,
    }
}

#[cfg(feature = "runtime-db")]
pub(crate) fn connect_postgres_verified_tls(
    connection_string: &str,
    ca_pem: &str,
    connect_timeout: std::time::Duration,
    statement_timeout_ms: u64,
) -> Result<PostgresClient, String> {
    const MAX_CONNECTION_STRING_BYTES: usize = 8192;
    const MAX_CA_PEM_BYTES: usize = 1024 * 1024;
    if connection_string.is_empty()
        || connection_string.len() > MAX_CONNECTION_STRING_BYTES
        || connection_string.contains('\0')
        || ca_pem.is_empty()
        || ca_pem.len() > MAX_CA_PEM_BYTES
        || ca_pem.contains('\0')
    {
        return Err("verified PostgreSQL TLS rejected invalid connection material".to_string());
    }
    let mut config = connection_string
        .parse::<postgres::Config>()
        .map_err(|_| "verified PostgreSQL TLS rejected the connection string".to_string())?;
    if config.get_hosts().is_empty()
        || !config.get_hosts().iter().all(|host| matches!(host, Host::Tcp(_)))
    {
        return Err("verified PostgreSQL TLS requires an explicit TCP host".to_string());
    }
    let certificates = X509::stack_from_pem(ca_pem.as_bytes())
        .map_err(|_| "verified PostgreSQL TLS rejected the CA trust bundle".to_string())?;
    if certificates.is_empty() {
        return Err("verified PostgreSQL TLS rejected the CA trust bundle".to_string());
    }
    config.ssl_mode(SslMode::Require).connect_timeout(connect_timeout);
    let mut builder = SslConnector::builder(SslMethod::tls())
        .map_err(|_| "verified PostgreSQL TLS could not initialize TLS".to_string())?;
    builder
        .set_min_proto_version(Some(SslVersion::TLS1_2))
        .map_err(|_| "verified PostgreSQL TLS could not enforce TLS 1.2".to_string())?;
    builder.set_verify(SslVerifyMode::PEER);
    for certificate in certificates {
        builder
            .cert_store_mut()
            .add_cert(certificate)
            .map_err(|_| "verified PostgreSQL TLS rejected the CA trust bundle".to_string())?;
    }
    let mut connector = MakeTlsConnector::new(builder.build());
    connector.set_callback(|configuration, _| {
        configuration.set_verify_hostname(true);
        Ok(())
    });
    let mut client = AsyncRuntime::run_runtime_safe_blocking(|| config.connect(connector))
        .map_err(|_| "failed to connect to PostgreSQL with verified TLS".to_string())?;
    client
        .batch_execute(&format!("SET statement_timeout = {statement_timeout_ms}"))
        .map_err(|_| "failed to initialize PostgreSQL session limits".to_string())?;
    Ok(client)
}

/// Runtime values in the Kujo interpreter
///
/// This enum represents all possible runtime values in Kujo. It's a large enum
/// with 30+ variants covering primitives, collections, functions, I/O types, and more.
#[derive(Clone)]
pub enum Value {
    /// Tagged enum variant with named fields
    Tagged { tag: String, fields: HashMap<String, Value> },
    /// 64-bit signed integer
    Int(i64),
    /// 64-bit floating point number
    Float(f64),
    /// String value (reference-counted for cheap cloning)
    Str(Arc<String>),
    /// Redacted string value for secrets that should not be printed or serialized
    Secret(Arc<String>),
    /// Boolean value
    Bool(bool),
    /// Null value for optional chaining and null coalescing
    Null,
    /// Binary data for files, HTTP downloads, etc.
    Bytes(Vec<u8>),
    /// Function: parameters, body, optional captured environment
    Function(Vec<String>, LeakyFunctionBody, Option<Arc<Mutex<Environment>>>),
    /// Async function: parameters, body, optional captured environment
    AsyncFunction(Vec<String>, LeakyFunctionBody, Option<Arc<Mutex<Environment>>>),
    /// Native (built-in) function by name
    NativeFunction(String),
    /// Bytecode function (experimental - VM not yet default)
    #[allow(dead_code)]
    BytecodeFunction {
        chunk: crate::bytecode::BytecodeChunk,
        /// Captured variables with shared mutable state
        captured: HashMap<String, Arc<Mutex<Value>>>,
        /// Captured binding mutability metadata keyed by variable name.
        captured_binding_kinds: HashMap<String, crate::bytecode::BytecodeBindingKind>,
    },
    /// Bytecode generator instance with execution state (VM-based generators)
    #[allow(dead_code)]
    BytecodeGenerator { state: Arc<Mutex<crate::vm::GeneratorState>> },
    /// Internal marker for dynamic array construction in VM
    ArrayMarker,
    /// Return value wrapper
    Return(Box<Value>),
    /// Legacy simple error string
    Error(String),
    /// Rich error object with stack trace and chaining
    ErrorObject {
        message: String,
        stack: Vec<String>,
        line: Option<usize>,
        cause: Option<Box<Value>>,
    },
    /// Enum type (currently unused)
    #[allow(dead_code)]
    Enum(String),
    /// Struct instance with fields
    Struct { name: String, fields: HashMap<String, Value> },
    /// Struct definition with methods
    StructDef { name: String, field_names: Vec<String>, methods: HashMap<String, Value> },
    /// Array of values (reference-counted for cheap cloning)
    Array(Arc<Vec<Value>>),
    /// Dictionary (hash map) of string keys to values (reference-counted for cheap cloning)
    Dict(Arc<DictMap>),
    /// Fixed-key dictionary for fast literal construction
    FixedDict { keys: Arc<Vec<Arc<str>>>, values: Vec<Value> },
    /// Dictionary (hash map) of integer keys to values (reference-counted for cheap cloning)
    IntDict(Arc<IntDictMap>),
    /// Dense integer dictionary for non-negative integer keys
    DenseIntDict(Arc<DenseIntDict>),
    /// Dense integer dictionary for non-negative integer keys with int values only
    DenseIntDictInt(Arc<DenseIntDictInt>),
    /// Dense integer dictionary for non-negative integer keys with int values only (no nulls)
    DenseIntDictIntFull(Arc<DenseIntDictIntFull>),
    /// Set of unique values
    Set(Vec<Value>),
    /// FIFO queue
    Queue(std::collections::VecDeque<Value>),
    /// LIFO stack
    Stack(Vec<Value>),
    /// Bounded thread-safe channel for message passing and backpressure.
    Channel(Arc<Mutex<(std::sync::mpsc::SyncSender<Value>, std::sync::mpsc::Receiver<Value>)>>),
    /// HTTP server with routes
    HttpServer {
        host: String,
        port: u16,
        routes: Vec<(String, String, Value)>, // (method, path, handler)
        upload_routes: Vec<HttpUploadRoute>,
    },
    /// HTTP response
    HttpResponse { status: u16, body: Vec<u8>, headers: HashMap<String, String> },
    /// Incremental, single-use HTTP response with an optional event callback.
    HttpStreamingResponse {
        status: u16,
        headers: HashMap<String, String>,
        stream: SharedHttpResponseStream,
        callback: Option<Box<Value>>,
    },
    /// Database connection
    /// Infrastructure for database.rs stub module
    #[cfg(feature = "runtime-db")]
    #[allow(dead_code)]
    Database {
        connection: DatabaseConnection,
        db_type: String,
        connection_string: String,
        in_transaction: Arc<Mutex<bool>>,
    },
    /// Database connection pool
    /// Infrastructure for database.rs stub module
    #[cfg(feature = "runtime-db")]
    #[allow(dead_code)]
    DatabasePool { pool: Arc<ConnectionPool> },
    /// Image data
    #[cfg(feature = "runtime-image")]
    Image { data: Arc<Mutex<DynamicImage>>, format: String },
    /// Zip archive writer
    /// Infrastructure for zip.rs stub module
    #[cfg(feature = "runtime-archive")]
    #[allow(dead_code)]
    ZipArchive { writer: Arc<Mutex<Option<ZipWriter<File>>>>, path: String },
    /// TCP listener for accepting connections
    /// Infrastructure for network.rs stub module
    #[allow(dead_code)]
    TcpListener { listener: Arc<Mutex<std::net::TcpListener>>, addr: String },
    /// TCP stream for bidirectional communication
    /// Infrastructure for network.rs stub module
    #[allow(dead_code)]
    TcpStream { stream: Arc<Mutex<Option<std::net::TcpStream>>>, peer_addr: String },
    /// TLS stream for verified client connections or server-side accepted sessions.
    TlsStream {
        stream: Arc<Mutex<Option<openssl::ssl::SslStream<std::net::TcpStream>>>>,
        peer_addr: String,
        server_name: String,
        protocol: String,
        cipher: String,
        peer_certificate_sha256: Option<String>,
        peer_verified: bool,
    },
    /// Reusable server TLS configuration loaded from a certificate chain and private key.
    TlsAcceptor {
        acceptor: Arc<openssl::ssl::SslAcceptor>,
        certificate_sha256: String,
        min_protocol: String,
    },
    /// Bounded private-file spool with an open, non-reopenable file handle.
    PrivateSpool { spool: Arc<Mutex<Option<PrivateSpoolState>>>, destination: String },
    /// UDP socket for datagram communication
    /// Infrastructure for network.rs stub module
    #[allow(dead_code)]
    UdpSocket { socket: Arc<Mutex<std::net::UdpSocket>>, addr: String },
    /// Result type: Ok(value) or Err(error)
    Result { is_ok: bool, value: Box<Value> },
    /// Option type: Some(value) or None
    Option { is_some: bool, value: Box<Value> },
    /// Generator definition (before being called)
    GeneratorDef(Vec<String>, LeakyFunctionBody),
    /// Generator instance with execution state
    Generator {
        params: Vec<String>,
        body: LeakyFunctionBody,
        env: Arc<Mutex<Environment>>,
        pc: usize, // Program counter
        is_exhausted: bool,
    },
    /// Iterator instance wrapping a collection or generator
    Iterator {
        source: Box<Value>,
        index: usize,
        transformer: Option<Box<Value>>,
        filter_fn: Option<Box<Value>>,
        take_count: Option<usize>,
    },
    /// Promise for async computation results
    Promise {
        receiver: Arc<Mutex<tokio::sync::oneshot::Receiver<Result<Value, String>>>>,
        is_polled: Arc<Mutex<bool>>,
        cached_result: Arc<Mutex<Option<Result<Value, String>>>>,
        /// Optional join handle for spawned async task (None for already-resolved promises)
        #[allow(dead_code)]
        task_handle: Option<Arc<Mutex<Option<tokio::task::JoinHandle<Result<Value, String>>>>>>,
    },
    /// Task handle for spawned async tasks
    TaskHandle {
        handle: Arc<Mutex<Option<tokio::task::JoinHandle<Value>>>>,
        is_cancelled: Arc<Mutex<bool>>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallableArity {
    pub name: String,
    pub min_args: usize,
    pub max_args: Option<usize>,
    pub variadic: bool,
    pub parameter_names: Vec<String>,
}

impl CallableArity {
    pub fn exact(name: impl Into<String>, parameter_names: Vec<String>) -> Self {
        let count = parameter_names.len();
        Self {
            name: name.into(),
            min_args: count,
            max_args: Some(count),
            variadic: false,
            parameter_names,
        }
    }

    pub fn range(
        name: impl Into<String>,
        min_args: usize,
        max_args: usize,
        parameter_names: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            min_args,
            max_args: Some(max_args),
            variadic: min_args != max_args,
            parameter_names,
        }
    }

    pub fn variadic(
        name: impl Into<String>,
        min_args: usize,
        parameter_names: Vec<String>,
    ) -> Self {
        Self { name: name.into(), min_args, max_args: None, variadic: true, parameter_names }
    }

    pub fn validate(&self, received_args: usize) -> Result<(), String> {
        if received_args < self.min_args {
            return Err(self.format_error(received_args));
        }

        if let Some(max_args) = self.max_args {
            if received_args > max_args {
                return Err(self.format_error(received_args));
            }
        }

        Ok(())
    }

    fn expected_description(&self) -> String {
        match self.max_args {
            Some(max_args) if self.min_args == max_args => format!("{}", self.min_args),
            Some(max_args) => format!("{} to {}", self.min_args, max_args),
            None => format!("at least {}", self.min_args),
        }
    }

    fn format_error(&self, received_args: usize) -> String {
        format!(
            "{} expects {} arguments, got {}",
            self.name,
            self.expected_description(),
            received_args
        )
    }
}

// Manual Debug implementation for Value
impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Tagged { tag, fields } => {
                f.debug_struct("Tagged").field("tag", tag).field("fields", fields).finish()
            }
            Value::Int(n) => write!(f, "Int({})", n),
            Value::Float(n) => write!(f, "Float({})", n),
            Value::Str(s) => write!(f, "Str({:?})", s.as_ref()),
            Value::Secret(_) => write!(f, "Secret(***)"),
            Value::Bool(b) => write!(f, "Bool({})", b),
            Value::Null => write!(f, "Null"),
            Value::Bytes(bytes) => write!(f, "Bytes({} bytes)", bytes.len()),
            Value::Function(params, body, captured_env) => {
                let env_info = if captured_env.is_some() { " +closure" } else { "" };
                write!(f, "Function({:?}, {} stmts{})", params, body.get().len(), env_info)
            }
            Value::AsyncFunction(params, body, captured_env) => {
                let env_info = if captured_env.is_some() { " +closure" } else { "" };
                write!(f, "AsyncFunction({:?}, {} stmts{})", params, body.get().len(), env_info)
            }
            Value::NativeFunction(name) => write!(f, "NativeFunction({})", name),
            Value::BytecodeFunction { chunk, captured, captured_binding_kinds: _ } => {
                let name = chunk.name.as_deref().unwrap_or("<lambda>");
                write!(
                    f,
                    "BytecodeFunction({}, {} instructions, {} captured)",
                    name,
                    chunk.instructions.len(),
                    captured.len()
                )
            }
            Value::BytecodeGenerator { state } => {
                let state_lock = state.lock().unwrap();
                write!(
                    f,
                    "BytecodeGenerator(ip={}, exhausted={})",
                    state_lock.ip, state_lock.is_exhausted
                )
            }
            Value::ArrayMarker => write!(f, "ArrayMarker"),
            Value::Return(v) => write!(f, "Return({:?})", v),
            Value::Error(e) => write!(f, "Error({})", e),
            Value::ErrorObject { message, stack, line, cause } => f
                .debug_struct("ErrorObject")
                .field("message", message)
                .field("stack", stack)
                .field("line", line)
                .field("cause", &cause.as_ref().map(|_| "..."))
                .finish(),
            Value::Enum(e) => write!(f, "Enum({})", e),
            Value::Struct { name, fields } => {
                f.debug_struct("Struct").field("name", name).field("fields", fields).finish()
            }
            Value::StructDef { name, field_names, methods } => f
                .debug_struct("StructDef")
                .field("name", name)
                .field("field_names", field_names)
                .field("methods", &format!("{} methods", methods.len()))
                .finish(),
            Value::Array(elements) => write!(f, "Array[{}]", elements.len()),
            Value::Dict(map) => write!(f, "Dict{{{} keys}}", map.len()),
            Value::FixedDict { keys, .. } => write!(f, "FixedDict{{{} keys}}", keys.len()),
            Value::IntDict(map) => write!(f, "IntDict{{{} keys}}", map.len()),
            Value::DenseIntDict(values) => write!(f, "DenseIntDict{{{} keys}}", values.len()),
            Value::DenseIntDictInt(values) => write!(f, "DenseIntDictInt{{{} keys}}", values.len()),
            Value::DenseIntDictIntFull(values) => {
                write!(f, "DenseIntDictIntFull{{{} keys}}", values.len())
            }
            Value::Set(elements) => write!(f, "Set{{{} items}}", elements.len()),
            Value::Queue(queue) => write!(f, "Queue({} items)", queue.len()),
            Value::Stack(stack) => write!(f, "Stack({} items)", stack.len()),
            Value::Channel(_) => write!(f, "Channel"),
            Value::HttpServer { host, port, routes, upload_routes } => {
                write!(
                    f,
                    "HttpServer(host={}, port={}, {} routes, {} upload routes)",
                    host,
                    port,
                    routes.len(),
                    upload_routes.len()
                )
            }
            Value::HttpResponse { status, body, .. } => {
                write!(f, "HttpResponse(status={}, body_len={})", status, body.len())
            }
            Value::HttpStreamingResponse { status, .. } => {
                write!(f, "HttpStreamingResponse(status={})", status)
            }
            #[cfg(feature = "runtime-db")]
            Value::Database { db_type, connection_string, .. } => {
                write!(f, "Database(type={}, connection={})", db_type, connection_string)
            }
            #[cfg(feature = "runtime-db")]
            Value::DatabasePool { pool } => {
                write!(f, "DatabasePool(type={}, max={})", pool.db_type, pool.max_connections)
            }
            #[cfg(feature = "runtime-image")]
            Value::Image { format, data } => {
                let img = data.lock().unwrap();
                write!(f, "Image({}x{}, format={})", img.width(), img.height(), format)
            }
            #[cfg(feature = "runtime-archive")]
            Value::ZipArchive { path, .. } => {
                write!(f, "ZipArchive(path={})", path)
            }
            Value::TcpListener { addr, .. } => {
                write!(f, "TcpListener(addr={})", addr)
            }
            Value::TcpStream { peer_addr, .. } => {
                write!(f, "TcpStream(peer={})", peer_addr)
            }
            Value::TlsStream { peer_addr, protocol, cipher, .. } => {
                write!(f, "TlsStream(peer={}, protocol={}, cipher={})", peer_addr, protocol, cipher)
            }
            Value::TlsAcceptor { certificate_sha256, min_protocol, .. } => {
                write!(
                    f,
                    "TlsAcceptor(certificate_sha256={}, min_protocol={})",
                    certificate_sha256, min_protocol
                )
            }
            Value::PrivateSpool { destination, .. } => {
                write!(f, "PrivateSpool(destination={})", destination)
            }
            Value::UdpSocket { addr, .. } => {
                write!(f, "UdpSocket(addr={})", addr)
            }
            Value::Result { is_ok, value } => {
                if *is_ok {
                    write!(f, "Ok({:?})", value)
                } else {
                    write!(f, "Err({:?})", value)
                }
            }
            Value::Option { is_some, value } => {
                if *is_some {
                    write!(f, "Some({:?})", value)
                } else {
                    write!(f, "None")
                }
            }
            Value::GeneratorDef(params, body) => {
                write!(f, "GeneratorDef({:?}, {} stmts)", params, body.get().len())
            }
            Value::Generator { params, is_exhausted, pc, .. } => {
                write!(f, "Generator({:?}, pc={}, exhausted={})", params, pc, is_exhausted)
            }
            Value::Iterator { source, index, .. } => {
                write!(f, "Iterator(source={:?}, index={})", source, index)
            }
            Value::Promise { cached_result, .. } => {
                let result = cached_result.lock().unwrap();
                match &*result {
                    None => write!(f, "Promise(Pending)"),
                    Some(Ok(_)) => write!(f, "Promise(Resolved)"),
                    Some(Err(err)) => write!(f, "Promise(Rejected: {})", err),
                }
            }
            Value::TaskHandle { is_cancelled, .. } => {
                let cancelled = is_cancelled.lock().unwrap();
                if *cancelled {
                    write!(f, "TaskHandle(Cancelled)")
                } else {
                    write!(f, "TaskHandle(Running)")
                }
            }
        }
    }
}

impl Value {
    /// Helper to create a Str value from a String
    #[allow(dead_code)]
    pub fn str(s: String) -> Self {
        Value::Str(Arc::new(s))
    }

    /// Helper to create a Str value from a &str
    #[allow(dead_code)]
    pub fn str_ref(s: &str) -> Self {
        Value::Str(Arc::new(s.to_string()))
    }

    /// Helper to create a Secret value from a String
    #[allow(dead_code)]
    pub fn secret(s: String) -> Self {
        Value::Secret(Arc::new(s))
    }

    /// Helper to create an Array value from a Vec<Value>
    #[allow(dead_code)]
    pub fn array(vec: Vec<Value>) -> Self {
        Value::Array(Arc::new(vec))
    }

    /// Helper to create a Dict value from a HashMap<String, Value>
    #[allow(dead_code)]
    pub fn dict(map: DictMap) -> Self {
        Value::Dict(Arc::new(map))
    }

    /// Kujo runtime truthiness contract used by interpreter and VM condition evaluation.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(value) => *value,
            Value::Null => false,
            Value::Int(value) => *value != 0,
            Value::Float(value) => *value != 0.0,
            Value::Str(value) => !value.is_empty(),
            Value::Secret(value) => !value.is_empty(),
            Value::Array(values) => !values.is_empty(),
            Value::Dict(values) => !values.is_empty(),
            _ => true,
        }
    }

    pub fn equals(left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Secret(a), Value::Secret(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => Self::float_equals(*a, *b),
            (Value::Int(a), Value::Float(b)) => Self::float_equals(*a as f64, *b),
            (Value::Float(a), Value::Int(b)) => Self::float_equals(*a, *b as f64),
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len()
                    && a.iter().zip(b.iter()).all(|(lhs, rhs)| Self::equals(lhs, rhs))
            }
            (Value::Dict(_), Value::Dict(_))
            | (Value::Dict(_), Value::FixedDict { .. })
            | (Value::Dict(_), Value::IntDict(_))
            | (Value::Dict(_), Value::DenseIntDict(_))
            | (Value::Dict(_), Value::DenseIntDictInt(_))
            | (Value::Dict(_), Value::DenseIntDictIntFull(_))
            | (Value::FixedDict { .. }, Value::Dict(_))
            | (Value::FixedDict { .. }, Value::FixedDict { .. })
            | (Value::FixedDict { .. }, Value::IntDict(_))
            | (Value::FixedDict { .. }, Value::DenseIntDict(_))
            | (Value::FixedDict { .. }, Value::DenseIntDictInt(_))
            | (Value::FixedDict { .. }, Value::DenseIntDictIntFull(_))
            | (Value::IntDict(_), Value::Dict(_))
            | (Value::IntDict(_), Value::FixedDict { .. })
            | (Value::IntDict(_), Value::IntDict(_))
            | (Value::IntDict(_), Value::DenseIntDict(_))
            | (Value::IntDict(_), Value::DenseIntDictInt(_))
            | (Value::IntDict(_), Value::DenseIntDictIntFull(_))
            | (Value::DenseIntDict(_), Value::Dict(_))
            | (Value::DenseIntDict(_), Value::FixedDict { .. })
            | (Value::DenseIntDict(_), Value::IntDict(_))
            | (Value::DenseIntDict(_), Value::DenseIntDict(_))
            | (Value::DenseIntDict(_), Value::DenseIntDictInt(_))
            | (Value::DenseIntDict(_), Value::DenseIntDictIntFull(_))
            | (Value::DenseIntDictInt(_), Value::Dict(_))
            | (Value::DenseIntDictInt(_), Value::FixedDict { .. })
            | (Value::DenseIntDictInt(_), Value::IntDict(_))
            | (Value::DenseIntDictInt(_), Value::DenseIntDict(_))
            | (Value::DenseIntDictInt(_), Value::DenseIntDictInt(_))
            | (Value::DenseIntDictInt(_), Value::DenseIntDictIntFull(_))
            | (Value::DenseIntDictIntFull(_), Value::Dict(_))
            | (Value::DenseIntDictIntFull(_), Value::FixedDict { .. })
            | (Value::DenseIntDictIntFull(_), Value::IntDict(_))
            | (Value::DenseIntDictIntFull(_), Value::DenseIntDict(_))
            | (Value::DenseIntDictIntFull(_), Value::DenseIntDictInt(_))
            | (Value::DenseIntDictIntFull(_), Value::DenseIntDictIntFull(_)) => {
                Self::map_values_equal(left, right)
            }
            (
                Value::Tagged { tag: left_tag, fields: left_fields },
                Value::Tagged { tag: right_tag, fields: right_fields },
            ) => {
                left_tag == right_tag
                    && Self::string_key_map_values_equal(left_fields, right_fields)
            }
            (
                Value::Struct { name: left_name, fields: left_fields },
                Value::Struct { name: right_name, fields: right_fields },
            ) => {
                left_name == right_name
                    && Self::string_key_map_values_equal(left_fields, right_fields)
            }
            (
                Value::StructDef {
                    name: left_name,
                    field_names: left_field_names,
                    methods: left_methods,
                },
                Value::StructDef {
                    name: right_name,
                    field_names: right_field_names,
                    methods: right_methods,
                },
            ) => {
                left_name == right_name
                    && left_field_names == right_field_names
                    && Self::string_key_map_values_equal(left_methods, right_methods)
            }
            (
                Value::Result { is_ok: left_ok, value: left_value },
                Value::Result { is_ok: right_ok, value: right_value },
            ) => left_ok == right_ok && Self::equals(left_value, right_value),
            (
                Value::Option { is_some: left_some, value: left_value },
                Value::Option { is_some: right_some, value: right_value },
            ) => left_some == right_some && Self::equals(left_value, right_value),
            (
                Value::Function(left_params, left_body, left_env),
                Value::Function(right_params, right_body, right_env),
            ) => {
                left_params == right_params
                    && left_body.same_identity(right_body)
                    && Self::optional_env_ptr_eq(left_env, right_env)
            }
            (
                Value::AsyncFunction(left_params, left_body, left_env),
                Value::AsyncFunction(right_params, right_body, right_env),
            ) => {
                left_params == right_params
                    && left_body.same_identity(right_body)
                    && Self::optional_env_ptr_eq(left_env, right_env)
            }
            (
                Value::BytecodeFunction {
                    chunk: left_chunk,
                    captured: left_captured,
                    captured_binding_kinds: left_captured_binding_kinds,
                },
                Value::BytecodeFunction {
                    chunk: right_chunk,
                    captured: right_captured,
                    captured_binding_kinds: right_captured_binding_kinds,
                },
            ) => {
                left_chunk == right_chunk
                    && Self::captured_value_map_ptr_eq(left_captured, right_captured)
                    && left_captured_binding_kinds == right_captured_binding_kinds
            }
            (
                Value::GeneratorDef(left_params, left_body),
                Value::GeneratorDef(right_params, right_body),
            ) => left_params == right_params && left_body.same_identity(right_body),
            (Value::NativeFunction(left_name), Value::NativeFunction(right_name)) => {
                left_name == right_name
            }
            _ => false,
        }
    }

    pub fn compare_order(left: &Value, op: &str, right: &Value) -> Result<bool, String> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => match op {
                "<" => Ok(a < b),
                ">" => Ok(a > b),
                "<=" => Ok(a <= b),
                ">=" => Ok(a >= b),
                _ => Err(format!("Unknown comparison: {}", op)),
            },
            (Value::Float(a), Value::Float(b)) => match op {
                "<" => Ok(a < b),
                ">" => Ok(a > b),
                "<=" => Ok(a <= b),
                ">=" => Ok(a >= b),
                _ => Err(format!("Unknown comparison: {}", op)),
            },
            (Value::Int(a), Value::Float(b)) => match op {
                "<" => Ok((*a as f64) < *b),
                ">" => Ok((*a as f64) > *b),
                "<=" => Ok((*a as f64) <= *b),
                ">=" => Ok((*a as f64) >= *b),
                _ => Err(format!("Unknown comparison: {}", op)),
            },
            (Value::Float(a), Value::Int(b)) => match op {
                "<" => Ok(*a < (*b as f64)),
                ">" => Ok(*a > (*b as f64)),
                "<=" => Ok(*a <= (*b as f64)),
                ">=" => Ok(*a >= (*b as f64)),
                _ => Err(format!("Unknown comparison: {}", op)),
            },
            (Value::Str(a), Value::Str(b)) => match op {
                "<" => Ok(a.as_ref() < b.as_ref()),
                ">" => Ok(a.as_ref() > b.as_ref()),
                "<=" => Ok(a.as_ref() <= b.as_ref()),
                ">=" => Ok(a.as_ref() >= b.as_ref()),
                _ => Err(format!("Unknown comparison: {}", op)),
            },
            _ => Err(format!(
                "Invalid binary operation: {} {} {}",
                Self::type_name(left),
                op,
                Self::type_name(right)
            )),
        }
    }

    fn optional_env_ptr_eq(
        left: &Option<Arc<Mutex<Environment>>>,
        right: &Option<Arc<Mutex<Environment>>>,
    ) -> bool {
        match (left, right) {
            (Some(left_env), Some(right_env)) => Arc::ptr_eq(left_env, right_env),
            (None, None) => true,
            _ => false,
        }
    }

    fn map_values_equal(left: &Value, right: &Value) -> bool {
        let Some(left_entries) = Self::map_entries(left) else {
            return false;
        };
        let Some(right_entries) = Self::map_entries(right) else {
            return false;
        };
        if left_entries.len() != right_entries.len() {
            return false;
        }

        left_entries.iter().all(|(left_key, left_value)| {
            right_entries
                .iter()
                .find(|(right_key, _)| left_key == right_key)
                .map(|(_, right_value)| Self::equals(left_value, right_value))
                .unwrap_or(false)
        })
    }

    fn map_entries(value: &Value) -> Option<Vec<(String, Value)>> {
        match value {
            Value::Dict(map) => Some(
                map.iter().map(|(key, value)| (key.as_ref().to_string(), value.clone())).collect(),
            ),
            Value::FixedDict { keys, values } => Some(
                keys.iter()
                    .zip(values.iter())
                    .map(|(key, value)| (key.as_ref().to_string(), value.clone()))
                    .collect(),
            ),
            Value::IntDict(map) => {
                Some(map.iter().map(|(key, value)| (key.to_string(), value.clone())).collect())
            }
            Value::DenseIntDict(values) => Some(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| (index.to_string(), value.clone()))
                    .collect(),
            ),
            Value::DenseIntDictInt(values) => Some(
                values
                    .iter()
                    .enumerate()
                    .filter_map(|(index, value)| {
                        value.map(|int_value| (index.to_string(), Value::Int(int_value)))
                    })
                    .collect(),
            ),
            Value::DenseIntDictIntFull(values) => Some(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| (index.to_string(), Value::Int(*value)))
                    .collect(),
            ),
            _ => None,
        }
    }

    fn string_key_map_values_equal(
        left: &HashMap<String, Value>,
        right: &HashMap<String, Value>,
    ) -> bool {
        if left.len() != right.len() {
            return false;
        }

        left.iter().all(|(left_key, left_value)| {
            right
                .get(left_key)
                .map(|right_value| Self::equals(left_value, right_value))
                .unwrap_or(false)
        })
    }

    fn captured_value_map_ptr_eq(
        left: &HashMap<String, Arc<Mutex<Value>>>,
        right: &HashMap<String, Arc<Mutex<Value>>>,
    ) -> bool {
        if left.len() != right.len() {
            return false;
        }

        left.iter().all(|(left_key, left_value)| {
            right
                .get(left_key)
                .map(|right_value| Arc::ptr_eq(left_value, right_value))
                .unwrap_or(false)
        })
    }

    fn type_name(value: &Value) -> &'static str {
        match value {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "string",
            Value::Secret(_) => "secret",
            Value::Array(_) => "array",
            Value::Dict(_)
            | Value::FixedDict { .. }
            | Value::IntDict(_)
            | Value::DenseIntDict(_)
            | Value::DenseIntDictInt(_)
            | Value::DenseIntDictIntFull(_) => "dict",
            Value::Struct { .. } => "struct",
            Value::Function(..)
            | Value::AsyncFunction(..)
            | Value::GeneratorDef(..)
            | Value::Generator { .. } => "function",
            Value::NativeFunction(_) => "native_function",
            Value::Null => "null",
            Value::Error(_) | Value::ErrorObject { .. } => "error",
            _ => "value",
        }
    }

    /// Checked integer arithmetic semantics for Kujo runtime operations.
    pub fn checked_int_arithmetic(left: i64, op: &str, right: i64) -> Result<i64, String> {
        let overflow_error = || format!("Integer overflow: {} {} {}", left, op, right);
        match op {
            "+" => left.checked_add(right).ok_or_else(overflow_error),
            "-" => left.checked_sub(right).ok_or_else(overflow_error),
            "*" => left.checked_mul(right).ok_or_else(overflow_error),
            "/" => {
                if right == 0 {
                    Err("Division by zero".to_string())
                } else {
                    left.checked_div(right).ok_or_else(overflow_error)
                }
            }
            "%" => {
                if right == 0 {
                    Err("Modulo by zero".to_string())
                } else {
                    left.checked_rem(right).ok_or_else(overflow_error)
                }
            }
            _ => Err(format!("Unsupported integer operator: {}", op)),
        }
    }

    /// Float arithmetic semantics for Kujo runtime operations.
    pub fn checked_float_arithmetic(left: f64, op: &str, right: f64) -> Result<f64, String> {
        match op {
            "+" => Ok(left + right),
            "-" => Ok(left - right),
            "*" => Ok(left * right),
            "/" => {
                if right == 0.0 {
                    Err("Division by zero".to_string())
                } else {
                    Ok(left / right)
                }
            }
            "%" => {
                if right == 0.0 {
                    Err("Modulo by zero".to_string())
                } else {
                    Ok(left % right)
                }
            }
            _ => Err(format!("Unsupported float operator: {}", op)),
        }
    }

    /// Float equality semantics:
    /// - NaN is never equal to any value (including itself)
    /// - infinities compare by exact IEEE sign/value
    /// - finite values retain epsilon-based comparison for compatibility
    pub fn float_equals(left: f64, right: f64) -> bool {
        if left.is_nan() || right.is_nan() {
            return false;
        }
        if left.is_infinite() || right.is_infinite() {
            return left == right;
        }
        (left - right).abs() < f64::EPSILON
    }
}
