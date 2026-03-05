use rusqlite::params;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::crawler::sitemap;
use crate::db::Database;

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CollectionProgress {
    project_id: String,
    source: String,
    urls_found: usize,
    status: String, // "running" | "done" | "error"
    message: Option<String>,
}

#[tauri::command]
pub async fn collect_from_sitemap(
    app: AppHandle,
    db: State<'_, Database>,
    project_id: String,
) -> Result<usize, String> {
    // 1. Get project domain from DB
    let domain = {
        let conn = db.connection();
        conn.query_row(
            "SELECT domain FROM projects WHERE id = ?1",
            params![project_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|e| format!("Project not found: {}", e))?
    };

    // 2. Emit progress: running
    let _ = app.emit(
        "collection-progress",
        CollectionProgress {
            project_id: project_id.clone(),
            source: "sitemap".to_string(),
            urls_found: 0,
            status: "running".to_string(),
            message: Some(format!("Fetching sitemap for {}", domain)),
        },
    );

    // 3. Fetch sitemap URLs
    let urls = match sitemap::fetch_sitemap_urls(&domain).await {
        Ok(urls) => urls,
        Err(e) => {
            let _ = app.emit(
                "collection-progress",
                CollectionProgress {
                    project_id: project_id.clone(),
                    source: "sitemap".to_string(),
                    urls_found: 0,
                    status: "error".to_string(),
                    message: Some(e.clone()),
                },
            );
            return Err(e);
        }
    };

    // 4. Insert URLs into the database
    let count = {
        let conn = db.connection();
        let mut inserted = 0usize;
        for url in &urls {
            let id = Uuid::new_v4().to_string();
            let result = conn.execute(
                "INSERT OR IGNORE INTO urls (id, project_id, url, source, indexed_status) VALUES (?1, ?2, ?3, 'sitemap', 'unknown')",
                params![id, project_id, url],
            );
            match result {
                Ok(changes) => inserted += changes,
                Err(e) => {
                    let _ = app.emit(
                        "collection-progress",
                        CollectionProgress {
                            project_id: project_id.clone(),
                            source: "sitemap".to_string(),
                            urls_found: inserted,
                            status: "error".to_string(),
                            message: Some(format!("DB insert error: {}", e)),
                        },
                    );
                    return Err(format!("Failed to insert URL: {}", e));
                }
            }
        }
        inserted
    };

    // 5. Emit progress: done
    let _ = app.emit(
        "collection-progress",
        CollectionProgress {
            project_id: project_id.clone(),
            source: "sitemap".to_string(),
            urls_found: count,
            status: "done".to_string(),
            message: Some(format!("Found {} URLs from sitemap", count)),
        },
    );

    // 6. Return count
    Ok(count)
}
