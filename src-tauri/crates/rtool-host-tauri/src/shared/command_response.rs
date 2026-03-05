use rtool_contracts::{AppError, InvokeError};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub struct CommandPayloadContext {
    domain: &'static str,
    response_error_message: &'static str,
}

impl CommandPayloadContext {
    pub const fn new(
        domain: &'static str,
        _payload_error_message: &'static str,
        response_error_message: &'static str,
        _unknown_command_message: &'static str,
    ) -> Self {
        Self {
            domain,
            response_error_message,
        }
    }

    pub fn serialize<T>(self, kind: &str, value: T) -> Result<Value, InvokeError>
    where
        T: Serialize,
    {
        serialize_command_response(self.domain, kind, value, self.response_error_message)
    }
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
