import React, { useEffect, useState } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { LayoutDashboard, ArrowLeftRight, Settings, Terminal, ChevronLeft, ChevronRight } from 'lucide-react'
import { LogoMark } from '@/components/brand/Logo'
import { cn } from '@/lib/utils'

export type TabId = 'dashboard' | 'jobs' | 'settings' | 'dev'

export interface SidebarProps {
  activeTab: TabId
  onSelectTab: (tab: TabId) => void
  isCollapsed: boolean
  onToggleCollapse: () => void
  version?: string
}

import { useI18n } from '@/hooks/useI18n'

interface NavItem {
  id: TabId
  label: string
  icon: React.ComponentType<{ className?: string }>
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeTab,
  onSelectTab,
  isCollapsed,
  onToggleCollapse,
  version = 'v0.2.0',
}) => {
  const { t } = useI18n()

  const navItems: NavItem[] = [
    { id: 'dashboard', label: t('nav.dashboard'), icon: LayoutDashboard },
    { id: 'jobs', label: t('nav.jobs'), icon: ArrowLeftRight },
    { id: 'settings', label: t('nav.settings'), icon: Settings },
    { id: 'dev', label: t('nav.dev'), icon: Terminal },
  ]

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
      className="h-screen bg-bg-elevated border-r border-border/60 flex flex-col justify-between select-none relative shrink-0 z-20 transition-colors overflow-hidden"
    >
      {/* Brand Header */}
      <div>
        <div className="h-14 border-b border-border/60 flex items-center px-3.5 gap-2.5 overflow-hidden">
          <LogoMark size="sm" />
          <AnimatePresence>
            {!isCollapsed && (
              <motion.div
                initial={{ opacity: 0, x: -6 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -6 }}
                transition={{ duration: 0.15 }}
                className="overflow-hidden whitespace-nowrap flex items-center gap-1.5"
              >
                <span className="font-bold text-sm tracking-tight text-text-primary">
                  <span className="text-accent font-extrabold">502</span>
                  <span>Drive</span>
                </span>
                <span className="text-[0.625rem] px-1.5 py-0.5 rounded-md bg-accent/15 text-accent font-mono font-semibold tracking-wide">CORE</span>
              </motion.div>
            )}
          </AnimatePresence>
        </div>

        {/* Navigation List */}
        <nav className="p-2 space-y-1 mt-1.5 overflow-hidden">
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
                  'w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium transition-all cursor-pointer group relative select-none overflow-hidden focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40',
                  isActive
                    ? 'text-accent bg-accent/12 font-semibold shadow-xs'
                    : 'text-text-secondary hover:text-text-primary hover:bg-bg-input/70'
                )}
              >
                {/* Active indicator — inset, vertically centered, slides between items */}
                {isActive && (
                  <motion.span
                    layoutId="sidebar-active-pill"
                    transition={{ type: 'spring', stiffness: 500, damping: 34 }}
                    className="absolute left-1.5 top-1/2 -translate-y-1/2 h-4 w-[3px] rounded-full bg-accent"
                  />
                )}
                <Icon
                  className={cn(
                    'h-4.5 w-4.5 shrink-0 stroke-[1.8] transition-transform duration-200 motion-safe:group-hover:scale-110 motion-safe:group-active:scale-90',
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
