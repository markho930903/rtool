#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub window_label: String,
}

impl RequestContext {
    pub fn new(request_id: Option<String>, window_label: Option<String>) -> Self {
        Self {
            request_id: request_id
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "unknown".to_string()),
            window_label: window_label
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "unknown".to_string()),
        }
    }

    pub fn request_id(&self) -> &str {
        self.request_id.as_str()
    }

    pub fn window_label(&self) -> Option<&str> {
        if self.window_label == "unknown" {
            return None;
        }
        Some(self.window_label.as_str())
    }
}
