use std::collections::HashMap;

use rusqlite::params;
use tauri::State;

use crate::db::Database;

#[tauri::command]
pub fn get_setting(db: State<'_, Database>, key: String) -> Result<Option<String>, String> {
    let conn = db.connection();

    let result = conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        params![key],
        |row| row.get::<_, String>(0),
    );

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn set_setting(db: State<'_, Database>, key: String, value: String) -> Result<(), String> {
    let conn = db.connection();

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_all_settings(db: State<'_, Database>) -> Result<HashMap<String, String>, String> {
    let conn = db.connection();

    let mut stmt = conn
        .prepare("SELECT key, value FROM settings")
        .map_err(|e| e.to_string())?;

    let settings = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<HashMap<String, String>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(settings)
}

#[tauri::command]
pub async fn test_serper_key(api_key: String) -> Result<bool, String> {
    let client = reqwest::Client::new();

    let response = client
        .post("https://google.serper.dev/search")
        .header("X-API-KEY", &api_key)
        .json(&serde_json::json!({"q": "test", "num": 1}))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(response.status().as_u16() == 200)
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use rusqlite::params;
    use std::collections::HashMap;

    fn setup() -> Database {
        Database::new(":memory:").unwrap()
    }

    #[test]
    fn test_set_and_get_setting() {
        let db = setup();
        let conn = db.connection();

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["theme", "dark"],
        )
        .unwrap();

        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params!["theme"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "dark");
    }

    #[test]
    fn test_get_missing_setting_returns_no_rows() {
        let db = setup();
        let conn = db.connection();

        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params!["nonexistent"],
            |row| row.get::<_, String>(0),
        );

        assert!(
            matches!(result, Err(rusqlite::Error::QueryReturnedNoRows)),
            "Missing key should return QueryReturnedNoRows"
        );
    }

    #[test]
    fn test_upsert_setting_overwrites() {
        let db = setup();
        let conn = db.connection();

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["api_key", "old_key"],
        )
        .unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["api_key", "new_key"],
        )
        .unwrap();

        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params!["api_key"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "new_key");

        // Verify there is only one row for this key
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM settings WHERE key = ?1",
                params!["api_key"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_all_settings_empty() {
        let db = setup();
        let conn = db.connection();

        let mut stmt = conn.prepare("SELECT key, value FROM settings").unwrap();
        let settings: HashMap<String, String> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(settings.is_empty(), "Freshly created DB should have no settings");
    }

    #[test]
    fn test_get_all_settings_multiple() {
        let db = setup();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            params!["key_a", "value_a"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            params!["key_b", "value_b"],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            params!["key_c", "value_c"],
        )
        .unwrap();

        let mut stmt = conn.prepare("SELECT key, value FROM settings").unwrap();
        let settings: HashMap<String, String> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert_eq!(settings.len(), 3);
        assert_eq!(settings.get("key_a").unwrap(), "value_a");
        assert_eq!(settings.get("key_b").unwrap(), "value_b");
        assert_eq!(settings.get("key_c").unwrap(), "value_c");
    }

    #[test]
    fn test_setting_value_can_be_empty_string() {
        let db = setup();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            params!["empty_val", ""],
        )
        .unwrap();

        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params!["empty_val"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "");
    }

    #[test]
    fn test_setting_value_with_special_characters() {
        let db = setup();
        let conn = db.connection();

        let special = "sk-abc123!@#$%^&*()_+{}|:<>?";
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params!["serper_api_key", special],
        )
        .unwrap();

        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params!["serper_api_key"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, special);
    }

    #[test]
    fn test_delete_setting() {
        let db = setup();
        let conn = db.connection();

        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            params!["to_delete", "some_value"],
        )
        .unwrap();

        conn.execute(
            "DELETE FROM settings WHERE key = ?1",
            params!["to_delete"],
        )
        .unwrap();

        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params!["to_delete"],
            |row| row.get::<_, String>(0),
        );
        assert!(matches!(result, Err(rusqlite::Error::QueryReturnedNoRows)));
    }
}
