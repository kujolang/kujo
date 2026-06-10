// File: src/interpreter/native_functions/database.rs
//
// Database access native functions

use crate::interpreter::{ConnectionPool, DatabaseConnection, DictMap, Value};
use mysql_async::prelude::Queryable;
use postgres::NoTls;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

macro_rules! lock_or_db_error {
    ($mutex:expr, $context:expr) => {
        match $mutex.lock() {
            Ok(guard) => guard,
            Err(_) => return Some(Value::Error(format!("{}: database lock poisoned", $context))),
        }
    };
}

fn map_sqlite_value(value: rusqlite::types::Value) -> Value {
    match value {
        rusqlite::types::Value::Integer(number) => Value::Int(number),
        rusqlite::types::Value::Real(number) => Value::Float(number),
        rusqlite::types::Value::Text(text) => Value::Str(Arc::new(text)),
        rusqlite::types::Value::Null => Value::Null,
        rusqlite::types::Value::Blob(_) => Value::Str(Arc::new("[blob]".to_string())),
    }
}

fn map_mysql_value(value: mysql_async::Value) -> Value {
    match value {
        mysql_async::Value::NULL => Value::Null,
        mysql_async::Value::Bytes(bytes) => {
            String::from_utf8(bytes).map(|v| Value::Str(Arc::new(v))).unwrap_or(Value::Null)
        }
        mysql_async::Value::Int(number) => Value::Int(number),
        mysql_async::Value::UInt(number) => Value::Int(number as i64),
        mysql_async::Value::Float(number) => Value::Int(number as i64),
        mysql_async::Value::Double(number) => Value::Float(number),
        mysql_async::Value::Date(year, month, day, hour, minute, second, micro) => {
            Value::Str(Arc::new(format!(
                "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
                year, month, day, hour, minute, second, micro
            )))
        }
        mysql_async::Value::Time(is_negative, days, hours, minutes, seconds, micros) => {
            let sign = if is_negative { "-" } else { "" };
            Value::Str(Arc::new(format!(
                "{}{}d {:02}:{:02}:{:02}.{:06}",
                sign, days, hours, minutes, seconds, micros
            )))
        }
    }
}

fn to_mysql_value(value: &Value) -> mysql_async::Value {
    match value {
        Value::Str(text) => mysql_async::Value::Bytes(text.as_bytes().to_vec()),
        Value::Int(number) => mysql_async::Value::Int(*number),
        Value::Float(number) => mysql_async::Value::Double(*number),
        Value::Bool(flag) => mysql_async::Value::Int(if *flag { 1 } else { 0 }),
        Value::Null => mysql_async::Value::NULL,
        other => mysql_async::Value::Bytes(format!("{:?}", other).as_bytes().to_vec()),
    }
}

