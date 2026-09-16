import React, { useState, useEffect, useRef } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { TabId } from './Sidebar'
import { useTheme } from '@/hooks/useTheme'
import {
  Search,
  LayoutDashboard,
  ArrowLeftRight,
  Settings,
  Terminal,
  RotateCcw,
  SunMoon,
  Send,
  LogIn,
  RefreshCw,
  X,
} from 'lucide-react'

export interface CommandPaletteProps {
  isOpen: boolean
  onClose: () => void
  onSelectTab: (tab: TabId) => void
  onRestartService: () => void
  onOpenBot: () => void
  onTriggerLogin: () => void
  onRefresh: () => void
}

interface CommandItem {
  id: string
  title: string
  subtitle: string
  icon: React.ComponentType<{ className?: string }>
  action: () => void
  category: 'Trang' | 'Thao tác' | 'Giao diện'
}

export const CommandPalette: React.FC<CommandPaletteProps> = ({
  isOpen,
  onClose,
  onSelectTab,
  onRestartService,
  onOpenBot,
  onTriggerLogin,
  onRefresh,
}) => {
  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const inputRef = useRef<HTMLInputElement>(null)
  const { toggleTheme } = useTheme()

  const commands: CommandItem[] = [
    {
      id: 'nav-dashboard',
      title: 'Mở Trang chính',
      subtitle: 'Xem tổng quan hệ thống, tài khoản và thống kê',
      icon: LayoutDashboard,
      category: 'Trang',
      action: () => {
        onSelectTab('dashboard')
        onClose()
      },
    },
    {
      id: 'nav-jobs',
      title: 'Mở Tiến độ & Đồng bộ',
      subtitle: 'Xem danh sách tác vụ clone và theo dõi thời gian thực',
      icon: ArrowLeftRight,
      category: 'Trang',
      action: () => {
        onSelectTab('jobs')
        onClose()
      },
    },
    {
      id: 'nav-settings',
      title: 'Mở Cài đặt',
      subtitle: 'Cấu hình concurrency, tài khoản, ngôn ngữ',
      icon: Settings,
      category: 'Trang',
      action: () => {
        onSelectTab('settings')
        onClose()
      },
    },
    {
      id: 'nav-dev',
      title: 'Mở Công cụ Dev',
      subtitle: 'Chẩn đoán doctor và xem nhật ký runtime',
      icon: Terminal,
      category: 'Trang',
      action: () => {
        onSelectTab('dev')
        onClose()
      },
    },
    {
      id: 'act-restart',
      title: 'Khởi động lại Background Service',
      subtitle: 'Tải lại tiến trình gdclone-bot',
      icon: RotateCcw,
      category: 'Thao tác',
      action: () => {
        onRestartService()
        onClose()
      },
    },
    {
      id: 'act-refresh',
      title: 'Làm mới dữ liệu',
      subtitle: 'Cập nhật lại trạng thái các tác vụ từ engine',
      icon: RefreshCw,
      category: 'Thao tác',
      action: () => {
        onRefresh()
        onClose()
      },
    },
    {
      id: 'act-bot',
      title: 'Mở bot Telegram',
      subtitle: 'Mở ứng dụng Telegram điều khiển bot',
      icon: Send,
      category: 'Thao tác',
      action: () => {
        onOpenBot()
        onClose()
      },
    },
    {
      id: 'act-login',
      title: 'Đăng nhập Google OAuth',
      subtitle: 'Xác thực tài khoản Google Drive',
      icon: LogIn,
      category: 'Thao tác',
      action: () => {
        onTriggerLogin()
        onClose()
      },
    },
    {
      id: 'act-theme',
      title: 'Chuyển đổi giao diện Sáng / Tối',
      subtitle: 'Chuyển đổi phong cách hiển thị',
      icon: SunMoon,
      category: 'Giao diện',
      action: () => {
        toggleTheme()
        onClose()
      },
    },
  ]

  const filtered = commands.filter(
    (c) =>
      c.title.toLowerCase().includes(query.toLowerCase()) ||
      c.subtitle.toLowerCase().includes(query.toLowerCase()) ||
      c.category.toLowerCase().includes(query.toLowerCase())
  )

  useEffect(() => {
    setSelectedIndex(0)
  }, [query])

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 50)
    } else {
      setQuery('')
    }
  }, [isOpen])

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) return

      if (e.key === 'Escape') {
        e.preventDefault()
        onClose()
      } else if (e.key === 'ArrowDown') {
        e.preventDefault()
        setSelectedIndex((prev) => (prev + 1) % (filtered.length || 1))
      } else if (e.key === 'ArrowUp') {
        e.preventDefault()
        setSelectedIndex((prev) => (prev - 1 + filtered.length) % (filtered.length || 1))
      } else if (e.key === 'Enter') {
        e.preventDefault()
        if (filtered[selectedIndex]) {
          filtered[selectedIndex].action()
        }
      }
    }

    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [isOpen, filtered, selectedIndex, onClose])

  const LISTBOX_ID = 'command-palette-listbox'
  const activeDescendantId = filtered[selectedIndex]
    ? `command-option-${filtered[selectedIndex].id}`
    : undefined

  return (
    <AnimatePresence>
      {isOpen && (
        <div className="fixed inset-0 z-50 flex items-start justify-center pt-24 px-4">
          {/* Backdrop blur */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            className="fixed inset-0 bg-black/40 "
          />

          {/* Dialog Container */}
          <motion.div
            initial={{ opacity: 0, scale: 0.96, y: -10 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: -10 }}
            transition={{ duration: 0.1, ease: "easeOut" }}
            role="dialog"
            aria-modal="true"
            aria-label="Thanh lệnh nhanh"
            className="w-full max-w-xl bg-bg-card border border-border/80 rounded-2xl shadow-modal overflow-hidden z-10 select-none flex flex-col max-h-[460px]"
          >
            {/* Search Input Bar */}
            <div className="flex items-center px-4 py-3 border-b border-border/60 gap-3">
              <Search className="h-4 w-4 text-text-muted shrink-0" />
              <input
                ref={inputRef}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Tìm kiếm tác vụ, thao tác hoặc trang..."
                role="combobox"
                aria-expanded="true"
                aria-controls={LISTBOX_ID}
                aria-activedescendant={activeDescendantId}
                aria-label="Tìm kiếm tác vụ, thao tác hoặc trang"
                className="w-full bg-transparent border-0 text-sm text-text-primary placeholder:text-text-muted focus:outline-none"
              />
              <button
                onClick={onClose}
                aria-label="Đóng thanh lệnh"
                className="p-1 rounded text-text-muted hover:text-text-primary hover:bg-bg-input focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            {/* Results List */}
            <div
              id={LISTBOX_ID}
              role="listbox"
              aria-label="Kết quả tìm kiếm"
              className="overflow-y-auto p-2 space-y-1"
            >
              {filtered.length === 0 ? (
                <div className="py-8 text-center text-xs text-text-muted">
                  Không tìm thấy thao tác phù hợp với "{query}"
                </div>
              ) : (
                filtered.map((item, index) => {
                  const Icon = item.icon
                  const isSelected = index === selectedIndex

                  return (
                    <button
                      key={item.id}
                      id={`command-option-${item.id}`}
                      role="option"
                      aria-selected={isSelected}
                      onClick={item.action}
                      onMouseEnter={() => setSelectedIndex(index)}
                      className={`w-full text-left flex items-center justify-between px-3 py-2.5 rounded-xl transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                        isSelected
                          ? 'bg-accent/12 text-accent'
                          : 'text-text-primary hover:bg-bg-input/60'
                      }`}
                    >
                      <div className="flex items-center gap-3 min-w-0">
                        <div
                          className={`p-1.5 rounded-lg shrink-0 ${
                            isSelected ? 'bg-accent/20 text-accent' : 'bg-bg-input text-text-secondary'
                          }`}
                        >
                          <Icon className="h-4 w-4 stroke-[1.75]" />
                        </div>
                        <div className="truncate">
                          <p className="text-xs font-medium truncate">{item.title}</p>
                          <p className="text-[0.6875rem] text-text-muted truncate">{item.subtitle}</p>
                        </div>
                      </div>
                      <span className="text-[0.625rem] font-mono px-2 py-0.5 rounded bg-bg-input/60 text-text-muted shrink-0 ml-2">
                        {item.category}
                      </span>
                    </button>
                  )
                })
              )}
            </div>

            {/* Footer hints */}
            <div className="px-4 py-2 border-t border-border/50 bg-bg-input/30 flex items-center justify-between text-[0.6875rem] text-text-muted">
              <div className="flex items-center gap-3 font-mono text-[0.625rem]">
                <span>↑↓ điều hướng</span>
                <span>↵ chọn</span>
                <span>esc đóng</span>
              </div>
              <span className="font-mono text-[0.625rem]">⌘K / Ctrl+K</span>
            </div>
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  )
}
