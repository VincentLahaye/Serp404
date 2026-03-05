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
    // 1. Query URLs where indexed_status = 'confirmed' AND checked_at IS NULL
    let urls: Vec<(String, String)> = {
        let conn = db.connection();
        let mut stmt = conn
            .prepare(
                "SELECT id, url FROM urls \
                 WHERE project_id = ?1 AND indexed_status = 'confirmed' AND checked_at IS NULL",
            )
            .map_err(|e| e.to_string())?;

        stmt.query_map(params![project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
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
