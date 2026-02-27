use serde::Serialize;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};

const DEFAULT_CODE: &str = "internal_error";
const DEFAULT_MESSAGE: &str = "操作失败";
const RELEASE_REDACTED_CAUSE: &str = "错误详情已隐藏，请查看日志";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorContextItem {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorPayload {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ErrorContextItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct AppError(Box<AppErrorPayload>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvokeError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ErrorContextItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl InvokeError {
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        let request_id = request_id.into();
        if !request_id.trim().is_empty() {
            self.request_id = Some(request_id);
        }
        self
    }

    pub fn from_anyhow(error: anyhow::Error) -> Self {
        if let Some(app_error) = error.downcast_ref::<AppError>() {
            return Self::from(app_error.clone());
        }

        let causes = visible_causes_for_mode(collect_error_chain(&error));
        Self {
            code: DEFAULT_CODE.to_string(),
            message: DEFAULT_MESSAGE.to_string(),
            context: Vec::new(),
            causes,
            request_id: None,
        }
    }
}

#[allow(dead_code)]
impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self(Box::new(AppErrorPayload {
            code: code.into(),
            message: message.into(),
            context: Vec::new(),
            causes: Vec::new(),
            request_id: None,
        }))
    }

    pub fn with_code(mut self, code: impl Into<String>, message: impl Into<String>) -> Self {
        self.0.code = code.into();
        self.0.message = message.into();
        self
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.0.context.push(ErrorContextItem {
            key: key.into(),
            value: value.into(),
        });
        self
    }

    pub fn with_cause(mut self, cause: impl Into<String>) -> Self {
        let cause = cause.into();
        if !cause.trim().is_empty() {
            self.0.causes.push(cause);
        }
        self
    }

    pub fn with_anyhow_source(mut self, error: anyhow::Error) -> Self {
        let chain = collect_error_chain(&error);
        if !chain.is_empty() {
            self.put_context_if_absent("sourceChainDepth", chain.len().to_string());
        }
        self.with_causes(chain)
    }

    pub fn with_source<E>(mut self, error: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        self.put_context_if_absent("sourceType", std::any::type_name::<E>().to_string());
        let chain = collect_std_error_chain(&error);
        if !chain.is_empty() {
            self.put_context_if_absent("sourceChainDepth", chain.len().to_string());
        }
        self.with_causes(chain)
    }

    pub fn with_boxed_source(mut self, error: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        self.put_context_if_absent("sourceType", "dyn StdError".to_string());
        let chain = collect_std_error_chain(error.as_ref());
        if !chain.is_empty() {
            self.put_context_if_absent("sourceChainDepth", chain.len().to_string());
        }
        self.with_causes(chain)
    }

    pub fn with_causes<I, S>(mut self, causes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for cause in causes {
            self = self.with_cause(cause);
        }
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        let request_id = request_id.into();
        if !request_id.trim().is_empty() {
            self.0.request_id = Some(request_id);
        }
        self
    }

    pub fn from_anyhow(error: anyhow::Error) -> Self {
        if let Some(app_error) = error.downcast_ref::<Self>() {
            return app_error.clone();
        }

        let causes = visible_causes_for_mode(collect_error_chain(&error));
        Self(Box::new(AppErrorPayload {
            code: DEFAULT_CODE.to_string(),
            message: DEFAULT_MESSAGE.to_string(),
            context: Vec::new(),
            causes,
            request_id: None,
        }))
    }

    pub fn from_error<E>(error: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::from_anyhow(anyhow::Error::new(error))
    }

    fn put_context_if_absent(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        if self.0.context.iter().any(|item| item.key == key) {
            return;
        }
        self.0.context.push(ErrorContextItem {
            key,
            value: value.into(),
        });
    }
}

