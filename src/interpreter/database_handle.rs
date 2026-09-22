//! Shared database ownership. Closing or returning a lease invalidates every alias.
use super::async_runtime::AsyncRuntime;
use super::DatabaseConnection;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct DatabaseHandle<T>(Mutex<Option<T>>);

#[derive(Debug)]
pub enum DatabaseHandleError {
    Closed,
    Poisoned,
}
impl DatabaseHandleError {
    pub fn message(&self, poison_message: &str) -> String {
        match self {
            Self::Closed => "database connection is closed".to_string(),
            Self::Poisoned => poison_message.to_string(),
        }
    }
}

pub struct DatabaseGuard<'a, T>(MutexGuard<'a, Option<T>>);
impl<T> DatabaseHandle<T> {
    pub fn new(value: T) -> Self {
        Self(Mutex::new(Some(value)))
    }
    pub fn lock(&self) -> Result<DatabaseGuard<'_, T>, DatabaseHandleError> {
        let guard = self.0.lock().map_err(|_| DatabaseHandleError::Poisoned)?;
        if guard.is_none() {
            return Err(DatabaseHandleError::Closed);
        }
        Ok(DatabaseGuard(guard))
    }
    fn slot(&self) -> Result<MutexGuard<'_, Option<T>>, String> {
        self.0.lock().map_err(|_| "database connection lock poisoned".to_string())
    }
    fn detach(&self) -> Result<Option<Arc<Self>>, String> {
        Ok(self.slot()?.take().map(|value| Arc::new(Self::new(value))))
    }
}
impl<T> Deref for DatabaseGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().expect("guard holds an open connection")
    }
}
impl<T> DerefMut for DatabaseGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_mut().expect("guard holds an open connection")
    }
}

impl DatabaseConnection {
    pub fn ensure_open(&self) -> Result<(), String> {
        match self {
            Self::Sqlite(c) => c.lock().map(|_| ()),
            Self::Postgres(c) => c.lock().map(|_| ()),
            Self::Mysql(c) => c.lock().map(|_| ()),
        }
        .map_err(|e| e.message("database connection lock poisoned"))
    }
    pub fn close(&self) -> Result<(), String> {
        // Keep the slot locked until release completes: another close cannot
        // acknowledge success while the native resource is still being closed.
        match self {
            Self::Sqlite(c) => {
                let mut slot = c.slot()?;
                if let Some(connection) = slot.take() {
                    if let Err((connection, error)) = connection.close() {
                        *slot = Some(connection);
                        return Err(format!("Failed to close SQLite connection: {error}"));
                    }
                }
                Ok(())
            }
            Self::Postgres(c) => {
                let mut slot = c.slot()?;
                if let Some(client) = slot.take() {
                    client.close().map_err(|error| {
                        format!("Failed to close PostgreSQL connection: {error}")
                    })?;
                }
                Ok(())
            }
            Self::Mysql(c) => {
                let mut slot = c.slot()?;
                if let Some(connection) = slot.take() {
                    AsyncRuntime::block_on(connection.disconnect())
                        .map_err(|error| format!("Failed to close MySQL connection: {error}"))?;
                }
                Ok(())
            }
        }
    }
    pub(crate) fn detach(&self) -> Result<Option<Self>, String> {
        match self {
            Self::Sqlite(c) => c.detach().map(|c| c.map(Self::Sqlite)),
            Self::Postgres(c) => c.detach().map(|c| c.map(Self::Postgres)),
            Self::Mysql(c) => c.detach().map(|c| c.map(Self::Mysql)),
        }
    }
}
