import React, { useState } from 'react'
import { Card } from '@/components/ui/Card'
import { Button } from '@/components/ui/Button'
import { ConfigSummary, SystemStatus } from '@/lib/types'
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
} from 'lucide-react'

export interface SettingsProps {
  config: ConfigSummary | null
  status: SystemStatus | null
  onUpdateConfig: (field: string, value: unknown) => void
  onTriggerLogin: () => void
  onTriggerRevoke: () => void
  onRestartService: () => void
  onCheckUpdate: () => void
}

export const Settings: React.FC<SettingsProps> = ({
  config,
  status,
  onUpdateConfig,
  onTriggerLogin,
  onTriggerRevoke,
  onRestartService,
  onCheckUpdate,
}) => {
  const [isUpdating, setIsUpdating] = useState(false)
  const [updateChecked, setUpdateChecked] = useState(false)

  const handleCheckUpdate = () => {
    setIsUpdating(true)
    setTimeout(() => {
      setIsUpdating(false)
      setUpdateChecked(true)
      onCheckUpdate()
    }, 1200)
  }

  const isAccountConnected = status?.account_status === 'connected'

  return (
    <div className="space-y-6 max-w-3xl pb-10">
      {/* ── Group 1: Tài khoản & Kết nối ─────────────────── */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-text-secondary px-1">
          Tài khoản & Kết nối
        </h2>
        <Card className="divide-y divide-border/60 p-0 overflow-hidden">
          {/* Google Account */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2 rounded bg-bg-elevated text-accent border border-border/60">
                <HardDrive className="h-4 w-4 stroke-[1.6]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">Tài khoản Google Drive</p>
                <p className="text-xs text-text-secondary font-mono">
                  {status?.google_account || 'Chưa liên kết tài khoản Google'}
                </p>
              </div>
            </div>

            <div>
              {isAccountConnected ? (
                <div className="flex items-center gap-2">
                  <Button size="sm" variant="secondary" onClick={onTriggerLogin} className="text-xs">
                    Đổi tài khoản
                  </Button>
                  <Button size="sm" variant="danger" onClick={onTriggerRevoke} className="text-xs gap-1">
                    <LogOut className="h-3 w-3" />
                    Ngắt kết nối
                  </Button>
                </div>
              ) : (
                <Button size="sm" variant="primary" onClick={onTriggerLogin} className="text-xs gap-1.5">
                  <LogIn className="h-3.5 w-3.5" />
                  Kết nối ngay
                </Button>
              )}
            </div>
          </div>

          {/* Telegram Bot */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2 rounded bg-bg-elevated text-accent border border-border/60">
                <Send className="h-4 w-4 stroke-[1.6]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">Telegram Owner ID</p>
                <p className="text-xs text-text-secondary">
                  Chỉ cho phép Telegram ID này điều khiển bot: <span className="font-mono text-text-primary">{config?.owner_telegram_id || 0}</span>
                </p>
              </div>
            </div>

            <Button size="sm" variant="secondary" onClick={onRestartService} className="text-xs gap-1.5">
              <RotateCcw className="h-3 w-3" />
              Tải lại service
            </Button>
          </div>
        </Card>
      </div>

      {/* ── Group 2: Engine & Hiệu năng ─────────────────── */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-text-secondary px-1">
          Engine & Đồng bộ
        </h2>
        <Card className="divide-y divide-border/60 p-0 overflow-hidden">
          {/* Concurrency */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2 rounded bg-bg-elevated text-accent border border-border/60">
                <Zap className="h-4 w-4 stroke-[1.6]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">Số luồng sao chép song song (Concurrency)</p>
                <p className="text-xs text-text-secondary">
                  Số lượng worker tải và nhân bản tệp đồng thời trên Google Drive
                </p>
              </div>
            </div>

            <div className="flex items-center gap-3">
              <input
                type="range"
                min="1"
                max="32"
                value={config?.engine_concurrency ?? 8}
                onChange={(e) => onUpdateConfig('engine_concurrency', parseInt(e.target.value))}
                className="w-24 accent-accent cursor-pointer"
              />
              <span className="text-sm font-mono font-bold text-accent min-w-[24px] text-right">
                {config?.engine_concurrency ?? 8}
              </span>
            </div>
          </div>

          {/* Auto confirm clone */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2 rounded bg-bg-elevated text-accent border border-border/60">
                <Power className="h-4 w-4 stroke-[1.6]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">Tự động xác nhận lệnh clone</p>
                <p className="text-xs text-text-secondary">
                  Bỏ qua bước hỏi Yes/No trong bot Telegram khi nhận link Drive
                </p>
              </div>
            </div>

            <button
              onClick={() => onUpdateConfig('auto_confirm_clone', !config?.auto_confirm_clone)}
              className={`w-11 h-6 flex items-center rounded-full p-1 cursor-pointer transition-colors duration-200 ease-in-out ${
                config?.auto_confirm_clone ? 'bg-accent' : 'bg-bg-elevated border border-border'
              }`}
            >
              <div
                className={`bg-bg-base w-4 h-4 rounded-full shadow-md transform transition-transform duration-200 ease-in-out ${
                  config?.auto_confirm_clone ? 'translate-x-5' : 'translate-x-0'
                }`}
              />
            </button>
          </div>
        </Card>
      </div>

      {/* ── Group 3: Hệ thống & Khởi động ────────────────── */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-text-secondary px-1">
          Hệ thống & Ứng dụng
        </h2>
        <Card className="divide-y divide-border/60 p-0 overflow-hidden">
          {/* Launch at startup */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2 rounded bg-bg-elevated text-text-secondary border border-border/60">
                <Power className="h-4 w-4 stroke-[1.6]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">Tự khởi động cùng hệ thống</p>
                <p className="text-xs text-text-secondary">
                  Tự động kích hoạt systemd service (Linux) hoặc Startup Task (Windows) khi đăng nhập
                </p>
              </div>
            </div>

            <button
              onClick={() => onUpdateConfig('launch_at_startup', !config?.launch_at_startup)}
              className={`w-11 h-6 flex items-center rounded-full p-1 cursor-pointer transition-colors duration-200 ease-in-out ${
                config?.launch_at_startup ? 'bg-accent' : 'bg-bg-elevated border border-border'
              }`}
            >
              <div
                className={`bg-bg-base w-4 h-4 rounded-full shadow-md transform transition-transform duration-200 ease-in-out ${
                  config?.launch_at_startup ? 'translate-x-5' : 'translate-x-0'
                }`}
              />
            </button>
          </div>

          {/* Language */}
          <div className="p-4 flex items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <div className="p-2 rounded bg-bg-elevated text-text-secondary border border-border/60">
                <Languages className="h-4 w-4 stroke-[1.6]" />
              </div>
              <div className="space-y-0.5">
                <p className="text-sm font-medium text-text-primary">Ngôn ngữ hiển thị</p>
                <p className="text-xs text-text-secondary">
                  Ngôn ngữ trong giao diện desktop và phản hồi bot Telegram
                </p>
              </div>
            </div>

            <select
              value={config?.language || 'vi'}
              onChange={(e) => onUpdateConfig('language', e.target.value)}
              className="bg-bg-elevated border border-border rounded-md px-2.5 py-1 text-xs text-text-primary font-medium focus:outline-none focus:border-accent"
            >
              <option value="vi">Tiếng Việt</option>
              <option value="en">English</option>
            </select>
          </div>
        </Card>
      </div>

      {/* ── Group 4: Cập nhật phần mềm ───────────────────── */}
      <div className="space-y-3">
        <h2 className="text-xs font-semibold uppercase tracking-wider text-text-secondary px-1">
          Cập nhật phần mềm
        </h2>
        <Card className="p-4 flex items-center justify-between gap-4">
          <div className="flex items-start gap-3">
            <div className="p-2 rounded bg-bg-elevated text-accent border border-border/60">
              <ArrowUpCircle className="h-4 w-4 stroke-[1.6]" />
            </div>
            <div className="space-y-0.5">
              <div className="flex items-center gap-2">
                <p className="text-sm font-medium text-text-primary">502Drive Desktop</p>
                <span className="text-xs font-mono px-1.5 py-0.2 rounded bg-bg-elevated text-accent border border-accent/20">
                  {status?.app_version || 'v0.1.0'}
                </span>
              </div>
              <p className="text-xs text-text-secondary">
                {updateChecked
                  ? 'Bạn đang sử dụng phiên bản mới nhất (v0.1.0).'
                  : 'Kiểm tra bản phát hành mới từ GitHub Release.'}
              </p>
            </div>
          </div>

          <Button
            size="sm"
            variant={updateChecked ? 'outline' : 'secondary'}
            onClick={handleCheckUpdate}
            disabled={isUpdating}
            className="text-xs gap-1.5"
          >
            {updateChecked ? <Check className="h-3.5 w-3.5 text-accent" /> : <ArrowUpCircle className="h-3.5 w-3.5" />}
            {isUpdating ? 'Đang kiểm tra...' : updateChecked ? 'Mới nhất' : 'Kiểm tra cập nhật'}
          </Button>
        </Card>
      </div>
    </div>
  )
}
