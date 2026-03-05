use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SerperRequest {
    q: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    num: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SerperResponse {
    pub organic: Option<Vec<OrganicResult>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrganicResult {
    pub title: String,
    pub link: String,
    pub snippet: Option<String>,
}

/// Search for indexed URLs using site: operator
pub async fn search_indexed_urls(
    client: &Client,
    api_key: &str,
    domain: &str,
    page: u32,
    num: u32,
) -> Result<SerperResponse, String> {
    let res = client
        .post("https://google.serper.dev/search")
        .header("X-API-KEY", api_key)
        .header("Content-Type", "application/json")
        .json(&SerperRequest {
            q: format!("site:{}", domain),
            num: Some(num),
            page: Some(page),
        })
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Serper API returned status {}", res.status()));
    }

    res.json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

/// Check if a specific URL is indexed by Google
pub async fn check_url_indexed(client: &Client, api_key: &str, url: &str) -> Result<bool, String> {
    let res = client
        .post("https://google.serper.dev/search")
        .header("X-API-KEY", api_key)
        .header("Content-Type", "application/json")
        .json(&SerperRequest {
            q: format!("site:{}", url),
            num: Some(10),
            page: None,
        })
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !res.status().is_success() {
        return Err(format!("Serper API returned status {}", res.status()));
    }

    let response: SerperResponse = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    // URL is indexed if Google returns any results containing it
    Ok(response.organic.map_or(false, |results| {
        results
            .iter()
            .any(|r| url.contains(&r.link) || r.link.contains(url))
    }))
}
