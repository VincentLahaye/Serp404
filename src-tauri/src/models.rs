use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub domain: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlEntry {
    pub id: String,
    pub project_id: String,
    pub url: String,
    pub source: String,
    pub indexed_status: String,
    pub http_status: Option<i32>,
    pub response_time_ms: Option<i64>,
    pub title: Option<String>,
    pub redirect_chain: Option<String>,
    pub error: Option<String>,
    pub checked_at: Option<String>,
}
