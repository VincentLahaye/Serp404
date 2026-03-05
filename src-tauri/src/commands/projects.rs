use std::collections::HashMap;

use rusqlite::params;
use tauri::State;
use uuid::Uuid;

use crate::db::Database;
use crate::models::Project;

#[tauri::command]
pub fn create_project(db: State<'_, Database>, domain: String) -> Result<Project, String> {
    let id = Uuid::new_v4().to_string();
    let conn = db.connection();

    conn.execute(
        "INSERT INTO projects (id, domain) VALUES (?1, ?2)",
        params![id, domain],
    )
    .map_err(|e| e.to_string())?;

    let project = conn
        .query_row(
            "SELECT id, domain, status, created_at, updated_at FROM projects WHERE id = ?1",
            params![id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    domain: row.get(1)?,
                    status: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(project)
}

#[tauri::command]
pub fn list_projects(db: State<'_, Database>) -> Result<Vec<Project>, String> {
    let conn = db.connection();

    let mut stmt = conn
        .prepare("SELECT id, domain, status, created_at, updated_at FROM projects ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;

    let projects = stmt
        .query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                domain: row.get(1)?,
                status: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(projects)
}

#[tauri::command]
pub fn get_project(db: State<'_, Database>, id: String) -> Result<Project, String> {
    let conn = db.connection();

    let project = conn
        .query_row(
            "SELECT id, domain, status, created_at, updated_at FROM projects WHERE id = ?1",
            params![id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    domain: row.get(1)?,
                    status: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(project)
}

#[tauri::command]
pub fn get_url_counts_by_source(
    db: State<'_, Database>,
    project_id: String,
) -> Result<HashMap<String, usize>, String> {
    let conn = db.connection();
    let mut stmt = conn
        .prepare("SELECT source, COUNT(*) FROM urls WHERE project_id = ?1 GROUP BY source")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![project_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
        })
        .map_err(|e| e.to_string())?;

    let mut counts = HashMap::new();
    for row in rows {
        let (source, count) = row.map_err(|e| e.to_string())?;
        counts.insert(source, count);
    }

    Ok(counts)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlListEntry {
    pub id: String,
    pub url: String,
    pub source: String,
    pub indexed_status: String,
}

#[tauri::command]
pub fn get_project_urls(
    db: State<'_, Database>,
    project_id: String,
    source: Option<String>,
    indexed_status: Option<String>,
) -> Result<Vec<UrlListEntry>, String> {
    let conn = db.connection();

    let mut query = "SELECT id, url, source, indexed_status FROM urls WHERE project_id = ?1".to_string();
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(project_id)];

    if let Some(ref s) = source {
        values.push(Box::new(s.clone()));
        query.push_str(&format!(" AND source = ?{}", values.len()));
    }
    if let Some(ref is) = indexed_status {
        values.push(Box::new(is.clone()));
        query.push_str(&format!(" AND indexed_status = ?{}", values.len()));
    }

    query.push_str(" ORDER BY url ASC");

    let mut stmt = conn.prepare(&query).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();

    let rows = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(UrlListEntry {
                id: row.get(0)?,
                url: row.get(1)?,
                source: row.get(2)?,
                indexed_status: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_project(db: State<'_, Database>, id: String) -> Result<(), String> {
    let conn = db.connection();

    let changes = conn
        .execute("DELETE FROM projects WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    if changes == 0 {
        return Err(format!("Project with id '{}' not found", id));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use rusqlite::params;
    use uuid::Uuid;

    fn setup() -> Database {
        Database::new(":memory:").unwrap()
    }

    #[test]
    fn test_create_and_get_project() {
        let db = setup();
        let conn = db.connection();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, domain) VALUES (?1, ?2)",
            params![id, "example.com"],
        )
        .unwrap();

        let domain: String = conn
            .query_row(
                "SELECT domain FROM projects WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(domain, "example.com");
    }

    #[test]
    fn test_project_default_status_is_created() {
        let db = setup();
        let conn = db.connection();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, domain) VALUES (?1, ?2)",
            params![id, "example.com"],
        )
        .unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM projects WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(status, "created");
    }

    #[test]
    fn test_project_timestamps_auto_set() {
        let db = setup();
        let conn = db.connection();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, domain) VALUES (?1, ?2)",
            params![id, "example.com"],
        )
        .unwrap();

        let (created_at, updated_at): (String, String) = conn
            .query_row(
                "SELECT created_at, updated_at FROM projects WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(!created_at.is_empty(), "created_at should be auto-populated");
        assert!(!updated_at.is_empty(), "updated_at should be auto-populated");
    }

    #[test]
    fn test_list_projects_ordered() {
        let db = setup();
        let conn = db.connection();
        // Insert two projects with different timestamps
        conn.execute(
            "INSERT INTO projects (id, domain, created_at) VALUES (?1, ?2, '2024-01-01')",
            params![Uuid::new_v4().to_string(), "first.com"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO projects (id, domain, created_at) VALUES (?1, ?2, '2024-02-01')",
            params![Uuid::new_v4().to_string(), "second.com"],
        )
        .unwrap();

        let mut stmt = conn
            .prepare("SELECT domain FROM projects ORDER BY created_at DESC")
            .unwrap();
        let domains: Vec<String> = stmt
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert_eq!(domains, vec!["second.com", "first.com"]);
    }

    #[test]
    fn test_delete_project_cascades_urls() {
        let db = setup();
        let conn = db.connection();
        let pid = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, domain) VALUES (?1, ?2)",
            params![pid, "test.com"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO urls (id, project_id, url, source) VALUES (?1, ?2, ?3, ?4)",
            params![
                Uuid::new_v4().to_string(),
                pid,
                "https://test.com/page",
                "sitemap"
            ],
        )
        .unwrap();

        conn.execute("DELETE FROM projects WHERE id = ?1", params![pid])
            .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1",
                params![pid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "URLs should be deleted by CASCADE");
    }

    #[test]
    fn test_delete_project_cascades_many_urls() {
        let db = setup();
        let conn = db.connection();
        let pid = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, domain) VALUES (?1, ?2)",
            params![pid, "multi.com"],
        )
        .unwrap();

        for i in 0..5 {
            conn.execute(
                "INSERT INTO urls (id, project_id, url, source) VALUES (?1, ?2, ?3, ?4)",
                params![
                    Uuid::new_v4().to_string(),
                    pid,
                    format!("https://multi.com/page{}", i),
                    "sitemap"
                ],
            )
            .unwrap();
        }

        let before: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1",
                params![pid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(before, 5);

        conn.execute("DELETE FROM projects WHERE id = ?1", params![pid])
            .unwrap();

        let after: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1",
                params![pid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(after, 0, "All 5 URLs should be cascade-deleted");
    }

    #[test]
    fn test_duplicate_project_url_ignored() {
        let db = setup();
        let conn = db.connection();
        let pid = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, domain) VALUES (?1, ?2)",
            params![pid, "test.com"],
        )
        .unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO urls (id, project_id, url, source) VALUES (?1, ?2, ?3, ?4)",
            params![
                Uuid::new_v4().to_string(),
                pid,
                "https://test.com/page",
                "sitemap"
            ],
        )
        .unwrap();
        // Second insert with same URL should be ignored
        conn.execute(
            "INSERT OR IGNORE INTO urls (id, project_id, url, source) VALUES (?1, ?2, ?3, ?4)",
            params![
                Uuid::new_v4().to_string(),
                pid,
                "https://test.com/page",
                "csv"
            ],
        )
        .unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM urls WHERE project_id = ?1",
                params![pid],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_delete_nonexistent_project_returns_zero_changes() {
        let db = setup();
        let conn = db.connection();
        let changes = conn
            .execute(
                "DELETE FROM projects WHERE id = ?1",
                params!["nonexistent-id"],
            )
            .unwrap();
        assert_eq!(changes, 0, "Deleting a non-existent project should affect 0 rows");
    }

    #[test]
    fn test_get_all_project_columns() {
        let db = setup();
        let conn = db.connection();
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (id, domain) VALUES (?1, ?2)",
            params![id, "full-test.com"],
        )
        .unwrap();

        let (rid, domain, status, created_at, updated_at): (
            String,
            String,
            String,
            String,
            String,
        ) = conn
            .query_row(
                "SELECT id, domain, status, created_at, updated_at FROM projects WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(rid, id);
        assert_eq!(domain, "full-test.com");
        assert_eq!(status, "created");
        assert!(!created_at.is_empty());
        assert!(!updated_at.is_empty());
    }
}
