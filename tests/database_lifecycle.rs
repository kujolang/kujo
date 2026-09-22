#![cfg(feature = "runtime-db")]

use kujo::interpreter::database_handle::DatabaseHandle;
use kujo::interpreter::{ConnectionPool, DatabaseConnection};
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Barrier};

#[test]
fn sqlite_close_releases_file_and_rolls_back_with_retained_aliases() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("retained.sqlite");
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute_batch("CREATE TABLE items(value); BEGIN IMMEDIATE; INSERT INTO items VALUES(1)")
        .unwrap();
    let connection = DatabaseConnection::Sqlite(Arc::new(DatabaseHandle::new(raw)));
    let alias = connection.clone();
    connection.close().unwrap();
    assert_eq!(alias.ensure_open().unwrap_err(), "database connection is closed");
    alias.close().unwrap();
    let reopened = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        reopened.query_row("SELECT count(*) FROM items", [], |row| row.get::<_, i64>(0)).unwrap(),
        0
    );
    reopened.close().unwrap();
    // Windows refuses this if either retained alias still owns the native file.
    std::fs::remove_file(&path).unwrap();
    assert!(alias.ensure_open().is_err());
}

#[test]
fn returned_alias_cannot_close_or_query_next_pool_lease() {
    let pool = ConnectionPool::new("sqlite".into(), ":memory:".into(), HashMap::new()).unwrap();
    let first = pool.acquire().unwrap();
    let alias = first.clone();
    pool.release(first).unwrap();
    let second = pool.acquire().unwrap();
    assert!(alias.ensure_open().is_err());
    alias.close().unwrap();
    second.ensure_open().unwrap();
    assert!(pool.release(alias).is_err());
    second.close().unwrap();
    pool.release(second).unwrap();
    assert_eq!(pool.stats()["in_use"], 0);
    let replacement = pool.acquire().unwrap();
    replacement.ensure_open().unwrap();
    pool.close();
    // Closing the pool does not interrupt existing borrowers.
    replacement.ensure_open().unwrap();
    pool.release(replacement.clone()).unwrap();
    assert!(replacement.ensure_open().is_err());
    assert_eq!(pool.stats()["total"], 0);
}

#[test]
fn simultaneous_close_and_query_are_serialized() {
    let connection = DatabaseConnection::Sqlite(Arc::new(DatabaseHandle::new(
        rusqlite::Connection::open_in_memory().unwrap(),
    )));
    let barrier = Arc::new(Barrier::new(9));
    std::thread::scope(|scope| {
        for index in 0..8 {
            let connection = connection.clone();
            let barrier = barrier.clone();
            scope.spawn(move || {
                barrier.wait();
                if index % 2 == 0 {
                    connection.close().unwrap();
                } else if let DatabaseConnection::Sqlite(handle) = connection {
                    match handle.lock() {
                        Ok(db) => assert_eq!(
                            db.query_row("SELECT 1", [], |r| r.get::<_, i64>(0)).unwrap(),
                            1
                        ),
                        Err(error) => {
                            assert_eq!(error.message("poisoned"), "database connection is closed")
                        }
                    }
                }
            });
        }
        barrier.wait();
    });
    assert!(connection.ensure_open().is_err());
}

#[test]
fn language_database_lifecycle_matches_vm_and_interpreter() {
    for interpreter in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args([
            "run",
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/database_lifecycle.kujo"),
        ]);
        if interpreter {
            command.arg("--interpreter");
        }
        let output = command.current_dir(dir.path()).output().unwrap();
        assert!(
            output.status.success(),
            "interpreter={interpreter}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "database lifecycle: passed");
    }
}
