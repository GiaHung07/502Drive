import { invoke } from '@tauri-apps/api/core'
import type {
  SystemStatus,
  JobSummary,
  WatchSummary,
  ConfigSummary,
  DoctorResult,
  PreflightReport,
  RemoteUpdateInfo,
  WizardConfigInput,
  TelegramBotVerifyResult,
} from './types'

// Check if running inside Tauri window
export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)
}

// Mock data exists ONLY for browser dev / visual testing. In a Tauri build a
// failing IPC call must surface as a real error, never as fake "all green" data.
const USE_MOCKS = !isTauri() && import.meta.env.DEV

/** Normalize unknown IPC errors (string | Error | serialized Rust error) into user-safe text. */
export function getErrorMessage(err: unknown): string {
  if (typeof err === 'string') return err
  if (err && typeof err === 'object' && 'message' in err) return String((err as { message: unknown }).message)
  try {
    return JSON.stringify(err)
  } catch {
    return String(err)
  }
}

const MOCK_UNAVAILABLE = 'Chỉ khả dụng trong ứng dụng 502Drive (chế độ dev trình duyệt không hỗ trợ).'

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
    speed_bytes_per_sec: 42_500_000,
    eta_seconds: 340,
    created_at_ms: Date.now() - 1000 * 60 * 25,
    updated_at_ms: Date.now() - 1000 * 5,
  },
  {
    id: 'job-1a2b3c4d-5e6f-7a8b-9c0d-ef1234567890',
    short_id: '1a2b3c4d',
    kind: 'one_shot',
    status: 'paused',
    source_root_id: '1Z9y8X7w6V5u4T3s2R1qPoNmLkJiHgFeD',
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
    if (isTauri()) return await invoke<SystemStatus>('get_system_status')
    if (USE_MOCKS) return { ...mockSystemStatus }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async listJobs(limit = 20): Promise<JobSummary[]> {
    if (isTauri()) return await invoke<JobSummary[]>('list_jobs', { limit })
    if (USE_MOCKS) return [...mockJobs.slice(0, limit)]
    throw new Error(MOCK_UNAVAILABLE)
  },

  async pauseJob(jobId: string): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('pause_job', { jobId })
    }
    if (USE_MOCKS) {
      mockJobs = mockJobs.map((j) => (j.id === jobId ? { ...j, status: 'paused' } : j))
      return
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async resumeJob(jobId: string): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('resume_job', { jobId })
    }
    if (USE_MOCKS) {
      mockJobs = mockJobs.map((j) => (j.id === jobId ? { ...j, status: 'running' } : j))
      return
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async cancelJob(jobId: string): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('cancel_job', { jobId })
    }
    if (USE_MOCKS) {
      mockJobs = mockJobs.map((j) => (j.id === jobId ? { ...j, status: 'cancelled' } : j))
      return
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async listWatches(): Promise<WatchSummary[]> {
    if (isTauri()) return await invoke<WatchSummary[]>('list_watches')
    if (USE_MOCKS) return [...mockWatches]
    throw new Error(MOCK_UNAVAILABLE)
  },

  async pauseWatch(watchId: string): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('pause_watch', { watchId })
    }
    if (USE_MOCKS) {
      mockWatches = mockWatches.map((w) => (w.id === watchId ? { ...w, status: 'paused' } : w))
      return
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async resumeWatch(watchId: string): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('resume_watch', { watchId })
    }
    if (USE_MOCKS) {
      mockWatches = mockWatches.map((w) => (w.id === watchId ? { ...w, status: 'active' } : w))
      return
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async getConfig(): Promise<ConfigSummary> {
    if (isTauri()) return await invoke<ConfigSummary>('get_config_summary')
    if (USE_MOCKS) return { ...mockConfig }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async updateConfig(field: string, value: unknown): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('update_config_field', { field, value: String(value) })
    }
    if (USE_MOCKS) {
      mockConfig = { ...mockConfig, [field]: value }
      return
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async saveWizardConfig(input: WizardConfigInput): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('save_wizard_config', { input })
    }
    if (USE_MOCKS) {
      if (input.owner_telegram_id) mockConfig.owner_telegram_id = input.owner_telegram_id
      if (input.bot_token) {
        mockConfig.bot_token = input.bot_token
        mockConfig.bot_token_configured = true
      }
      if (input.engine_concurrency) mockConfig.engine_concurrency = input.engine_concurrency
      if (input.auto_confirm_clone !== undefined) mockConfig.auto_confirm_clone = input.auto_confirm_clone
      return
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async verifyTelegramBot(token: string): Promise<TelegramBotVerifyResult> {
    if (isTauri()) {
      return await invoke<TelegramBotVerifyResult>('verify_telegram_bot', { token })
    }
    // Browser dev convenience only — never ships in production webview usage.
    if (USE_MOCKS) {
      try {
        const res = await fetch(`https://api.telegram.org/bot${token.trim()}/getMe`)
        const data = await res.json()
        if (data.ok) {
          return {
            ok: true,
            username: data.result.username,
            first_name: data.result.first_name,
            bot_id: data.result.id,
          }
        }
        return { ok: false, error: data.description || 'Token không hợp lệ hoặc bot không tồn tại' }
      } catch (e) {
        return { ok: false, error: getErrorMessage(e) }
      }
    }
    return { ok: false, error: MOCK_UNAVAILABLE }
  },

  async startService(): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('start_service')
    }
    if (USE_MOCKS) return console.log('[Dev Mock] Service started')
    throw new Error(MOCK_UNAVAILABLE)
  },

  async restartService(): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('restart_service')
    }
    if (USE_MOCKS) return console.log('[Dev Mock] Systemd service restarted')
    throw new Error(MOCK_UNAVAILABLE)
  },

  async openTelegramBot(): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('open_telegram_bot')
    }
    if (USE_MOCKS) {
      window.open('https://t.me/Drive502_Bot', '_blank')
      return
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async triggerLogin(): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('trigger_auth_login')
    }
    if (USE_MOCKS) return alert('[Dev Mock] Mở trình duyệt đăng nhập Google OAuth')
    throw new Error(MOCK_UNAVAILABLE)
  },

  async triggerRevoke(): Promise<void> {
    if (isTauri()) {
      return await invoke<void>('trigger_auth_revoke')
    }
    if (USE_MOCKS) {
      mockSystemStatus.google_account = null
      mockSystemStatus.account_status = 'disconnected'
      return
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async runPreflightCheck(): Promise<PreflightReport> {
    if (isTauri()) return await invoke<PreflightReport>('run_preflight_check')
    if (USE_MOCKS) {
      return {
        all_passed: true,
        needs_setup: false,
        steps: [
          {
            id: 'storage',
            title: 'Hệ thống tệp & thư mục lưu trữ',
            status: 'passed',
            message: 'Các thư mục lưu trữ cục bộ sẵn sàng',
            auto_fixed: false,
          },
          {
            id: 'database',
            title: 'Cơ sở dữ liệu SQLite (WAL)',
            status: 'passed',
            message: 'Cơ sở dữ liệu toàn vẹn, chế độ WAL hoạt động',
            auto_fixed: false,
          },
          {
            id: 'config',
            title: 'Tệp cấu hình ứng dụng',
            status: 'passed',
            message: 'Cấu hình hợp lệ và đã sẵn sàng',
            auto_fixed: false,
          },
          {
            id: 'service',
            title: 'Dịch vụ chạy nền (Daemon)',
            status: 'passed',
            message: 'Service gdclone-bot đang chạy ổn định',
            auto_fixed: false,
          },
          {
            id: 'google_oauth',
            title: 'Tài khoản Google Drive',
            status: 'passed',
            message: 'Đã liên kết tài khoản developer@502drive.dev',
            auto_fixed: false,
          },
        ],
        remote_update: {
          current_version: 'v0.1.0',
          latest_version: 'v0.1.0',
          update_available: false,
          changelog: 'Phiên bản ổn định mới nhất.',
        },
      }
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async checkRemoteUpdate(): Promise<RemoteUpdateInfo> {
    if (isTauri()) return await invoke<RemoteUpdateInfo>('check_remote_update')
    if (USE_MOCKS) {
      return {
        current_version: 'v0.1.0',
        latest_version: 'v0.1.0',
        update_available: false,
        changelog: 'Bạn đang sử dụng phiên bản mới nhất v0.1.0.',
      }
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async runDoctor(): Promise<DoctorResult> {
    if (isTauri()) return await invoke<DoctorResult>('run_doctor')
    if (USE_MOCKS) {
      return {
        timestamp_ms: Date.now(),
        checks: [
          { name: 'Cấu hình ứng dụng', passed: true, detail: 'Đã tải (~/.config/gdclone-bot/config.toml)' },
          { name: 'Cơ sở dữ liệu', passed: true, detail: 'Toàn vẹn WAL (~/.local/share/gdclone-bot/state.db)' },
          { name: 'Telegram Bot', passed: true, detail: '@Drive502_Bot (Dịch vụ sẵn sàng)' },
          { name: 'Tài khoản Google', passed: true, detail: 'Đã kết nối (Token hợp lệ)' },
          { name: 'Thư mục đích', passed: true, detail: 'My Drive / Backup 502' },
          { name: 'Dịch vụ nền', passed: true, detail: 'gdclone-bot.service đang hoạt động' },
        ],
        raw_output: `$ 502drive doctor\n502Drive doctor\n===============\n[OK] Cấu hình: Đã tải\n  Đường dẫn DB : ~/.local/share/gdclone-bot/state.db\n  Thư mục Log  : ~/.local/share/gdclone-bot/logs\n[OK] Cơ sở dữ liệu: Toàn vẹn WAL\n[OK] Tác vụ: Đang hoạt động\n[OK] Telegram: @Drive502_Bot\n[OK] Google Account: Đã kết nối\n[OK] Thư mục đích: My Drive / Backup 502\nToàn bộ kiểm tra đạt chuẩn. Hệ thống sẵn sàng.\n`,
      }
    }
    throw new Error(MOCK_UNAVAILABLE)
  },

  async getLogs(lines = 50): Promise<string[]> {
    if (isTauri()) return await invoke<string[]>('get_recent_logs', { lines })
    if (USE_MOCKS) {
      return [
        `[${new Date().toISOString()}] [INFO] gdclone_bot::engine: Daemon started, polling worker initialized`,
        `[${new Date().toISOString()}] [INFO] gdclone_bot::drive: Token refresh check: valid (expires in 48m)`,
        `[${new Date().toISOString()}] [INFO] gdclone_bot::telegram: Telegram bot long polling connected`,
        `[${new Date().toISOString()}] [INFO] gdclone_bot::engine::worker: Processing job 9a8b7c6d: 8233/12847 files (64.1%)`,
        `[${new Date().toISOString()}] [INFO] gdclone_bot::watch: Change event stream caught up at sequence 1204`,
      ]
    }
    throw new Error(MOCK_UNAVAILABLE)
  },
}
