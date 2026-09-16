import React from 'react'
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/Card'
import { Button } from '@/components/ui/Button'
import { Badge } from '@/components/ui/Badge'
import { StatCard } from '@/components/primitives/StatCard'
import { JobCard } from '@/components/primitives/JobCard'
import { SystemStatus, JobSummary } from '@/lib/types'
import { formatBytes } from '@/lib/utils'
import {
  FolderUp,
  Files,
  HardDrive,
  Database,
  ExternalLink,
  RotateCcw,
  LogIn,
  LogOut,
  Send,
  FolderSync,
  Layers,
} from 'lucide-react'

export interface DashboardProps {
  status: SystemStatus | null
  recentJobs: JobSummary[]
  onOpenBot: () => void
  onTriggerLogin: () => void
  onTriggerRevoke: () => void
  onRestartService: () => void
  onPauseJob: (id: string) => void
  onResumeJob: (id: string) => void
  onCancelJob: (id: string) => void
  onNavigateToJobs: () => void
}

export const Dashboard: React.FC<DashboardProps> = ({
  status,
  recentJobs,
  onOpenBot,
  onTriggerLogin,
  onTriggerRevoke,
  onRestartService,
  onPauseJob,
  onResumeJob,
  onCancelJob,
  onNavigateToJobs,
}) => {
  const isAccountConnected = status?.account_status === 'connected'
  const isServiceActive = status?.service_active ?? false

  return (
    <div className="space-y-5">
      {/* ── Metric Stats Bar ─────────────────────────────── */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <StatCard
          label="Tác vụ đang chạy"
          value={status?.stats.active_jobs ?? 0}
          subtext={`/ ${status?.stats.total_jobs ?? 0} tổng cộng`}
          icon={FolderUp}
        />
        <StatCard
          label="Tệp đã sao chép"
          value={(status?.stats.total_cloned_files ?? 0).toLocaleString()}
          icon={Files}
        />
        <StatCard
          label="Dung lượng đã clone"
          value={formatBytes(status?.stats.total_cloned_bytes ?? 0)}
          icon={HardDrive}
        />
        <StatCard
          label="Cơ sở dữ liệu"
          value={status?.db_integrity?.toUpperCase() || 'OK'}
          subtext="Integrity check"
          icon={Database}
        />
      </div>

      {/* ── Status Cards Grid ────────────────────────────── */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        {/* Card 1: Google Account */}
        <Card className="flex flex-col justify-between">
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>
                <HardDrive className="h-4 w-4 text-accent" />
                Tài khoản Google
              </CardTitle>
              <Badge variant={isAccountConnected ? 'connected' : 'disconnected'}>
                {isAccountConnected ? 'Đã kết nối' : 'Chưa kết nối'}
              </Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="p-2.5 rounded bg-bg-elevated border border-border/50">
              <p className="text-xs font-mono text-text-primary truncate">
                {status?.google_account || 'Chưa đăng nhập Google'}
              </p>
              <p className="text-[11px] text-text-muted mt-0.5">
                {isAccountConnected ? 'OAuth token hoạt động bình thường' : 'Cần đăng nhập để clone dữ liệu'}
              </p>
            </div>

            <div className="flex items-center gap-2 pt-1">
              {isAccountConnected ? (
                <>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={onTriggerLogin}
                    className="flex-1 text-xs"
                  >
                    Đăng nhập lại
                  </Button>
                  <Button
                    size="sm"
                    variant="danger"
                    onClick={onTriggerRevoke}
                    className="h-8.5 px-3"
                    title="Ngắt kết nối tài khoản"
                  >
                    <LogOut className="h-3.5 w-3.5" />
                  </Button>
                </>
              ) : (
                <Button
                  size="sm"
                  variant="primary"
                  onClick={onTriggerLogin}
                  className="w-full text-xs gap-1.5"
                >
                  <LogIn className="h-3.5 w-3.5" />
                  Đăng nhập Google
                </Button>
              )}
            </div>
          </CardContent>
        </Card>

        {/* Card 2: Telegram Bot */}
        <Card className="flex flex-col justify-between">
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>
                <Send className="h-4 w-4 text-accent" />
                Telegram Bot
              </CardTitle>
              <Badge variant={isServiceActive ? 'active' : 'paused'} pulse={isServiceActive}>
                {isServiceActive ? 'Đang chạy' : 'Dừng'}
              </Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="p-2.5 rounded bg-bg-elevated border border-border/50">
              <p className="text-xs font-mono text-text-primary truncate">
                @{status?.bot_username || 'Drive502_Bot'}
              </p>
              <p className="text-[11px] text-text-muted mt-0.5">
                {isServiceActive ? 'Long polling đang nhận lệnh bot' : 'Background service chưa được bật'}
              </p>
            </div>

            <div className="flex items-center gap-2 pt-1">
              <Button
                size="sm"
                variant="primary"
                onClick={onOpenBot}
                className="flex-1 text-xs gap-1.5"
              >
                <ExternalLink className="h-3.5 w-3.5" />
                Mở bot Telegram
              </Button>
              <Button
                size="sm"
                variant="secondary"
                onClick={onRestartService}
                className="h-8.5 px-3"
                title="Khởi động lại service"
              >
                <RotateCcw className="h-3.5 w-3.5" />
              </Button>
            </div>
          </CardContent>
        </Card>

        {/* Card 3: Destination Folder */}
        <Card className="flex flex-col justify-between">
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>
                <FolderSync className="h-4 w-4 text-accent" />
                Thư mục đích
              </CardTitle>
              <Badge variant="default">Mặc định</Badge>
            </div>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="p-2.5 rounded bg-bg-elevated border border-border/50">
              <p className="text-xs font-medium text-text-primary truncate">
                {status?.destination_label || 'My Drive / Backup 502'}
              </p>
              <p className="text-[10px] font-mono text-text-muted mt-0.5 truncate">
                ID: {status?.destination_id || '1aBcDeFgHiJkLmNoPqRsTuVwXyZ01234'}
              </p>
            </div>

            <div className="flex items-center justify-between text-xs text-text-secondary pt-1">
              <span className="text-[11px] text-text-muted">Đích lưu trữ cho các lệnh clone từ bot</span>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* ── Active / Recent Jobs Section ──────────────────── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Layers className="h-4 w-4 text-accent" />
            <h2 className="text-xs font-semibold uppercase tracking-wider text-text-secondary">
              Tiến trình công việc gần đây
            </h2>
          </div>
          <Button
            size="sm"
            variant="ghost"
            onClick={onNavigateToJobs}
            className="text-xs text-text-secondary hover:text-text-primary"
          >
            Xem tất cả &rarr;
          </Button>
        </div>

        {recentJobs.length === 0 ? (
          <Card className="p-8 text-center text-text-muted space-y-2">
            <Layers className="h-6 w-6 mx-auto text-text-muted stroke-[1.5]" />
            <p className="text-xs">Chưa có tác vụ clone nào đang chạy hoặc hoàn tất.</p>
            <p className="text-[11px] text-text-muted">Gửi link thư mục Google Drive vào bot Telegram để bắt đầu.</p>
          </Card>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {recentJobs.slice(0, 2).map((job) => (
              <JobCard
                key={job.id}
                job={job}
                onPause={onPauseJob}
                onResume={onResumeJob}
                onCancel={onCancelJob}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
