import React, { useState } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { Sidebar, TabId } from './Sidebar'
import { TopBar } from './TopBar'
import { SystemStatus } from '@/lib/types'

export interface ShellProps {
  activeTab: TabId
  onSelectTab: (tab: TabId) => void
  status: SystemStatus | null
  isRefreshing?: boolean
  onRefresh?: () => void
  onRestartService?: () => void
  children: React.ReactNode
}

const tabTitles: Record<TabId, { title: string; subtitle: string }> = {
  dashboard: { title: 'Trang chính', subtitle: 'Tổng quan hệ thống & tài khoản' },
  jobs: { title: 'Tiến độ & Đồng bộ', subtitle: 'Quản lý tác vụ clone và theo dõi thư mục' },
  settings: { title: 'Cài đặt', subtitle: 'Cấu hình engine, tài khoản và tham số ứng dụng' },
  dev: { title: 'Công cụ Dev', subtitle: 'Báo cáo chẩn đoán hệ thống và nhật ký vận hành' },
}

export const Shell: React.FC<ShellProps> = ({
  activeTab,
  onSelectTab,
  status,
  isRefreshing,
  onRefresh,
  onRestartService,
  children,
}) => {
  const [isCollapsed, setIsCollapsed] = useState(false)
  const currentInfo = tabTitles[activeTab]

  return (
    <div className="flex h-screen w-screen bg-bg-base overflow-hidden font-sans">
      <Sidebar
        activeTab={activeTab}
        onSelectTab={onSelectTab}
        isCollapsed={isCollapsed}
        onToggleCollapse={() => setIsCollapsed(!isCollapsed)}
        version={status?.app_version || 'v0.1.0'}
      />

      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        <TopBar
          title={currentInfo.title}
          subtitle={currentInfo.subtitle}
          status={status}
          isRefreshing={isRefreshing}
          onRefresh={onRefresh}
          onRestartService={onRestartService}
        />

        <main className="flex-1 overflow-y-auto p-5 bg-bg-base">
          <AnimatePresence mode="wait">
            <motion.div
              key={activeTab}
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.16, ease: [0.16, 1, 0.3, 1] }}
              className="max-w-5xl mx-auto space-y-5"
            >
              {children}
            </motion.div>
          </AnimatePresence>
        </main>
      </div>
    </div>
  )
}
