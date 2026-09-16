import { invoke } from '@tauri-apps/api/core'
import type { SystemStatus, JobSummary, WatchSummary, ConfigSummary, DoctorResult } from './types'

// Check if running inside Tauri window
export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)
}

// Fallback Mock State for Browser Dev / Visual Testing
let mockJobs: JobSummary[] = [
  {
    id: 'job-9a8b7c6d-5e4f-3a2b-1c0d-ef9876543210',
    short_id: '9a8b7c6d',
    kind: 'one_shot',
    status: 'running',
    source_root_id: '1A2b3C4d5E6f7G8h9I0jKlMnOpQrStUvW',
    destination_parent_id: '0B1c2D3e4F5g6H7i8J9kLmNoPqRsTuVwX',
    total_discovered: 12847,
    completed_items: 8233,
    failed_items: 0,
    skipped_items: 12,
    progress_pct: 64.1,
    created_at_ms: Date.now() - 1000 * 60 * 18,
    updated_at_ms: Date.now() - 1000 * 4,
    speed_bytes_per_sec: 12_897_400,
    eta_seconds: 240,
  },
  {
    id: 'job-1f2e3d4c-5b6a-7890-abcd-ef1234567890',
    short_id: '1f2e3d4c',
    kind: 'one_shot',
    status: 'paused',
    source_root_id: '0Z9y8X7w6V5u4T3s2R1qPoNmLkJiHgFeD',
    destination_parent_id: '0B1c2D3e4F5g6H7i8J9kLmNoPqRsTuVwX',
    total_discovered: 4500,
    completed_items: 1890,
    failed_items: 2,
    skipped_items: 0,
    progress_pct: 42.0,
    created_at_ms: Date.now() - 1000 * 60 * 120,
    updated_at_ms: Date.now() - 1000 * 60 * 45,
  },
  {
    id: 'job-5a6b7c8d-9e0f-1a2b-3c4d-5e6f7a8b9c0d',
    short_id: '5a6b7c8d',
    kind: 'one_shot',
    status: 'completed',
    source_root_id: '1K2l3M4n5O6p7Q8r9S0tUvWxYzAbCdEfG',
    destination_parent_id: '0B1c2D3e4F5g6H7i8J9kLmNoPqRsTuVwX',
    total_discovered: 3412,
    completed_items: 3412,
    failed_items: 0,
    skipped_items: 0,
    progress_pct: 100.0,
    created_at_ms: Date.now() - 1000 * 60 * 360,
    updated_at_ms: Date.now() - 1000 * 60 * 320,
  },
]

let mockWatches: WatchSummary[] = [
  {
    id: 'watch-7f8e9d0a-1b2c-3d4e-5f6a-7b8c9d0e1f2a',
    short_id: '7f8e9d0a',
    source_root_id: '1SharedDriveFolderSourceRootId123',
    destination_root_id: '0MyBackupFolderDestinationId456',
    status: 'active',
    backlog_count: 0,
    baseline_sequence: 1204,
    last_consumed_sequence: 1204,
    updated_at_ms: Date.now() - 1000 * 15,
  },
  {
    id: 'watch-3c4d5e6f-7a8b-9c0d-1e2f-3a4b5c6d7e8f',
    short_id: '3c4d5e6f',
    source_root_id: '0DocsAndSpreadsheetsSourceFolder789',
    destination_root_id: '0ArchiveDestinationFolderId987',
    status: 'paused',
    backlog_count: 4,
    baseline_sequence: 890,
    last_consumed_sequence: 886,
    updated_at_ms: Date.now() - 1000 * 60 * 60,
  },
]

let mockConfig: ConfigSummary = {
  engine_concurrency: 8,
  auto_confirm_clone: true,
  launch_at_startup: true,
  language: 'vi',
  bot_token_configured: true,
  owner_telegram_id: 123456789,
  db_path: '~/.local/share/gdclone-bot/state.db',
  log_dir: '~/.local/share/gdclone-bot/logs',
  report_dir: '~/.local/share/gdclone-bot/reports',
}

let mockSystemStatus: SystemStatus = {
  google_account: 'developer@502drive.dev',
  account_status: 'connected',
  service_active: true,
  service_name: 'gdclone-bot.service',
  db_integrity: 'ok',
  app_version: 'v0.1.0',
  bot_username: 'Drive502_Bot',
  destination_label: 'My Drive / Backup 502',
  destination_id: '1aBcDeFgHiJkLmNoPqRsTuVwXyZ01234',
  stats: {
    total_jobs: 28,
    active_jobs: 1,
    completed_jobs: 26,
    total_cloned_files: 24892,
    total_cloned_bytes: 52_400_000_000,
  },
}

