import React, { useState, useEffect, useRef } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import {
  X,
  CheckCircle2,
  AlertCircle,
  HardDrive,
  Send,
  Sparkles,
  ArrowRight,
  ArrowLeft,
  Loader2,
  ExternalLink,
  Clipboard,
  ShieldCheck,
  Zap,
  Key,
  Check,
  Eye,
  EyeOff,
  Bot,
  Clock,
} from 'lucide-react'
import { Button } from '@/components/ui/Button'
import { Card } from '@/components/ui/Card'
import { useToast } from '@/components/primitives/Toast'
import { api, getErrorMessage } from '@/lib/ipc'
import { ConfigSummary, SystemStatus, TelegramBotVerifyResult } from '@/lib/types'

export interface SetupWizardModalProps {
  isOpen: boolean
  onClose: () => void
  config: ConfigSummary | null
  status: SystemStatus | null
  onTriggerLogin: () => void
  onRefreshData: () => Promise<void>
  onRestartService: () => Promise<void>
}

type SetupMethod = 'preset_oauth' | 'service_account' | 'custom_oauth'

export const SetupWizardModal: React.FC<SetupWizardModalProps> = ({
  isOpen,
  onClose,
  config,
  status,
  onTriggerLogin,
  onRefreshData,
  onRestartService,
}) => {
  const [currentStep, setCurrentStep] = useState<1 | 2 | 3 | 4>(1)
  const [method, setMethod] = useState<SetupMethod>('preset_oauth')

  // Step 2: Custom OAuth form
  const [clientId, setClientId] = useState(config?.oauth_client_id || '')
  const [clientSecret, setClientSecret] = useState('')
  const [showSecret, setShowSecret] = useState(false)
  const [isVerifyingDrive, setIsVerifyingDrive] = useState(false)
  const [driveVerified, setDriveVerified] = useState(status?.account_status === 'connected')
  const [driveVerifyError, setDriveVerifyError] = useState<string | null>(null)

  // Step 2: Service account — feature not yet supported; the UI states this honestly

  // Step 3: Telegram bot form
  const [botToken, setBotToken] = useState(config?.bot_token || '')
  const [ownerId, setOwnerId] = useState(
    config?.owner_telegram_id && config.owner_telegram_id > 0 ? String(config.owner_telegram_id) : ''
  )
  const [isVerifyingBot, setIsVerifyingBot] = useState(false)
  const [botVerifyResult, setBotVerifyResult] = useState<TelegramBotVerifyResult | null>(null)

  // Step 4: Finalizing
  const [isSaving, setIsSaving] = useState(false)
  const [isComplete, setIsComplete] = useState(false)

  const { toast } = useToast()
  const pollCancelledRef = useRef(false)

  useEffect(() => {
    return () => {
      pollCancelledRef.current = true
    }
  }, [])

  // Sync state if external status updates
  useEffect(() => {
    if (status?.account_status === 'connected') {
      setDriveVerified(true)
      setDriveVerifyError(null)
    } else if (status?.account_status) {
      // A real, fresh status that is not "connected" invalidates stale success.
      setDriveVerified(false)
    }
  }, [status?.account_status])

  useEffect(() => {
    if (config?.bot_token && !botToken) {
      setBotToken(config.bot_token)
    }
    if (config?.owner_telegram_id && !ownerId) {
      setOwnerId(String(config.owner_telegram_id))
    }
  }, [config])

  if (!isOpen) return null

  // Paste helper
  const handlePaste = async (setter: (val: string) => void) => {
    try {
      const text = await navigator.clipboard.readText()
      if (text) setter(text.trim())
    } catch {
      // Ignore if clipboard access denied
    }
  }

  // Single-shot Drive status check (no artificial delay, no parallel race with OAuth)
  const handleCheckDriveStatus = async () => {
    if (isVerifyingDrive) return
    setIsVerifyingDrive(true)
    setDriveVerifyError(null)
    try {
      await onRefreshData()
      const current = await api.getSystemStatus()
      if (current.account_status === 'connected') {
        setDriveVerified(true)
      } else {
        setDriveVerifyError('Chưa phát hiện phiên đăng nhập Google. Vui lòng thử lại.')
      }
    } catch (e) {
      setDriveVerifyError(getErrorMessage(e))
    } finally {
      setIsVerifyingDrive(false)
    }
  }

  // Fire the OAuth flow, then poll until the browser round-trip lands (or timeout).
  const handleLoginAndVerify = async () => {
    if (isVerifyingDrive) return
    onTriggerLogin()
    pollCancelledRef.current = false
    setIsVerifyingDrive(true)
    setDriveVerifyError(null)
    try {
      for (let i = 0; i < 36; i++) {
        if (pollCancelledRef.current) return
        await new Promise((r) => setTimeout(r, 2500))
        const current = await api.getSystemStatus()
        if (current.account_status === 'connected') {
          setDriveVerified(true)
          return
        }
      }
      setDriveVerifyError('Hết thời gian chờ đăng nhập Google. Vui lòng thử lại.')
    } catch (e) {
      setDriveVerifyError(getErrorMessage(e))
    } finally {
      setIsVerifyingDrive(false)
    }
  }

  // Live Verify Telegram Bot
  const handleVerifyTelegramBot = async () => {
    if (!botToken.trim()) {
      setBotVerifyResult({ ok: false, error: 'Vui lòng nhập Telegram Bot Token' })
      return
    }
    setIsVerifyingBot(true)
    setBotVerifyResult(null)
    try {
      const res = await api.verifyTelegramBot(botToken.trim())
      setBotVerifyResult(res)
    } catch (err) {
      setBotVerifyResult({ ok: false, error: getErrorMessage(err) })
    } finally {
      setIsVerifyingBot(false)
    }
  }

  // Final submit
  const handleFinishSetup = async () => {
    setIsSaving(true)
    try {
      const parsedOwnerId = ownerId.trim() ? parseInt(ownerId.trim(), 10) : undefined
      await api.saveWizardConfig({
        oauth_client_id: method === 'custom_oauth' && clientId ? clientId.trim() : undefined,
        oauth_client_secret: method === 'custom_oauth' && clientSecret ? clientSecret.trim() : undefined,
        bot_token: botToken.trim() ? botToken.trim() : undefined,
        owner_telegram_id:
          parsedOwnerId !== undefined && Number.isSafeInteger(parsedOwnerId) && parsedOwnerId > 0
            ? parsedOwnerId
            : undefined,
        engine_concurrency: config?.engine_concurrency ?? 8,
        auto_confirm_clone: config?.auto_confirm_clone ?? true,
      })

      // Restart service to apply immediately
      await onRestartService()
      await onRefreshData()

      setIsComplete(true)
      setTimeout(() => {
        setIsSaving(false)
        onClose()
      }, 1200)
    } catch (e) {
      setIsSaving(false)
      toast({
        title: 'Lỗi lưu thiết lập',
        description: getErrorMessage(e),
        variant: 'error',
      })
    }
  }

  // Per-step completion gates — "Tiếp tục" only advances on verified state.
  const canContinueStep2 = driveVerified
  const canContinueStep3 = botVerifyResult?.ok === true && ownerId.trim().length > 0
  const canFinish = driveVerified || botVerifyResult?.ok === true

  const steps = [
    { num: 1, label: 'Phương thức' },
    { num: 2, label: 'Google Drive' },
    { num: 3, label: 'Telegram Bot' },
    { num: 4, label: 'Hoàn tất' },
  ]

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs">
      <motion.div
        initial={{ opacity: 0, scale: 0.96, y: 8 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.96, y: 8 }}
        transition={{ duration: 0.15, ease: 'easeOut' }}
        className="w-full max-w-2xl bg-bg-elevated border border-border/80 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]"
      >
        {/* Modal Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-border/60 bg-bg-card/50">
          <div className="flex items-center gap-2.5">
            <div className="p-2 rounded-xl bg-accent/10 text-accent">
              <Sparkles className="h-4 w-4 stroke-[2]" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-text-primary">
                Trình Hướng Dẫn Thiết Lập Nhanh (Setup Wizard)
              </h2>
              <p className="text-[0.6875rem] text-text-secondary">
                Cấu hình kết nối 1-chạm thông minh, xác thực tức thì cho 502Drive
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-input/60 transition-colors cursor-pointer"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Stepper Progress Bar */}
        <div className="px-6 py-3 bg-bg-card/30 border-b border-border/40">
          <div className="flex items-center justify-between relative">
            {steps.map((s, idx) => {
              const isActive = currentStep === s.num
              const isPassed = currentStep > s.num
              return (
                <React.Fragment key={s.num}>
                  <div className="flex items-center gap-2 z-10">
                    <div
                      className={`w-6 h-6 rounded-full flex items-center justify-center text-xs font-semibold transition-colors ${
                        isPassed
                          ? 'bg-success text-white'
                          : isActive
                          ? 'bg-accent text-white shadow-xs'
                          : 'bg-bg-input text-text-muted border border-border/60'
                      }`}
                    >
                      {isPassed ? <Check className="h-3 w-3 stroke-[3]" /> : s.num}
                    </div>
                    <span
                      className={`text-xs font-medium hidden sm:inline ${
                        isActive ? 'text-text-primary font-semibold' : 'text-text-secondary'
                      }`}
                    >
                      {s.label}
                    </span>
                  </div>
                  {idx < steps.length - 1 && (
                    <div
                      className={`flex-1 h-[2px] mx-2 transition-colors ${
                        currentStep > idx + 1 ? 'bg-success' : 'bg-border/60'
                      }`}
                    />
                  )}
                </React.Fragment>
              )
            })}
          </div>
        </div>

        {/* Modal Body: Scrollable steps */}
        <div className="p-6 overflow-y-auto space-y-5 flex-1">
          {/* ── BƯỚC 1: Chọn Phương thức ───────────────────────── */}
          {currentStep === 1 && (
            <div className="space-y-4">
              <div className="space-y-1">
                <h3 className="text-sm font-semibold text-text-primary">
                  1. Chọn phương thức xác thực Google Drive
                </h3>
                <p className="text-xs text-text-secondary">
                  502Drive hỗ trợ 3 cơ chế kết nối tối ưu cho từng nhu cầu sử dụng:
                </p>
              </div>

              <div className="grid grid-cols-1 gap-3">
                {/* Method 1: Preset OAuth (1-Chạm) */}
                <div
                  onClick={() => setMethod('preset_oauth')}
                  className={`p-4 rounded-xl border transition-all cursor-pointer flex items-start gap-3.5 ${
                    method === 'preset_oauth'
                      ? 'bg-accent/5 border-accent shadow-xs'
                      : 'bg-bg-card border-border/70 hover:border-border'
                  }`}
                >
                  <div className="p-2 rounded-lg bg-accent/10 text-accent shrink-0 mt-0.5">
                    <Zap className="h-4 w-4" />
                  </div>
                  <div className="space-y-1 flex-1">
                    <div className="flex items-center justify-between">
                      <p className="text-sm font-semibold text-text-primary">
                        Đăng nhập 1-Chạm Mặc định
                      </p>
                      <span className="text-[0.625rem] font-semibold uppercase tracking-wider px-2 py-0.5 rounded-full bg-success/10 text-success border border-success/25">
                        Khuyên dùng (10 Giây)
                      </span>
                    </div>
                    <p className="text-xs text-text-secondary leading-relaxed">
                      Không cần tạo tài khoản Google Cloud, không cần thiết lập phức tạp. Bạn chỉ cần bấm Đăng nhập & cấp quyền. Token được mã hóa AES-256-GCM cục bộ trên máy bạn.
                    </p>
                  </div>
                </div>

                {/* Method 2: Service Accounts — NOT yet supported by the engine; shown honestly as coming soon */}
                <div
                  aria-disabled="true"
                  className={`p-4 rounded-xl border flex items-start gap-3.5 opacity-60 saturate-50 cursor-not-allowed select-none ${
                    'bg-bg-card border-border/70'
                  }`}
                  title="Tính năng đang được hoàn thiện"
                >
                  <div className="p-2 rounded-lg bg-info/10 text-info shrink-0 mt-0.5">
                    <HardDrive className="h-4 w-4" />
                  </div>
                  <div className="space-y-1 flex-1">
                    <div className="flex items-center justify-between">
                      <p className="text-sm font-semibold text-text-primary">
                        Service Accounts (SA) - Chuẩn Reddit r/DataHoarder
                      </p>
                      <span className="text-[0.625rem] font-semibold uppercase tracking-wider px-2 py-0.5 rounded-full bg-warning/10 text-warning border border-warning/25 flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        Sắp ra mắt
                      </span>
                    </div>
                    <p className="text-xs text-text-secondary leading-relaxed">
                      Dành cho tải dữ liệu lớn, bỏ qua giới hạn quota tài khoản cá nhân. Phương thức này đang được hoàn thiện và sẽ sớm hỗ trợ nạp file JSON/ZIP chứa nhiều SA để tự động xoay vòng.
                    </p>
                  </div>
                </div>

                {/* Method 3: Custom OAuth */}
                <div
                  onClick={() => setMethod('custom_oauth')}
                  className={`p-4 rounded-xl border transition-all cursor-pointer flex items-start gap-3.5 ${
                    method === 'custom_oauth'
                      ? 'bg-accent/5 border-accent shadow-xs'
                      : 'bg-bg-card border-border/70 hover:border-border'
                  }`}
                >
                  <div className="p-2 rounded-lg bg-info/10 text-info shrink-0 mt-0.5">
                    <Key className="h-4 w-4" />
                  </div>
                  <div className="space-y-1 flex-1">
                    <div className="flex items-center justify-between">
                      <p className="text-sm font-semibold text-text-primary">
                        Custom Google Cloud OAuth (Lập trình viên)
                      </p>
                      <span className="text-[0.625rem] font-semibold uppercase tracking-wider px-2 py-0.5 rounded-full bg-info/10 text-info border border-info/25">
                        Tự cấu hình
                      </span>
                    </div>
                    <p className="text-xs text-text-secondary leading-relaxed">
                      Sử dụng Client ID & Secret từ Google Cloud Console cá nhân của bạn để sở hữu 100% hạn mức API độc lập.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* ── BƯỚC 2: Kết nối & Xác thực Google Drive ────────── */}
          {currentStep === 2 && (
            <div className="space-y-4">
              <div className="space-y-1">
                <h3 className="text-sm font-semibold text-text-primary">
                  2. Xác thực và Liên kết Google Drive
                </h3>
                <p className="text-xs text-text-secondary">
                  {method === 'preset_oauth' && 'Bấm đăng nhập để mở trình duyệt cấp quyền an toàn.'}
                  {method === 'service_account' && 'Kéo thả file SA (.json hoặc .zip) để nạp tài khoản Service.'}
                  {method === 'custom_oauth' && 'Điền Client ID và Client Secret từ Google Cloud Console.'}
                </p>
              </div>

              {/* Form Method: 1-Chạm */}
              {method === 'preset_oauth' && (
                <Card className="p-5 space-y-4 bg-bg-card border border-border/70">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className="p-3 rounded-xl bg-accent/10 text-accent">
                        <HardDrive className="h-5 w-5" />
                      </div>
                      <div>
                        <p className="text-sm font-medium text-text-primary">
                          Ủy quyền truy cập Google Drive
                        </p>
                        <p className="text-xs text-text-secondary font-mono">
                          {status?.google_account ? status.google_account : 'Chưa có tài khoản liên kết'}
                        </p>
                      </div>
                    </div>

                    <Button
                      size="sm"
                      variant="primary"
                      onClick={handleLoginAndVerify}
                      className="text-xs gap-1.5 rounded-xl"
                    >
                      <ExternalLink className="h-3.5 w-3.5" />
                      <span>{status?.google_account ? 'Đăng nhập lại' : 'Đăng nhập Google'}</span>
                    </Button>
                  </div>

                  {/* Live Status Feedback */}
                  <div className="pt-2 border-t border-border/40 flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      {isVerifyingDrive ? (
                        <>
                          <Loader2 className="h-4 w-4 animate-spin text-accent" />
                          <span className="text-xs text-text-secondary">Đang kiểm tra kết nối Google...</span>
                        </>
                      ) : driveVerified ? (
                        <>
                          <CheckCircle2 className="h-4 w-4 text-success" />
                          <span className="text-xs text-success font-medium">
                            Đã kết nối thành công ({status?.google_account || 'Hợp lệ'})
                          </span>
                        </>
                      ) : (
                        <>
                          <AlertCircle className="h-4 w-4 text-warning" />
                          <span className="text-xs text-warning">
                            {driveVerifyError || 'Vui lòng bấm Đăng nhập Google để tiếp tục.'}
                          </span>
                        </>
                      )}
                    </div>

                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={handleCheckDriveStatus}
                      disabled={isVerifyingDrive}
                      className="text-xs h-7 px-2.5 rounded-lg"
                    >
                      Load xác nhận lại
                    </Button>
                  </div>
                </Card>
              )}

              {/* Form Method: Service Account — engine does not support SA yet; be honest about it */}
              {method === 'service_account' && (
                <Card className="p-5 space-y-3 bg-bg-card border border-border/70">
                  <div className="flex items-start gap-3">
                    <div className="p-2.5 rounded-xl bg-warning/10 text-warning shrink-0">
                      <Clock className="h-5 w-5" />
                    </div>
                    <div className="space-y-1.5">
                      <p className="text-sm font-semibold text-text-primary">
                        Tính năng Service Account đang được hoàn thiện
                      </p>
                      <p className="text-xs text-text-secondary leading-relaxed">
                        Engine của 502Drive hiện chưa hỗ trợ xác thực bằng Service Account, nên chúng tôi không thể cho phép nạp file SA tại đây. Để sở hữu hạn mức API độc lập ngay hôm nay, hãy quay lại chọn <strong>Custom Google Cloud OAuth</strong> — quy trình chỉ mất vài phút và hoàn toàn miễn phí.
                      </p>
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => setMethod('custom_oauth')}
                        className="text-xs rounded-xl mt-1"
                      >
                        <Key className="h-3.5 w-3.5" />
                        <span>Chuyển sang Custom OAuth</span>
                      </Button>
                    </div>
                  </div>
                </Card>
              )}

              {/* Form Method: Custom OAuth */}
              {method === 'custom_oauth' && (
                <div className="space-y-3">
                  <div className="p-3 rounded-xl bg-blue-500/10 border border-blue-500/20 text-xs text-text-secondary space-y-1">
                    <p className="font-semibold text-blue-400">Hướng dẫn nhanh Google Cloud:</p>
                    <ol className="list-decimal list-inside space-y-0.5 text-[0.6875rem]">
                      <li>
                        Vào <a href="https://console.cloud.google.com/apis/credentials" target="_blank" rel="noreferrer" className="text-accent underline">Google Cloud Console</a> &rarr; Bật <strong>Google Drive API</strong>.
                      </li>
                      <li>Tạo OAuth Client ID loại <strong>Desktop App</strong>.</li>
                      <li>
                        Tại màn hình <strong>OAuth consent screen</strong>, bấm <strong>"Publish App"</strong> (để tránh lỗi 403: access_denied).
                      </li>
                    </ol>
                  </div>

                  <div className="space-y-2">
                    <label className="text-xs font-medium text-text-secondary">Google Client ID</label>
                    <div className="flex gap-2">
                      <input
                        type="text"
                        value={clientId}
                        onChange={(e) => setClientId(e.target.value)}
                          placeholder="VD: 937289685889-abcdefg1234567.apps.googleusercontent.com"
                        className="flex-1 px-3 py-2 text-xs rounded-xl bg-bg-input border border-border/80 text-text-primary focus:outline-none focus:border-accent font-mono"
                      />
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => handlePaste(setClientId)}
                        className="text-xs px-2.5 rounded-xl gap-1"
                        title="Dán từ Clipboard"
                      >
                        <Clipboard className="h-3.5 w-3.5" />
                        <span>Dán</span>
                      </Button>
                    </div>
                  </div>

                  <div className="space-y-2">
                    <label className="text-xs font-medium text-text-secondary">Google Client Secret</label>
                    <div className="flex gap-2">
                      <div className="relative flex-1">
                        <input
                          type={showSecret ? 'text' : 'password'}
                          value={clientSecret}
                          onChange={(e) => setClientSecret(e.target.value)}
                          placeholder="VD: GOCSPX-xxxxxxxxxxxxxxxxxxxxxxxx (định dạng ví dụ)"
                          className="w-full px-3 py-2 pr-9 text-xs rounded-xl bg-bg-input border border-border/80 text-text-primary focus:outline-none focus:border-accent font-mono"
                        />
                        <button
                          type="button"
                          onClick={() => setShowSecret(!showSecret)}
                          className="absolute right-2.5 top-2.5 text-text-muted hover:text-text-primary cursor-pointer"
                        >
                          {showSecret ? <EyeOff className="h-3.5 w-3.5" /> : <Eye className="h-3.5 w-3.5" />}
                        </button>
                      </div>
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => handlePaste(setClientSecret)}
                        className="text-xs px-2.5 rounded-xl gap-1"
                        title="Dán từ Clipboard"
                      >
                        <Clipboard className="h-3.5 w-3.5" />
                        <span>Dán</span>
                      </Button>
                    </div>
                  </div>

                  {/* Nút lưu tạm & load xác nhận */}
                  <div className="pt-2 flex items-center justify-between">
                    <span className="text-[0.6875rem] text-text-muted">
                      Sau khi điền, bấm Đăng nhập để xác thực thông tin
                    </span>
                    <Button
                      size="sm"
                      variant="secondary"
                      disabled={!clientId.trim() || !clientSecret.trim()}
                      onClick={async () => {
                        await api.saveWizardConfig({
                          oauth_client_id: clientId.trim(),
                          oauth_client_secret: clientSecret.trim(),
                        })
                        handleLoginAndVerify()
                      }}
                      className="text-xs rounded-xl"
                    >
                      Lưu & Đăng nhập
                    </Button>
                  </div>
                </div>
              )}
            </div>
          )}

          {/* ── BƯỚC 3: Cấu hình Telegram Bot ──────────────────── */}
          {currentStep === 3 && (
            <div className="space-y-4">
              <div className="space-y-1">
                <h3 className="text-sm font-semibold text-text-primary">
                  3. Kết nối & Xác thực Telegram Bot
                </h3>
                <p className="text-xs text-text-secondary">
                  Điều khiển sao chép từ xa qua Telegram với xác thực danh tính tức thì:
                </p>
              </div>

              {/* Bot Token Field */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="text-xs font-medium text-text-secondary">
                    Telegram Bot Token
                  </label>
                  <a
                    href="https://t.me/BotFather"
                    target="_blank"
                    rel="noreferrer"
                    className="text-[0.6875rem] text-accent hover:underline flex items-center gap-1"
                  >
                    <span>Lấy token tại @BotFather</span>
                    <ExternalLink className="h-3 w-3" />
                  </a>
                </div>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={botToken}
                    onChange={(e) => setBotToken(e.target.value)}
                    placeholder="VD: 123456789:AAExample_Token_Format_Only_xxxxxxxxx"
                    className="flex-1 px-3 py-2 text-xs rounded-xl bg-bg-input border border-border/80 text-text-primary focus:outline-none focus:border-accent font-mono"
                  />
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => handlePaste(setBotToken)}
                    className="text-xs px-2.5 rounded-xl gap-1"
                  >
                    <Clipboard className="h-3.5 w-3.5" />
                    <span>Dán</span>
                  </Button>
                </div>
              </div>

              {/* Owner Telegram ID Field */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <label className="text-xs font-medium text-text-secondary">
                    Telegram Owner ID (ID của bạn)
                  </label>
                  <a
                    href="https://t.me/userinfobot"
                    target="_blank"
                    rel="noreferrer"
                    className="text-[0.6875rem] text-accent hover:underline flex items-center gap-1"
                  >
                    <span>Lấy ID tại @userinfobot</span>
                    <ExternalLink className="h-3 w-3" />
                  </a>
                </div>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={ownerId}
                    onChange={(e) => setOwnerId(e.target.value.replace(/\D/g, ''))}
                    placeholder="VD: 123456789 (chỉ con số)"
                    className="flex-1 px-3 py-2 text-xs rounded-xl bg-bg-input border border-border/80 text-text-primary focus:outline-none focus:border-accent font-mono"
                  />
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => handlePaste(setOwnerId)}
                    className="text-xs px-2.5 rounded-xl gap-1"
                  >
                    <Clipboard className="h-3.5 w-3.5" />
                    <span>Dán</span>
                  </Button>
                </div>
              </div>

              {/* Nút Load xác nhận Bot */}
              <div className="pt-2">
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={handleVerifyTelegramBot}
                  disabled={isVerifyingBot || !botToken.trim()}
                  className="w-full text-xs gap-1.5 rounded-xl h-9 font-medium"
                >
                  {isVerifyingBot ? (
                    <>
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                      <span>Đang kiểm tra Telegram API...</span>
                    </>
                  ) : (
                    <>
                      <Zap className="h-3.5 w-3.5 text-accent" />
                      <span>Load xác nhận Bot trực tiếp</span>
                    </>
                  )}
                </Button>
              </div>

              {/* Kết quả Load xác nhận */}
              <AnimatePresence>
                {botVerifyResult && (
                  <motion.div
                    initial={{ opacity: 0, y: 4 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, y: 4 }}
                    className="pt-1"
                  >
                    {botVerifyResult.ok ? (
                      <div className="p-3.5 rounded-xl bg-success/10 border border-success/25 space-y-1.5">
                        <div className="flex items-center gap-2 text-success font-semibold text-xs">
                          <Bot className="h-4 w-4" />
                          <span>
                            Xác thực Bot thành công: @{botVerifyResult.username} ({botVerifyResult.first_name})
                          </span>
                        </div>
                        <p className="text-[0.6875rem] text-text-secondary">
                          Bot ID: <span className="font-mono text-text-primary">{botVerifyResult.bot_id}</span>
                          {ownerId && (
                            <>
                              {' '}• Gán quyền điều khiển độc quyền cho User ID:{' '}
                              <span className="font-mono text-success font-semibold">{ownerId}</span>
                            </>
                          )}
                        </p>
                      </div>
                    ) : (
                      <div className="p-3.5 rounded-xl bg-error/10 border border-error/25 flex items-center gap-2.5 text-xs text-error">
                        <AlertCircle className="h-4 w-4 shrink-0" />
                        <span>{botVerifyResult.error || 'Token không hợp lệ. Vui lòng kiểm tra lại.'}</span>
                      </div>
                    )}
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          )}

          {/* ── BƯỚC 4: Hoàn tất & Khởi chạy ──────────────────── */}
          {currentStep === 4 && (
            <div className="space-y-4">
              <div className="space-y-1">
                <h3 className="text-sm font-semibold text-text-primary">
                  4. Tổng kết & Sẵn sàng khởi chạy
                </h3>
                <p className="text-xs text-text-secondary">
                  Hệ thống đã thu thập đầy đủ thông số. Hãy kiểm tra lại trước khi kích hoạt:
                </p>
              </div>

              <Card className="divide-y divide-border/60 p-0 overflow-hidden bg-bg-card border border-border/70">
                {/* Google item */}
                <div className="p-3.5 flex items-center justify-between text-xs">
                  <div className="flex items-center gap-2.5">
                    <div className="p-2 rounded-lg bg-accent/10 text-accent">
                      <HardDrive className="h-4 w-4" />
                    </div>
                    <div>
                      <p className="font-medium text-text-primary">Google Drive Account</p>
                      <p className="text-text-muted text-[0.6875rem] font-mono">
                        {status?.google_account || 'Chưa đăng nhập'}
                      </p>
                    </div>
                  </div>
                  <span
                    className={`px-2 py-0.5 rounded-full text-[0.625rem] font-medium border ${
                      driveVerified
                        ? 'bg-success/10 text-success border-success/25'
                        : 'bg-warning/10 text-warning border-warning/25'
                    }`}
                  >
                    {driveVerified ? 'Đã chuẩn bị' : 'Chưa xong'}
                  </span>
                </div>

                {/* Telegram item */}
                <div className="p-3.5 flex items-center justify-between text-xs">
                  <div className="flex items-center gap-2.5">
                    <div className="p-2 rounded-lg bg-accent/10 text-accent">
                      <Send className="h-4 w-4" />
                    </div>
                    <div>
                      <p className="font-medium text-text-primary">Telegram Bot Controller</p>
                      <p className="text-text-muted text-[0.6875rem] font-mono">
                        {botVerifyResult?.username ? `@${botVerifyResult.username}` : botToken ? 'Đã nhập Token (chưa xác thực)' : 'Chưa cấu hình'}
                        {ownerId ? ` (Owner: ${ownerId})` : ''}
                      </p>
                    </div>
                  </div>
                  <span
                    className={`px-2 py-0.5 rounded-full text-[0.625rem] font-medium border ${
                      botVerifyResult?.ok
                        ? 'bg-success/10 text-success border-success/25'
                        : 'bg-warning/10 text-warning border-warning/25'
                    }`}
                  >
                    {botVerifyResult?.ok ? 'Sẵn sàng' : 'Chưa xong'}
                  </span>
                </div>

                {/* Engine item */}
                <div className="p-3.5 flex items-center justify-between text-xs">
                  <div className="flex items-center gap-2.5">
                    <div className="p-2 rounded-lg bg-accent/10 text-accent">
                      <Zap className="h-4 w-4" />
                    </div>
                    <div>
                      <p className="font-medium text-text-primary">Hiệu năng & Tác vụ nền</p>
                      <p className="text-text-muted text-[0.6875rem]">
                        8 Luồng song song • Tự động khởi chạy nền cùng máy
                      </p>
                    </div>
                  </div>
                  <span className="px-2 py-0.5 rounded-full text-[0.625rem] font-medium bg-success/10 text-success border border-success/25">
                    Tối ưu
                  </span>
                </div>
              </Card>

              <div className="p-3.5 rounded-xl bg-bg-card/50 border border-border/50 text-[0.6875rem] text-text-secondary flex items-start gap-2.5">
                <ShieldCheck className="h-4 w-4 text-success shrink-0 mt-0.5" />
                <p className="leading-relaxed">
                  Toàn bộ thông tin bí mật (Token Google, Bot Token) được mã hóa an toàn trong cơ sở dữ liệu nội bộ trên máy bạn. Không có bất kỳ dữ liệu nào bị chuyển tiếp ra ngoài.
                </p>
              </div>
            </div>
          )}
        </div>

        {/* Modal Footer: Action controls */}
        <div className="flex items-center justify-between px-6 py-4 border-t border-border/60 bg-bg-card/50">
          <div className="flex items-center gap-3">
            {currentStep > 1 && (
              <Button
                size="sm"
                variant="outline"
                onClick={() => setCurrentStep((s) => (s - 1) as 1 | 2 | 3 | 4)}
                className="text-xs gap-1 rounded-xl"
              >
                <ArrowLeft className="h-3.5 w-3.5" />
                <span>Quay lại</span>
              </Button>
            )}
            {currentStep === 2 && !canContinueStep2 && (
              <span className="text-[0.6875rem] text-warning">
                Hoàn tất đăng nhập Google để tiếp tục
              </span>
            )}
            {currentStep === 3 && !canContinueStep3 && (
              <span className="text-[0.6875rem] text-warning">
                Xác thực Bot và nhập Owner ID để tiếp tục
              </span>
            )}
          </div>

          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant="secondary"
              onClick={onClose}
              className="text-xs rounded-xl"
            >
              Để sau
            </Button>

            {currentStep < 4 ? (
              <Button
                size="sm"
                variant="primary"
                disabled={
                  (currentStep === 2 && !canContinueStep2) ||
                  (currentStep === 3 && !canContinueStep3)
                }
                onClick={() => setCurrentStep((s) => (s + 1) as 1 | 2 | 3 | 4)}
                className="text-xs gap-1.5 rounded-xl shadow-xs"
              >
                <span>Tiếp tục</span>
                <ArrowRight className="h-3.5 w-3.5" />
              </Button>
            ) : (
              <Button
                size="sm"
                variant="primary"
                onClick={handleFinishSetup}
                disabled={isSaving || isComplete || !canFinish}
                title={canFinish ? undefined : 'Cần đăng nhập Google hoặc xác thực Bot trước khi kích hoạt'}
                className="text-xs gap-1.5 rounded-xl shadow-xs bg-accent hover:bg-accent-hover text-white font-semibold"
              >
                {isSaving ? (
                  <>
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    <span>Đang kích hoạt...</span>
                  </>
                ) : isComplete ? (
                  <>
                    <Check className="h-3.5 w-3.5" />
                    <span>Thành công!</span>
                  </>
                ) : (
                  <>
                    <Sparkles className="h-3.5 w-3.5" />
                    <span>Kích hoạt 502Drive ngay</span>
                  </>
                )}
              </Button>
            )}
          </div>
        </div>
      </motion.div>
    </div>
  )
}
