export type ServiceStatus = 'active' | 'inactive' | 'failed' | 'unknown'

export type AccountStatus = 'connected' | 'reconnect_required' | 'revoked' | 'disabled' | 'disconnected'

export type JobStatus =
  | 'queued'
  | 'discovering'
  | 'running'
  | 'pausing'
  | 'paused'
  | 'cancelling'
  | 'cancelled'
  | 'recovering'
  | 'completed'
  | 'partially_completed'
  | 'failed'

export type WatchStatus =
  | 'initializing'
  | 'catching_up'
  | 'active'
  | 'paused'
  | 'degraded'
  | 'needs_reconcile'
  | 'stopped'

export interface SystemStatus {
  google_account: string | null
  account_status: AccountStatus
  service_active: boolean
  service_name: string
  db_integrity: string
  app_version: string
  bot_username?: string | null
  destination_label?: string | null
  destination_id?: string | null
  stats: {
    total_jobs: number
    active_jobs: number
    completed_jobs: number
    total_cloned_files: number
    total_cloned_bytes: number
  }
}

export interface JobSummary {
  id: string
  short_id: string
  kind: string
  status: JobStatus
  source_root_id: string
  destination_parent_id: string
  total_discovered: number
  completed_items: number
  failed_items: number
  skipped_items: number
  progress_pct: number
  error_summary?: string | null
  created_at_ms: number
  updated_at_ms: number
  speed_bytes_per_sec?: number
  eta_seconds?: number
}

export interface WatchSummary {
  id: string
  short_id: string
  source_root_id: string
  destination_root_id: string
  status: WatchStatus
  backlog_count: number
  baseline_sequence: number
  last_consumed_sequence: number
  updated_at_ms: number
}

export interface ConfigSummary {
  engine_concurrency: number
  auto_confirm_clone: boolean
  launch_at_startup: boolean
  language: 'vi' | 'en'
  bot_token_configured: boolean
  owner_telegram_id: number
  db_path: string
  log_dir: string
  report_dir: string
}

export interface DoctorCheckItem {
  name: string
  passed: boolean
  detail: string
}

export interface DoctorResult {
  timestamp_ms: number
  checks: DoctorCheckItem[]
  raw_output: string
}

export interface ToastMessage {
  id: string
  title: string
  description?: string
  variant?: 'default' | 'success' | 'warning' | 'error'
  duration?: number
}