impl Deref for AppError {
    type Target = AppErrorPayload;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl DerefMut for AppError {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

fn collect_error_chain(error: &anyhow::Error) -> Vec<String> {
    let mut causes = Vec::new();
    for cause in error.chain() {
        let text = cause.to_string();
        if text.trim().is_empty() {
            continue;
        }

        if causes.last().is_some_and(|last| last == &text) {
            continue;
        }
        causes.push(text);
    }
    causes
}

fn collect_std_error_chain(error: &(dyn StdError + 'static)) -> Vec<String> {
    let mut causes = Vec::new();
    let mut current: Option<&(dyn StdError + 'static)> = Some(error);
    while let Some(cause) = current {
        let text = cause.to_string();
        if !text.trim().is_empty() && causes.last().is_none_or(|last| last != &text) {
            causes.push(text);
        }
        current = cause.source();
    }
    causes
}

fn visible_causes_for_mode(causes: Vec<String>) -> Vec<String> {
    if cfg!(debug_assertions) {
        return causes;
    }

    match causes.into_iter().next() {
        Some(first) => vec![sanitize_cause_for_release(&first)],
        None => Vec::new(),
    }
}

fn sanitize_cause_for_release(cause: &str) -> String {
    let normalized = cause.replace('\n', " ").trim().to_string();
    if normalized.is_empty() {
        return RELEASE_REDACTED_CAUSE.to_string();
    }

    let lower = normalized.to_ascii_lowercase();
    if lower.contains("token")
        || lower.contains("password")
        || lower.contains("secret")
        || lower.contains("apikey")
    {
        return RELEASE_REDACTED_CAUSE.to_string();
    }

    if normalized.contains('/')
        || normalized.contains('\\')
        || normalized.contains(" - ")
        || normalized.len() > 220
    {
        return RELEASE_REDACTED_CAUSE.to_string();
    }

    normalized
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl StdError for AppError {}

impl Display for InvokeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl StdError for InvokeError {}

impl From<anyhow::Error> for AppError {
    fn from(value: anyhow::Error) -> Self {
        Self::from_anyhow(value)
    }
}

impl From<Box<dyn StdError + Send + Sync + 'static>> for AppError {
    fn from(value: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        Self::new("clipboard_error", "剪贴板操作失败").with_boxed_source(value)
    }
}

impl From<AppError> for InvokeError {
    fn from(value: AppError) -> Self {
        let AppErrorPayload {
            code,
            message,
            context,
            causes,
            request_id,
        } = *value.0;

        Self {
            code,
            message,
            context,
            causes: visible_causes_for_mode(causes),
            request_id,
        }
    }
}

impl From<anyhow::Error> for InvokeError {
    fn from(value: anyhow::Error) -> Self {
        Self::from_anyhow(value)
    }
}

impl From<&anyhow::Error> for InvokeError {
    fn from(value: &anyhow::Error) -> Self {
        if let Some(app_error) = value.downcast_ref::<AppError>() {
            return Self::from(app_error.clone());
        }

        Self {
            code: DEFAULT_CODE.to_string(),
            message: DEFAULT_MESSAGE.to_string(),
            context: Vec::new(),
            causes: visible_causes_for_mode(collect_error_chain(value)),
            request_id: None,
        }
    }
}

#[allow(dead_code)]
pub trait ResultExt<T> {
    fn with_code(self, code: impl Into<String>, message: impl Into<String>) -> AppResult<T>;
    fn with_ctx(self, key: impl Into<String>, value: impl Into<String>) -> AppResult<T>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn with_code(self, code: impl Into<String>, message: impl Into<String>) -> AppResult<T> {
        let code = code.into();
        let message = message.into();
        self.map_err(|error| AppError::from_anyhow(error.into()).with_code(code, message))
    }

    fn with_ctx(self, key: impl Into<String>, value: impl Into<String>) -> AppResult<T> {
        let key = key.into();
        let value = value.into();
        self.map_err(|error| AppError::from_anyhow(error.into()).with_context(key, value))
    }
}

pub type AppResult<T> = Result<T, AppError>;

