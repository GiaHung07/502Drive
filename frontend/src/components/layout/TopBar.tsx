import React from "react"
import { motion, AnimatePresence } from "motion/react"
import { StatusDot } from "@/components/primitives/StatusDot"
import { Button } from "@/components/ui/Button"
import { RefreshCw, RotateCcw, Search, Sun, Moon } from "lucide-react"
import { SystemStatus } from "@/lib/types"
import { useTheme } from "@/hooks/useTheme"

export interface TopBarProps {
  title: string
  subtitle?: string
  status: SystemStatus | null
  isRefreshing?: boolean
  onRefresh?: () => void
  onRestartService?: () => void
  onOpenCommandPalette?: () => void
  /** Currently active language ("vi" | "en"). */
  currentLang?: string
  /** Called when user toggles language from the TopBar pill. */
  onChangeLang?: (lang: string) => void
}

export const TopBar: React.FC<TopBarProps> = ({
  title,
  subtitle,
  status,
  isRefreshing,
  onRefresh,
  onRestartService,
  onOpenCommandPalette,
  currentLang = "vi",
  onChangeLang,
}) => {
  const isServiceActive = status?.service_active ?? false
  const isAccountConnected = status?.account_status === "connected"
  const { resolvedTheme, toggleTheme } = useTheme()

  return (
    <header className="h-14 border-b border-border/70 bg-bg-elevated px-4 sm:px-6 lg:px-8 flex items-center justify-between shrink-0 select-none z-20 transition-colors">
      {/* Left: Breadcrumbs & title */}
      <div className="flex items-center gap-3 min-w-0">
        <div className="flex items-baseline gap-2 min-w-0">
          <h1 className="text-sm sm:text-base font-semibold text-text-primary tracking-tight truncate">
            {title}
          </h1>
          {subtitle && (
            <span className="text-xs text-text-muted hidden md:inline truncate font-normal">
              {subtitle}
            </span>
          )}
        </div>
      </div>

      {/* Right: Quick actions */}
      <div className="flex items-center gap-2 sm:gap-2.5">
        {/* Quick Search / Command Palette button */}
        {onOpenCommandPalette && (
          <motion.button
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            onClick={onOpenCommandPalette}
            aria-label="Mở thanh lệnh nhanh (Ctrl+K / ⌘K)"
            className="flex items-center gap-2.5 px-3 py-1.5 h-9 rounded-xl bg-bg-input/70 hover:bg-bg-input text-text-muted hover:text-text-primary transition-all text-xs sm:text-sm font-sans border border-border/50 shadow-xs cursor-pointer whitespace-nowrap shrink-0 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
            title="Mở thanh lệnh nhanh (Ctrl+K / ⌘K)"
          >
            <Search className="h-4 w-4 text-text-muted" />
            <span className="text-[0.6875rem] font-sans hidden sm:inline text-text-secondary">Tìm kiếm...</span>
            <kbd className="px-1.5 py-0.5 rounded-md bg-bg-card border border-border/80 text-[0.625rem] text-text-muted font-mono shadow-xs">
              ⌘K
            </kbd>
          </motion.button>
        )}

        {/* Live system state badge */}
        <div className="flex items-center gap-2 px-3 py-1.5 h-9 rounded-xl bg-bg-input/70 border border-border/50 text-xs sm:text-sm shadow-xs whitespace-nowrap shrink-0">
          <StatusDot
            variant={isServiceActive && isAccountConnected ? "active" : !isServiceActive ? "warning" : "error"}
            size="sm"
          />
          <span className="text-[0.6875rem] font-medium text-text-secondary hidden sm:inline">
            {isServiceActive && isAccountConnected
              ? "Sẵn sàng"
              : !isServiceActive
              ? "Service tạm dừng"
              : "Chưa kết nối"}
          </span>
        </div>

        {/* Quick Language Toggle Pill — VI / EN */}
        {onChangeLang && (
          <div className="flex items-center p-0.5 rounded-xl bg-bg-input/70 border border-border/50 shadow-xs shrink-0">
            {(["vi", "en"] as const).map((lang) => {
              const isActive = currentLang === lang
              return (
                <motion.button
                  key={lang}
                  onClick={() => onChangeLang(lang)}
                  aria-pressed={isActive}
                  whileHover={{ scale: isActive ? 1 : 1.05 }}
                  whileTap={{ scale: 0.96 }}
                  className={`relative px-2.5 py-1 text-[0.6875rem] font-semibold rounded-lg cursor-pointer transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                    isActive
                      ? "text-text-primary"
                      : "text-text-secondary hover:text-text-primary"
                  }`}
                  title={lang === "vi" ? "Tiếng Việt" : "English"}
                >
                  {isActive && (
                    <motion.div
                      layoutId="topbar-lang-pill"
                      transition={{ type: "spring", stiffness: 500, damping: 32 }}
                      className="absolute inset-0 bg-bg-card rounded-lg shadow-sm border border-border/60 -z-10"
                    />
                  )}
                  <span className="relative z-10">{lang.toUpperCase()}</span>
                </motion.button>
              )
            })}
          </div>
        )}

        {/* Apple-style Theme Toggle Button with Spring Morph */}
        <motion.button
          whileHover={{ scale: 1.08 }}
          whileTap={{ scale: 0.92 }}
          onClick={toggleTheme}
          aria-label={resolvedTheme === "dark" ? "Chuyển sang chế độ sáng" : "Chuyển sang chế độ tối"}
          className="relative h-9 w-9 rounded-xl flex items-center justify-center bg-bg-input/70 hover:bg-bg-input text-text-secondary hover:text-text-primary border border-border/50 shadow-xs cursor-pointer overflow-hidden focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
          title={resolvedTheme === "dark" ? "Chuyển sang chế độ sáng" : "Chuyển sang chế độ tối"}
        >
          <AnimatePresence mode="wait" initial={false}>
            {resolvedTheme === "dark" ? (
              <motion.div
                key="sun"
                initial={{ rotate: -90, scale: 0, opacity: 0 }}
                animate={{ rotate: 0, scale: 1, opacity: 1 }}
                exit={{ rotate: 90, scale: 0, opacity: 0 }}
                transition={{ duration: 0.15, ease: "easeOut" }}
              >
                <Sun className="h-4.5 w-4.5 text-warning stroke-[2]" />
              </motion.div>
            ) : (
              <motion.div
                key="moon"
                initial={{ rotate: 90, scale: 0, opacity: 0 }}
                animate={{ rotate: 0, scale: 1, opacity: 1 }}
                exit={{ rotate: -90, scale: 0, opacity: 0 }}
                transition={{ duration: 0.15, ease: "easeOut" }}
              >
                <Moon className="h-4.5 w-4.5 text-accent stroke-[2]" />
              </motion.div>
            )}
          </AnimatePresence>
        </motion.button>

        {/* Action buttons */}
        <div className="flex items-center gap-1">
          {onRestartService && (
            <Button
              size="sm"
              variant="ghost"
              onClick={onRestartService}
              className="h-9 px-3 text-xs sm:text-sm text-text-secondary hover:text-text-primary gap-2 rounded-xl border border-border/50 font-medium"
              title="Khởi động lại background service"
              aria-label="Khởi động lại background service"
            >
              <RotateCcw className="h-4 w-4" />
              <span className="hidden lg:inline text-[0.6875rem] font-medium">Tải lại service</span>
            </Button>
          )}

          {onRefresh && (
            <Button
              size="sm"
              variant="ghost"
              onClick={onRefresh}
              disabled={isRefreshing}
              className="h-9 w-9 p-0 text-text-secondary hover:text-text-primary rounded-xl border border-border/50"
              title="Làm mới dữ liệu"
              aria-label="Làm mới dữ liệu"
            >
              <RefreshCw className={`h-4 w-4 ${isRefreshing ? "animate-spin text-accent" : ""}`} />
            </Button>
          )}
        </div>
      </div>
    </header>
  )
}

