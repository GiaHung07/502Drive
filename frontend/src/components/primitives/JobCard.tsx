import React from 'react'
import { Card } from '@/components/ui/Card'
import { Badge } from '@/components/ui/Badge'
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

  const statusIcons = {
    running: <Loader2 className="h-3.5 w-3.5 text-accent animate-spin" />,
    discovering: <Loader2 className="h-3.5 w-3.5 text-accent animate-spin" />,
    paused: <Pause className="h-3.5 w-3.5 text-warning" />,
    pausing: <Pause className="h-3.5 w-3.5 text-warning" />,
    completed: <CheckCircle2 className="h-3.5 w-3.5 text-success" />,
    partially_completed: <CheckCircle2 className="h-3.5 w-3.5 text-warning" />,
    failed: <AlertCircle className="h-3.5 w-3.5 text-error" />,
    cancelled: <X className="h-3.5 w-3.5 text-text-muted" />,
    queued: <Clock className="h-3.5 w-3.5 text-info" />,
    recovering: <Loader2 className="h-3.5 w-3.5 text-info animate-spin" />,
    cancelling: <Loader2 className="h-3.5 w-3.5 text-error animate-spin" />,
  }

  return (
    <Card className="p-3.5 space-y-3 hover:border-border-strong transition-colors">
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-center gap-2">
          {statusIcons[job.status] || <Clock className="h-3.5 w-3.5 text-text-muted" />}
          <Badge
            variant={
              isRunning
                ? 'running'
                : isCompleted
                ? 'done'
                : isPaused
                ? 'paused'
                : isFailed
                ? 'failed'
                : 'default'
            }
            pulse={isRunning}
          >
            {job.status}
          </Badge>
          <span className="text-xs font-mono text-text-muted">ID: {job.short_id || shortId(job.id)}</span>
        </div>

        {/* Action buttons */}
        <div className="flex items-center gap-1.5">
          {isRunning && onPause && (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => onPause(job.id)}
              className="h-6 px-2 text-xs gap-1"
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
              className="h-6 px-2 text-xs gap-1 text-accent border-accent/30 hover:bg-accent/10"
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
              className="h-6 w-6 p-0 text-text-muted hover:text-error"
              title="Hủy job"
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
          <span className="text-text-secondary font-mono text-[11px]">
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
        <div className="flex flex-wrap items-center justify-between pt-1 text-[11px] text-text-muted border-t border-border/40 gap-y-1">
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
          <span className="text-[10px] text-text-muted">{formatTimeAgo(job.updated_at_ms || job.created_at_ms)}</span>
        </div>
      )}
    </Card>
  )
}
