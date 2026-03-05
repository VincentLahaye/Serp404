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

        stmt.query_map(params![project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
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
    let client = reqwest::Client::new();

    for (i, (url_id, url)) in urls.iter().enumerate() {
        // 4a. Check cancellation token
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
            // Clean up cancellation token
            let mut tokens = CANCEL_TOKENS.lock().map_err(|e| e.to_string())?;
            tokens.remove(&project_id);
            return Ok(confirmed_count);
        }

        // 4b. Call serper::check_url_indexed
        let is_indexed = serper::check_url_indexed(&client, &api_key, url).await?;

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
                "UPDATE urls SET indexed_status = ?1, checked_at = datetime('now') WHERE id = ?2",
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
