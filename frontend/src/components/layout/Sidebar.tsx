import React from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { LayoutDashboard, ArrowLeftRight, Settings, Terminal, ChevronLeft, ChevronRight, HardDrive } from 'lucide-react'
import { cn } from '@/lib/utils'

export type TabId = 'dashboard' | 'jobs' | 'settings' | 'dev'

export interface SidebarProps {
  activeTab: TabId
  onSelectTab: (tab: TabId) => void
  isCollapsed: boolean
  onToggleCollapse: () => void
  version?: string
}

interface NavItem {
  id: TabId
  label: string
  icon: React.ComponentType<{ className?: string }>
}

const navItems: NavItem[] = [
  { id: 'dashboard', label: 'Trang chính', icon: LayoutDashboard },
  { id: 'jobs', label: 'Tiến độ & Đồng bộ', icon: ArrowLeftRight },
  { id: 'settings', label: 'Cài đặt', icon: Settings },
  { id: 'dev', label: 'Công cụ Dev', icon: Terminal },
]

export const Sidebar: React.FC<SidebarProps> = ({
  activeTab,
  onSelectTab,
  isCollapsed,
  onToggleCollapse,
  version = 'v0.1.0',
}) => {
  return (
    <motion.aside
      animate={{ width: isCollapsed ? 60 : 210 }}
      transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
      className="h-screen bg-bg-elevated border-r border-border flex flex-col justify-between select-none relative shrink-0 z-20"
    >
      {/* Brand Header */}
      <div>
        <div className="h-13 border-b border-border flex items-center px-3.5 gap-3">
          <div className="h-7 w-7 rounded-lg bg-accent/15 border border-accent/30 flex items-center justify-center shrink-0 text-accent">
            <HardDrive className="h-4 w-4 stroke-[1.8]" />
          </div>
          <AnimatePresence>
            {!isCollapsed && (
              <motion.div
                initial={{ opacity: 0, x: -8 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -8 }}
                transition={{ duration: 0.15 }}
                className="overflow-hidden whitespace-nowrap"
              >
                <span className="font-bold text-sm tracking-tight text-text-primary">502Drive</span>
                <span className="ml-1 text-[10px] text-accent font-mono">core</span>
              </motion.div>
            )}
          </AnimatePresence>
        </div>

        {/* Navigation List */}
        <nav className="p-2 space-y-1 mt-2">
          {navItems.map((item) => {
            const Icon = item.icon
            const isActive = activeTab === item.id

            return (
              <button
                key={item.id}
                onClick={() => onSelectTab(item.id)}
                title={isCollapsed ? item.label : undefined}
                className={cn(
                  'w-full flex items-center gap-3 px-3 py-2 rounded-md text-xs font-medium transition-colors relative cursor-pointer group',
                  isActive
                    ? 'text-accent bg-accent/10 border border-accent/20'
                    : 'text-text-secondary hover:text-text-primary hover:bg-white/5 border border-transparent'
                )}
              >
                {/* Active indicator bar */}
                {isActive && (
                  <motion.span
                    layoutId="sidebar-active-indicator"
                    className="absolute left-0 top-1.5 bottom-1.5 w-1 rounded-r bg-accent"
                  />
                )}
                <Icon className={cn('h-4 w-4 shrink-0 stroke-[1.6]', isActive ? 'text-accent' : 'text-text-secondary group-hover:text-text-primary')} />
                <AnimatePresence>
                  {!isCollapsed && (
                    <motion.span
                      initial={{ opacity: 0 }}
                      animate={{ opacity: 1 }}
                      exit={{ opacity: 0 }}
                      transition={{ duration: 0.12 }}
                      className="whitespace-nowrap overflow-hidden"
                    >
                      {item.label}
                    </motion.span>
                  )}
                </AnimatePresence>
              </button>
            )
          })}
        </nav>
      </div>

      {/* Footer / Collapse Toggle */}
      <div className="p-2 border-t border-border space-y-1">
        <div className="flex items-center justify-between px-2 py-1 text-[11px] text-text-muted">
          <AnimatePresence>
            {!isCollapsed && (
              <motion.span
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="font-mono text-[10px]"
              >
                {version}
              </motion.span>
            )}
          </AnimatePresence>

          <button
            onClick={onToggleCollapse}
            className="p-1 rounded hover:bg-white/5 hover:text-text-primary text-text-muted transition-colors ml-auto"
            title={isCollapsed ? 'Mở rộng sidebar' : 'Thu gọn sidebar'}
          >
            {isCollapsed ? <ChevronRight className="h-3.5 w-3.5" /> : <ChevronLeft className="h-3.5 w-3.5" />}
          </button>
        </div>
      </div>
    </motion.aside>
  )
}
