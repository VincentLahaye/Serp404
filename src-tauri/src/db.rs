use rusqlite::{Connection, Result};
use std::sync::Mutex;

pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                domain TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'created',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS urls (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                url TEXT NOT NULL,
                source TEXT NOT NULL,
                indexed_status TEXT NOT NULL DEFAULT 'unknown',
                http_status INTEGER,
                response_time_ms INTEGER,
                title TEXT,
                redirect_chain TEXT,
                error TEXT,
                checked_at TEXT,
                UNIQUE(project_id, url)
            );
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
        ",
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_in_memory_database() {
        let db = Database::new(":memory:");
        assert!(db.is_ok(), "Should create in-memory database successfully");
    }

    #[test]
    fn test_all_tables_exist() {
        let db = Database::new(":memory:").unwrap();
        let conn = db.conn.lock().unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"projects".to_string()), "projects table should exist");
        assert!(tables.contains(&"urls".to_string()), "urls table should exist");
        assert!(tables.contains(&"settings".to_string()), "settings table should exist");
    }

    #[test]
    fn test_projects_schema() {
        let db = Database::new(":memory:").unwrap();
        let conn = db.conn.lock().unwrap();

        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(projects)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        let expected = vec!["id", "domain", "status", "created_at", "updated_at"];
        for col in &expected {
            assert!(
                columns.contains(&col.to_string()),
                "projects table should have column '{}'",
                col
            );
        }
    }

    #[test]
    fn test_urls_schema() {
        let db = Database::new(":memory:").unwrap();
        let conn = db.conn.lock().unwrap();

        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(urls)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        let expected = vec![
            "id",
            "project_id",
            "url",
            "source",
            "indexed_status",
            "http_status",
            "response_time_ms",
            "title",
            "redirect_chain",
            "error",
            "checked_at",
        ];
        for col in &expected {
            assert!(
                columns.contains(&col.to_string()),
                "urls table should have column '{}'",
                col
            );
        }
    }

    #[test]
    fn test_settings_schema() {
        let db = Database::new(":memory:").unwrap();
        let conn = db.conn.lock().unwrap();

        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(settings)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        let expected = vec!["key", "value"];
        for col in &expected {
            assert!(
                columns.contains(&col.to_string()),
                "settings table should have column '{}'",
                col
            );
        }
    }
}
