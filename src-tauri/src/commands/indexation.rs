use rusqlite::params;
use tauri::{AppHandle, Emitter, State};

use crate::crawler::serper;
use crate::db::Database;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

/// Global map to store cancellation tokens per project.
static CANCEL_TOKENS: LazyLock<Mutex<HashMap<String, Arc<AtomicBool>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexationProgress {
    project_id: String,
    checked: usize,
    total: usize,
    current_url: String,
    status: String, // "running" | "done" | "cancelled"
}

#[tauri::command]
pub fn get_unverified_count(db: State<'_, Database>, project_id: String) -> Result<usize, String> {
    let conn = db.connection();

    let count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM urls WHERE project_id = ?1 AND indexed_status = 'unknown'",
            params![project_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    Ok(count)
}

#[tauri::command]
pub async fn verify_indexation(
    app: AppHandle,
    db: State<'_, Database>,
    project_id: String,
) -> Result<usize, String> {
    // 1. Get serper API key from settings
    let api_key = {
        let conn = db.connection();
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'serper_api_key'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| "No serper.dev API key configured".to_string())?
    };

    // 2. Get all unverified URLs (indexed_status = 'unknown') for this project
    let urls: Vec<(String, String)> = {
        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, url FROM urls WHERE project_id = ?1 AND indexed_status = 'unknown'",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
        rows
    };

    let total = urls.len();
    if total == 0 {
        return Ok(0);
    }

    // 3. Set up cancellation token
    let cancel_token = Arc::new(AtomicBool::new(false));
    {
        let mut tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.insert(project_id.clone(), cancel_token.clone());
    }

    // 4. For each URL, check indexation
    let mut confirmed_count: usize = 0;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    for (i, (url_id, url)) in urls.iter().enumerate() {
        // 4a. Check cancellation before and after each async operation
        if cancel_token.load(Ordering::Relaxed) {
            let _ = app.emit(
                "indexation-progress",
                IndexationProgress {
                    project_id: project_id.clone(),
                    checked: i,
                    total,
                    current_url: url.clone(),
                    status: "cancelled".to_string(),
                },
            );
            let mut tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
            tokens.remove(&project_id);
            return Ok(confirmed_count);
        }

        // 4b. Call serper::check_url_indexed
        let is_indexed = serper::check_url_indexed(&client, &api_key, url).await?;

        // 4b'. Re-check cancellation after HTTP request — skip DB write if cancelled
        if cancel_token.load(Ordering::Relaxed) {
            let _ = app.emit(
                "indexation-progress",
                IndexationProgress {
                    project_id: project_id.clone(),
                    checked: i,
                    total,
                    current_url: url.clone(),
                    status: "cancelled".to_string(),
                },
            );
            let mut tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
            tokens.remove(&project_id);
            return Ok(confirmed_count);
        }

        // 4c. Update URL's indexed_status
        let new_status = if is_indexed {
            confirmed_count += 1;
            "confirmed"
        } else {
            "not_indexed"
        };

        {
            let conn = db.connection();
            conn.execute(
                "UPDATE urls SET indexed_status = ?1 WHERE id = ?2",
                params![new_status, url_id],
            )
            .map_err(|e| e.to_string())?;
        }

        // 4d. Emit indexation-progress event
        let _ = app.emit(
            "indexation-progress",
            IndexationProgress {
                project_id: project_id.clone(),
                checked: i + 1,
                total,
                current_url: url.clone(),
                status: "running".to_string(),
            },
        );
    }

    // 5. Clean up cancellation token
    {
        let mut tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
        tokens.remove(&project_id);
    }

    // Emit final done event
    let _ = app.emit(
        "indexation-progress",
        IndexationProgress {
            project_id: project_id.clone(),
            checked: total,
            total,
            current_url: String::new(),
            status: "done".to_string(),
        },
    );

    // 6. Return count of confirmed indexed URLs
    Ok(confirmed_count)
}

