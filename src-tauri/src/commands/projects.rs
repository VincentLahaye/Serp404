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
