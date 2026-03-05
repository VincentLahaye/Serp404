mod commands;
mod crawler;
mod db;
mod models;

use db::Database;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            use tauri::Manager;

            let app_data_dir = app.path().app_data_dir().expect("failed to get app data dir");
            std::fs::create_dir_all(&app_data_dir).expect("failed to create app data dir");

            let db_path = app_data_dir.join("serp404.db");
            let database = Database::new(
                db_path.to_str().expect("invalid db path"),
            )
            .expect("failed to initialize database");

            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::projects::create_project,
            commands::projects::list_projects,
            commands::projects::get_project,
            commands::projects::delete_project,
            commands::settings::get_setting,
            commands::settings::set_setting,
            commands::settings::get_all_settings,
            commands::settings::test_serper_key,
            commands::collection::collect_from_sitemap,
            commands::collection::detect_csv_columns,
            commands::collection::collect_from_csv,
            commands::collection::collect_from_serper,
            commands::indexation::get_unverified_count,
            commands::indexation::verify_indexation,
            commands::indexation::stop_indexation,
            commands::audit::start_audit,
            commands::audit::pause_audit,
            commands::audit::resume_audit,
            commands::audit::stop_audit,
            commands::audit::update_concurrency,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
