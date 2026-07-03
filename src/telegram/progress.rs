use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct ProgressThrottle {
    min_interval: Duration,
    last_edit: Option<Instant>,
}

impl ProgressThrottle {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last_edit: None,
        }
    }

    pub fn should_edit(&mut self) -> bool {
        let now = Instant::now();
        match self.last_edit {
            Some(last) if now.duration_since(last) < self.min_interval => false,
            _ => {
                self.last_edit = Some(now);
                true
            }
        }
    }
}

pub fn render_progress(total: Option<u64>, completed: u64, failed: u64) -> String {
    match total {
        Some(total) if total > 0 => {
            let width = 20_usize;
            let filled = ((completed.min(total) * width as u64) / total) as usize;
            format!(
                "[{}{}] {}/{} item - xong: {} lỗi: {}",
                "#".repeat(filled),
                "-".repeat(width - filled),
                completed,
                total,
                completed,
                failed
            )
        }
        _ => format!("Đang quét... xong: {completed} lỗi: {failed}"),
    }
}
