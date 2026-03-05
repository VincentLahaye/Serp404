use rusqlite::params;
use tauri::{AppHandle, Emitter, State};

use crate::crawler::checker;
use crate::db::Database;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

/// Per-project audit control state (cancel, pause, concurrency).
struct AuditControl {
    cancel: Arc<AtomicBool>,
    pause: Arc<AtomicBool>,
    concurrency: Arc<AtomicU32>,
}

static AUDIT_STATE: LazyLock<Mutex<HashMap<String, AuditControl>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditProgress {
    project_id: String,
    checked: usize,
    total: usize,
    current_url: String,
    status: String, // "running" | "paused" | "done" | "cancelled"
    stats: AuditStats,
}

#[derive(Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct AuditStats {
    ok_count: usize,
    redirect_count: usize,
    not_found_count: usize,
    error_count: usize,
    empty_title_count: usize,
    slow_count: usize, // response > 2000ms
}

#[tauri::command]
pub async fn start_audit(
    app: AppHandle,
    db: State<'_, Database>,
    project_id: String,
    concurrency: u32,
) -> Result<(), String> {
    // 1. Query confirmed URLs that haven't been audited yet (no http_status)
    let urls: Vec<(String, String)> = {
        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, url FROM urls \
                 WHERE project_id = ?1 AND indexed_status = 'confirmed' AND http_status IS NULL",
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
        // Nothing to audit; emit done immediately
        let _ = app.emit(
            "audit-progress",
            AuditProgress {
                project_id: project_id.clone(),
                checked: 0,
                total: 0,
                current_url: String::new(),
                status: "done".to_string(),
                stats: AuditStats::default(),
            },
        );
        return Ok(());
    }

    // 2. Check if an audit is already running for this project
    {
        let state = AUDIT_STATE.lock().map_err(|e| e.to_string())?;
        if state.contains_key(&project_id) {
            return Err("Audit already running for this project".to_string());
        }
    }

    // 3. Setup AuditControl with cancel/pause/concurrency atomics
    let cancel = Arc::new(AtomicBool::new(false));
    let pause = Arc::new(AtomicBool::new(false));
    let conc = Arc::new(AtomicU32::new(concurrency.max(1)));

    {
        let mut state = AUDIT_STATE.lock().map_err(|e| e.to_string())?;
        state.insert(
            project_id.clone(),
            AuditControl {
                cancel: cancel.clone(),
                pause: pause.clone(),
                concurrency: conc.clone(),
            },
        );
    }

    // 3. Build no-redirect HTTP client
    let client = checker::build_no_redirect_client(30);

    // 4. Process URLs in chunks based on current concurrency
    let mut stats = AuditStats::default();
    let mut checked: usize = 0;

    let mut idx = 0;
    while idx < urls.len() {
        // 4a. Check cancel flag
        if cancel.load(Ordering::Relaxed) {
            let _ = app.emit(
                "audit-progress",
                AuditProgress {
                    project_id: project_id.clone(),
                    checked,
                    total,
                    current_url: String::new(),
                    status: "cancelled".to_string(),
                    stats: stats.clone(),
                },
            );
            cleanup_state(&project_id);
            return Ok(());
        }

        // 4b. While pause flag is set, sleep and emit paused status
        while pause.load(Ordering::Relaxed) {
            if cancel.load(Ordering::Relaxed) {
                let _ = app.emit(
                    "audit-progress",
                    AuditProgress {
                        project_id: project_id.clone(),
                        checked,
                        total,
                        current_url: String::new(),
                        status: "cancelled".to_string(),
                        stats: stats.clone(),
                    },
                );
                cleanup_state(&project_id);
                return Ok(());
            }
            let _ = app.emit(
                "audit-progress",
                AuditProgress {
                    project_id: project_id.clone(),
                    checked,
                    total,
                    current_url: String::new(),
                    status: "paused".to_string(),
                    stats: stats.clone(),
                },
            );
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        // 4c. Determine chunk size from current concurrency value
        let chunk_size = conc.load(Ordering::Relaxed).max(1) as usize;
        let end = (idx + chunk_size).min(urls.len());
        let chunk = &urls[idx..end];

        // 4d. Spawn concurrent tasks for the chunk
        let mut handles = Vec::new();
        for (url_id, url) in chunk {
            let client = client.clone();
            let url = url.clone();
            let url_id = url_id.clone();
            handles.push(tokio::spawn(async move {
                let result = checker::check_url(&client, &url).await;
                (url_id, result)
            }));
        }

        // 4e. Await all tasks in the chunk
        for handle in handles {
            let join_result = handle.await;
            match join_result {
                Ok((url_id, result)) => {
                    // Update stats
                    let status = result.http_status;
                    if (200..300).contains(&status) {
                        stats.ok_count += 1;
                    }
                    if !result.redirect_chain.is_empty() {
                        stats.redirect_count += 1;
                    }
                    if status == 404 {
                        stats.not_found_count += 1;
                    }
                    if result.error.is_some() || status >= 500 {
                        stats.error_count += 1;
                    }
                    if status == 200 && result.title.is_none() {
                        stats.empty_title_count += 1;
                    }
                    if result.response_time_ms > 2000 {
                        stats.slow_count += 1;
                    }

                    // Save to DB
                    let redirect_json = if result.redirect_chain.is_empty() {
                        None
                    } else {
                        Some(serde_json::to_string(&result.redirect_chain).unwrap_or_default())
                    };

                    {
                        let conn = db.connection();
                        let _ = conn.execute(
                            "UPDATE urls SET http_status = ?1, response_time_ms = ?2, \
                             title = ?3, redirect_chain = ?4, error = ?5, \
                             checked_at = datetime('now') WHERE id = ?6",
                            params![
                                result.http_status,
                                result.response_time_ms,
                                result.title,
                                redirect_json,
                                result.error,
                                url_id,
                            ],
                        );
                    }

                    checked += 1;

                    // Emit progress
                    let _ = app.emit(
                        "audit-progress",
                        AuditProgress {
                            project_id: project_id.clone(),
                            checked,
                            total,
                            current_url: result.url.clone(),
                            status: "running".to_string(),
                            stats: stats.clone(),
                        },
                    );
                }
                Err(e) => {
                    // JoinError (task panic or cancellation) -- count as error
                    stats.error_count += 1;
                    checked += 1;
                    let _ = app.emit(
                        "audit-progress",
                        AuditProgress {
                            project_id: project_id.clone(),
                            checked,
                            total,
                            current_url: format!("(task error: {})", e),
                            status: "running".to_string(),
                            stats: stats.clone(),
                        },
                    );
                }
            }
        }

        idx = end;
    }

    // 5. Clean up state and emit done
    cleanup_state(&project_id);

    let _ = app.emit(
        "audit-progress",
        AuditProgress {
            project_id: project_id.clone(),
            checked,
            total,
            current_url: String::new(),
            status: "done".to_string(),
            stats,
        },
    );

    Ok(())
}

#[tauri::command]
pub fn pause_audit(project_id: String) -> Result<(), String> {
    let state = AUDIT_STATE.lock().map_err(|e| e.to_string())?;
    if let Some(ctrl) = state.get(&project_id) {
        ctrl.pause.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub fn resume_audit(project_id: String) -> Result<(), String> {
    let state = AUDIT_STATE.lock().map_err(|e| e.to_string())?;
    if let Some(ctrl) = state.get(&project_id) {
        ctrl.pause.store(false, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub fn stop_audit(project_id: String) -> Result<(), String> {
    let mut state = AUDIT_STATE.lock().map_err(|e| e.to_string())?;
    if let Some(ctrl) = state.get(&project_id) {
        ctrl.cancel.store(true, Ordering::Relaxed);
    }
    // Remove the entry so a new audit can be started after stopping
    state.remove(&project_id);
    Ok(())
}

#[tauri::command]
pub fn update_concurrency(project_id: String, concurrency: u32) -> Result<(), String> {
    let state = AUDIT_STATE.lock().map_err(|e| e.to_string())?;
    if let Some(ctrl) = state.get(&project_id) {
        ctrl.concurrency
            .store(concurrency.max(1), Ordering::Relaxed);
    }
    Ok(())
}

fn cleanup_state(project_id: &str) {
    if let Ok(mut state) = AUDIT_STATE.lock() {
        state.remove(project_id);
    }
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

    fn insert_url_full(
        db: &Database,
        project_id: &str,
        url: &str,
        indexed_status: &str,
        checked_at: Option<&str>,
    ) -> String {
        let conn = db.connection();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO urls (id, project_id, url, source, indexed_status, checked_at) \
             VALUES (?1, ?2, ?3, 'sitemap', ?4, ?5)",
            params![id, project_id, url, indexed_status, checked_at],
        )
        .unwrap();
        id.clone()
    }

    // ── AuditControl creation and cancellation ──────────────────────

    #[test]
    fn test_audit_control_creation() {
        let project_id = format!("audit-ctrl-{}", Uuid::new_v4());
        let cancel = Arc::new(AtomicBool::new(false));
        let pause = Arc::new(AtomicBool::new(false));
        let conc = Arc::new(AtomicU32::new(5));

        {
            let mut state = AUDIT_STATE.lock().unwrap();
            state.insert(
                project_id.clone(),
                AuditControl {
                    cancel: cancel.clone(),
                    pause: pause.clone(),
                    concurrency: conc.clone(),
                },
            );
        }

        // Verify it exists
        {
            let state = AUDIT_STATE.lock().unwrap();
            assert!(state.contains_key(&project_id));
        }

        // Cleanup
        cleanup_state(&project_id);

        {
            let state = AUDIT_STATE.lock().unwrap();
            assert!(!state.contains_key(&project_id));
        }
    }

    #[test]
    fn test_pause_and_resume_audit() {
        let project_id = format!("pause-test-{}", Uuid::new_v4());
        let cancel = Arc::new(AtomicBool::new(false));
        let pause = Arc::new(AtomicBool::new(false));
        let conc = Arc::new(AtomicU32::new(3));

        {
            let mut state = AUDIT_STATE.lock().unwrap();
            state.insert(
                project_id.clone(),
                AuditControl {
                    cancel: cancel.clone(),
                    pause: pause.clone(),
                    concurrency: conc.clone(),
                },
            );
        }

        // Pause
        pause_audit(project_id.clone()).unwrap();
        assert!(pause.load(Ordering::Relaxed), "Pause flag should be true");

        // Resume
        resume_audit(project_id.clone()).unwrap();
        assert!(!pause.load(Ordering::Relaxed), "Pause flag should be false after resume");

        // Cleanup
        cleanup_state(&project_id);
    }

    #[test]
    fn test_stop_audit_sets_cancel_and_removes_state() {
        let project_id = format!("stop-test-{}", Uuid::new_v4());
        let cancel = Arc::new(AtomicBool::new(false));
        let pause = Arc::new(AtomicBool::new(false));
        let conc = Arc::new(AtomicU32::new(3));

        {
            let mut state = AUDIT_STATE.lock().unwrap();
            state.insert(
                project_id.clone(),
                AuditControl {
                    cancel: cancel.clone(),
                    pause: pause.clone(),
                    concurrency: conc.clone(),
                },
            );
        }

        stop_audit(project_id.clone()).unwrap();

        assert!(cancel.load(Ordering::Relaxed), "Cancel flag should be true after stop");

        // State should be removed
        {
            let state = AUDIT_STATE.lock().unwrap();
            assert!(!state.contains_key(&project_id), "State should be removed after stop");
        }
    }

    #[test]
    fn test_update_concurrency() {
        let project_id = format!("conc-test-{}", Uuid::new_v4());
        let cancel = Arc::new(AtomicBool::new(false));
        let pause = Arc::new(AtomicBool::new(false));
        let conc = Arc::new(AtomicU32::new(3));

        {
            let mut state = AUDIT_STATE.lock().unwrap();
            state.insert(
                project_id.clone(),
                AuditControl {
                    cancel: cancel.clone(),
                    pause: pause.clone(),
                    concurrency: conc.clone(),
                },
            );
        }

        update_concurrency(project_id.clone(), 10).unwrap();
        assert_eq!(conc.load(Ordering::Relaxed), 10);

        // Concurrency of 0 should be clamped to 1
        update_concurrency(project_id.clone(), 0).unwrap();
        assert_eq!(conc.load(Ordering::Relaxed), 1, "Concurrency should be clamped to minimum of 1");

        // Cleanup
        cleanup_state(&project_id);
    }

    #[test]
    fn test_pause_nonexistent_project_is_ok() {
        let result = pause_audit("nonexistent".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_resume_nonexistent_project_is_ok() {
        let result = resume_audit("nonexistent".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_stop_nonexistent_project_is_ok() {
        let result = stop_audit("nonexistent".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_concurrency_nonexistent_project_is_ok() {
        let result = update_concurrency("nonexistent".to_string(), 5);
        assert!(result.is_ok());
    }

    // ── DB query tests (what start_audit fetches) ───────────────────

    #[test]
    fn test_audit_query_selects_confirmed_unaudited_only() {
        let db = setup();
        let pid = create_project(&db, "audit-query.com");

        // Should be selected: confirmed + no http_status (not audited)
        insert_url_full(&db, &pid, "https://audit-query.com/a", "confirmed", None);
        insert_url_full(&db, &pid, "https://audit-query.com/b", "confirmed", None);

        // Should also be selected: confirmed with checked_at set (by indexation) but no http_status
        let url_c = insert_url_full(&db, &pid, "https://audit-query.com/c", "confirmed", Some("2024-01-01"));

        // Should NOT be selected: confirmed but already audited (has http_status)
        let url_d = insert_url_full(&db, &pid, "https://audit-query.com/d", "confirmed", None);
        {
            let conn = db.connection();
            conn.execute(
                "UPDATE urls SET http_status = 200, checked_at = datetime('now') WHERE id = ?1",
                params![url_d],
            )
            .unwrap();
        }

        // Should NOT be selected: not confirmed
        insert_url_full(&db, &pid, "https://audit-query.com/e", "unknown", None);
        insert_url_full(&db, &pid, "https://audit-query.com/f", "not_indexed", None);

        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, url FROM urls \
                 WHERE project_id = ?1 AND indexed_status = 'confirmed' AND http_status IS NULL",
            )
            .unwrap();
        let rows: Vec<(String, String)> = stmt
            .query_map(params![pid], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(rows.len(), 3);
        let urls: Vec<&str> = rows.iter().map(|(_, u)| u.as_str()).collect();
        assert!(urls.contains(&"https://audit-query.com/a"));
        assert!(urls.contains(&"https://audit-query.com/b"));
        assert!(urls.contains(&"https://audit-query.com/c"), "URLs with checked_at from indexation should still be auditable");
        let _ = url_c; // used above
    }

    #[test]
    fn test_audit_result_db_update() {
        let db = setup();
        let pid = create_project(&db, "audit-update.com");
        let url_id = insert_url_full(&db, &pid, "https://audit-update.com/page", "confirmed", None);

        // Simulate what audit does after checking a URL
        let conn = db.connection();
        conn.execute(
            "UPDATE urls SET http_status = ?1, response_time_ms = ?2, \
             title = ?3, redirect_chain = ?4, error = ?5, \
             checked_at = datetime('now') WHERE id = ?6",
            params![200i32, 350i64, "My Page Title", Option::<String>::None, Option::<String>::None, url_id],
        )
        .unwrap();

        let (http_status, response_time, title, checked_at): (
            Option<i32>,
            Option<i64>,
            Option<String>,
            Option<String>,
        ) = conn
            .query_row(
                "SELECT http_status, response_time_ms, title, checked_at FROM urls WHERE id = ?1",
                params![url_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();

        assert_eq!(http_status, Some(200));
        assert_eq!(response_time, Some(350));
        assert_eq!(title.as_deref(), Some("My Page Title"));
        assert!(checked_at.is_some());
    }

    #[test]
    fn test_audit_result_db_update_with_redirect() {
        let db = setup();
        let pid = create_project(&db, "redir-audit.com");
        let url_id = insert_url_full(&db, &pid, "https://redir-audit.com/old", "confirmed", None);

        let redirect_json = serde_json::to_string(&vec![
            "https://redir-audit.com/old",
            "https://redir-audit.com/new",
        ])
        .unwrap();

        let conn = db.connection();
        conn.execute(
            "UPDATE urls SET http_status = ?1, response_time_ms = ?2, \
             title = ?3, redirect_chain = ?4, error = ?5, \
             checked_at = datetime('now') WHERE id = ?6",
            params![301i32, 120i64, Option::<String>::None, Some(&redirect_json), Option::<String>::None, url_id],
        )
        .unwrap();

        let (http_status, chain): (Option<i32>, Option<String>) = conn
            .query_row(
                "SELECT http_status, redirect_chain FROM urls WHERE id = ?1",
                params![url_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(http_status, Some(301));
        assert!(chain.is_some());
        let parsed: Vec<String> = serde_json::from_str(&chain.unwrap()).unwrap();
        assert_eq!(parsed.len(), 2);
    }

    #[test]
    fn test_audit_result_db_update_with_error() {
        let db = setup();
        let pid = create_project(&db, "err-audit.com");
        let url_id = insert_url_full(&db, &pid, "https://err-audit.com/broken", "confirmed", None);

        let conn = db.connection();
        conn.execute(
            "UPDATE urls SET http_status = ?1, response_time_ms = ?2, \
             title = ?3, redirect_chain = ?4, error = ?5, \
             checked_at = datetime('now') WHERE id = ?6",
            params![0i32, 0i64, Option::<String>::None, Option::<String>::None, Some("Connection refused"), url_id],
        )
        .unwrap();

        let error: Option<String> = conn
            .query_row(
                "SELECT error FROM urls WHERE id = ?1",
                params![url_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(error.as_deref(), Some("Connection refused"));
    }

    // ── AuditStats counting logic ───────────────────────────────────

    #[test]
    fn test_audit_stats_counting() {
        // This tests the in-memory stats counting logic that mirrors what start_audit does.
        let mut stats = AuditStats::default();

        // Simulate a 200 OK response with title
        let status = 200;
        if (200..300).contains(&status) {
            stats.ok_count += 1;
        }

        // Simulate a 301 redirect
        let _status = 301;
        let has_redirects = true;
        if has_redirects {
            stats.redirect_count += 1;
        }

        // Simulate a 404
        let status = 404;
        if status == 404 {
            stats.not_found_count += 1;
        }

        // Simulate a 500 error
        let status = 500;
        let has_error = false;
        if has_error || status >= 500 {
            stats.error_count += 1;
        }

        // Simulate a 200 with no title
        let status = 200;
        let has_title = false;
        if status == 200 && !has_title {
            stats.empty_title_count += 1;
        }

        // Simulate slow response
        let response_time_ms: i64 = 3500;
        if response_time_ms > 2000 {
            stats.slow_count += 1;
        }

        assert_eq!(stats.ok_count, 1);
        assert_eq!(stats.redirect_count, 1);
        assert_eq!(stats.not_found_count, 1);
        assert_eq!(stats.error_count, 1);
        assert_eq!(stats.empty_title_count, 1);
        assert_eq!(stats.slow_count, 1);
    }

    #[test]
    fn test_audit_stats_default_is_zero() {
        let stats = AuditStats::default();
        assert_eq!(stats.ok_count, 0);
        assert_eq!(stats.redirect_count, 0);
        assert_eq!(stats.not_found_count, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.empty_title_count, 0);
        assert_eq!(stats.slow_count, 0);
    }
}
