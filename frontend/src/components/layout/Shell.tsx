import React, { useState, useEffect } from "react"
import { motion, AnimatePresence } from "motion/react"
import { Sidebar, TabId } from "./Sidebar"
import { TopBar } from "./TopBar"
import { CommandPalette } from "./CommandPalette"
import { SystemStatus } from "@/lib/types"
import { useToast } from "@/components/primitives/Toast"
import { FolderUp } from "lucide-react"

export interface ShellProps {
  activeTab: TabId
  onSelectTab: (tab: TabId) => void
  status: SystemStatus | null
  isRefreshing?: boolean
  onRefresh?: () => void
  onRestartService?: () => void
  onOpenBot?: () => void
  onTriggerLogin?: () => void
  /** Current language setting ("vi" | "en") for the TopBar pill. */
  currentLang?: string
  /** Callback when user toggles language from the TopBar. */
  onChangeLang?: (lang: string) => void
  children: React.ReactNode
}

const tabTitles: Record<TabId, { title: string; subtitle: string }> = {
  dashboard: { title: "Trang chính", subtitle: "Tổng quan hệ thống & tài khoản" },
  jobs: { title: "Tiến độ & đồng bộ", subtitle: "Quản lý tác vụ sao chép và theo dõi thư mục" },
  settings: { title: "Cài đặt", subtitle: "Cấu hình tham số engine, tài khoản và giao diện" },
  dev: { title: "Công cụ Dev", subtitle: "Báo cáo chẩn đoán hệ thống và nhật ký vận hành" },
}

export const Shell: React.FC<ShellProps> = ({
  activeTab,
  onSelectTab,
  status,
  isRefreshing,
  onRefresh,
  onRestartService,
  onOpenBot,
  onTriggerLogin,
  currentLang,
  onChangeLang,
  children,
}) => {
  const [isCollapsed, setIsCollapsed] = useState(false)
  const [isCommandOpen, setIsCommandOpen] = useState(false)
  const [isDraggingOver, setIsDraggingOver] = useState(false)
  const { toast } = useToast()

  const currentInfo = tabTitles[activeTab]

  // Global shortcut for ⌘K / Ctrl+K
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault()
        setIsCommandOpen((prev) => !prev)
      }
    }
    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [])

  // Drag & Drop handler for Google Drive links
  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDraggingOver(true)
  }

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDraggingOver(false)
  }

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault()
    setIsDraggingOver(false)

    const text = e.dataTransfer.getData("text/plain")
    if (text && (text.includes("drive.google.com") || text.includes("folders/") || text.includes("id="))) {
      toast({
        title: "Nhận diện liên kết Google Drive",
        description: `Đã phát hiện liên kết: ${text.slice(0, 48)}... Vui lòng xác nhận lệnh clone trong bot.`,
        variant: "success",
        duration: 5000,
      })
    } else if (text) {
      toast({
        title: "Liên kết không hợp lệ",
        description: "Vui lòng kéo thả liên kết thư mục hoặc tệp Google Drive.",
        variant: "warning",
      })
    }
  }

  return (
    <div
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className="flex h-screen w-screen bg-bg-base overflow-hidden font-sans relative"
    >
      {/* Drag & Drop Visual Overlay */}
      <AnimatePresence>
        {isDraggingOver && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 z-50 bg-bg-base/80  border-2 border-dashed border-accent flex flex-col items-center justify-center gap-3 select-none pointer-events-none"
          >
            <div className="w-14 h-14 rounded-2xl bg-accent/15 text-accent flex items-center justify-center">
              <FolderUp className="h-7 w-7 stroke-[1.75]" />
            </div>
            <div className="text-center space-y-1">
              <p className="text-base font-semibold text-text-primary">Thả liên kết Google Drive vào đây</p>
              <p className="text-xs text-text-muted">502Drive sẽ tự động phân tích và bắt đầu sao chép</p>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      <Sidebar
        activeTab={activeTab}
        onSelectTab={onSelectTab}
        isCollapsed={isCollapsed}
        onToggleCollapse={() => setIsCollapsed(!isCollapsed)}
        version={status?.app_version || "v0.2.0"}
      />

      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        <TopBar
          title={currentInfo.title}
          subtitle={currentInfo.subtitle}
          status={status}
          isRefreshing={isRefreshing}
          onRefresh={onRefresh}
          onRestartService={onRestartService}
          onOpenCommandPalette={() => setIsCommandOpen(true)}
          currentLang={currentLang}
          onChangeLang={onChangeLang}
        />

        {/* Responsive viewport container with fluid auto-centering */}
        <main className="flex-1 overflow-y-auto overflow-x-hidden bg-bg-base p-4 sm:p-5 lg:p-6 xl:p-8 transition-colors">
          <AnimatePresence mode="wait">
            <motion.div
              key={activeTab}
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.12, ease: "easeOut" }}
              className="w-full max-w-7xl mx-auto space-y-6 lg:space-y-8"
            >
              {children}
            </motion.div>
          </AnimatePresence>
        </main>
      </div>

      {/* ⌘K / Ctrl+K Command Palette Modal */}
      <CommandPalette
        isOpen={isCommandOpen}
        onClose={() => setIsCommandOpen(false)}
        onSelectTab={onSelectTab}
        onRestartService={onRestartService || (() => {})}
        onOpenBot={onOpenBot || (() => {})}
        onTriggerLogin={onTriggerLogin || (() => {})}
        onRefresh={onRefresh || (() => {})}
      />
    </div>
  )
}
