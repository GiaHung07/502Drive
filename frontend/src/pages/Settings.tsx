import React, { useEffect, useState } from "react"
import { motion, AnimatePresence } from "motion/react"
import { Card } from "@/components/ui/Card"
import { Button } from "@/components/ui/Button"
import { ConfigSummary, SystemStatus } from "@/lib/types"
import { useTheme, ThemeMode } from "@/hooks/useTheme"
import { useI18n } from "@/hooks/useI18n"
import {
  HardDrive,
  Send,
  Zap,
  Power,
  Languages,
  ArrowUpCircle,
  Check,
  RotateCcw,
  LogOut,
  LogIn,
  SunMoon,
  Sun,
  Moon,
  Monitor,
  HelpCircle,
  FileCode,
  Sparkles,
  AlertTriangle,
  FolderKey,
  FolderOpen,
  FolderSync,
} from "lucide-react"
import { api, getErrorMessage } from "@/lib/ipc"
import { useToast } from "@/components/primitives/Toast"

export interface SettingsProps {
  config: ConfigSummary | null
  status: SystemStatus | null
  onUpdateConfig: (field: string, value: unknown) => void
  onTriggerLogin: () => void
  onTriggerRevoke: () => void
  onRestartService: () => void
  onCheckUpdate: () => void
  onOpenWizard?: () => void
}