#[tauri::command]
pub fn stop_indexation(project_id: String) -> Result<(), String> {
    let tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
    if let Some(token) = tokens.get(&project_id) {
        token.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use rusqlite::params;
    use uuid::Uuid;

    fn setup() -> Database {
        Database::new(":memory:").unwrap()
    }

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

    fn insert_url(db: &Database, project_id: &str, url: &str, indexed_status: &str) {
        let conn = db.connection();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO urls (id, project_id, url, source, indexed_status) VALUES (?1, ?2, ?3, 'sitemap', ?4)",
            params![id, project_id, url, indexed_status],
        )
        .unwrap();
    }

    // ── get_unverified_count tests ──────────────────────────────────

    #[test]
    fn test_unverified_count_empty_project() {
        let db = setup();
        let pid = create_project(&db, "empty.com");
        let conn = db.connection();

        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1 AND indexed_status = 'unknown'",
                params![pid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_unverified_count_all_unknown() {
        let db = setup();
        let pid = create_project(&db, "unknown.com");
        insert_url(&db, &pid, "https://unknown.com/a", "unknown");
        insert_url(&db, &pid, "https://unknown.com/b", "unknown");
        insert_url(&db, &pid, "https://unknown.com/c", "unknown");

        let conn = db.connection();
        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1 AND indexed_status = 'unknown'",
                params![pid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_unverified_count_mixed_statuses() {
        let db = setup();
        let pid = create_project(&db, "mixed.com");
        insert_url(&db, &pid, "https://mixed.com/a", "unknown");
        insert_url(&db, &pid, "https://mixed.com/b", "confirmed");
        insert_url(&db, &pid, "https://mixed.com/c", "not_indexed");
        insert_url(&db, &pid, "https://mixed.com/d", "unknown");

        let conn = db.connection();
        let count: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1 AND indexed_status = 'unknown'",
                params![pid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2, "Only 'unknown' status URLs should be counted");
    }

    #[test]
    fn test_unverified_count_isolated_between_projects() {
        let db = setup();
        let pid_a = create_project(&db, "a.com");
        let pid_b = create_project(&db, "b.com");
        insert_url(&db, &pid_a, "https://a.com/1", "unknown");
        insert_url(&db, &pid_a, "https://a.com/2", "unknown");
        insert_url(&db, &pid_a, "https://a.com/3", "unknown");
        insert_url(&db, &pid_b, "https://b.com/1", "unknown");

        let conn = db.connection();
        let count_a: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1 AND indexed_status = 'unknown'",
                params![pid_a],
                |row| row.get(0),
            )
            .unwrap();
        let count_b: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1 AND indexed_status = 'unknown'",
                params![pid_b],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count_a, 3);
        assert_eq!(count_b, 1);
    }

    // ── Indexation status update tests ──────────────────────────────

    #[test]
    fn test_update_indexed_status() {
        let db = setup();
        let pid = create_project(&db, "update.com");
        let conn = db.connection();
        let url_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO urls (id, project_id, url, source, indexed_status) VALUES (?1, ?2, ?3, 'sitemap', 'unknown')",
            params![url_id, pid, "https://update.com/page"],
        )
        .unwrap();

        // Simulate what verify_indexation does: update status only
        conn.execute(
            "UPDATE urls SET indexed_status = ?1 WHERE id = ?2",
            params!["confirmed", url_id],
        )
        .unwrap();

        let (status, checked_at): (String, Option<String>) = conn
            .query_row(
                "SELECT indexed_status, checked_at FROM urls WHERE id = ?1",
                params![url_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(status, "confirmed");
        assert!(checked_at.is_none(), "checked_at should remain NULL after indexation (reserved for audit)");
    }

    #[test]
    fn test_update_indexed_status_not_indexed() {
        let db = setup();
        let pid = create_project(&db, "notidx.com");
        let conn = db.connection();
        let url_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO urls (id, project_id, url, source, indexed_status) VALUES (?1, ?2, ?3, 'sitemap', 'unknown')",
            params![url_id, pid, "https://notidx.com/page"],
        )
        .unwrap();

        conn.execute(
            "UPDATE urls SET indexed_status = ?1 WHERE id = ?2",
            params!["not_indexed", url_id],
        )
        .unwrap();

        let status: String = conn
            .query_row(
                "SELECT indexed_status FROM urls WHERE id = ?1",
                params![url_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "not_indexed");
    }

    // ── Cancel token tests ──────────────────────────────────────────

    #[test]
    fn test_cancel_token_setup_and_teardown() {
        let project_id = "test-cancel-project".to_string();

        // Setup: insert a cancel token
        let cancel_token = Arc::new(AtomicBool::new(false));
        {
            let mut tokens = CANCEL_TOKENS.lock().unwrap();
            tokens.insert(project_id.clone(), cancel_token.clone());
        }

        // Verify it exists
        {
            let tokens = CANCEL_TOKENS.lock().unwrap();
            assert!(tokens.contains_key(&project_id));
        }

        // Simulate cancellation
        cancel_token.store(true, Ordering::Relaxed);
        assert!(cancel_token.load(Ordering::Relaxed));

        // Teardown
        {
            let mut tokens = CANCEL_TOKENS.lock().unwrap();
            tokens.remove(&project_id);
        }

        // Verify it's gone
        {
            let tokens = CANCEL_TOKENS.lock().unwrap();
            assert!(!tokens.contains_key(&project_id));
        }
    }

    #[test]
    fn test_stop_indexation_sets_cancel_flag() {
        let project_id = format!("stop-test-{}", Uuid::new_v4());

        let cancel_token = Arc::new(AtomicBool::new(false));
        {
            let mut tokens = CANCEL_TOKENS.lock().unwrap();
            tokens.insert(project_id.clone(), cancel_token.clone());
        }

        // Call stop_indexation
        stop_indexation(project_id.clone()).unwrap();

        // Verify the cancel flag was set
        assert!(cancel_token.load(Ordering::Relaxed), "Cancel token should be true after stop_indexation");

        // Cleanup
        {
            let mut tokens = CANCEL_TOKENS.lock().unwrap();
            tokens.remove(&project_id);
        }
    }

    #[test]
    fn test_stop_indexation_nonexistent_project_is_ok() {
        // Stopping a non-existent project should not error
        let result = stop_indexation("nonexistent-project".to_string());
        assert!(result.is_ok());
    }

    // ── Query for unverified URLs (what verify_indexation fetches) ──

    #[test]
    fn test_fetch_unverified_urls_for_verification() {
        let db = setup();
        let pid = create_project(&db, "verify.com");
        insert_url(&db, &pid, "https://verify.com/a", "unknown");
        insert_url(&db, &pid, "https://verify.com/b", "confirmed");
        insert_url(&db, &pid, "https://verify.com/c", "unknown");

        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, url FROM urls WHERE project_id = ?1 AND indexed_status = 'unknown'",
            )
            .unwrap();
        let rows: Vec<(String, String)> = stmt
            .query_map(params![pid], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(rows.len(), 2);
        let urls: Vec<&str> = rows.iter().map(|(_, u)| u.as_str()).collect();
        assert!(urls.contains(&"https://verify.com/a"));
        assert!(urls.contains(&"https://verify.com/c"));
    }
}
