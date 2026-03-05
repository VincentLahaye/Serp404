use rusqlite::params;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use crate::crawler::csv_import;
use crate::crawler::csv_import::CsvColumn;
use crate::crawler::serper;
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

#[tauri::command]
pub fn detect_csv_columns(content: String) -> Result<Vec<CsvColumn>, String> {
    csv_import::detect_url_columns(&content)
}

#[tauri::command]
pub async fn collect_from_csv(
    app: AppHandle,
    db: State<'_, Database>,
    project_id: String,
    content: String,
    column_index: usize,
) -> Result<usize, String> {
    // 1. Extract URLs from the specified column
    let urls = csv_import::extract_urls_from_column(&content, column_index)?;

    // 2. Emit progress: running
    let _ = app.emit(
        "collection-progress",
        CollectionProgress {
            project_id: project_id.clone(),
            source: "csv".to_string(),
            urls_found: 0,
            status: "running".to_string(),
            message: Some(format!("Importing {} URLs from CSV", urls.len())),
        },
    );

    // 3. INSERT OR IGNORE each URL into urls table
    let count = {
        let conn = db.connection();
        let mut inserted = 0usize;
        for url in &urls {
            let id = Uuid::new_v4().to_string();
            let result = conn.execute(
                "INSERT OR IGNORE INTO urls (id, project_id, url, source, indexed_status) VALUES (?1, ?2, ?3, 'csv', 'unknown')",
                params![id, project_id, url],
            );
            match result {
                Ok(changes) => inserted += changes,
                Err(e) => {
                    let _ = app.emit(
                        "collection-progress",
                        CollectionProgress {
                            project_id: project_id.clone(),
                            source: "csv".to_string(),
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

    // 4. Emit progress: done
    let _ = app.emit(
        "collection-progress",
        CollectionProgress {
            project_id: project_id.clone(),
            source: "csv".to_string(),
            urls_found: count,
            status: "done".to_string(),
            message: Some(format!("Imported {} URLs from CSV", count)),
        },
    );

    // 5. Return count
    Ok(count)
}

#[tauri::command]
pub async fn collect_from_serper(
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

    // 2. Get serper API key from settings table
    let api_key = {
        let conn = db.connection();
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'serper_api_key'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map_err(|_| "No serper.dev API key configured".to_string())?
    };

    // 3. Emit progress: running
    let _ = app.emit(
        "collection-progress",
        CollectionProgress {
            project_id: project_id.clone(),
            source: "serper".to_string(),
            urls_found: 0,
            status: "running".to_string(),
            message: Some(format!("Searching Google index for {}", domain)),
        },
    );

    // 4. Paginate through site: results
    let num: u32 = 100;
    let max_pages: u32 = 50;
    let mut total_inserted: usize = 0;
    let client = reqwest::Client::new();

    for page in 1..=max_pages {
        let response =
            match serper::search_indexed_urls(&client, &api_key, &domain, page, num).await {
                Ok(r) => r,
                Err(e) => {
                    let _ = app.emit(
                        "collection-progress",
                        CollectionProgress {
                            project_id: project_id.clone(),
                            source: "serper".to_string(),
                            urls_found: total_inserted,
                            status: "error".to_string(),
                            message: Some(format!("Serper API error on page {}: {}", page, e)),
                        },
                    );
                    return Err(e);
                }
            };

        let results = match response.organic {
            Some(ref r) if !r.is_empty() => r.clone(),
            _ => break, // No organic results, stop paginating
        };

        let results_count = results.len();

        // Insert URLs into the database
        {
            let conn = db.connection();
            for result in &results {
                let id = Uuid::new_v4().to_string();
                let insert_result = conn.execute(
                    "INSERT OR IGNORE INTO urls (id, project_id, url, source, indexed_status) VALUES (?1, ?2, ?3, 'serper', 'confirmed')",
                    params![id, project_id, result.link],
                );
                match insert_result {
                    Ok(changes) => total_inserted += changes,
                    Err(e) => {
                        let _ = app.emit(
                            "collection-progress",
                            CollectionProgress {
                                project_id: project_id.clone(),
                                source: "serper".to_string(),
                                urls_found: total_inserted,
                                status: "error".to_string(),
                                message: Some(format!("DB insert error: {}", e)),
                            },
                        );
                        return Err(format!("Failed to insert URL: {}", e));
                    }
                }
            }
        }

        // Emit progress with updated count
        let _ = app.emit(
            "collection-progress",
            CollectionProgress {
                project_id: project_id.clone(),
                source: "serper".to_string(),
                urls_found: total_inserted,
                status: "running".to_string(),
                message: Some(format!(
                    "Page {}: found {} results ({} total new URLs)",
                    page, results_count, total_inserted
                )),
            },
        );

        // Stop when results < num (last page)
        if results_count < num as usize {
            break;
        }
    }

    // 5. Emit progress: done
    let _ = app.emit(
        "collection-progress",
        CollectionProgress {
            project_id: project_id.clone(),
            source: "serper".to_string(),
            urls_found: total_inserted,
            status: "done".to_string(),
            message: Some(format!(
                "Found {} indexed URLs from Google via Serper",
                total_inserted
            )),
        },
    );

    // 6. Return total URL count
    Ok(total_inserted)
}
