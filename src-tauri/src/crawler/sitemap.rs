use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::HashSet;

/// Parse sitemap XML content and extract all <loc> URLs.
///
/// Works for both `<urlset>` (standard sitemaps) and `<sitemapindex>` (sitemap index files).
/// Extracts the text content of every `<loc>` element found.
pub fn parse_sitemap_xml(xml: &str) -> Result<Vec<String>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut urls: Vec<String> = Vec::new();
    let mut inside_loc = false;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"loc" {
                    inside_loc = true;
                }
            }
            Ok(Event::Text(e)) => {
                if inside_loc {
                    let text = e.unescape().map_err(|err| {
                        format!("Failed to unescape XML text: {}", err)
                    })?;
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        urls.push(trimmed);
                    }
                }
            }
            Ok(Event::End(e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"loc" {
                    inside_loc = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!(
                    "XML parse error at position {}: {}",
                    reader.error_position(),
                    e
                ));
            }
            _ => {}
        }
    }

    Ok(urls)
}

/// Fetch and parse a sitemap for a domain, handling nested sitemap indexes.
///
/// Steps:
/// 1. Fetches `https://{domain}/sitemap.xml`
/// 2. Parses the XML
/// 3. If it is a sitemap index (`<sitemapindex>`), recursively fetches each nested
///    sitemap up to `max_depth` levels
/// 4. Deduplicates the collected URLs
/// 5. Returns the full list of unique URLs
///
/// Returns an empty `Vec` if the sitemap is not found (HTTP 404).
pub async fn fetch_sitemap_urls(domain: &str) -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let root_url = format!("https://{}/sitemap.xml", domain);
    let mut all_urls: Vec<String> = Vec::new();

    fetch_sitemap_recursive(&client, &root_url, 3, &mut all_urls).await?;

    // Deduplicate while preserving order
    let mut seen = HashSet::new();
    all_urls.retain(|url| seen.insert(url.clone()));

    Ok(all_urls)
}

/// Recursively fetch and parse a sitemap URL.
///
/// If the response is a sitemap index, each child sitemap is fetched up to `depth` levels.
/// Standard URL entries are collected into `out`.
async fn fetch_sitemap_recursive(
    client: &reqwest::Client,
    url: &str,
    depth: u32,
    out: &mut Vec<String>,
) -> Result<(), String> {
    if depth == 0 {
        return Ok(());
    }

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch {}: {}", url, e))?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(());
    }

    if !response.status().is_success() {
        return Err(format!(
            "HTTP {} when fetching {}",
            response.status(),
            url
        ));
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body from {}: {}", url, e))?;

    let urls = parse_sitemap_xml(&body)?;

    if is_sitemap_index(&body) {
        // Each URL in a sitemap index points to another sitemap
        for child_url in urls {
            fetch_sitemap_recursive(client, &child_url, depth - 1, out).await?;
        }
    } else {
        out.extend(urls);
    }

    Ok(())
}

/// Check whether the XML content represents a sitemap index file.
///
/// A sitemap index uses `<sitemapindex>` as its root element and contains
/// `<sitemap>` child elements (as opposed to `<urlset>` with `<url>` children).
fn is_sitemap_index(xml: &str) -> bool {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let local_name = e.local_name();
                if local_name.as_ref() == b"sitemapindex" {
                    return true;
                }
                if local_name.as_ref() == b"urlset" {
                    return false;
                }
            }
            Ok(Event::Eof) => return false,
            Err(_) => return false,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_urlset() {
        let xml = r#"<?xml version="1.0"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
            <url><loc>https://example.com/page1</loc></url>
            <url><loc>https://example.com/page2</loc></url>
        </urlset>"#;
        let urls = parse_sitemap_xml(xml).unwrap();
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://example.com/page1".to_string()));
        assert!(urls.contains(&"https://example.com/page2".to_string()));
    }

    #[test]
    fn test_parse_sitemap_index() {
        let xml = r#"<?xml version="1.0"?>
        <sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
            <sitemap><loc>https://example.com/sitemap-posts.xml</loc></sitemap>
            <sitemap><loc>https://example.com/sitemap-pages.xml</loc></sitemap>
        </sitemapindex>"#;
        let urls = parse_sitemap_xml(xml).unwrap();
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_parse_empty_sitemap() {
        let xml = r#"<?xml version="1.0"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
        </urlset>"#;
        let urls = parse_sitemap_xml(xml).unwrap();
        assert!(urls.is_empty());
    }

    #[test]
    fn test_parse_invalid_xml() {
        let xml = "not xml at all";
        // Should return an empty vec or an error, not panic
        let result = parse_sitemap_xml(xml);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_is_sitemap_index_true() {
        let xml = r#"<?xml version="1.0"?>
        <sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
            <sitemap><loc>https://example.com/sitemap1.xml</loc></sitemap>
        </sitemapindex>"#;
        assert!(is_sitemap_index(xml));
    }

    #[test]
    fn test_is_sitemap_index_false() {
        let xml = r#"<?xml version="1.0"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
            <url><loc>https://example.com/page1</loc></url>
        </urlset>"#;
        assert!(!is_sitemap_index(xml));
    }

    #[test]
    fn test_parse_loc_with_whitespace() {
        let xml = r#"<?xml version="1.0"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
            <url>
                <loc>
                    https://example.com/page1
                </loc>
            </url>
        </urlset>"#;
        let urls = parse_sitemap_xml(xml).unwrap();
        assert_eq!(urls.len(), 1);
        assert_eq!(urls[0], "https://example.com/page1");
    }

    #[test]
    fn test_parse_multiple_loc_fields() {
        let xml = r#"<?xml version="1.0"?>
        <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
            <url>
                <loc>https://example.com/a</loc>
                <lastmod>2024-01-01</lastmod>
                <changefreq>weekly</changefreq>
                <priority>0.8</priority>
            </url>
            <url>
                <loc>https://example.com/b</loc>
            </url>
            <url>
                <loc>https://example.com/c</loc>
            </url>
        </urlset>"#;
        let urls = parse_sitemap_xml(xml).unwrap();
        assert_eq!(urls.len(), 3);
        assert_eq!(urls[0], "https://example.com/a");
        assert_eq!(urls[1], "https://example.com/b");
        assert_eq!(urls[2], "https://example.com/c");
    }
}
