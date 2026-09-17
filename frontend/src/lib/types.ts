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
  /** New bridge fields — optional so existing dev mocks keep compiling. */
  exclude_globs?: string[]
  cursor_last_event_sequence?: number | null
}

// ── GUI ↔ Engine bridge (ui_requests queue) ─────────────────────────────────

export type CloneDuplicatePolicy = 'keep_both' | 'skip_same_source' | 'replace_safe'

export type WatchPolicyKind = 'content_update' | 'deletion' | 'move_out'

/** A Drive folder/file reference returned by `browse_drive_children`. */
export interface DriveItemRef {
  id: string
  name: string
  is_folder: boolean
  /** Set on shared-drive entries; pass back when listing that folder's children. */
  drive_id?: string
}

export type UiRequestState = 'pending' | 'accepted' | 'rejected'

/** Decision record for a queued GUI request. `note` carries the rejection
 *  reason on 'rejected' and the job id / watch id on 'accepted'. */
export interface UiRequestStatus {
  request_id: string
  kind: string
  status: UiRequestState
  note: string | null
  created_at_ms: number
  decided_at_ms: number | null
}

export interface CreateRequestResult {
  request_id: string
}

export interface ConfigSummary {
  engine_concurrency: number
  auto_confirm_clone: boolean
  launch_at_startup: boolean
  language: 'vi' | 'en'
  bot_token_configured: boolean
  bot_token?: string
  owner_telegram_id: number
  oauth_client_id?: string
  oauth_client_secret_configured?: boolean
  db_path: string
  log_dir: string
  report_dir: string
}

export interface WizardConfigInput {
  oauth_client_id?: string
  oauth_client_secret?: string
  bot_token?: string
  owner_telegram_id?: number
  engine_concurrency?: number
  auto_confirm_clone?: boolean
  sa_directory?: string
  shared_drive_id?: string
}

export interface TelegramBotVerifyResult {
  ok: boolean
  username?: string | null
  first_name?: string | null
  bot_id?: number | null
  error?: string | null
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

export interface PreflightStep {
  id: string
  title: string
  status: 'pending' | 'running' | 'passed' | 'warning' | 'failed'
  message: string
  auto_fixed?: boolean
  fix_action?: 'login' | 'start_service' | 'config'
}

export interface RemoteUpdateInfo {
  current_version: string
  latest_version: string
  update_available: boolean
  changelog: string
  download_url?: string
}

export interface PreflightReport {
  all_passed: boolean
  needs_setup: boolean
  steps: PreflightStep[]
  remote_update?: RemoteUpdateInfo
}

export interface AuthorizedUserDto {
  telegram_user_id: number
  role: string
  enabled: boolean
}

export interface BackupInfoDto {
  name: string
  timestamp_ms: number
  size_bytes: number
  files_count: number
}

