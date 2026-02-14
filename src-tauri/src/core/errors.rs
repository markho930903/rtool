use serde::Serialize;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl StdError for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(value: rusqlite::Error) -> Self {
        AppError::new("db_error", "数据库操作失败").with_detail(value.to_string())
    }
}

impl From<r2d2::Error> for AppError {
    fn from(value: r2d2::Error) -> Self {
        AppError::new("db_pool_error", "数据库连接池操作失败").with_detail(value.to_string())
    }
}

impl From<arboard::Error> for AppError {
    fn from(value: arboard::Error) -> Self {
        AppError::new("clipboard_error", "剪贴板操作失败").with_detail(value.to_string())
    }
}

impl From<Box<dyn StdError + Send + Sync + 'static>> for AppError {
    fn from(value: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        AppError::new("clipboard_error", "剪贴板操作失败").with_detail(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