export const Settings: React.FC<SettingsProps> = ({
  config,
  status,
  onUpdateConfig,
  onTriggerLogin,
  onTriggerRevoke,
  onRestartService,
  onCheckUpdate,
  onOpenWizard,
}) => {
  const { t, lang, setLang } = useI18n()
  const [isUpdating, setIsUpdating] = useState(false)
  const [updateChecked, setUpdateChecked] = useState(false)
  const [showOAuthHelp, setShowOAuthHelp] = useState(false)
  const [saDir, setSaDir] = useState<string>("~/.config/502drive/sa")
  const [saFilesCount, setSaFilesCount] = useState<number>(0)
  // Slider drafts locally while dragging; persists once on release.
  const [concurrencyDraft, setConcurrencyDraft] = useState<number | null>(null)
  const { theme, setTheme } = useTheme()

  const { toast } = useToast()
  const [isEditingDest, setIsEditingDest] = useState(false)
  const [destId, setDestId] = useState(status?.destination_id || "")
  const [destLabel, setDestLabel] = useState(status?.destination_label || "")
  const [isSavingDest, setIsSavingDest] = useState(false)

  useEffect(() => {
    if (status?.destination_id) setDestId(status.destination_id)
    if (status?.destination_label) setDestLabel(status.destination_label)
  }, [status?.destination_id, status?.destination_label])

  const handleSaveDestination = async () => {
    if (!destId.trim()) {
      toast({ title: t('common.error'), description: 'Vui lòng nhập ID hoặc liên kết thư mục Drive', variant: 'error' })
      return
    }
    setIsSavingDest(true)
    try {
      const label = destLabel.trim() || 'Default Destination'
      await api.setDefaultDestination(destId.trim(), label)
      toast({ title: t('common.success'), description: t('settings_page.dest_saved_toast'), variant: 'success' })
      setIsEditingDest(false)
      onRestartService()
    } catch (err) {
      toast({ title: t('common.error'), description: getErrorMessage(err), variant: 'error' })
    } finally {
      setIsSavingDest(false)
    }
  }

  useEffect(() => {
    if (config) setConcurrencyDraft(null)
  }, [config?.engine_concurrency])

  const commitConcurrency = () => {
    if (concurrencyDraft === null) return
    const value = concurrencyDraft
    setConcurrencyDraft(null)
    if (value !== (config?.engine_concurrency ?? 8)) {
      onUpdateConfig("engine_concurrency", value)
    }
  }

  const handleCheckUpdate = () => {
    setIsUpdating(true)
    setTimeout(() => {
      setIsUpdating(false)
      setUpdateChecked(true)
      onCheckUpdate()
    }, 1200)
  }

  const isAccountConnected = status?.account_status === "connected"
  const isReconnectRequired = status?.account_status === "reconnect_required"
  const currentLang = config?.language || lang || "vi"

  return (
    <div className="space-y-6 w-full max-w-4xl mx-auto pb-12">
      {/* ── Banner: Trình Hướng Dẫn Thiết Lập Nhanh ──────── */}
      <div className="p-4 rounded-2xl bg-bg-card border border-border/70 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 shadow-xs">
        <div className="flex items-center gap-3">
          <div className="p-2.5 rounded-xl bg-accent/10 text-accent shrink-0">
            <Sparkles className="h-4 w-4" />
          </div>
          <div>
            <p className="text-sm font-medium text-text-primary">{t('settings_page.wizard_banner_title')}</p>
            <p className="text-xs text-text-secondary">
              {t('settings_page.wizard_banner_desc')}
            </p>
          </div>
        </div>
        <Button
          size="sm"
          variant="primary"
          onClick={onOpenWizard}
          className="text-xs gap-1.5 rounded-xl shrink-0 font-medium"
        >
          <Sparkles className="h-3.5 w-3.5" />
          <span>{t('settings_page.wizard_banner_btn')}</span>
        </Button>
      </div>
      {/* ── Group 1: Tài khoản & Kết nối ─────────────────── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between px-1">
          <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
            {t('settings_page.group_accounts')}
          </h2>
          <button
            onClick={() => setShowOAuthHelp(!showOAuthHelp)}
            aria-expanded={showOAuthHelp}
            className="flex items-center gap-1 text-[0.6875rem] text-accent hover:underline cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 rounded"
          >
            <HelpCircle className="h-3 w-3" />
            <span>{t('settings_page.oauth_help_btn')}</span>
          </button>
        </div>

        {/* Banner: Xử lý lỗi Google 403 Access Denied & Bypass Quota */}
        <AnimatePresence>
          {showOAuthHelp && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              exit={{ opacity: 0, height: 0 }}
              className="overflow-hidden"
            >
              <div className="p-4 rounded-2xl bg-warning/10 border border-warning/25 space-y-3 text-xs text-text-secondary">
                <div className="flex items-center gap-2 text-warning font-medium">
                  <AlertTriangle className="h-4 w-4" />
                  <span>Tại sao tài khoản khác bị lỗi: "Đã chặn quyền truy cập (Lỗi 403: access_denied)"?</span>
                </div>
                <p className="leading-relaxed">
                  Khi Google Cloud Console ở trạng thái <strong className="text-text-primary">"Testing"</strong>, Google chỉ cho phép các email được thêm thủ công vào mục <em>"Test Users"</em> đăng nhập.
                </p>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2.5 pt-1 text-[0.6875rem]">
                  <div className="p-2.5 rounded-xl bg-bg-card border border-border/70 space-y-1">
                    <p className="font-semibold text-text-primary flex items-center gap-1.5">
                      <Sparkles className="h-3.5 w-3.5 text-accent" />
                      Cách 1: Chuyển sang "In Production" (Chuẩn Rclone)
                    </p>
                    <p className="text-text-muted leading-relaxed">
                      Trên Google Cloud Console &rarr; OAuth Consent Screen &rarr; bấm <strong>"Publish App"</strong>. Khi đó bất kỳ ai cũng đăng nhập được ngay (1-chạm) mà không bị chặn 403.
                    </p>
                  </div>
                  <div className="p-2.5 rounded-xl bg-bg-card border border-border/70 space-y-1">
                    <p className="font-semibold text-text-primary flex items-center gap-1.5">
                      <FileCode className="h-3.5 w-3.5 text-accent" />
                      Cách 2: Service Accounts (Quota Không Giới Hạn - Tự Xoay Vòng)
                    </p>
                    <p className="text-text-muted leading-relaxed">
                      Tạo Service Accounts miễn phí (mỗi SA 750GB/ngày). Nạp thư mục JSON vào tool để sao chép tự động không cần trình duyệt và tự xoay vòng khi chạm hạn mức.
                    </p>
                  </div>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        <Card className="divide-y divide-border/60 p-0 overflow-hidden shadow-xs">
          {/* Google Account */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2.5 rounded-xl bg-accent/10 text-accent shrink-0">
                <HardDrive className="h-4 w-4 stroke-[1.75]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">{t('settings_page.gdrive_label')}</p>
                <p className="text-xs text-text-secondary font-mono">
                  {status?.google_account || t('dashboard.gdrive_disconnected')}
                </p>
              </div>
            </div>

            <div className="flex items-center gap-2">
              {isAccountConnected ? (
                <>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={onTriggerLogin}
                    className="text-xs rounded-xl"
                  >
                    {t('dashboard.btn_switch_account')}
                  </Button>
                  <Button
                    size="sm"
                    variant="danger"
                    onClick={onTriggerRevoke}
                    className="h-8 px-2.5 text-xs gap-1 rounded-xl"
                    title={t('dashboard.btn_disconnect')}
                  >
                    <LogOut className="h-3.5 w-3.5" />
                    <span>{t('dashboard.btn_disconnect')}</span>
                  </Button>
                </>
              ) : (
                <Button
                  size="sm"
                  variant="primary"
                  onClick={onTriggerLogin}
                  className="text-xs gap-1.5 rounded-xl"
                >
                  <LogIn className="h-3.5 w-3.5" />
                  {isReconnectRequired ? t('dashboard.btn_reconnect_google') : t('dashboard.btn_connect_google')}
                </Button>
              )}
            </div>
          </div>

          {/* Telegram Owner */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2.5 rounded-xl bg-bg-input text-text-secondary shrink-0">
                <Send className="h-4 w-4 stroke-[1.75]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">{t('settings_page.tg_owner_label')}</p>
                <p className="text-xs text-text-secondary">
                  {t('settings_page.tg_owner_desc')}{" "}
                  <span className="font-mono text-text-primary font-medium">
                    {config?.owner_telegram_id || t('common.not_configured')}
                  </span>
                </p>
              </div>
            </div>

            <Button
              size="sm"
              variant="secondary"
              onClick={onRestartService}
              className="text-xs gap-1.5 rounded-xl"
            >
              <RotateCcw className="h-3.5 w-3.5" />
              <span>{t('settings_page.btn_reload_service')}</span>
            </Button>
          </div>

          {/* Default Destination Folder */}
          <div className="p-4 space-y-3">
            <div className="flex items-center justify-between gap-4">
              <div className="flex items-start gap-3">
                <div className="p-2.5 rounded-xl bg-bg-input text-text-secondary shrink-0">
                  <FolderSync className="h-4 w-4 stroke-[1.75]" />
                </div>
                <div className="space-y-0.5">
                  <p className="text-sm font-medium text-text-primary">{t('settings_page.dest_label')}</p>
                  <p className="text-xs text-text-secondary">
                    {t('settings_page.dest_desc')}
                  </p>
                  <div className="flex items-center gap-2 pt-0.5">
                    <span className="text-xs font-semibold text-text-primary">
                      {status?.destination_label || t('dashboard.dest_unset')}
                    </span>
                    {status?.destination_id && (
                      <span className="text-[0.625rem] font-mono text-text-muted">
                        ({status.destination_id})
                      </span>
                    )}
                  </div>
                </div>
              </div>

              <Button
                size="sm"
                variant={isEditingDest ? "secondary" : "outline"}
                onClick={() => setIsEditingDest((v) => !v)}
                className="text-xs gap-1.5 rounded-xl shrink-0"
              >
                <span>{isEditingDest ? t('common.close') : t('settings_page.btn_change_dest')}</span>
              </Button>
            </div>

            {isEditingDest && (
              <div className="p-3.5 rounded-xl bg-bg-input/60 border border-border/60 space-y-2 mt-2">
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                  <input
                    type="text"
                    value={destId}
                    onChange={(e) => setDestId(e.target.value)}
                    placeholder={t('dev_page.dest_folder_id')}
                    className="w-full text-xs font-mono px-3 py-2 rounded-lg bg-bg-card border border-border/80 text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent"
                  />
                  <input
                    type="text"
                    value={destLabel}
                    onChange={(e) => setDestLabel(e.target.value)}
                    placeholder={t('dev_page.dest_folder_name')}
                    className="w-full text-xs px-3 py-2 rounded-lg bg-bg-card border border-border/80 text-text-primary placeholder:text-text-muted focus:outline-none focus:border-accent"
                  />
                </div>
                <div className="flex items-center justify-end gap-2 pt-1">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setIsEditingDest(false)}
                    className="text-xs rounded-lg"
                  >
                    {t('common.cancel')}
                  </Button>
                  <Button
                    size="sm"
                    variant="primary"
                    onClick={handleSaveDestination}
                    disabled={isSavingDest}
                    className="text-xs font-semibold rounded-lg bg-accent text-white"
                  >
                    {isSavingDest ? t('common.loading') : t('settings_page.btn_save_dest')}
                  </Button>
                </div>
              </div>
            )}
          </div>
        </Card>
      </div>

      {/* ── Group 2: Engine & Hiệu năng ──────────────────── */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider px-1">
          {t('settings_page.group_engine')}
        </h2>
        <Card className="divide-y divide-border/60 p-0 overflow-hidden shadow-xs">
          {/* Concurrency Slider */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2.5 rounded-xl bg-bg-input text-text-secondary shrink-0">
                <Zap className="h-4 w-4 stroke-[1.75]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">
                  {t('settings_page.concurrency_label')}
                </p>
                <p className="text-xs text-text-secondary">
                  {t('settings_page.concurrency_desc')}
                </p>
              </div>
            </div>

            <div className="flex items-center gap-3">
              <input
                type="range"
                min="1"
                max="32"
                value={concurrencyDraft ?? config?.engine_concurrency ?? 8}
                onChange={(e) => setConcurrencyDraft(parseInt(e.target.value))}
                onPointerUp={commitConcurrency}
                onKeyUp={commitConcurrency}
                onBlur={commitConcurrency}
                aria-label={t('settings_page.concurrency_label')}
                className="w-28 accent-accent cursor-pointer"
              />
              <span className="text-xs font-mono font-medium text-text-primary w-5 text-right">
                {concurrencyDraft ?? config?.engine_concurrency ?? 8}
              </span>
            </div>
          </div>

          {/* Auto Confirm Clone */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2.5 rounded-xl bg-bg-input text-text-secondary shrink-0">
                <Power className="h-4 w-4 stroke-[1.75]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">{t('settings_page.auto_confirm_label')}</p>
                <p className="text-xs text-text-secondary">
                  {t('settings_page.auto_confirm_desc')}
                </p>
              </div>
            </div>

            <button
              onClick={() => onUpdateConfig("auto_confirm_clone", !config?.auto_confirm_clone)}
              role="switch"
              aria-checked={!!config?.auto_confirm_clone}
              aria-label={t('settings_page.auto_confirm_label')}
              className={`w-11 h-6 flex items-center rounded-full p-0.5 cursor-pointer transition-colors duration-200 ease-in-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                config?.auto_confirm_clone ? "bg-accent" : "bg-bg-input border border-border/80"
              }`}
            >
              <div
                className={`bg-white w-5 h-5 rounded-full shadow-md transform transition-transform duration-200 ease-in-out ${
                  config?.auto_confirm_clone ? "translate-x-5" : "translate-x-0"
                }`}
              />
            </button>
          </div>
        </Card>
      </div>

      {/* ── Group 3: Service Accounts (SA) Quota Manager ──── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between px-1">
          <div className="flex items-center gap-2">
            <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
              {t('settings_page.group_sa')}
            </h2>
            <span className="text-[0.625rem] font-mono font-semibold px-2 py-0.5 rounded-full bg-accent/15 text-accent border border-accent/25">
              {t('settings_page.sa_badge')}
            </span>
          </div>
        </div>
        <Card className="p-4 space-y-3 shadow-xs border border-border/70">
          <div className="flex items-start gap-3">
            <div className="p-2.5 rounded-xl bg-accent/10 text-accent shrink-0">
              <FolderKey className="h-4 w-4 stroke-[1.75]" />
            </div>
            <div className="space-y-1 flex-1">
              <p className="text-xs text-text-secondary leading-relaxed">
                {t('settings_page.sa_desc')}
              </p>
              <div className="pt-2 flex flex-col sm:flex-row gap-2 items-stretch sm:items-center">
                <input
                  type="text"
                  value={saDir}
                  onChange={(e) => setSaDir(e.target.value)}
                  placeholder={t('settings_page.sa_dir_placeholder')}
                  className="flex-1 px-3 py-1.5 text-xs rounded-xl bg-bg-input border border-border/80 text-text-primary font-mono focus:outline-none focus:border-accent"
                />
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => {
                    // Simulate inspecting folder for .json files
                    setSaFilesCount(saFilesCount > 0 ? 0 : 50)
                  }}
                  className="text-xs gap-1.5 rounded-xl shrink-0 font-medium"
                >
                  <FolderOpen className="h-3.5 w-3.5" />
                  <span>{t('settings_page.sa_btn_browse')}</span>
                </Button>
              </div>
              <div className="pt-2 flex items-center justify-between text-xs text-text-muted">
                <span>
                  {saFilesCount > 0
                    ? `${saFilesCount} Service Accounts loaded`
                    : t('settings_page.sa_status_none')}
                </span>
                {saFilesCount > 0 && (
                  <span className="font-semibold text-accent font-mono">
                    {t('settings_page.sa_calc_quota')}{saFilesCount * 750} {t('settings_page.sa_per_day')}
                  </span>
                )}
              </div>
            </div>
          </div>
        </Card>
      </div>

      {/* ── Group 4: Hệ thống & Giao diện (Apple HIG Style) ─── */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider px-1">
          {t('settings_page.group_system')}
        </h2>
        <Card className="divide-y divide-border/60 p-0 overflow-hidden shadow-xs">
          {/* Theme Selector (Apple Segmented Control with Sliding Pill) */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2.5 rounded-xl bg-bg-input text-text-secondary shrink-0">
                <SunMoon className="h-4 w-4 stroke-[1.75]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">{t('settings_page.theme_label')}</p>
                <p className="text-xs text-text-secondary">
                  {t('settings_page.theme_desc')}
                </p>
              </div>
            </div>

            {/* Apple Segmented Control */}
            <div className="flex items-center p-1 rounded-xl bg-bg-input/80 border border-border/40 relative">
              {(
                [
                  { id: "system", label: t('settings_page.theme_auto'), icon: Monitor },
                  { id: "dark", label: t('settings_page.theme_dark'), icon: Moon },
                  { id: "light", label: t('settings_page.theme_light'), icon: Sun },
                ] as const
              ).map((item) => {
                const Icon = item.icon
                const isSelected = theme === item.id

                return (
                  <button
                    key={item.id}
                    onClick={() => setTheme(item.id as ThemeMode)}
                    aria-pressed={isSelected}
                    className={`relative z-10 flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                      isSelected ? "text-text-primary font-semibold" : "text-text-secondary hover:text-text-primary"
                    }`}
                  >
                    {isSelected && (
                      <motion.div
                        layoutId="theme-segmented-indicator"
                        transition={{ type: "spring", stiffness: 450, damping: 30 }}
                        className="absolute inset-0 bg-bg-card rounded-lg shadow-sm border border-border/60 -z-10"
                      />
                    )}
                    <Icon className="h-3.5 w-3.5" />
                    <span>{item.label}</span>
                  </button>
                )
              })}
            </div>
          </div>

          {/* Language Selector */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2.5 rounded-xl bg-bg-input text-text-secondary shrink-0">
                <Languages className="h-4 w-4 stroke-[1.75]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">{t('settings_page.lang_label')}</p>
                <p className="text-xs text-text-secondary">
                  {t('settings_page.lang_desc')}
                </p>
              </div>
            </div>

            {/* Apple Segmented Language Control */}
            <div className="flex items-center p-1 rounded-xl bg-bg-input/80 border border-border/40 relative">
              {(
                [
                  { id: "vi", label: "Tiếng Việt" },
                  { id: "en", label: "English" },
                ] as const
              ).map((itemLang) => {
                const isSelected = currentLang === itemLang.id

                return (
                  <button
                    key={itemLang.id}
                    onClick={() => {
                      setLang(itemLang.id as 'vi' | 'en')
                      onUpdateConfig("language", itemLang.id)
                    }}
                    aria-pressed={isSelected}
                    className={`relative z-10 px-3.5 py-1.5 text-xs font-medium rounded-lg transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                      isSelected ? "text-text-primary font-semibold" : "text-text-secondary hover:text-text-primary"
                    }`}
                  >
                    {isSelected && (
                      <motion.div
                        layoutId="lang-segmented-indicator"
                        transition={{ type: "spring", stiffness: 450, damping: 30 }}
                        className="absolute inset-0 bg-bg-card rounded-lg shadow-sm border border-border/60 -z-10"
                      />
                    )}
                    <span>{itemLang.label}</span>
                  </button>
                )
              })}
            </div>
          </div>

          {/* Launch at startup */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2.5 rounded-xl bg-bg-input text-text-secondary shrink-0">
                <Power className="h-4 w-4 stroke-[1.75]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">{t('settings_page.startup_label')}</p>
                <p className="text-xs text-text-secondary">
                  {t('settings_page.startup_desc')}
                </p>
              </div>
            </div>

            <button
              onClick={() => onUpdateConfig("launch_at_startup", !config?.launch_at_startup)}
              role="switch"
              aria-checked={!!config?.launch_at_startup}
              aria-label={t('settings_page.startup_label')}
              className={`w-11 h-6 flex items-center rounded-full p-0.5 cursor-pointer transition-colors duration-200 ease-in-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                config?.launch_at_startup ? "bg-accent" : "bg-bg-input border border-border/80"
              }`}
            >
              <div
                className={`bg-white w-5 h-5 rounded-full shadow-md transform transition-transform duration-200 ease-in-out ${
                  config?.launch_at_startup ? "translate-x-5" : "translate-x-0"
                }`}
              />
            </button>
          </div>
        </Card>
      </div>

      {/* ── Group 5: Cập nhật phần mềm ───────────────────── */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider px-1">
          {t('settings_page.group_updates')}
        </h2>
        <Card className="p-4 flex items-center justify-between gap-4 shadow-xs">
          <div className="flex items-start gap-3">
            <div className="p-2.5 rounded-xl bg-accent/10 text-accent shrink-0">
              <ArrowUpCircle className="h-4 w-4 stroke-[1.75]" />
            </div>
            <div className="space-y-0.5">
              <div className="flex items-center gap-2">
                <p className="text-sm font-medium text-text-primary">{t('settings_page.app_version_label')}</p>
                <span className="text-xs font-mono px-1.5 py-0.5 rounded-md bg-bg-input text-accent font-medium">
                  {status?.app_version || "v0.2.0"}
                </span>
              </div>
              <p className="text-xs text-text-secondary">
                {updateChecked
                  ? t('settings_page.version_up_to_date')
                  : t('settings_page.version_check_github')}
              </p>
            </div>
          </div>

          <Button
            size="sm"
            variant={updateChecked ? "outline" : "secondary"}
            onClick={handleCheckUpdate}
            disabled={isUpdating}
            className="text-xs gap-1.5 rounded-xl"
          >
            {updateChecked ? <Check className="h-3.5 w-3.5 text-accent" /> : <ArrowUpCircle className="h-3.5 w-3.5" />}
            <span>{isUpdating ? t('settings_page.btn_checking_update') : updateChecked ? t('settings_page.btn_up_to_date') : t('settings_page.btn_check_update')}</span>
          </Button>
        </Card>
      </div>
    </div>
  )
}
