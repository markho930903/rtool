use protocol::{AppError, AppResult, InvokeError};
use serde::Deserialize;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CommandRequestDto {
    pub kind: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Copy)]
pub struct CommandPayloadContext {
    domain: &'static str,
    payload_error_message: &'static str,
    response_error_message: &'static str,
    unknown_command_message: &'static str,
}

impl CommandPayloadContext {
    pub const fn new(
        domain: &'static str,
        payload_error_message: &'static str,
        response_error_message: &'static str,
        unknown_command_message: &'static str,
    ) -> Self {
        Self {
            domain,
            payload_error_message,
            response_error_message,
            unknown_command_message,
        }
    }

    pub fn parse<T>(self, kind: &str, payload: Value) -> AppResult<T>
    where
        T: DeserializeOwned,
    {
        parse_command_payload(self.domain, kind, payload, self.payload_error_message)
    }

    pub fn serialize<T>(self, kind: &str, value: T) -> Result<Value, InvokeError>
    where
        T: Serialize,
    {
        serialize_command_response(self.domain, kind, value, self.response_error_message)
    }

    pub fn unknown(self, kind: String) -> InvokeError {
        unknown_command_error(self.domain, kind, self.unknown_command_message)
    }
}

pub fn parse_command_payload<T>(
    domain: &str,
    kind: &str,
    payload: Value,
    message: &str,
) -> AppResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value::<T>(payload).map_err(|error| {
        AppError::new(format!("{domain}_command_payload_invalid"), message)
            .with_context("kind", kind.to_string())
            .with_source(error)
    })
}

pub fn serialize_command_response<T>(
    domain: &str,
    kind: &str,
    value: T,
    message: &str,
) -> Result<Value, InvokeError>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(|error| {
        AppError::new(format!("{domain}_command_response_invalid"), message)
            .with_context("kind", kind.to_string())
            .with_source(error)
            .into()
    })
}

pub fn unknown_command_error(domain: &str, kind: String, message: &str) -> InvokeError {
    AppError::new(format!("{domain}_command_not_found"), message)
        .with_context("kind", kind)
        .into()
}