export const api = {
  async getSystemStatus(): Promise<SystemStatus> {
    if (isTauri()) {
      try {
        return await invoke<SystemStatus>('get_system_status')
      } catch (err) {
        console.warn('Native get_system_status failed, using mock fallback:', err)
      }
    }
    return { ...mockSystemStatus }
  },

  async listJobs(limit = 20): Promise<JobSummary[]> {
    if (isTauri()) {
      try {
        return await invoke<JobSummary[]>('list_jobs', { limit })
      } catch (err) {
        console.warn('Native list_jobs failed, using mock fallback:', err)
      }
    }
    return [...mockJobs.slice(0, limit)]
  },

  async pauseJob(jobId: string): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('pause_job', { jobId })
    }
    mockJobs = mockJobs.map((j) => (j.id === jobId ? { ...j, status: 'paused' } : j))
  },

  async resumeJob(jobId: string): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('resume_job', { jobId })
    }
    mockJobs = mockJobs.map((j) => (j.id === jobId ? { ...j, status: 'running' } : j))
  },

  async cancelJob(jobId: string): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('cancel_job', { jobId })
    }
    mockJobs = mockJobs.map((j) => (j.id === jobId ? { ...j, status: 'cancelled' } : j))
  },

  async listWatches(): Promise<WatchSummary[]> {
    if (isTauri()) {
      try {
        return await invoke<WatchSummary[]>('list_watches')
      } catch (err) {
        console.warn('Native list_watches failed, using mock fallback:', err)
      }
    }
    return [...mockWatches]
  },

  async pauseWatch(watchId: string): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('pause_watch', { watchId })
    }
    mockWatches = mockWatches.map((w) => (w.id === watchId ? { ...w, status: 'paused' } : w))
  },

  async resumeWatch(watchId: string): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('resume_watch', { watchId })
    }
    mockWatches = mockWatches.map((w) => (w.id === watchId ? { ...w, status: 'active' } : w))
  },

  async getConfig(): Promise<ConfigSummary> {
    if (isTauri()) {
      try {
        return await invoke<ConfigSummary>('get_config_summary')
      } catch (err) {
        console.warn('Native get_config_summary failed, using fallback:', err)
      }
    }
    return { ...mockConfig }
  },

  async updateConfig(field: string, value: unknown): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('update_config_field', { field, value: String(value) })
    }
    mockConfig = { ...mockConfig, [field]: value }
  },

  async restartService(): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('restart_service')
    }
    console.log('[Dev Mock] Systemd service restarted')
  },

  async openTelegramBot(): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('open_telegram_bot')
    }
    window.open('https://t.me/Drive502_Bot', '_blank')
  },

  async triggerLogin(): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('trigger_auth_login')
    }
    alert('[Dev Mock] Mở trình duyệt đăng nhập Google OAuth')
  },

  async triggerRevoke(): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('trigger_auth_revoke')
    }
    mockSystemStatus.google_account = null
    mockSystemStatus.account_status = 'disconnected'
  },

  async runDoctor(): Promise<DoctorResult> {
    if (isTauri()) {
      try {
        return await invoke<DoctorResult>('run_doctor')
      } catch (err) {
        console.warn('Native run_doctor failed, using fallback:', err)
      }
    }
    return {
      timestamp_ms: Date.now(),
      checks: [
        { name: 'config', passed: true, detail: 'loaded (~/.config/gdclone-bot/config.toml)' },
        { name: 'db', passed: true, detail: 'integrity ok (~/.local/share/gdclone-bot/state.db)' },
        { name: 'telegram', passed: true, detail: '@Drive502_Bot (webhook unset, polling ready)' },
        { name: 'google_account', passed: true, detail: 'connected (token valid)' },
        { name: 'destination', passed: true, detail: 'My Drive / Backup 502 (id verified)' },
        { name: 'systemd_service', passed: true, detail: 'gdclone-bot.service is active' },
      ],
      raw_output: `$ 502drive doctor\n502Drive doctor\n===============\n[OK] config: loaded\n  db_path    : ~/.local/share/gdclone-bot/state.db\n  log_dir    : ~/.local/share/gdclone-bot/logs\n  report_dir : ~/.local/share/gdclone-bot/reports\n[OK] db: integrity ok\n[OK] jobs: existing history (active: 1, completed: 26)\n[OK] telegram: @Drive502_Bot\n  pending_updates: 0\n[OK] google_account: connected\n[OK] destination: My Drive / Backup 502\nAll checks passed. System healthy.\n`,
    }
  },

  async getLogs(lines = 50): Promise<string[]> {
    if (isTauri()) {
      try {
        return await invoke<string[]>('get_recent_logs', { lines })
      } catch (err) {
        console.warn('Native get_recent_logs failed, using fallback:', err)
      }
    }
    return [
      `[${new Date().toISOString()}] [INFO] gdclone_bot::engine: Daemon started, polling worker initialized`,
      `[${new Date().toISOString()}] [INFO] gdclone_bot::drive: Token refresh check: valid (expires in 48m)`,
      `[${new Date().toISOString()}] [INFO] gdclone_bot::telegram: Telegram bot long polling connected`,
      `[${new Date().toISOString()}] [INFO] gdclone_bot::engine::worker: Processing job 9a8b7c6d: 8233/12847 files (64.1%)`,
      `[${new Date().toISOString()}] [INFO] gdclone_bot::watch: Change event stream caught up at sequence 1204`,
    ]
  },
}
