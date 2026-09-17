pub mod commands;
pub mod events;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::status::get_system_status,
            commands::jobs::list_jobs,
            commands::jobs::pause_job,
            commands::jobs::resume_job,
            commands::jobs::cancel_job,
            commands::watches::list_watches,
            commands::watches::pause_watch,
            commands::watches::resume_watch,
            commands::watches::unwatch,
            commands::watches::set_watch_policy,
            commands::features::create_clone_request,
            commands::features::create_watch_request,
            commands::features::retry_job,
            commands::features::get_ui_request_status,
            commands::features::browse_drive_children,
            commands::config::get_config_summary,
            commands::config::update_config_field,
            commands::config::save_wizard_config,
            commands::config::verify_telegram_bot,
            commands::service::start_service,
            commands::service::restart_service,
            commands::service::open_telegram_bot,
            commands::service::trigger_auth_login,
            commands::service::trigger_auth_revoke,
            commands::doctor::run_preflight_check,
            commands::doctor::check_remote_update,
            commands::doctor::run_doctor,
            commands::logs::get_recent_logs,
            commands::admin::set_default_destination,
            commands::admin::list_authorized_users,
            commands::admin::add_authorized_user,
            commands::admin::batch_add_authorized_users,
            commands::admin::toggle_authorized_user,
            commands::admin::remove_authorized_user,
            commands::admin::backup_database,
            commands::admin::list_backups,
            commands::admin::restore_backup,
            commands::admin::delete_backup,
            commands::admin::vacuum_database,
            commands::admin::clear_app_logs,
            commands::admin::apply_remote_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running 502Drive application");
}
