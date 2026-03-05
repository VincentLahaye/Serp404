use rusqlite::params;
use tauri::State;

use crate::db::Database;
use crate::models::UrlEntry;

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

#[tauri::command]
pub fn get_checked_urls(
    db: State<'_, Database>,
    project_id: String,
    filter: Option<String>,
) -> Result<Vec<UrlEntry>, String> {
    let conn = db.connection();
    let mut query = "SELECT id, project_id, url, source, indexed_status, http_status, \
                     response_time_ms, title, redirect_chain, error, checked_at \
                     FROM urls WHERE project_id = ?1 AND checked_at IS NOT NULL"
        .to_string();

    match filter.as_deref() {
        Some("404") => query.push_str(" AND http_status = 404"),
        Some("redirects") => query.push_str(" AND http_status >= 300 AND http_status < 400"),
        Some("empty_title") => {
            query.push_str(" AND (title IS NULL OR title = '') AND http_status = 200")
        }
        Some("slow") => query.push_str(" AND response_time_ms > 2000"),
        Some("errors") => query.push_str(" AND (http_status >= 500 OR error IS NOT NULL)"),
        _ => {}
    }

    query.push_str(" ORDER BY checked_at DESC");

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![project_id], |row| {
            Ok(UrlEntry {
                id: row.get(0)?,
                project_id: row.get(1)?,
                url: row.get(2)?,
                source: row.get(3)?,
                indexed_status: row.get(4)?,
                http_status: row.get(5)?,
                response_time_ms: row.get(6)?,
                title: row.get(7)?,
                redirect_chain: row.get(8)?,
                error: row.get(9)?,
                checked_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::models::UrlEntry;
    use rusqlite::params;
    use uuid::Uuid;

    fn setup() -> Database {
        Database::new(":memory:").unwrap()
    }

    /// Helper: create a project and return its id.
    fn create_project(db: &Database, domain: &str) -> String {
        let conn = db.connection();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, domain) VALUES (?1, ?2)",
            params![id, domain],
        )
        .unwrap();
        id
    }

    /// Helper: insert a URL with full audit data.
    fn insert_url(
        db: &Database,
        project_id: &str,
        url: &str,
        source: &str,
        indexed_status: &str,
        http_status: Option<i32>,
        response_time_ms: Option<i64>,
        title: Option<&str>,
        redirect_chain: Option<&str>,
        error: Option<&str>,
        checked_at: Option<&str>,
    ) {
        let conn = db.connection();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO urls (id, project_id, url, source, indexed_status, http_status, \
             response_time_ms, title, redirect_chain, error, checked_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                project_id,
                url,
                source,
                indexed_status,
                http_status,
                response_time_ms,
                title,
                redirect_chain,
                error,
                checked_at,
            ],
        )
        .unwrap();
    }

    // ── CSV generation tests ────────────────────────────────────────

    #[test]
    fn test_export_csv_empty_project() {
        let db = setup();
        let pid = create_project(&db, "empty.com");
        let conn = db.connection();

        let query = "SELECT url, source, indexed_status, http_status, response_time_ms, \
                     title, redirect_chain, error \
                     FROM urls WHERE project_id = ?1";

        let mut stmt = conn.prepare(query).unwrap();
        let rows: Vec<String> = stmt
            .query_map(params![pid], |row| row.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(rows.is_empty(), "Empty project should return no URL rows");
    }

    #[test]
    fn test_export_csv_basic_data() {
        let db = setup();
        let pid = create_project(&db, "test.com");
        insert_url(
            &db,
            &pid,
            "https://test.com/page1",
            "sitemap",
            "confirmed",
            Some(200),
            Some(150),
            Some("Page 1"),
            None,
            None,
            Some("2024-01-15"),
        );

        let conn = db.connection();
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
        .unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT url, source, indexed_status, http_status, response_time_ms, \
                 title, redirect_chain, error FROM urls WHERE project_id = ?1",
            )
            .unwrap();
        let rows = stmt
            .query_map(params![pid], |row| {
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
            .unwrap();

        let mut count = 0;
        for row in rows {
            let (url, source, indexed, status, time, title, chain, error) = row.unwrap();
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
            .unwrap();
            count += 1;
        }

        assert_eq!(count, 1);
        let csv_bytes = wtr.into_inner().unwrap();
        let csv_string = String::from_utf8(csv_bytes).unwrap();
        assert!(csv_string.contains("https://test.com/page1"));
        assert!(csv_string.contains("sitemap"));
        assert!(csv_string.contains("confirmed"));
        assert!(csv_string.contains("200"));
        assert!(csv_string.contains("Page 1"));
    }

    // ── Filter logic tests ──────────────────────────────────────────

    #[test]
    fn test_filter_404() {
        let db = setup();
        let pid = create_project(&db, "filter.com");
        insert_url(&db, &pid, "https://filter.com/ok", "sitemap", "confirmed", Some(200), Some(100), Some("OK"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://filter.com/missing", "sitemap", "confirmed", Some(404), Some(80), None, None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://filter.com/also-missing", "sitemap", "confirmed", Some(404), Some(90), None, None, None, Some("2024-01-01"));

        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT url FROM urls WHERE project_id = ?1 AND http_status = 404",
            )
            .unwrap();
        let urls: Vec<String> = stmt
            .query_map(params![pid], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://filter.com/missing".to_string()));
        assert!(urls.contains(&"https://filter.com/also-missing".to_string()));
    }

    #[test]
    fn test_filter_redirects() {
        let db = setup();
        let pid = create_project(&db, "redir.com");
        insert_url(&db, &pid, "https://redir.com/ok", "sitemap", "confirmed", Some(200), Some(100), Some("OK"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://redir.com/moved", "sitemap", "confirmed", Some(301), Some(50), None, Some("[\"https://redir.com/new\"]"), None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://redir.com/temp", "sitemap", "confirmed", Some(302), Some(60), None, Some("[\"https://redir.com/other\"]"), None, Some("2024-01-01"));

        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT url FROM urls WHERE project_id = ?1 AND http_status >= 300 AND http_status < 400",
            )
            .unwrap();
        let urls: Vec<String> = stmt
            .query_map(params![pid], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn test_filter_empty_title() {
        let db = setup();
        let pid = create_project(&db, "title.com");
        insert_url(&db, &pid, "https://title.com/with-title", "sitemap", "confirmed", Some(200), Some(100), Some("My Page"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://title.com/no-title", "sitemap", "confirmed", Some(200), Some(100), None, None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://title.com/empty-title", "sitemap", "confirmed", Some(200), Some(100), Some(""), None, None, Some("2024-01-01"));
        // 404 with no title should NOT match this filter (requires http_status = 200)
        insert_url(&db, &pid, "https://title.com/404-no-title", "sitemap", "confirmed", Some(404), Some(100), None, None, None, Some("2024-01-01"));

        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT url FROM urls WHERE project_id = ?1 AND (title IS NULL OR title = '') AND http_status = 200",
            )
            .unwrap();
        let urls: Vec<String> = stmt
            .query_map(params![pid], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://title.com/no-title".to_string()));
        assert!(urls.contains(&"https://title.com/empty-title".to_string()));
    }

    #[test]
    fn test_filter_slow() {
        let db = setup();
        let pid = create_project(&db, "slow.com");
        insert_url(&db, &pid, "https://slow.com/fast", "sitemap", "confirmed", Some(200), Some(500), Some("Fast"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://slow.com/slow", "sitemap", "confirmed", Some(200), Some(3000), Some("Slow"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://slow.com/very-slow", "sitemap", "confirmed", Some(200), Some(5000), Some("Very Slow"), None, None, Some("2024-01-01"));
        // Exactly 2000ms is NOT slow (filter is > 2000)
        insert_url(&db, &pid, "https://slow.com/borderline", "sitemap", "confirmed", Some(200), Some(2000), Some("Borderline"), None, None, Some("2024-01-01"));

        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT url FROM urls WHERE project_id = ?1 AND response_time_ms > 2000",
            )
            .unwrap();
        let urls: Vec<String> = stmt
            .query_map(params![pid], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://slow.com/slow".to_string()));
        assert!(urls.contains(&"https://slow.com/very-slow".to_string()));
    }

    #[test]
    fn test_filter_errors_in_export() {
        let db = setup();
        let pid = create_project(&db, "errors.com");
        insert_url(&db, &pid, "https://errors.com/ok", "sitemap", "confirmed", Some(200), Some(100), Some("OK"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://errors.com/404", "sitemap", "confirmed", Some(404), Some(80), None, None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://errors.com/500", "sitemap", "confirmed", Some(500), Some(200), None, None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://errors.com/conn-err", "sitemap", "confirmed", None, None, None, None, Some("Connection refused"), Some("2024-01-01"));

        // export_csv uses: http_status >= 400 OR error IS NOT NULL
        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT url FROM urls WHERE project_id = ?1 AND (http_status >= 400 OR error IS NOT NULL)",
            )
            .unwrap();
        let urls: Vec<String> = stmt
            .query_map(params![pid], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(urls.len(), 3);
        assert!(urls.contains(&"https://errors.com/404".to_string()));
        assert!(urls.contains(&"https://errors.com/500".to_string()));
        assert!(urls.contains(&"https://errors.com/conn-err".to_string()));
    }

    // ── get_project_stats counting logic ────────────────────────────

    #[test]
    fn test_project_stats_empty_project() {
        let db = setup();
        let pid = create_project(&db, "empty-stats.com");
        let conn = db.connection();

        let mut stmt = conn
            .prepare(
                "SELECT
                    COUNT(*) AS total_urls,
                    COUNT(CASE WHEN indexed_status = 'confirmed' THEN 1 END),
                    COUNT(CASE WHEN indexed_status = 'not_indexed' THEN 1 END),
                    COUNT(CASE WHEN indexed_status = 'unknown' THEN 1 END),
                    COUNT(CASE WHEN checked_at IS NOT NULL THEN 1 END),
                    COUNT(CASE WHEN http_status >= 200 AND http_status < 300 THEN 1 END),
                    COUNT(CASE WHEN http_status >= 300 AND http_status < 400 THEN 1 END),
                    COUNT(CASE WHEN http_status = 404 THEN 1 END),
                    COUNT(CASE WHEN http_status >= 400 OR error IS NOT NULL THEN 1 END),
                    COUNT(CASE WHEN (title IS NULL OR title = '') AND http_status = 200 THEN 1 END),
                    COUNT(CASE WHEN response_time_ms > 2000 THEN 1 END)
                FROM urls WHERE project_id = ?1",
            )
            .unwrap();

        let (total, confirmed, not_indexed, unknown, checked, ok, redir, not_found, errors, empty_title, slow): (
            usize, usize, usize, usize, usize, usize, usize, usize, usize, usize, usize,
        ) = stmt
            .query_row(params![pid], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            })
            .unwrap();

        assert_eq!(total, 0);
        assert_eq!(confirmed, 0);
        assert_eq!(not_indexed, 0);
        assert_eq!(unknown, 0);
        assert_eq!(checked, 0);
        assert_eq!(ok, 0);
        assert_eq!(redir, 0);
        assert_eq!(not_found, 0);
        assert_eq!(errors, 0);
        assert_eq!(empty_title, 0);
        assert_eq!(slow, 0);
    }

    #[test]
    fn test_project_stats_comprehensive() {
        let db = setup();
        let pid = create_project(&db, "stats.com");

        // 1) OK, confirmed, checked, fast, has title
        insert_url(&db, &pid, "https://stats.com/good", "sitemap", "confirmed", Some(200), Some(100), Some("Good"), None, None, Some("2024-01-01"));
        // 2) 404, confirmed, checked
        insert_url(&db, &pid, "https://stats.com/missing", "sitemap", "confirmed", Some(404), Some(80), None, None, None, Some("2024-01-01"));
        // 3) Redirect 301, confirmed, checked
        insert_url(&db, &pid, "https://stats.com/redir", "sitemap", "confirmed", Some(301), Some(50), None, Some("[\"https://stats.com/new\"]"), None, Some("2024-01-01"));
        // 4) 200 with empty title, confirmed, checked, slow
        insert_url(&db, &pid, "https://stats.com/no-title", "sitemap", "confirmed", Some(200), Some(3000), Some(""), None, None, Some("2024-01-01"));
        // 5) Error with no http_status, unknown indexation, checked
        insert_url(&db, &pid, "https://stats.com/error", "sitemap", "unknown", None, None, None, None, Some("timeout"), Some("2024-01-01"));
        // 6) Not indexed, not checked
        insert_url(&db, &pid, "https://stats.com/not-indexed", "sitemap", "not_indexed", None, None, None, None, None, None);
        // 7) Unknown indexation, not checked
        insert_url(&db, &pid, "https://stats.com/unchecked", "csv", "unknown", None, None, None, None, None, None);

        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT
                    COUNT(*),
                    COUNT(CASE WHEN indexed_status = 'confirmed' THEN 1 END),
                    COUNT(CASE WHEN indexed_status = 'not_indexed' THEN 1 END),
                    COUNT(CASE WHEN indexed_status = 'unknown' THEN 1 END),
                    COUNT(CASE WHEN checked_at IS NOT NULL THEN 1 END),
                    COUNT(CASE WHEN http_status >= 200 AND http_status < 300 THEN 1 END),
                    COUNT(CASE WHEN http_status >= 300 AND http_status < 400 THEN 1 END),
                    COUNT(CASE WHEN http_status = 404 THEN 1 END),
                    COUNT(CASE WHEN http_status >= 400 OR error IS NOT NULL THEN 1 END),
                    COUNT(CASE WHEN (title IS NULL OR title = '') AND http_status = 200 THEN 1 END),
                    COUNT(CASE WHEN response_time_ms > 2000 THEN 1 END)
                FROM urls WHERE project_id = ?1",
            )
            .unwrap();

        let (total, confirmed, not_indexed, unknown, checked, ok, redir, not_found, errors, empty_title, slow): (
            usize, usize, usize, usize, usize, usize, usize, usize, usize, usize, usize,
        ) = stmt
            .query_row(params![pid], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            })
            .unwrap();

        assert_eq!(total, 7, "total_urls");
        assert_eq!(confirmed, 4, "confirmed_indexed");
        assert_eq!(not_indexed, 1, "not_indexed");
        assert_eq!(unknown, 2, "unknown_status");
        assert_eq!(checked, 5, "checked (have checked_at)");
        assert_eq!(ok, 2, "ok_count (200-299)");
        assert_eq!(redir, 1, "redirect_count (300-399)");
        assert_eq!(not_found, 1, "not_found_count (404)");
        assert_eq!(errors, 2, "error_count (>=400 or error IS NOT NULL)"); // 404 + timeout error
        assert_eq!(empty_title, 1, "empty_title_count");
        assert_eq!(slow, 1, "slow_count (>2000ms)");
    }

    // ── get_checked_urls with filters ───────────────────────────────

    #[test]
    fn test_get_checked_urls_only_returns_checked() {
        let db = setup();
        let pid = create_project(&db, "checked.com");
        insert_url(&db, &pid, "https://checked.com/a", "sitemap", "confirmed", Some(200), Some(100), Some("A"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://checked.com/b", "sitemap", "confirmed", None, None, None, None, None, None);

        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, url, source, indexed_status, http_status, \
                 response_time_ms, title, redirect_chain, error, checked_at \
                 FROM urls WHERE project_id = ?1 AND checked_at IS NOT NULL",
            )
            .unwrap();
        let entries: Vec<UrlEntry> = stmt
            .query_map(params![pid], |row| {
                Ok(UrlEntry {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    url: row.get(2)?,
                    source: row.get(3)?,
                    indexed_status: row.get(4)?,
                    http_status: row.get(5)?,
                    response_time_ms: row.get(6)?,
                    title: row.get(7)?,
                    redirect_chain: row.get(8)?,
                    error: row.get(9)?,
                    checked_at: row.get(10)?,
                })
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "https://checked.com/a");
        assert!(entries[0].checked_at.is_some());
    }

    #[test]
    fn test_get_checked_urls_ordered_by_checked_at_desc() {
        let db = setup();
        let pid = create_project(&db, "order.com");
        insert_url(&db, &pid, "https://order.com/first", "sitemap", "confirmed", Some(200), Some(100), Some("First"), None, None, Some("2024-01-01 10:00:00"));
        insert_url(&db, &pid, "https://order.com/second", "sitemap", "confirmed", Some(200), Some(100), Some("Second"), None, None, Some("2024-01-02 10:00:00"));
        insert_url(&db, &pid, "https://order.com/third", "sitemap", "confirmed", Some(200), Some(100), Some("Third"), None, None, Some("2024-01-03 10:00:00"));

        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT url FROM urls WHERE project_id = ?1 AND checked_at IS NOT NULL \
                 ORDER BY checked_at DESC",
            )
            .unwrap();
        let urls: Vec<String> = stmt
            .query_map(params![pid], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(urls, vec![
            "https://order.com/third",
            "https://order.com/second",
            "https://order.com/first",
        ]);
    }

    #[test]
    fn test_get_checked_urls_filter_errors_uses_500_threshold() {
        let db = setup();
        let pid = create_project(&db, "err-filter.com");
        insert_url(&db, &pid, "https://err-filter.com/ok", "sitemap", "confirmed", Some(200), Some(100), Some("OK"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://err-filter.com/404", "sitemap", "confirmed", Some(404), Some(80), None, None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://err-filter.com/500", "sitemap", "confirmed", Some(500), Some(200), None, None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://err-filter.com/conn", "sitemap", "confirmed", None, None, None, None, Some("refused"), Some("2024-01-01"));

        // get_checked_urls uses: http_status >= 500 OR error IS NOT NULL (different from export_csv!)
        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT url FROM urls WHERE project_id = ?1 AND checked_at IS NOT NULL \
                 AND (http_status >= 500 OR error IS NOT NULL)",
            )
            .unwrap();
        let urls: Vec<String> = stmt
            .query_map(params![pid], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://err-filter.com/500".to_string()));
        assert!(urls.contains(&"https://err-filter.com/conn".to_string()));
        // Note: 404 is NOT included in get_checked_urls errors filter (>= 500)
        assert!(!urls.contains(&"https://err-filter.com/404".to_string()));
    }

    #[test]
    fn test_no_filter_returns_all_checked() {
        let db = setup();
        let pid = create_project(&db, "allchecked.com");
        insert_url(&db, &pid, "https://allchecked.com/a", "sitemap", "confirmed", Some(200), Some(100), Some("A"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid, "https://allchecked.com/b", "sitemap", "confirmed", Some(404), Some(80), None, None, None, Some("2024-01-02"));
        insert_url(&db, &pid, "https://allchecked.com/c", "sitemap", "confirmed", Some(301), Some(50), None, None, None, Some("2024-01-03"));

        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT url FROM urls WHERE project_id = ?1 AND checked_at IS NOT NULL",
            )
            .unwrap();
        let urls: Vec<String> = stmt
            .query_map(params![pid], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(urls.len(), 3, "No filter should return all checked URLs");
    }

    #[test]
    fn test_stats_isolation_between_projects() {
        let db = setup();
        let pid_a = create_project(&db, "project-a.com");
        let pid_b = create_project(&db, "project-b.com");

        insert_url(&db, &pid_a, "https://project-a.com/page1", "sitemap", "confirmed", Some(200), Some(100), Some("A1"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid_a, "https://project-a.com/page2", "sitemap", "confirmed", Some(200), Some(100), Some("A2"), None, None, Some("2024-01-01"));
        insert_url(&db, &pid_b, "https://project-b.com/page1", "csv", "unknown", None, None, None, None, None, None);

        let conn = db.connection();

        // Stats for project A
        let total_a: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1",
                params![pid_a],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(total_a, 2);

        // Stats for project B
        let total_b: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1",
                params![pid_b],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(total_b, 1);
    }
}
