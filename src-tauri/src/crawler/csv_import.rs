use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvColumn {
    pub index: usize,
    pub name: String,
    pub url_count: usize,
    pub sample_url: Option<String>,
}

/// Detect which columns in a CSV contain URLs
pub fn detect_url_columns(content: &str) -> Result<Vec<CsvColumn>, String> {
    let url_regex = Regex::new(r#"https?://[^\s,"]+"#).unwrap();
    let mut reader = csv::Reader::from_reader(content.as_bytes());

    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|h| h.to_string())
        .collect();

    // For each column, count how many rows have URLs in the first 20 rows
    let mut column_stats: Vec<(usize, String, usize, Option<String>)> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (i, h.clone(), 0, None))
        .collect();

    let mut row_count = 0;
    for result in reader.records().take(20) {
        let record = result.map_err(|e| e.to_string())?;
        row_count += 1;
        for (i, field) in record.iter().enumerate() {
            if i < column_stats.len() && url_regex.is_match(field) {
                column_stats[i].2 += 1;
                if column_stats[i].3.is_none() {
                    column_stats[i].3 = Some(field.to_string());
                }
            }
        }
    }

    // Return columns that have URLs in at least 50% of sampled rows
    let threshold = row_count / 2;
    Ok(column_stats
        .into_iter()
        .filter(|(_, _, count, _)| *count > threshold)
        .map(|(index, name, url_count, sample)| CsvColumn {
            index,
            name,
            url_count,
            sample_url: sample,
        })
        .collect())
}

/// Extract all URLs from a specific column in a CSV
pub fn extract_urls_from_column(content: &str, column_index: usize) -> Result<Vec<String>, String> {
    let url_regex = Regex::new(r#"https?://[^\s,"]+"#).unwrap();
    let mut reader = csv::Reader::from_reader(content.as_bytes());
    let mut urls = Vec::new();
    let mut seen = HashSet::new();

    for result in reader.records() {
        let record = result.map_err(|e| e.to_string())?;
        if let Some(field) = record.get(column_index) {
            if let Some(m) = url_regex.find(field) {
                let url = m.as_str().to_string();
                if seen.insert(url.clone()) {
                    urls.push(url);
                }
            }
        }
    }

    Ok(urls)
}

/// Auto-extract URLs from the best detected column
pub fn auto_extract_urls(content: &str) -> Result<Vec<String>, String> {
    let columns = detect_url_columns(content)?;
    if let Some(best) = columns.first() {
        extract_urls_from_column(content, best.index)
    } else {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_url_columns_screaming_frog() {
        let csv = "Address,Status Code,Title\nhttps://example.com/page1,200,Home\nhttps://example.com/page2,404,Not Found\nhttps://example.com/page3,200,About";
        let columns = detect_url_columns(csv).unwrap();
        assert_eq!(columns.len(), 1);
        assert_eq!(columns[0].name, "Address");
        assert_eq!(columns[0].index, 0);
    }

    #[test]
    fn test_extract_urls_from_column() {
        let csv = "URL,Status\nhttps://example.com/a,200\nhttps://example.com/b,404\nhttps://example.com/c,301";
        let urls = extract_urls_from_column(csv, 0).unwrap();
        assert_eq!(urls.len(), 3);
        assert_eq!(urls[0], "https://example.com/a");
    }

    #[test]
    fn test_auto_extract_urls() {
        let csv =
            "Page,URL,Clicks\nHome,https://example.com/,100\nAbout,https://example.com/about,50";
        let urls = auto_extract_urls(csv).unwrap();
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_deduplication() {
        let csv = "URL\nhttps://example.com/a\nhttps://example.com/a\nhttps://example.com/b";
        let urls = extract_urls_from_column(csv, 0).unwrap();
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_no_urls_found() {
        let csv = "Name,Age\nAlice,30\nBob,25";
        let columns = detect_url_columns(csv).unwrap();
        assert!(columns.is_empty());
    }
}
