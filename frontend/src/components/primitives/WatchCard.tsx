import React from 'react'
import { Card } from '@/components/ui/Card'
import { Badge } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { WatchSummary } from '@/lib/types'
import { shortId, formatTimeAgo } from '@/lib/utils'
import { Radio, Pause, Play, FolderSync, ArrowRight } from 'lucide-react'

export interface WatchCardProps {
  watch: WatchSummary
  onPause?: (id: string) => void
  onResume?: (id: string) => void
}

export const WatchCard: React.FC<WatchCardProps> = ({ watch, onPause, onResume }) => {
  const isActive = watch.status === 'active'
  const isPaused = watch.status === 'paused'

  return (
    <Card className="p-3.5 space-y-3 hover:border-border-strong transition-colors">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <FolderSync className="h-4 w-4 text-accent stroke-[1.6]" />
          <Badge variant={isActive ? 'active' : isPaused ? 'paused' : 'default'} pulse={isActive}>
            {watch.status}
          </Badge>
          <span className="text-xs font-mono text-text-muted">ID: {watch.short_id || shortId(watch.id)}</span>
        </div>

        <div className="flex items-center gap-2">
          {isActive && onPause && (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => onPause(watch.id)}
              className="h-6 px-2 text-xs gap-1"
            >
              <Pause className="h-3 w-3" />
              Tạm dừng
            </Button>
          )}
          {isPaused && onResume && (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => onResume(watch.id)}
              className="h-6 px-2 text-xs gap-1 text-accent border-accent/30 hover:bg-accent/10"
            >
              <Play className="h-3 w-3" />
              Tiếp tục
            </Button>
          )}
        </div>
      </div>

      <div className="flex items-center gap-2 text-xs bg-bg-elevated p-2 rounded border border-border/40 font-mono text-text-secondary overflow-hidden">
        <span className="text-text-muted truncate max-w-[40%]">src: {watch.source_root_id}</span>
        <ArrowRight className="h-3 w-3 shrink-0 text-text-muted" />
        <span className="text-text-primary truncate max-w-[40%]">dst: {watch.destination_root_id}</span>
      </div>

      <div className="flex items-center justify-between text-[11px] text-text-muted pt-1">
        <div className="flex items-center gap-2">
          <Radio className="h-3 w-3 text-accent" />
          <span>Backlog: <span className="font-mono text-text-primary">{watch.backlog_count}</span> sự kiện</span>
        </div>
        <span>Cập nhật: {formatTimeAgo(watch.updated_at_ms)}</span>
      </div>
    </Card>
  )
}
