import React from 'react'
import { Card } from '@/components/ui/Card'
import { Badge, BadgeVariant } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { ProgressBar } from '@/components/primitives/ProgressBar'
import { JobSummary } from '@/lib/types'
import { formatBytes, formatDuration, formatTimeAgo, shortId } from '@/lib/utils'
import { Play, Pause, X, Loader2, CheckCircle2, AlertCircle, Clock } from 'lucide-react'

export interface JobCardProps {
  job: JobSummary
  onPause?: (id: string) => void
  onResume?: (id: string) => void
  onCancel?: (id: string) => void
  compact?: boolean
}

const statusMap: Record<string, { label: string; variant: BadgeVariant; icon: React.ReactNode }> = {
  running: {
    label: 'Đang sao chép',
    variant: 'running',
    icon: <Loader2 className="h-3.5 w-3.5 text-accent animate-spin" />,
  },
  discovering: {
    label: 'Đang quét tệp',
    variant: 'running',
    icon: <Loader2 className="h-3.5 w-3.5 text-accent animate-spin" />,
  },
  paused: {
    label: 'Tạm dừng',
    variant: 'paused',
    icon: <Pause className="h-3.5 w-3.5 text-warning" />,
  },
  pausing: {
    label: 'Đang tạm dừng',
    variant: 'paused',
    icon: <Pause className="h-3.5 w-3.5 text-warning" />,
  },
  completed: {
    label: 'Hoàn tất',
    variant: 'completed',
    icon: <CheckCircle2 className="h-3.5 w-3.5 text-success" />,
  },
  partially_completed: {
    label: 'Một phần hoàn tất',
    variant: 'paused',
    icon: <CheckCircle2 className="h-3.5 w-3.5 text-warning" />,
  },
  failed: {
    label: 'Lỗi',
    variant: 'failed',
    icon: <AlertCircle className="h-3.5 w-3.5 text-error" />,
  },
  cancelled: {
    label: 'Đã hủy',
    variant: 'default',
    icon: <X className="h-3.5 w-3.5 text-text-muted" />,
  },
  queued: {
    label: 'Đang chờ',
    variant: 'queued',
    icon: <Clock className="h-3.5 w-3.5 text-info" />,
  },
}

export const JobCard: React.FC<JobCardProps> = ({
  job,
  onPause,
  onResume,
  onCancel,
  compact = false,
}) => {
  const isRunning = job.status === 'running' || job.status === 'discovering'
  const isPaused = job.status === 'paused' || job.status === 'pausing'
  const isCompleted = job.status === 'completed'
  const isFailed = job.status === 'failed'

  const info = statusMap[job.status] || {
    label: job.status,
    variant: 'default' as BadgeVariant,
    icon: <Clock className="h-3.5 w-3.5 text-text-muted" />,
  }

  return (
    <Card className="p-4 space-y-3">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          {info.icon}
          <Badge variant={info.variant} pulse={isRunning}>
            {info.label}
          </Badge>
          <span className="text-xs font-mono text-text-muted">#{job.short_id || shortId(job.id)}</span>
        </div>

        {/* Action buttons */}
        <div className="flex items-center gap-1.5">
          {isRunning && onPause && (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => onPause(job.id)}
              className="h-6.5 px-2 text-[0.6875rem] gap-1"
              title="Tạm dừng job"
            >
              <Pause className="h-3 w-3" />
              Tạm dừng
            </Button>
          )}

          {isPaused && onResume && (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => onResume(job.id)}
              className="h-6.5 px-2 text-[0.6875rem] gap-1 text-accent hover:bg-accent/10"
              title="Tiếp tục job"
            >
              <Play className="h-3 w-3" />
              Tiếp tục
            </Button>
          )}

          {(isRunning || isPaused) && onCancel && (
            <Button
              size="sm"
              variant="ghost"
              onClick={() => onCancel(job.id)}
              className="h-6.5 w-6.5 p-0 text-text-muted hover:text-error"
              title="Hủy job"
              aria-label="Hủy job"
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          )}
        </div>
      </div>

      {/* Progress section */}
      <div className="space-y-1.5">
        <div className="flex items-center justify-between text-xs">
          <span className="text-text-primary font-mono font-medium">
            {job.progress_pct.toFixed(1)}%
          </span>
          <span className="text-text-secondary font-mono text-[0.6875rem]">
            {job.completed_items.toLocaleString()} / {job.total_discovered.toLocaleString()} tệp
          </span>
        </div>
        <ProgressBar
          progress={job.progress_pct}
          isRunning={isRunning}
          variant={isFailed ? 'error' : isPaused ? 'warning' : isCompleted ? 'success' : 'accent'}
        />
      </div>

      {/* Details metadata */}
      {!compact && (
        <div className="flex flex-wrap items-center justify-between pt-1.5 text-[0.6875rem] text-text-muted border-t border-border/40 gap-y-1">
          <div className="flex items-center gap-3">
            {job.speed_bytes_per_sec ? (
              <span>Tốc độ: <span className="font-mono text-text-secondary">{formatBytes(job.speed_bytes_per_sec)}/s</span></span>
            ) : null}
            {job.eta_seconds ? (
              <span>Còn lại: <span className="font-mono text-text-secondary">~{formatDuration(job.eta_seconds)}</span></span>
            ) : null}
            {job.skipped_items > 0 ? (
              <span>Bỏ qua: <span className="font-mono text-text-secondary">{job.skipped_items}</span></span>
            ) : null}
            {job.failed_items > 0 ? (
              <span className="text-error">Lỗi: <span className="font-mono">{job.failed_items}</span></span>
            ) : null}
          </div>
          <span className="text-[0.625rem] text-text-muted">{formatTimeAgo(job.updated_at_ms || job.created_at_ms)}</span>
        </div>
      )}
    </Card>
  )
}
