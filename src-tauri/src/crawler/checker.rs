use reqwest::redirect::Policy;
use reqwest::Client;
use serde::Serialize;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub url: String,
    pub http_status: i32,
    pub response_time_ms: i64,
    pub title: Option<String>,
    pub redirect_chain: Vec<String>,
    pub error: Option<String>,
}

/// Build a client that does NOT follow redirects automatically
pub fn build_no_redirect_client(timeout_secs: u64) -> Client {
    Client::builder()
        .redirect(Policy::none())
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .user_agent("Serp404/1.0")
        .build()
        .unwrap()
}

/// Check a single URL's health: status code, response time, redirects, title
pub async fn check_url(client: &Client, url: &str) -> CheckResult {
    let start = Instant::now();
    let mut current_url = url.to_string();
    let mut redirect_chain: Vec<String> = Vec::new();
    let mut final_status = 0i32;
    let mut final_error: Option<String> = None;
    let max_redirects = 10;

    for _ in 0..max_redirects {
        match client.get(&current_url).send().await {
            Ok(response) => {
                let status = response.status().as_u16() as i32;
                final_status = status;

                if (300..400).contains(&status) {
                    // Redirect: record hop and follow Location header
                    if let Some(location) = response.headers().get("location") {
                        let next_url = location.to_str().unwrap_or("").to_string();
                        redirect_chain.push(current_url.clone());
                        // Handle relative URLs
                        if next_url.starts_with('/') {
                            if let Ok(base) = url::Url::parse(&current_url) {
                                current_url = format!(
                                    "{}://{}{}",
                                    base.scheme(),
                                    base.host_str().unwrap_or(""),
                                    next_url
                                );
                            } else {
                                current_url = next_url;
                            }
                        } else {
                            current_url = next_url;
                        }
                        continue;
                    }
                }

                // Not a redirect or no Location header: this is our final response
                let response_time = start.elapsed().as_millis() as i64;

                // Extract title from HTML if status is 200
                let title = if status == 200 {
                    if let Ok(body) = response.text().await {
                        extract_title(&body)
                    } else {
                        None
                    }
                } else {
                    None
                };

                return CheckResult {
                    url: url.to_string(),
                    http_status: final_status,
                    response_time_ms: response_time,
                    title,
                    redirect_chain,
                    error: None,
                };
            }
            Err(e) => {
                final_error = Some(e.to_string());
                break;
            }
        }
    }

    // If we got here: either max redirects reached or error
    if redirect_chain.len() >= max_redirects {
        final_error = Some("Too many redirects (possible redirect loop)".to_string());
    }

    CheckResult {
        url: url.to_string(),
        http_status: final_status,
        response_time_ms: start.elapsed().as_millis() as i64,
        title: None,
        redirect_chain,
        error: final_error,
    }
}

/// Extract <title> from HTML using simple string search (no heavy parser)
fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title>")?;
    let end = lower.find("</title>")?;
    if end > start + 7 {
        let title = html[start + 7..end].trim().to_string();
        if title.is_empty() {
            None
        } else {
            Some(title)
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_title_basic() {
        let html = "<html><head><title>Hello World</title></head></html>";
        assert_eq!(extract_title(html), Some("Hello World".to_string()));
    }

    #[test]
    fn test_extract_title_mixed_case() {
        let html = "<html><head><TITLE>Mixed Case</TITLE></head></html>";
        // Our lowercase search finds <title> in lowered version but extracts from original
        // Since the original has <TITLE>, the offset math still works because
        // "<TITLE>" has the same length as "<title>"
        assert_eq!(extract_title(html), Some("Mixed Case".to_string()));
    }

    #[test]
    fn test_extract_title_empty() {
        let html = "<html><head><title></title></head></html>";
        assert_eq!(extract_title(html), None);
    }

    #[test]
    fn test_extract_title_whitespace_only() {
        let html = "<html><head><title>   </title></head></html>";
        assert_eq!(extract_title(html), None);
    }

    #[test]
    fn test_extract_title_missing() {
        let html = "<html><head></head></html>";
        assert_eq!(extract_title(html), None);
    }

    #[test]
    fn test_extract_title_with_whitespace() {
        let html = "<html><head><title>  Trimmed Title  </title></head></html>";
        assert_eq!(extract_title(html), Some("Trimmed Title".to_string()));
    }

    #[test]
    fn test_build_no_redirect_client() {
        let client = build_no_redirect_client(30);
        // Just verify it builds without panicking
        assert!(std::mem::size_of_val(&client) > 0);
    }
}
