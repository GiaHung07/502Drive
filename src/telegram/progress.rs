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

/// Render a one-line progress bar with counts.
///
/// When `total` is known: `[####----------------] 4/20 (20%) done:4 err:0`
/// When scanning:          `Scanning… done:4 err:0`
pub fn render_progress(total: Option<u64>, completed: u64, failed: u64) -> String {
    match total {
        Some(total) if total > 0 => {
            let width = 20_usize;
            let filled = ((completed.min(total) * width as u64) / total) as usize;
            let pct = (completed.min(total) * 100) / total;
            format!(
                "[{}{}] {}/{} ({}%) done:{} err:{}",
                "#".repeat(filled),
                "-".repeat(width - filled),
                completed,
                total,
                pct,
                completed,
                failed,
            )
        }
        _ => format!("Scanning… done:{completed} err:{failed}"),
    }
}

/// Format seconds as `HH:MM:SS` or `MM:SS`.
pub fn format_duration_secs(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Estimate ETA in seconds from (completed, total, elapsed_secs).
/// Returns None if there is not enough data or rate is 0.
pub fn estimate_eta_secs(completed: u64, total: u64, elapsed_secs: u64) -> Option<u64> {
    if completed == 0 || elapsed_secs == 0 || completed >= total {
        return None;
    }
    let rate = completed as f64 / elapsed_secs as f64; // items/sec
    if rate < 0.001 {
        return None;
    }
    let remaining = total.saturating_sub(completed) as f64;
    Some((remaining / rate).ceil() as u64)
}