fn create_runtime() -> Result<tokio::runtime::Runtime, String> {
    tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create async runtime: {}", e))
}

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "db_connect" => {
            if arg_values.len() > 2 {
                return Some(Value::Error(
                    "db_connect requires database type ('sqlite'|'postgres'|'mysql') and connection string"
                        .to_string(),
                ));
            }

            if let (Some(Value::Str(db_type)), Some(Value::Str(connection_string))) =
                (arg_values.first(), arg_values.get(1))
            {
                let db_type_lower = db_type.to_lowercase();
                match db_type_lower.as_str() {
                    "sqlite" => match rusqlite::Connection::open(connection_string.as_ref()) {
                        Ok(connection) => Value::Database {
                            connection: DatabaseConnection::Sqlite(Arc::new(Mutex::new(
                                connection,
                            ))),
                            db_type: "sqlite".to_string(),
                            connection_string: connection_string.as_ref().to_string(),
                            in_transaction: Arc::new(Mutex::new(false)),
                        },
                        Err(error) => {
                            Value::Error(format!("Failed to connect to SQLite: {}", error))
                        }
                    },
                    "postgres" | "postgresql" => {
                        match postgres::Client::connect(connection_string.as_ref(), NoTls) {
                            Ok(client) => Value::Database {
                                connection: DatabaseConnection::Postgres(Arc::new(Mutex::new(
                                    client,
                                ))),
                                db_type: "postgres".to_string(),
                                connection_string: connection_string.as_ref().to_string(),
                                in_transaction: Arc::new(Mutex::new(false)),
                            },
                            Err(error) => {
                                Value::Error(format!("Failed to connect to PostgreSQL: {}", error))
                            }
                        }
                    }
                    "mysql" => {
                        let opts = match mysql_async::Opts::from_url(connection_string.as_ref()) {
                            Ok(opts) => mysql_async::OptsBuilder::from_opts(opts),
                            Err(error) => {
                                return Some(Value::Error(format!(
                                    "Invalid MySQL connection string: {}",
                                    error
                                )))
                            }
                        };

                        match create_runtime() {
                            Ok(runtime) => match runtime
                                .block_on(async { mysql_async::Conn::new(opts).await })
                            {
                                Ok(connection) => Value::Database {
                                    connection: DatabaseConnection::Mysql(Arc::new(Mutex::new(
                                        connection,
                                    ))),
                                    db_type: "mysql".to_string(),
                                    connection_string: connection_string.as_ref().to_string(),
                                    in_transaction: Arc::new(Mutex::new(false)),
                                },
                                Err(error) => {
                                    Value::Error(format!("Failed to connect to MySQL: {}", error))
                                }
                            },
                            Err(error) => Value::Error(error),
                        }
                    }
                    _ => Value::Error(format!(
                        "Unsupported database type: {}. Currently supported: 'sqlite', 'postgres'",
                        db_type
                    )),
                }
            } else {
                Value::Error(
                    "db_connect requires database type ('sqlite'|'postgres'|'mysql') and connection string"
                        .to_string(),
                )
            }
        }

        "db_execute" => {
            if arg_values.len() > 3 {
                return Some(Value::Error(
                    "db_execute requires 2 or 3 arguments: connection, SQL string, optional params array"
                        .to_string(),
                ));
            }

            if let Some(Value::Database { connection, db_type, .. }) = arg_values.first() {
                if let Some(Value::Str(sql)) = arg_values.get(1) {
                    let params = arg_values.get(2);
                    match (connection, db_type.as_str()) {
                        (DatabaseConnection::Sqlite(connection), "sqlite") => {
                            let connection = lock_or_db_error!(connection, "database.connection");
                            let execute_result = if let Some(Value::Array(param_arr)) = params {
                                let param_values: Vec<Box<dyn rusqlite::ToSql>> = param_arr
                                    .iter()
                                    .map(|value| match value {
                                        Value::Str(text) => Box::new(text.as_ref().to_string())
                                            as Box<dyn rusqlite::ToSql>,
                                        Value::Int(number) => {
                                            Box::new(*number) as Box<dyn rusqlite::ToSql>
                                        }
                                        Value::Float(number) => {
                                            Box::new(*number) as Box<dyn rusqlite::ToSql>
                                        }
                                        Value::Bool(flag) => {
                                            Box::new(*flag) as Box<dyn rusqlite::ToSql>
                                        }
                                        Value::Null => Box::new(rusqlite::types::Null)
                                            as Box<dyn rusqlite::ToSql>,
                                        other => Box::new(format!("{:?}", other))
                                            as Box<dyn rusqlite::ToSql>,
                                    })
                                    .collect();
                                let params_refs: Vec<&dyn rusqlite::ToSql> =
                                    param_values.iter().map(|value| value.as_ref()).collect();
                                connection.execute(sql.as_ref(), params_refs.as_slice())
                            } else {
                                connection.execute(sql.as_ref(), [])
                            };

                            match execute_result {
                                Ok(rows_affected) => Value::Int(rows_affected as i64),
                                Err(error) => {
                                    Value::Error(format!("SQLite execution error: {}", error))
                                }
                            }
                        }
                        (DatabaseConnection::Postgres(client), "postgres") => {
                            let mut client = lock_or_db_error!(client, "database.client_mut");
                            let execute_result = if let Some(Value::Array(param_arr)) = params {
                                let postgres_params: Vec<String> = param_arr
                                    .iter()
                                    .map(|value| match value {
                                        Value::Str(text) => text.as_ref().to_string(),
                                        Value::Int(number) => number.to_string(),
                                        Value::Float(number) => number.to_string(),
                                        Value::Bool(flag) => flag.to_string(),
                                        Value::Null => String::new(),
                                        other => format!("{:?}", other),
                                    })
                                    .collect();
                                let params_refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
                                    postgres_params
                                        .iter()
                                        .map(|value| value as &(dyn postgres::types::ToSql + Sync))
                                        .collect();
                                client.execute(sql.as_ref(), params_refs.as_slice())
                            } else {
                                client.execute(sql.as_ref(), &[])
                            };

                            match execute_result {
                                Ok(rows_affected) => Value::Int(rows_affected as i64),
                                Err(error) => {
                                    Value::Error(format!("PostgreSQL execution error: {}", error))
                                }
                            }
                        }
                        (DatabaseConnection::Mysql(connection), "mysql") => {
                            let mut connection =
                                lock_or_db_error!(connection, "database.connection_mut");
                            match create_runtime() {
                                Ok(runtime) => {
                                    let execute_result = if let Some(Value::Array(param_arr)) =
                                        params
                                    {
                                        let mysql_params: Vec<mysql_async::Value> =
                                            param_arr.iter().map(to_mysql_value).collect();
                                        runtime.block_on(async {
                                            connection.exec_drop(sql.as_ref(), mysql_params).await
                                        })
                                    } else {
                                        runtime.block_on(async {
                                            connection.exec_drop(sql.as_ref(), ()).await
                                        })
                                    };

                                    match execute_result {
                                        Ok(_) => Value::Int(connection.affected_rows() as i64),
                                        Err(error) => Value::Error(format!(
                                            "MySQL execution error: {}",
                                            error
                                        )),
                                    }
                                }
                                Err(error) => Value::Error(error),
                            }
                        }
                        _ => Value::Error(
                            "Invalid database connection type or database type not yet supported"
                                .to_string(),
                        ),
                    }
                } else {
                    Value::Error("db_execute requires SQL string as second argument".to_string())
                }
            } else {
                Value::Error(
                    "db_execute requires a database connection as first argument".to_string(),
                )
            }
        }

        "db_query" => {
            if arg_values.len() > 3 {
                return Some(Value::Error(
                    "db_query requires 2 or 3 arguments: connection, SQL string, optional params array"
                        .to_string(),
                ));
            }

            if let Some(Value::Database { connection, db_type, .. }) = arg_values.first() {
                if let Some(Value::Str(sql)) = arg_values.get(1) {
                    let params = arg_values.get(2);
                    match (connection, db_type.as_str()) {
                        (DatabaseConnection::Sqlite(connection), "sqlite") => {
                            let connection = lock_or_db_error!(connection, "database.connection");

                            let param_values: Vec<Box<dyn rusqlite::ToSql>> =
                                if let Some(Value::Array(param_arr)) = params {
                                    param_arr
                                        .iter()
                                        .map(|value| match value {
                                            Value::Str(text) => Box::new(text.as_ref().to_string())
                                                as Box<dyn rusqlite::ToSql>,
                                            Value::Int(number) => {
                                                Box::new(*number) as Box<dyn rusqlite::ToSql>
                                            }
                                            Value::Float(number) => {
                                                Box::new(*number) as Box<dyn rusqlite::ToSql>
                                            }
                                            Value::Bool(flag) => {
                                                Box::new(*flag) as Box<dyn rusqlite::ToSql>
                                            }
                                            Value::Null => Box::new(rusqlite::types::Null)
                                                as Box<dyn rusqlite::ToSql>,
                                            other => Box::new(format!("{:?}", other))
                                                as Box<dyn rusqlite::ToSql>,
                                        })
                                        .collect()
                                } else {
                                    Vec::new()
                                };
                            let params_refs: Vec<&dyn rusqlite::ToSql> =
                                param_values.iter().map(|value| value.as_ref()).collect();

                            let mut stmt = match connection.prepare(sql.as_ref()) {
                                Ok(statement) => statement,
                                Err(error) => {
                                    return Some(Value::Error(format!(
                                        "SQLite prepare error: {}",
                                        error
                                    )))
                                }
                            };

                            let column_names: Vec<String> =
                                stmt.column_names().iter().map(|v| v.to_string()).collect();

                            let query_result = if params_refs.is_empty() {
                                stmt.query([])
                            } else {
                                stmt.query(params_refs.as_slice())
                            };

                            let mut rows = match query_result {
                                Ok(rows) => rows,
                                Err(error) => {
                                    return Some(Value::Error(format!(
                                        "SQLite query error: {}",
                                        error
                                    )))
                                }
                            };

                            let mut results = Vec::new();
                            loop {
                                match rows.next() {
                                    Ok(Some(row)) => {
                                        let mut row_dict = DictMap::default();
                                        for (index, col_name) in column_names.iter().enumerate() {
                                            let value = row
                                                .get::<_, rusqlite::types::Value>(index)
                                                .map(map_sqlite_value)
                                                .unwrap_or(Value::Null);
                                            row_dict.insert(col_name.clone().into(), value);
                                        }
                                        results.push(Value::Dict(Arc::new(row_dict)));
                                    }
                                    Ok(None) => break,
                                    Err(error) => {
                                        return Some(Value::Error(format!(
                                            "SQLite row error: {}",
                                            error
                                        )))
                                    }
                                }
                            }

                            Value::Array(Arc::new(results))
                        }
                        (DatabaseConnection::Postgres(client), "postgres") => {
                            let mut client = lock_or_db_error!(client, "database.client_mut");
                            let query_result = if let Some(Value::Array(param_arr)) = params {
                                let postgres_params: Vec<String> = param_arr
                                    .iter()
                                    .map(|value| match value {
                                        Value::Str(text) => text.as_ref().to_string(),
                                        Value::Int(number) => number.to_string(),
                                        Value::Float(number) => number.to_string(),
                                        Value::Bool(flag) => flag.to_string(),
                                        Value::Null => String::new(),
                                        other => format!("{:?}", other),
                                    })
                                    .collect();
                                let params_refs: Vec<&(dyn postgres::types::ToSql + Sync)> =
                                    postgres_params
                                        .iter()
                                        .map(|value| value as &(dyn postgres::types::ToSql + Sync))
                                        .collect();
                                client.query(sql.as_ref(), params_refs.as_slice())
                            } else {
                                client.query(sql.as_ref(), &[])
                            };

                            match query_result {
                                Ok(rows) => {
                                    let mut results = Vec::new();
                                    for row in rows {
                                        let mut row_dict = DictMap::default();
                                        for (index, column) in row.columns().iter().enumerate() {
                                            let col_name = column.name().to_string();
                                            let value = if let Ok(v) = row.try_get::<_, i32>(index)
                                            {
                                                Value::Int(v as i64)
                                            } else if let Ok(v) = row.try_get::<_, i64>(index) {
                                                Value::Int(v)
                                            } else if let Ok(v) = row.try_get::<_, f64>(index) {
                                                Value::Float(v)
                                            } else if let Ok(v) = row.try_get::<_, String>(index) {
                                                Value::Str(Arc::new(v))
                                            } else if let Ok(v) = row.try_get::<_, bool>(index) {
                                                Value::Bool(v)
                                            } else {
                                                Value::Null
                                            };
                                            row_dict.insert(col_name.into(), value);
                                        }
                                        results.push(Value::Dict(Arc::new(row_dict)));
                                    }
                                    Value::Array(Arc::new(results))
                                }
                                Err(error) => {
                                    Value::Error(format!("PostgreSQL query error: {}", error))
                                }
                            }
                        }
                        (DatabaseConnection::Mysql(connection), "mysql") => {
                            let mut connection =
                                lock_or_db_error!(connection, "database.connection_mut");
                            match create_runtime() {
                                Ok(runtime) => {
                                    let query_result: Result<
                                        Vec<mysql_async::Row>,
                                        mysql_async::Error,
                                    > = if let Some(Value::Array(param_arr)) = params {
                                        let mysql_params: Vec<mysql_async::Value> =
                                            param_arr.iter().map(to_mysql_value).collect();
                                        runtime.block_on(async {
                                            connection.exec(sql.as_ref(), mysql_params).await
                                        })
                                    } else {
                                        runtime.block_on(async {
                                            connection.exec(sql.as_ref(), ()).await
                                        })
                                    };

                                    match query_result {
                                        Ok(rows) => {
                                            let mut results = Vec::new();
                                            for mut row in rows {
                                                let mut row_dict = DictMap::default();
                                                let columns = row.columns();
                                                for (index, column) in columns.iter().enumerate() {
                                                    let col_name = column.name_str().to_string();
                                                    let value = row
                                                        .take::<mysql_async::Value, _>(index)
                                                        .map(map_mysql_value)
                                                        .unwrap_or(Value::Null);
                                                    row_dict.insert(col_name.into(), value);
                                                }
                                                results.push(Value::Dict(Arc::new(row_dict)));
                                            }
                                            Value::Array(Arc::new(results))
                                        }
                                        Err(error) => {
                                            Value::Error(format!("MySQL query error: {}", error))
                                        }
                                    }
                                }
                                Err(error) => Value::Error(error),
                            }
                        }
                        _ => Value::Error(
                            "Invalid database connection type or database type not yet supported"
                                .to_string(),
                        ),
                    }
                } else {
                    Value::Error("db_query requires SQL string as second argument".to_string())
                }
            } else {
                Value::Error(
                    "db_query requires a database connection as first argument".to_string(),
                )
            }
        }

        "db_close" => {
            if arg_values.len() > 1 {
                return Some(Value::Error("db_close requires a database connection".to_string()));
            }

            if let Some(Value::Database { .. }) = arg_values.first() {
                Value::Bool(true)
            } else {
                Value::Error("db_close requires a database connection".to_string())
            }
        }

        "db_pool" => {
            if arg_values.len() > 3 {
                return Some(Value::Error(
                    "db_pool requires 2 or 3 arguments: database type, connection string, optional config dict"
                        .to_string(),
                ));
            }

            if let (Some(Value::Str(db_type)), Some(Value::Str(connection_string))) =
                (arg_values.first(), arg_values.get(1))
            {
                let config = match arg_values.get(2) {
                    Some(Value::Dict(config)) => {
                        let mut standard_map = HashMap::new();
                        for (key, value) in config.iter() {
                            standard_map.insert(key.as_ref().to_string(), value.clone());
                        }
                        standard_map
                    }
                    Some(Value::FixedDict { keys, values }) => {
                        let mut standard_map = HashMap::new();
                        for (key, value) in keys.iter().zip(values.iter()) {
                            standard_map.insert(key.as_ref().to_string(), value.clone());
                        }
                        standard_map
                    }
                    Some(_) => {
                        return Some(Value::Error(
                            "db_pool optional config argument must be a dictionary".to_string(),
                        ));
                    }
                    None => HashMap::new(),
                };

                match ConnectionPool::new(
                    db_type.as_ref().to_string(),
                    connection_string.as_ref().to_string(),
                    config,
                ) {
                    Ok(pool) => Value::DatabasePool { pool: Arc::new(Mutex::new(pool)) },
                    Err(error) => {
                        Value::Error(format!("Failed to create connection pool: {}", error))
                    }
                }
            } else {
                Value::Error("db_pool requires database type and connection string".to_string())
            }
        }

        "db_pool_acquire" => {
            if arg_values.len() > 1 {
                return Some(Value::Error("db_pool_acquire requires a database pool".to_string()));
            }

            if let Some(Value::DatabasePool { pool }) = arg_values.first() {
                let pool_guard = lock_or_db_error!(pool, "database.pool");
                match pool_guard.acquire() {
                    Ok(connection) => Value::Database {
                        connection,
                        db_type: pool_guard.db_type.clone(),
                        connection_string: pool_guard.connection_string.clone(),
                        in_transaction: Arc::new(Mutex::new(false)),
                    },
                    Err(error) => Value::Error(format!("Failed to acquire connection: {}", error)),
                }
            } else {
                Value::Error("db_pool_acquire requires a database pool".to_string())
            }
        }

        "db_pool_release" => {
            if arg_values.len() > 2 {
                return Some(Value::Error(
                    "db_pool_release requires two arguments: database pool and database connection"
                        .to_string(),
                ));
            }

            if let Some(Value::DatabasePool { pool }) = arg_values.first() {
                if let Some(Value::Database { connection, .. }) = arg_values.get(1) {
                    let pool_guard = lock_or_db_error!(pool, "database.pool");
                    pool_guard.release(connection.clone());
                    Value::Bool(true)
                } else {
                    Value::Error(
                        "db_pool_release requires a database connection as second argument"
                            .to_string(),
                    )
                }
            } else {
                Value::Error(
                    "db_pool_release requires a database pool as first argument".to_string(),
                )
            }
        }

        "db_pool_stats" => {
            if arg_values.len() > 1 {
                return Some(Value::Error("db_pool_stats requires a database pool".to_string()));
            }

            if let Some(Value::DatabasePool { pool }) = arg_values.first() {
                let pool_guard = lock_or_db_error!(pool, "database.pool");
                let stats = pool_guard.stats();
                let mut dict = DictMap::default();
                for (key, value) in stats {
                    dict.insert(key.into(), Value::Int(value as i64));
                }
                Value::Dict(Arc::new(dict))
            } else {
                Value::Error("db_pool_stats requires a database pool".to_string())
            }
        }

        "db_pool_close" => {
            if arg_values.len() > 1 {
                return Some(Value::Error("db_pool_close requires a database pool".to_string()));
            }

            if let Some(Value::DatabasePool { pool }) = arg_values.first() {
                let pool_guard = lock_or_db_error!(pool, "database.pool");
                pool_guard.close();
                Value::Bool(true)
            } else {
                Value::Error("db_pool_close requires a database pool".to_string())
            }
        }

        "db_begin" => {
            if arg_values.len() > 1 {
                return Some(Value::Error(
                    "db_begin requires a database connection as first argument".to_string(),
                ));
            }

            let (connection, db_type, in_transaction) =
                if let Some(Value::Database { connection, db_type, in_transaction, .. }) =
                    arg_values.first()
                {
                    (connection.clone(), db_type.clone(), in_transaction.clone())
                } else {
                    return Some(Value::Error(
                        "db_begin requires a database connection as first argument".to_string(),
                    ));
                };

            {
                let in_trans = lock_or_db_error!(in_transaction, "database.in_transaction");
                if *in_trans {
                    return Some(Value::Error(
                        "Transaction already in progress. Commit or rollback first.".to_string(),
                    ));
                }
            }

            let result = match (connection, db_type.as_str()) {
                (DatabaseConnection::Sqlite(connection), "sqlite") => {
                    let connection = lock_or_db_error!(connection, "database.connection");
                    connection
                        .execute("BEGIN TRANSACTION", [])
                        .map(|_| ())
                        .map_err(|e| format!("Failed to begin transaction: {}", e))
                }
                (DatabaseConnection::Postgres(client), "postgres") => {
                    let mut client = lock_or_db_error!(client, "database.client_mut");
                    client
                        .execute("BEGIN", &[])
                        .map(|_| ())
                        .map_err(|e| format!("Failed to begin transaction: {}", e))
                }
                (DatabaseConnection::Mysql(connection), "mysql") => {
                    let mut connection = lock_or_db_error!(connection, "database.connection_mut");
                    match create_runtime() {
                        Ok(runtime) => runtime
                            .block_on(async {
                                connection
                                    .exec_drop("START TRANSACTION", mysql_async::Params::Empty)
                                    .await
                            })
                            .map(|_| ())
                            .map_err(|e| format!("Failed to begin transaction: {}", e)),
                        Err(error) => Err(error),
                    }
                }
                _ => Err("Invalid database connection".to_string()),
            };

            match result {
                Ok(()) => {
                    let mut in_trans =
                        lock_or_db_error!(in_transaction, "database.in_transaction_mut");
                    *in_trans = true;
                    Value::Bool(true)
                }
                Err(error) => Value::Error(error),
            }
        }

        "db_commit" => match arg_values.first().cloned() {
            _ if arg_values.len() > 1 => Value::Error(
                "db_commit requires a database connection as first argument".to_string(),
            ),
            Some(Value::Database { connection, db_type, in_transaction, .. }) => {
                {
                    let in_trans = lock_or_db_error!(in_transaction, "database.in_transaction");
                    if !*in_trans {
                        return Some(Value::Error(
                            "No transaction in progress. Use db_begin() first.".to_string(),
                        ));
                    }
                }

                let result = match (connection, db_type.as_str()) {
                    (DatabaseConnection::Sqlite(connection), "sqlite") => {
                        let connection = lock_or_db_error!(connection, "database.connection");
                        connection
                            .execute("COMMIT", [])
                            .map(|_| ())
                            .map_err(|e| format!("Failed to commit transaction: {}", e))
                    }
                    (DatabaseConnection::Postgres(client), "postgres") => {
                        let mut client = lock_or_db_error!(client, "database.client_mut");
                        client
                            .execute("COMMIT", &[])
                            .map(|_| ())
                            .map_err(|e| format!("Failed to commit transaction: {}", e))
                    }
                    (DatabaseConnection::Mysql(connection), "mysql") => {
                        let mut connection =
                            lock_or_db_error!(connection, "database.connection_mut");
                        match create_runtime() {
                            Ok(runtime) => runtime
                                .block_on(async {
                                    connection.exec_drop("COMMIT", mysql_async::Params::Empty).await
                                })
                                .map(|_| ())
                                .map_err(|e| format!("Failed to commit transaction: {}", e)),
                            Err(error) => Err(error),
                        }
                    }
                    _ => Err("Invalid database connection".to_string()),
                };

                match result {
                    Ok(()) => {
                        let mut in_trans =
                            lock_or_db_error!(in_transaction, "database.in_transaction_mut");
                        *in_trans = false;
                        Value::Bool(true)
                    }
                    Err(error) => Value::Error(error),
                }
            }
            _ => Value::Error(
                "db_commit requires a database connection as first argument".to_string(),
            ),
        },

        "db_rollback" => match arg_values.first().cloned() {
            _ if arg_values.len() > 1 => Value::Error(
                "db_rollback requires a database connection as first argument".to_string(),
            ),
            Some(Value::Database { connection, db_type, in_transaction, .. }) => {
                {
                    let in_trans = lock_or_db_error!(in_transaction, "database.in_transaction");
                    if !*in_trans {
                        return Some(Value::Error(
                            "No transaction in progress. Use db_begin() first.".to_string(),
                        ));
                    }
                }

                let result = match (connection, db_type.as_str()) {
                    (DatabaseConnection::Sqlite(connection), "sqlite") => {
                        let connection = lock_or_db_error!(connection, "database.connection");
                        connection
                            .execute("ROLLBACK", [])
                            .map(|_| ())
                            .map_err(|e| format!("Failed to rollback transaction: {}", e))
                    }
                    (DatabaseConnection::Postgres(client), "postgres") => {
                        let mut client = lock_or_db_error!(client, "database.client_mut");
                        client
                            .execute("ROLLBACK", &[])
                            .map(|_| ())
                            .map_err(|e| format!("Failed to rollback transaction: {}", e))
                    }
                    (DatabaseConnection::Mysql(connection), "mysql") => {
                        let mut connection =
                            lock_or_db_error!(connection, "database.connection_mut");
                        match create_runtime() {
                            Ok(runtime) => runtime
                                .block_on(async {
                                    connection
                                        .exec_drop("ROLLBACK", mysql_async::Params::Empty)
                                        .await
                                })
                                .map(|_| ())
                                .map_err(|e| format!("Failed to rollback transaction: {}", e)),
                            Err(error) => Err(error),
                        }
                    }
                    _ => Err("Invalid database connection".to_string()),
                };

                match result {
                    Ok(()) => {
                        let mut in_trans =
                            lock_or_db_error!(in_transaction, "database.in_transaction_mut");
                        *in_trans = false;
                        Value::Bool(true)
                    }
                    Err(error) => Value::Error(error),
                }
            }
            _ => Value::Error(
                "db_rollback requires a database connection as first argument".to_string(),
            ),
        },

        "db_last_insert_id" => {
            if arg_values.len() > 1 {
                return Some(Value::Error(
                    "db_last_insert_id requires a database connection as first argument"
                        .to_string(),
                ));
            }

            if let Some(Value::Database { connection, db_type, .. }) = arg_values.first() {
                match (connection, db_type.as_str()) {
                    (DatabaseConnection::Sqlite(connection), "sqlite") => {
                        let connection = lock_or_db_error!(connection, "database.connection");
                        Value::Float(connection.last_insert_rowid() as f64)
                    }
                    (DatabaseConnection::Postgres(client), "postgres") => {
                        let mut client = lock_or_db_error!(client, "database.client_mut");
                        match client.query("SELECT lastval()", &[]) {
                            Ok(rows) => {
                                if let Some(row) = rows.first() {
                                    let id: i64 = row.get(0);
                                    Value::Int(id)
                                } else {
                                    Value::Error("No last insert ID available".to_string())
                                }
                            }
                            Err(error) => Value::Error(format!(
                                "Failed to get last insert ID: {}. Use RETURNING clause instead.",
                                error
                            )),
                        }
                    }
                    (DatabaseConnection::Mysql(connection), "mysql") => {
                        let mut connection =
                            lock_or_db_error!(connection, "database.connection_mut");
                        match create_runtime() {
                            Ok(runtime) => match runtime.block_on(async {
                                connection.query_first::<u64, _>("SELECT LAST_INSERT_ID()").await
                            }) {
                                Ok(Some(id)) => Value::Int(id as i64),
                                Ok(None) => Value::Error("No last insert ID available".to_string()),
                                Err(error) => {
                                    Value::Error(format!("Failed to get last insert ID: {}", error))
                                }
                            },
                            Err(error) => Value::Error(error),
                        }
                    }
                    _ => Value::Error("Invalid database connection".to_string()),
                }
            } else {
                Value::Error(
                    "db_last_insert_id requires a database connection as first argument"
                        .to_string(),
                )
            }
        }

        _ => return None,
    };

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_db_path(file_name: &str) -> String {
        let mut path = std::env::current_dir().expect("current_dir should resolve");
        path.push("tmp");
        path.push("native_db_tests");
        std::fs::create_dir_all(&path).expect("db tmp dir should be created");
        path.push(file_name);
        path.to_string_lossy().to_string()
    }

    fn str_value(value: &str) -> Value {
        Value::Str(Arc::new(value.to_string()))
    }

    #[test]
    fn test_db_connect_execute_query_close_sqlite() {
        let db_path = tmp_db_path("sqlite_basic.db");

        let db = handle("db_connect", &[str_value("sqlite"), str_value(&db_path)]).unwrap();
        assert!(matches!(db, Value::Database { .. }));

        let create_result = handle(
            "db_execute",
            &[
                db.clone(),
                str_value("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT)"),
            ],
        )
        .unwrap();
        assert!(matches!(create_result, Value::Int(_)));

        let insert_result = handle(
            "db_execute",
            &[
                db.clone(),
                str_value("INSERT INTO users (name) VALUES (?)"),
                Value::Array(Arc::new(vec![str_value("alice")])),
            ],
        )
        .unwrap();
        assert!(matches!(insert_result, Value::Int(1)));

        let query_result =
            handle("db_query", &[db.clone(), str_value("SELECT name FROM users ORDER BY id")])
                .unwrap();
        assert!(matches!(query_result, Value::Array(rows) if rows.len() == 1));

        let close_result = handle("db_close", &[db]).unwrap();
        assert!(matches!(close_result, Value::Bool(true)));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn test_db_transaction_begin_commit_and_rollback_sqlite() {
        let db_path = tmp_db_path("sqlite_tx.db");
        let db = handle("db_connect", &[str_value("sqlite"), str_value(&db_path)]).unwrap();

        let _ = handle(
            "db_execute",
            &[
                db.clone(),
                str_value(
                    "CREATE TABLE IF NOT EXISTS tx_items (id INTEGER PRIMARY KEY, value TEXT)",
                ),
            ],
        )
        .unwrap();

        let begin = handle("db_begin", &[db.clone()]).unwrap();
        assert!(matches!(begin, Value::Bool(true)));

        let _ = handle(
            "db_execute",
            &[db.clone(), str_value("INSERT INTO tx_items (value) VALUES ('rollback')")],
        )
        .unwrap();

        let rollback = handle("db_rollback", &[db.clone()]).unwrap();
        assert!(matches!(rollback, Value::Bool(true)));

        let count_after_rollback =
            handle("db_query", &[db.clone(), str_value("SELECT COUNT(*) as count FROM tx_items")])
                .unwrap();
        assert!(matches!(count_after_rollback, Value::Array(rows) if !rows.is_empty()));

        let begin_again = handle("db_begin", &[db.clone()]).unwrap();
        assert!(matches!(begin_again, Value::Bool(true)));

        let _ = handle(
            "db_execute",
            &[db.clone(), str_value("INSERT INTO tx_items (value) VALUES ('commit')")],
        )
        .unwrap();

        let commit = handle("db_commit", &[db.clone()]).unwrap();
        assert!(matches!(commit, Value::Bool(true)));

        let last_id = handle("db_last_insert_id", &[db.clone()]).unwrap();
        assert!(matches!(last_id, Value::Float(_) | Value::Int(_)));

        let close_result = handle("db_close", &[db]).unwrap();
        assert!(matches!(close_result, Value::Bool(true)));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn test_db_pool_acquire_release_stats_close_sqlite() {
        let db_path = tmp_db_path("sqlite_pool.db");

        let pool = handle("db_pool", &[str_value("sqlite"), str_value(&db_path)]).unwrap();
        assert!(matches!(pool, Value::DatabasePool { .. }));

        let connection = handle("db_pool_acquire", &[pool.clone()]).unwrap();
        assert!(matches!(connection, Value::Database { .. }));

        let release = handle("db_pool_release", &[pool.clone(), connection]).unwrap();
        assert!(matches!(release, Value::Bool(true)));

        let stats = handle("db_pool_stats", &[pool.clone()]).unwrap();
        assert!(matches!(stats, Value::Dict(_)));

        let close = handle("db_pool_close", &[pool]).unwrap();
        assert!(matches!(close, Value::Bool(true)));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn test_db_pool_accepts_fixed_dict_optional_config() {
        let db_path = tmp_db_path("sqlite_pool_fixed_dict.db");
        let fixed_config = Value::FixedDict { keys: Arc::new(vec![]), values: vec![] };

        let pool = handle("db_pool", &[str_value("sqlite"), str_value(&db_path), fixed_config])
            .expect("db_pool should return a value");
        assert!(matches!(pool, Value::DatabasePool { .. }));

        let close = handle("db_pool_close", &[pool]).expect("db_pool_close should return a value");
        assert!(matches!(close, Value::Bool(true)));

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn test_db_argument_shape_errors() {
        let execute_error = handle("db_execute", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(execute_error, Value::Error(message) if message.contains("db_execute requires a database connection"))
        );

        let query_error = handle("db_query", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(query_error, Value::Error(message) if message.contains("db_query requires a database connection"))
        );

        let begin_error = handle("db_begin", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(begin_error, Value::Error(message) if message.contains("db_begin requires a database connection"))
        );

        let pool_error = handle("db_pool", &[Value::Int(1)]).unwrap();
        assert!(
            matches!(pool_error, Value::Error(message) if message.contains("db_pool requires database type and connection string"))
        );

        let pool_invalid_config =
            handle("db_pool", &[str_value("sqlite"), str_value(":memory:"), Value::Int(3)])
                .unwrap();
        assert!(matches!(
            pool_invalid_config,
            Value::Error(message)
                if message.contains("db_pool optional config argument must be a dictionary")
        ));
    }

    #[test]
    fn test_db_strict_arity_rejects_extra_arguments() {
        let db_connect_extra =
            handle("db_connect", &[str_value("sqlite"), str_value(":memory:"), Value::Int(1)])
                .unwrap();
        assert!(matches!(
            db_connect_extra,
            Value::Error(message)
                if message
                    .contains("db_connect requires database type ('sqlite'|'postgres'|'mysql') and connection string")
        ));

        let db_execute_extra = handle(
            "db_execute",
            &[Value::Int(1), str_value("SELECT 1"), Value::Array(Arc::new(vec![])), Value::Int(99)],
        )
        .unwrap();
        assert!(matches!(
            db_execute_extra,
            Value::Error(message)
                if message.contains("db_execute requires 2 or 3 arguments")
        ));

        let db_query_extra = handle(
            "db_query",
            &[Value::Int(1), str_value("SELECT 1"), Value::Array(Arc::new(vec![])), Value::Int(99)],
        )
        .unwrap();
        assert!(matches!(
            db_query_extra,
            Value::Error(message)
                if message.contains("db_query requires 2 or 3 arguments")
        ));

        let db_pool_extra = handle(
            "db_pool",
            &[
                str_value("sqlite"),
                str_value(":memory:"),
                Value::Dict(Arc::new(DictMap::default())),
                Value::Int(1),
            ],
        )
        .unwrap();
        assert!(matches!(
            db_pool_extra,
            Value::Error(message)
                if message.contains("db_pool requires 2 or 3 arguments")
        ));

        for (name, args, expected) in vec![
            (
                "db_close",
                vec![Value::Int(1), Value::Int(2)],
                "db_close requires a database connection",
            ),
            (
                "db_pool_acquire",
                vec![Value::Int(1), Value::Int(2)],
                "db_pool_acquire requires a database pool",
            ),
            (
                "db_pool_release",
                vec![Value::Int(1), Value::Int(2), Value::Int(3)],
                "db_pool_release requires two arguments",
            ),
            (
                "db_pool_stats",
                vec![Value::Int(1), Value::Int(2)],
                "db_pool_stats requires a database pool",
            ),
            (
                "db_pool_close",
                vec![Value::Int(1), Value::Int(2)],
                "db_pool_close requires a database pool",
            ),
            (
                "db_begin",
                vec![Value::Int(1), Value::Int(2)],
                "db_begin requires a database connection as first argument",
            ),
            (
                "db_commit",
                vec![Value::Int(1), Value::Int(2)],
                "db_commit requires a database connection as first argument",
            ),
            (
                "db_rollback",
                vec![Value::Int(1), Value::Int(2)],
                "db_rollback requires a database connection as first argument",
            ),
            (
                "db_last_insert_id",
                vec![Value::Int(1), Value::Int(2)],
                "db_last_insert_id requires a database connection as first argument",
            ),
        ] {
            let result = handle(name, &args).unwrap();
            assert!(
                matches!(result, Value::Error(ref message) if message.contains(expected)),
                "Expected strict-arity contract for {} to contain '{}', got {:?}",
                name,
                expected,
                result
            );
        }
    }
}
