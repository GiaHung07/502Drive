use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Clone)]
pub struct JobProgressEvent {
    pub job_id: String,
    pub progress_pct: f64,
    pub status: String,
    pub completed_items: i64,
    pub total_discovered: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ServiceStatusEvent {
    pub active: bool,
    pub service_name: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AccountStatusEvent {
    pub connected: bool,
    pub email: Option<String>,
}

pub fn emit_job_progress(app: &AppHandle, event: JobProgressEvent) {
    let _ = app.emit("job:progress", event);
}

pub fn emit_service_status(app: &AppHandle, active: bool) {
    let _ = app.emit(
        "service:status",
        ServiceStatusEvent {
            active,
            service_name: "gdclone-bot".to_string(),
        },
    );
}
