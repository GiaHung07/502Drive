import React from 'react'
import { Card } from '@/components/ui/Card'
import { Badge, BadgeVariant } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { ProgressBar } from '@/components/primitives/ProgressBar'
import { JobSummary } from '@/lib/types'
import { formatBytes, formatDuration, formatTimeAgo, shortId } from '@/lib/utils'
import { Play, Pause, X, Loader2, CheckCircle2, AlertCircle, Clock, RotateCcw } from 'lucide-react'
import { useI18n } from '@/hooks/useI18n'

export interface JobCardProps {
  job: JobSummary
  onPause?: (id: string) => void
  onResume?: (id: string) => void
  onCancel?: (id: string) => void
  onRetry?: (id: string) => void
  compact?: boolean
}

export const JobCard: React.FC<JobCardProps> = ({
  job,
  onPause,
  onResume,
  onCancel,
  onRetry,
  compact = false,
}) => {
  const { t } = useI18n()
  const isRunning = job.status === 'running' || job.status === 'discovering'
  const isPaused = job.status === 'paused' || job.status === 'pausing'
  const isCompleted = job.status === 'completed'
  const isFailed = job.status === 'failed'

  const getStatusInfo = (status: string) => {
    switch (status) {
      case 'running':
        return {
          label: t('jobs_page.status_running'),
          variant: 'running' as BadgeVariant,
          icon: <Loader2 className="h-3.5 w-3.5 text-accent animate-spin" />,
        }
      case 'discovering':
        return {
          label: t('jobs_page.status_discovering'),
          variant: 'running' as BadgeVariant,
          icon: <Loader2 className="h-3.5 w-3.5 text-accent animate-spin" />,
        }
      case 'paused':
        return {
          label: t('jobs_page.status_paused'),
          variant: 'paused' as BadgeVariant,
          icon: <Pause className="h-3.5 w-3.5 text-warning" />,
        }
      case 'pausing':
        return {
          label: t('jobs_page.status_pausing'),
          variant: 'paused' as BadgeVariant,
          icon: <Pause className="h-3.5 w-3.5 text-warning" />,
        }
      case 'completed':
        return {
          label: t('jobs_page.status_completed'),
          variant: 'completed' as BadgeVariant,
          icon: <CheckCircle2 className="h-3.5 w-3.5 text-success" />,
        }
      case 'partially_completed':
        return {
          label: t('jobs_page.status_partially_completed'),
          variant: 'paused' as BadgeVariant,
          icon: <CheckCircle2 className="h-3.5 w-3.5 text-warning" />,
        }
      case 'failed':
        return {
          label: t('jobs_page.status_failed'),
          variant: 'failed' as BadgeVariant,
          icon: <AlertCircle className="h-3.5 w-3.5 text-error" />,
        }
      case 'cancelled':
        return {
          label: t('jobs_page.status_cancelled'),
          variant: 'default' as BadgeVariant,
          icon: <X className="h-3.5 w-3.5 text-text-muted" />,
        }
      case 'queued':
      default:
        return {
          label: t('jobs_page.status_queued'),
          variant: 'queued' as BadgeVariant,
          icon: <Clock className="h-3.5 w-3.5 text-info" />,
        }
    }
  }

  const info = getStatusInfo(job.status)

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
              title={t('jobs_page.btn_pause')}
            >
              <Pause className="h-3 w-3" />
              {t('jobs_page.btn_pause')}
            </Button>
          )}

          {isPaused && onResume && (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => onResume(job.id)}
              className="h-6.5 px-2 text-[0.6875rem] gap-1 text-accent hover:bg-accent/10"
              title={t('jobs_page.btn_resume')}
            >
              <Play className="h-3 w-3" />
              {t('jobs_page.btn_resume')}
            </Button>
          )}

          {(isRunning || isPaused) && onCancel && (
            <Button
              size="sm"
              variant="ghost"
              onClick={() => onCancel(job.id)}
              className="h-6.5 w-6.5 p-0 text-text-muted hover:text-error"
              title={t('jobs_page.btn_cancel')}
              aria-label={t('jobs_page.btn_cancel')}
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          )}

          {isFailed && onRetry && (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => onRetry(job.id)}
              className="h-6.5 px-2 text-[0.6875rem] gap-1 text-error hover:bg-error/10 hover:border-error/40"
              title={t('jobs_page.btn_retry')}
            >
              <RotateCcw className="h-3 w-3" />
              {t('jobs_page.btn_retry')}
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
            {job.completed_items.toLocaleString()} / {job.total_discovered.toLocaleString()} {t('jobs_page.items')}
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
              <span>{t('jobs_page.speed')}: <span className="font-mono text-text-secondary">{formatBytes(job.speed_bytes_per_sec)}/s</span></span>
            ) : null}
            {job.eta_seconds ? (
              <span>{t('jobs_page.eta')}: <span className="font-mono text-text-secondary">~{formatDuration(job.eta_seconds)}</span></span>
            ) : null}
            {job.skipped_items > 0 ? (
              <span>{t('common.cancel')}: <span className="font-mono text-text-secondary">{job.skipped_items}</span></span>
            ) : null}
            {job.failed_items > 0 ? (
              <span className="text-error">{t('common.error')}: <span className="font-mono">{job.failed_items}</span></span>
            ) : null}
          </div>
          <span className="text-[0.625rem] text-text-muted">{formatTimeAgo(job.updated_at_ms || job.created_at_ms)}</span>
        </div>
      )}
    </Card>
  )
}
