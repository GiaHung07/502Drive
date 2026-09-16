pub mod commands;
pub mod events;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::status::get_system_status,
            commands::jobs::list_jobs,
            commands::jobs::pause_job,
            commands::jobs::resume_job,
            commands::jobs::cancel_job,
            commands::watches::list_watches,
            commands::watches::pause_watch,
            commands::watches::resume_watch,
            commands::config::get_config_summary,
            commands::config::update_config_field,
            commands::service::restart_service,
            commands::service::open_telegram_bot,
            commands::service::trigger_auth_login,
            commands::service::trigger_auth_revoke,
            commands::doctor::run_doctor,
            commands::logs::get_recent_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running 502Drive application");
}
