import React, { useEffect, useState } from 'react'
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
  { id: 'jobs', label: 'Tiến độ & đồng bộ', icon: ArrowLeftRight },
  { id: 'settings', label: 'Cài đặt', icon: Settings },
  { id: 'dev', label: 'Công cụ Dev', icon: Terminal },
]

export const Sidebar: React.FC<SidebarProps> = ({
  activeTab,
  onSelectTab,
  isCollapsed,
  onToggleCollapse,
  version = 'v0.2.0',
}) => {
  // Sidebar base widths (64/228px @16px root) must track the responsive root
  // font-size so labels never clip when the window grows.
  const [scale, setScale] = useState(1)
  useEffect(() => {
    const update = () => setScale(parseFloat(getComputedStyle(document.documentElement).fontSize) / 16)
    update()
    window.addEventListener('resize', update)
    return () => window.removeEventListener('resize', update)
  }, [])

  return (
    <motion.aside
      animate={{ width: Math.round((isCollapsed ? 64 : 228) * scale) }}
      transition={{ duration: 0.18, ease: [0.16, 1, 0.3, 1] }}
      className="h-screen bg-bg-elevated border-r border-border/60 flex flex-col justify-between select-none relative shrink-0 z-20 transition-colors"
    >
      {/* Brand Header */}
      <div>
        <div className="h-14 border-b border-border/60 flex items-center px-4 gap-3">
          <div className="h-8 w-8 rounded-xl bg-accent/12 flex items-center justify-center shrink-0 text-accent shadow-xs">
            <HardDrive className="h-4.5 w-4.5 stroke-[2]" />
          </div>
          <AnimatePresence>
            {!isCollapsed && (
              <motion.div
                initial={{ opacity: 0, x: -6 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -6 }}
                transition={{ duration: 0.15 }}
                className="overflow-hidden whitespace-nowrap flex items-center gap-1.5"
              >
                <span className="font-bold text-sm tracking-tight text-text-primary">502Drive</span>
                <span className="text-[0.625rem] px-1.5 py-0.5 rounded-md bg-accent/15 text-accent font-mono font-semibold tracking-wide">CORE</span>
              </motion.div>
            )}
          </AnimatePresence>
        </div>

        {/* Navigation List */}
        <nav className="p-2 space-y-1 mt-1.5">
          {navItems.map((item) => {
            const Icon = item.icon
            const isActive = activeTab === item.id

            return (
              <button
                key={item.id}
                onClick={() => onSelectTab(item.id)}
                title={isCollapsed ? item.label : undefined}
                aria-current={isActive ? 'page' : undefined}
                className={cn(
                  'w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium transition-all cursor-pointer group relative select-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40',
                  isActive
                    ? 'text-accent bg-accent/12 font-semibold shadow-xs'
                    : 'text-text-secondary hover:text-text-primary hover:bg-bg-input/70'
                )}
              >
                {/* Active indicator pill */}
                {isActive && (
                  <motion.span
                    layoutId="sidebar-active-pill"
                    className="absolute left-0 top-1.5 bottom-1.5 w-1 rounded-r-full bg-accent"
                  />
                )}
                <Icon
                  className={cn(
                    'h-4.5 w-4.5 shrink-0 stroke-[1.8]',
                    isActive ? 'text-accent' : 'text-text-secondary group-hover:text-text-primary'
                  )}
                />
                <AnimatePresence>
                  {!isCollapsed && (
                    <motion.span
                      initial={{ opacity: 0 }}
                      animate={{ opacity: 1 }}
                      exit={{ opacity: 0 }}
                      transition={{ duration: 0.12 }}
                      className="whitespace-nowrap overflow-hidden text-sm"
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
      <div className="p-2 border-t border-border/50 space-y-1">
        <div className="flex items-center justify-between px-1.5 py-1 text-text-muted">
          <AnimatePresence>
            {!isCollapsed && (
              <motion.span
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="font-mono text-[0.625rem]"
              >
                {version}
              </motion.span>
            )}
          </AnimatePresence>

          <button
            onClick={onToggleCollapse}
            aria-label={isCollapsed ? 'Mở rộng thanh bên' : 'Thu gọn thanh bên'}
            className="p-1 rounded-md hover:bg-bg-input hover:text-text-primary text-text-muted transition-colors ml-auto cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
            title={isCollapsed ? 'Mở rộng thanh bên' : 'Thu gọn thanh bên'}
          >
            {isCollapsed ? <ChevronRight className="h-3.5 w-3.5" /> : <ChevronLeft className="h-3.5 w-3.5" />}
          </button>
        </div>
      </div>
    </motion.aside>
  )
}
