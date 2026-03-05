use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InvokeMeta {
    pub request_id: Option<String>,
    pub window_label: Option<String>,
}

impl InvokeMeta {
    pub fn split(self) -> (Option<String>, Option<String>) {
        (self.request_id, self.window_label)
    }
}
