use rusqlite::params;
use tauri::State;

use crate::db::Database;

#[tauri::command]
pub fn export_csv(
    db: State<'_, Database>,
    project_id: String,
    filter: Option<String>,
) -> Result<String, String> {
    let conn = db.connection();

    // Build query based on filter
    let mut query = "SELECT url, source, indexed_status, http_status, response_time_ms, \
                     title, redirect_chain, error \
                     FROM urls WHERE project_id = ?1"
        .to_string();

    match filter.as_deref() {
        Some("404") => query.push_str(" AND http_status = 404"),
        Some("redirects") => query.push_str(" AND http_status >= 300 AND http_status < 400"),
        Some("empty_title") => {
            query.push_str(" AND (title IS NULL OR title = '') AND http_status = 200")
        }
        Some("slow") => query.push_str(" AND response_time_ms > 2000"),
        Some("errors") => query.push_str(" AND (http_status >= 400 OR error IS NOT NULL)"),
        _ => {} // "all" or None — return everything
    }

    // Execute query and build CSV
    let mut wtr = csv::Writer::from_writer(Vec::new());

    wtr.write_record([
        "URL",
        "Source",
        "Indexed",
        "HTTP Status",
        "Response Time (ms)",
        "Title",
        "Redirect Chain",
        "Error",
    ])
    .map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i32>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (url, source, indexed, status, time, title, chain, error) =
            row.map_err(|e| e.to_string())?;
        wtr.write_record([
            url,
            source,
            indexed,
            status.map(|s| s.to_string()).unwrap_or_default(),
            time.map(|t| t.to_string()).unwrap_or_default(),
            title.unwrap_or_default(),
            chain.unwrap_or_default(),
            error.unwrap_or_default(),
        ])
        .map_err(|e| e.to_string())?;
    }

    let csv_bytes = wtr.into_inner().map_err(|e| e.to_string())?;
    String::from_utf8(csv_bytes).map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectStats {
    pub total_urls: usize,
    pub confirmed_indexed: usize,
    pub not_indexed: usize,
    pub unknown_status: usize,
    pub checked: usize,
    pub ok_count: usize,
    pub redirect_count: usize,
    pub not_found_count: usize,
    pub error_count: usize,
    pub empty_title_count: usize,
    pub slow_count: usize,
}

#[tauri::command]
pub fn get_project_stats(
    db: State<'_, Database>,
    project_id: String,
) -> Result<ProjectStats, String> {
    let conn = db.connection();

    let mut stmt = conn
        .prepare(
            "SELECT
                COUNT(*) AS total_urls,
                COUNT(CASE WHEN indexed_status = 'confirmed' THEN 1 END) AS confirmed_indexed,
                COUNT(CASE WHEN indexed_status = 'not_indexed' THEN 1 END) AS not_indexed,
                COUNT(CASE WHEN indexed_status = 'unknown' THEN 1 END) AS unknown_status,
                COUNT(CASE WHEN checked_at IS NOT NULL THEN 1 END) AS checked,
                COUNT(CASE WHEN http_status >= 200 AND http_status < 300 THEN 1 END) AS ok_count,
                COUNT(CASE WHEN http_status >= 300 AND http_status < 400 THEN 1 END) AS redirect_count,
                COUNT(CASE WHEN http_status = 404 THEN 1 END) AS not_found_count,
                COUNT(CASE WHEN http_status >= 400 OR error IS NOT NULL THEN 1 END) AS error_count,
                COUNT(CASE WHEN (title IS NULL OR title = '') AND http_status = 200 THEN 1 END) AS empty_title_count,
                COUNT(CASE WHEN response_time_ms > 2000 THEN 1 END) AS slow_count
            FROM urls
            WHERE project_id = ?1",
        )
        .map_err(|e| e.to_string())?;

    let stats = stmt
        .query_row(params![project_id], |row| {
            Ok(ProjectStats {
                total_urls: row.get::<_, usize>(0)?,
                confirmed_indexed: row.get::<_, usize>(1)?,
                not_indexed: row.get::<_, usize>(2)?,
                unknown_status: row.get::<_, usize>(3)?,
                checked: row.get::<_, usize>(4)?,
                ok_count: row.get::<_, usize>(5)?,
                redirect_count: row.get::<_, usize>(6)?,
                not_found_count: row.get::<_, usize>(7)?,
                error_count: row.get::<_, usize>(8)?,
                empty_title_count: row.get::<_, usize>(9)?,
                slow_count: row.get::<_, usize>(10)?,
            })
        })
        .map_err(|e| e.to_string())?;

    Ok(stats)
}
