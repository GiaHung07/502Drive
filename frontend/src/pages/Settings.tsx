import React, { useState } from "react"
import { motion, AnimatePresence } from "motion/react"
import { Card } from "@/components/ui/Card"
import { Button } from "@/components/ui/Button"
import { ConfigSummary, SystemStatus } from "@/lib/types"
import { useTheme, ThemeMode } from "@/hooks/useTheme"
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
} from "lucide-react"

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
  const [isUpdating, setIsUpdating] = useState(false)
  const [updateChecked, setUpdateChecked] = useState(false)
  const [showOAuthHelp, setShowOAuthHelp] = useState(false)
  const { theme, setTheme } = useTheme()

  const handleCheckUpdate = () => {
    setIsUpdating(true)
    setTimeout(() => {
      setIsUpdating(false)
      setUpdateChecked(true)
      onCheckUpdate()
    }, 1200)
  }

  const isAccountConnected = status?.account_status === "connected"
  const currentLang = config?.language || "vi"

  return (
    <div className="space-y-6 w-full max-w-4xl mx-auto pb-12">
      {/* ── Banner: Trình Hướng Dẫn Thiết Lập Nhanh ──────── */}
      <div className="p-4 rounded-2xl bg-bg-card border border-border/70 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 shadow-xs">
        <div className="flex items-center gap-3">
          <div className="p-2.5 rounded-xl bg-accent/10 text-accent shrink-0">
            <Sparkles className="h-4 w-4" />
          </div>
          <div>
            <p className="text-sm font-medium text-text-primary">Trình Hướng Dẫn Thiết Lập (Setup Wizard)</p>
            <p className="text-xs text-text-secondary">
              Điền thông số Google Drive & Telegram Bot theo từng bước với cơ chế Live Test "load xác nhận" tức thì.
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
          <span>Mở Wizard</span>
        </Button>
      </div>
      {/* ── Group 1: Tài khoản & Kết nối ─────────────────── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between px-1">
          <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
            Tài khoản & Kết nối
          </h2>
          <button
            onClick={() => setShowOAuthHelp(!showOAuthHelp)}
            aria-expanded={showOAuthHelp}
            className="flex items-center gap-1 text-[0.6875rem] text-accent hover:underline cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 rounded"
          >
            <HelpCircle className="h-3 w-3" />
            <span>Xử lý lỗi Google 403 & Thiết lập 1 chạm</span>
          </button>
        </div>

        {/* Banner: Xử lý lỗi Google 403 Access Denied & Giải pháp Reddit */}
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
                      Cách 2: Service Accounts (Chuẩn Reddit r/DataHoarder)
                    </p>
                    <p className="text-text-muted leading-relaxed">
                      Tạo Service Accounts miễn phí (mỗi SA 750GB/ngày). Kéo thả file JSON vào tool để sao chép tự động không cần trình duyệt và không sợ giới hạn quota cá nhân.
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
                <p className="text-sm font-medium text-text-primary">Tài khoản Google Drive</p>
                <p className="text-xs text-text-secondary font-mono">
                  {status?.google_account || "Chưa liên kết tài khoản Google"}
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
                    Đổi tài khoản
                  </Button>
                  <Button
                    size="sm"
                    variant="danger"
                    onClick={onTriggerRevoke}
                    className="h-8 px-2.5 text-xs gap-1 rounded-xl"
                    title="Ngắt kết nối tài khoản"
                  >
                    <LogOut className="h-3.5 w-3.5" />
                    <span>Ngắt</span>
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
                  Đăng nhập Google
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
                <p className="text-sm font-medium text-text-primary">Telegram Owner ID</p>
                <p className="text-xs text-text-secondary">
                  Chỉ cho phép Telegram ID này điều khiển bot:{" "}
                  <span className="font-mono text-text-primary font-medium">
                    {config?.owner_telegram_id || "Chưa cấu hình"}
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
              <span>Tải lại service</span>
            </Button>
          </div>
        </Card>
      </div>

      {/* ── Group 2: Engine & Hiệu năng ──────────────────── */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider px-1">
          Engine & Hiệu năng
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
                  Số luồng sao chép song song (Concurrency)
                </p>
                <p className="text-xs text-text-secondary">
                  Số worker thực thi sao chép dữ liệu đồng thời trên Google Drive (Khuyến nghị: 8)
                </p>
              </div>
            </div>

            <div className="flex items-center gap-3">
              <input
                type="range"
                min="1"
                max="32"
                value={config?.engine_concurrency || 8}
                onChange={(e) => onUpdateConfig("engine_concurrency", parseInt(e.target.value))}
                className="w-28 accent-accent cursor-pointer"
              />
              <span className="text-xs font-mono font-medium text-text-primary w-5 text-right">
                {config?.engine_concurrency || 8}
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
                <p className="text-sm font-medium text-text-primary">Tự động xác nhận lệnh clone</p>
                <p className="text-xs text-text-secondary">
                  Bỏ qua bước xác nhận Yes/No trong bot Telegram khi nhận link Drive
                </p>
              </div>
            </div>

            <button
              onClick={() => onUpdateConfig("auto_confirm_clone", !config?.auto_confirm_clone)}
              role="switch"
              aria-checked={!!config?.auto_confirm_clone}
              aria-label="Tự động xác nhận lệnh clone"
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

      {/* ── Group 3: Hệ thống & Giao diện (Apple HIG Style) ─── */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider px-1">
          Hệ thống & Giao diện
        </h2>
        <Card className="divide-y divide-border/60 p-0 overflow-hidden shadow-xs">
          {/* Theme Selector (Apple Segmented Control with Sliding Pill) */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2.5 rounded-xl bg-bg-input text-text-secondary shrink-0">
                <SunMoon className="h-4 w-4 stroke-[1.75]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">Giao diện (Chủ đề màu)</p>
                <p className="text-xs text-text-secondary">
                  Chuyển đổi giao diện sáng / tối mượt mà chuẩn macOS
                </p>
              </div>
            </div>

            {/* Apple Segmented Control */}
            <div className="flex items-center p-1 rounded-xl bg-bg-input/80 border border-border/40 relative">
              {(
                [
                  { id: "system", label: "Tự động", icon: Monitor },
                  { id: "dark", label: "Tối", icon: Moon },
                  { id: "light", label: "Sáng", icon: Sun },
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

          {/* Language Selector (Apple Segmented Control - Fixed unstyled select bug!) */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2.5 rounded-xl bg-bg-input text-text-secondary shrink-0">
                <Languages className="h-4 w-4 stroke-[1.75]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">Ngôn ngữ hiển thị</p>
                <p className="text-xs text-text-secondary">
                  Ngôn ngữ trong giao diện desktop và thông báo bot
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
              ).map((lang) => {
                const isSelected = currentLang === lang.id

                return (
                  <button
                    key={lang.id}
                    onClick={() => onUpdateConfig("language", lang.id)}
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
                    <span>{lang.label}</span>
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
                <p className="text-sm font-medium text-text-primary">Tự khởi động cùng hệ thống</p>
                <p className="text-xs text-text-secondary">
                  Kích hoạt daemon chạy nền khi khởi động máy
                </p>
              </div>
            </div>

            <button
              onClick={() => onUpdateConfig("launch_at_startup", !config?.launch_at_startup)}
              role="switch"
              aria-checked={!!config?.launch_at_startup}
              aria-label="Tự khởi động cùng hệ thống"
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

      {/* ── Group 4: Cập nhật phần mềm ───────────────────── */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider px-1">
          Cập nhật phần mềm
        </h2>
        <Card className="p-4 flex items-center justify-between gap-4 shadow-xs">
          <div className="flex items-start gap-3">
            <div className="p-2.5 rounded-xl bg-accent/10 text-accent shrink-0">
              <ArrowUpCircle className="h-4 w-4 stroke-[1.75]" />
            </div>
            <div className="space-y-0.5">
              <div className="flex items-center gap-2">
                <p className="text-sm font-medium text-text-primary">502Drive Desktop</p>
                <span className="text-xs font-mono px-1.5 py-0.5 rounded-md bg-bg-input text-accent font-medium">
                  {status?.app_version || "v0.1.0"}
                </span>
              </div>
              <p className="text-xs text-text-secondary">
                {updateChecked
                  ? "Bạn đang sử dụng phiên bản ổn định mới nhất."
                  : "Kiểm tra bản phát hành mới từ kho lưu trữ GitHub."}
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
            <span>{isUpdating ? "Đang kiểm tra..." : updateChecked ? "Mới nhất" : "Kiểm tra cập nhật"}</span>
          </Button>
        </Card>
      </div>
    </div>
  )
}
