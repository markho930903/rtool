#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestContext {
    request_id: String,
    window_label: Option<String>,
}

impl RequestContext {
    pub fn new(request_id: Option<String>, window_label: Option<String>) -> Self {
        let request_id = request_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "unknown".to_string());

        let window_label = window_label
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        Self {
            request_id,
            window_label,
        }
    }

    pub fn request_id(&self) -> &str {
        self.request_id.as_str()
    }

    pub fn window_label(&self) -> Option<&str> {
        self.window_label.as_deref()
    }

    pub fn into_parts(self) -> (String, Option<String>) {
        (self.request_id, self.window_label)
    }
}
