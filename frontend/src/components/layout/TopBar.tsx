import React from 'react'
import { StatusDot } from '@/components/primitives/StatusDot'
import { Button } from '@/components/ui/Button'
import { RefreshCw, RotateCcw } from 'lucide-react'
import { SystemStatus } from '@/lib/types'

export interface TopBarProps {
  title: string
  subtitle?: string
  status: SystemStatus | null
  isRefreshing?: boolean
  onRefresh?: () => void
  onRestartService?: () => void
}

export const TopBar: React.FC<TopBarProps> = ({
  title,
  subtitle,
  status,
  isRefreshing,
  onRefresh,
  onRestartService,
}) => {
  const isServiceActive = status?.service_active ?? false
  const isAccountConnected = status?.account_status === 'connected'

  return (
    <header className="h-13 border-b border-border bg-bg-elevated/80 backdrop-blur-md px-5 flex items-center justify-between shrink-0 select-none">
      <div className="flex items-baseline gap-2.5">
        <h1 className="text-sm font-semibold text-text-primary tracking-tight">{title}</h1>
        {subtitle && <span className="text-xs text-text-muted">{subtitle}</span>}
      </div>

      <div className="flex items-center gap-3">
        {/* Live system state badge */}
        <div className="flex items-center gap-2 px-2.5 py-1 rounded-full bg-bg-card border border-border text-xs">
          <StatusDot
            variant={isServiceActive && isAccountConnected ? 'active' : !isServiceActive ? 'warning' : 'error'}
            size="sm"
          />
          <span className="text-[11px] font-medium text-text-secondary">
            {isServiceActive && isAccountConnected
              ? 'Hệ thống sẵn sàng'
              : !isServiceActive
              ? 'Service tạm dừng'
              : 'Chưa đăng nhập'}
          </span>
        </div>

        {/* Action buttons */}
        <div className="flex items-center gap-1">
          {onRestartService && (
            <Button
              size="sm"
              variant="ghost"
              onClick={onRestartService}
              className="h-7 px-2 text-xs text-text-secondary hover:text-text-primary gap-1.5"
              title="Khởi động lại background service"
            >
              <RotateCcw className="h-3.5 w-3.5" />
              <span className="hidden sm:inline">Khởi động lại</span>
            </Button>
          )}

          {onRefresh && (
            <Button
              size="sm"
              variant="ghost"
              onClick={onRefresh}
              disabled={isRefreshing}
              className="h-7 w-7 p-0 text-text-secondary hover:text-text-primary"
              title="Làm mới dữ liệu"
            >
              <RefreshCw className={`h-3.5 w-3.5 ${isRefreshing ? 'animate-spin text-accent' : ''}`} />
            </Button>
          )}
        </div>
      </div>
    </header>
  )
}
