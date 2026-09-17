import React, { useState } from "react"
import { motion, AnimatePresence } from "motion/react"
import { StatusDot } from "@/components/primitives/StatusDot"
import { Button } from "@/components/ui/Button"
import { RefreshCw, RotateCcw, Search, Sun, Moon, ArrowUpCircle } from "lucide-react"
import { SystemStatus } from "@/lib/types"
import { useTheme } from "@/hooks/useTheme"
import { useI18n } from "@/hooks/useI18n"
import { useCircularThemeToggle } from "@/hooks/useCircularThemeToggle"

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
  /** True when a newer remote version is detected. */
  updateAvailable?: boolean
  /** Latest detected version string. */
  latestVersion?: string
  /** Handler to open the UpdateModal. */
  onOpenUpdateModal?: () => void
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
  updateAvailable,
  latestVersion,
  onOpenUpdateModal,
}) => {
  const isServiceActive = status?.service_active ?? false
  const isAccountConnected = status?.account_status === "connected"
  const isReconnectRequired = status?.account_status === "reconnect_required"
  const { resolvedTheme, setTheme } = useTheme()
  const { t, lang, setLang } = useI18n()
  const { toggleTheme: circularToggleTheme } = useCircularThemeToggle({
    currentTheme: resolvedTheme,
    setTheme,
  })
  const [isRestarting, setIsRestarting] = useState(false)

  const handleRestartService = () => {
    if (isRestarting) return
    setIsRestarting(true)
    Promise.resolve(onRestartService?.()).finally(() => {
      // Minimum spin so the affordance reads even on instant responses.
      setTimeout(() => setIsRestarting(false), 800)
    })
  }

  const activeLang = currentLang || lang

  const handleLangToggle = (selected: 'vi' | 'en') => {
    setLang(selected)
    onChangeLang?.(selected)
  }

  const getStatusInfo = () => {
    if (!isServiceActive) return { label: t('topbar.service_paused'), variant: 'warning' as const }
    if (isReconnectRequired) return { label: t('topbar.reconnect_required'), variant: 'warning' as const }
    if (isAccountConnected) return { label: t('topbar.ready'), variant: 'active' as const }
    return { label: t('topbar.not_connected'), variant: 'error' as const }
  }

  const statusInfo = getStatusInfo()

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
            aria-label={`${t('command_palette.placeholder')} (${t('topbar.search_shortcut')})`}
            className="flex items-center gap-2.5 px-3 py-1.5 h-9 rounded-xl bg-bg-input/70 hover:bg-bg-input text-text-muted hover:text-text-primary transition-all text-xs sm:text-sm font-sans border border-border/50 shadow-xs cursor-pointer whitespace-nowrap shrink-0 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
            title={`${t('command_palette.placeholder')} (${t('topbar.search_shortcut')})`}
          >
            <Search className="h-4 w-4 text-text-muted" />
            <span className="text-[0.6875rem] font-sans hidden sm:inline text-text-secondary">{t('topbar.search_placeholder')}</span>
            <kbd className="px-1.5 py-0.5 rounded-md bg-bg-card border border-border/80 text-[0.625rem] text-text-muted font-mono shadow-xs">
              {t('topbar.search_shortcut')}
            </kbd>
          </motion.button>
        )}

        {/* Live system state badge */}
        <div className="flex items-center gap-2 px-3 py-1.5 h-9 rounded-xl bg-bg-input/70 border border-border/50 text-xs sm:text-sm shadow-xs whitespace-nowrap shrink-0">
          <StatusDot
            variant={statusInfo.variant}
            size="sm"
          />
          <span className="text-[0.6875rem] font-medium text-text-secondary hidden sm:inline">
            {statusInfo.label}
          </span>
        </div>

        {/* Quick Language Toggle Pill — VI / EN */}
        <div className="flex items-center p-0.5 rounded-xl bg-bg-input/70 border border-border/50 shadow-xs shrink-0">
          {(["vi", "en"] as const).map((itemLang) => {
            const isActive = activeLang === itemLang
            return (
              <motion.button
                key={itemLang}
                onClick={() => handleLangToggle(itemLang)}
                aria-pressed={isActive}
                whileHover={{ scale: isActive ? 1 : 1.05 }}
                whileTap={{ scale: 0.96 }}
                className={`relative w-9 py-1 flex items-center justify-center text-[0.6875rem] font-semibold rounded-lg cursor-pointer transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                  isActive
                    ? "text-text-primary"
                    : "text-text-secondary hover:text-text-primary"
                }`}
                title={itemLang === "vi" ? "Tiếng Việt" : "English"}
              >
                {isActive && (
                  <motion.div
                    layoutId="topbar-lang-pill"
                    transition={{ type: "spring", stiffness: 500, damping: 32 }}
                    className="absolute inset-0 bg-bg-card rounded-lg shadow-sm border border-border/60 -z-10"
                  />
                )}
                <span className="relative z-10">{itemLang.toUpperCase()}</span>
              </motion.button>
            )
          })}
        </div>

        {/* Remote Update Notification Button */}
        {updateAvailable && onOpenUpdateModal && (
          <motion.button
            initial={{ scale: 0.85, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            whileHover={{ scale: 1.04 }}
            whileTap={{ scale: 0.96 }}
            onClick={onOpenUpdateModal}
            className="h-9 px-2.5 rounded-xl flex items-center gap-1.5 bg-accent/15 hover:bg-accent/25 border border-accent/40 text-accent font-semibold text-xs shadow-xs cursor-pointer transition-colors"
            title={t('update_modal.badge_available')}
          >
            <span className="relative flex h-2 w-2">
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-accent opacity-75"></span>
              <span className="relative inline-flex rounded-full h-2 w-2 bg-accent"></span>
            </span>
            <ArrowUpCircle className="h-4 w-4" />
            <span className="hidden sm:inline font-mono">{latestVersion || 'v0.2.1'}</span>
          </motion.button>
        )}

        {/* Theme Toggle — Telegram-style circular reveal outside the VDOM */}
        <motion.button
          whileHover={{ scale: 1.08 }}
          whileTap={{ scale: 0.92 }}
          onClick={(e) => circularToggleTheme(e)}
          aria-label={resolvedTheme === "dark" ? t('topbar.toggle_light') : t('topbar.toggle_dark')}
          className="relative h-9 w-9 rounded-xl flex items-center justify-center bg-bg-input/70 hover:bg-bg-input text-text-secondary hover:text-text-primary border border-border/50 shadow-xs cursor-pointer overflow-hidden focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
          title={resolvedTheme === "dark" ? t('topbar.toggle_light') : t('topbar.toggle_dark')}
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
                exit={{ rotate: 90, scale: 0, opacity: 0 }}
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
              onClick={handleRestartService}
              disabled={isRestarting}
              className="h-9 px-3 text-xs sm:text-sm text-text-secondary hover:text-text-primary gap-2 rounded-xl border border-border/50 font-medium"
              title={t('topbar.reload_service')}
              aria-label={t('topbar.reload_service')}
            >
              <RotateCcw className={`h-4 w-4 ${isRestarting ? "animate-spin text-accent" : ""}`} />
              <span className="hidden lg:inline text-[0.6875rem] font-medium">{t('topbar.reload_service')}</span>
            </Button>
          )}

          {onRefresh && (
            <Button
              size="sm"
              variant="ghost"
              onClick={onRefresh}
              disabled={isRefreshing}
              className="h-9 w-9 p-0 text-text-secondary hover:text-text-primary rounded-xl border border-border/50"
              title={t('topbar.refresh_data')}
              aria-label={t('topbar.refresh_data')}
            >
              <RefreshCw className={`h-4 w-4 ${isRefreshing ? "animate-spin text-accent" : ""}`} />
            </Button>
          )}
        </div>
      </div>
    </header>
  )
}

