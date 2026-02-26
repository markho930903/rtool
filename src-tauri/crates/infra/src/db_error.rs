use app_core::AppError;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct DbAppError {
    inner: AppError,
}

impl DbAppError {
    pub fn into_inner(self) -> AppError {
        self.inner
    }
}

impl Display for DbAppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl StdError for DbAppError {}

impl From<AppError> for DbAppError {
    fn from(value: AppError) -> Self {
        Self { inner: value }
    }
}

impl From<anyhow::Error> for DbAppError {
    fn from(value: anyhow::Error) -> Self {
        Self {
            inner: AppError::from(value),
        }
    }
}

impl From<libsql::Error> for DbAppError {
    fn from(value: libsql::Error) -> Self {
        Self {
            inner: AppError::new("db_error", "数据库操作失败").with_source(value),
        }
    }
}

impl From<DbAppError> for AppError {
    fn from(value: DbAppError) -> Self {
        value.into_inner()
    }
}

pub type DbResult<T> = Result<T, DbAppError>;
