import React, { useState } from 'react'
import { Card } from '@/components/ui/Card'
import { JobCard } from '@/components/primitives/JobCard'
import { WatchCard } from '@/components/primitives/WatchCard'
import { Button } from '@/components/ui/Button'
import { JobSummary, WatchSummary } from '@/lib/types'
import { ArrowLeftRight, Radio, CheckCircle2, RefreshCw, FolderSearch } from 'lucide-react'

export interface JobsProps {
  jobs: JobSummary[]
  watches: WatchSummary[]
  onPauseJob: (id: string) => void
  onResumeJob: (id: string) => void
  onCancelJob: (id: string) => void
  onPauseWatch: (id: string) => void
  onResumeWatch: (id: string) => void
  onRefresh: () => void
  isRefreshing?: boolean
}

export const Jobs: React.FC<JobsProps> = ({
  jobs,
  watches,
  onPauseJob,
  onResumeJob,
  onCancelJob,
  onPauseWatch,
  onResumeWatch,
  onRefresh,
  isRefreshing,
}) => {
  const [filter, setFilter] = useState<'all' | 'running' | 'completed' | 'watches'>('all')

  const runningJobs = jobs.filter((j) => j.status === 'running' || j.status === 'discovering' || j.status === 'paused')
  const completedJobs = jobs.filter((j) => j.status === 'completed' || j.status === 'failed' || j.status === 'cancelled')

  return (
    <div className="space-y-5">
      {/* ── Filter Bar & Actions ─────────────────────────── */}
      <div className="flex flex-wrap items-center justify-between gap-3 pb-1 border-b border-border/60">
        <div className="flex items-center gap-1.5 p-1 rounded-lg bg-bg-elevated border border-border">
          <button
            onClick={() => setFilter('all')}
            className={`px-3 py-1 text-xs font-medium rounded-md transition-colors ${
              filter === 'all'
                ? 'bg-bg-card text-text-primary shadow-sm border border-border/80'
                : 'text-text-secondary hover:text-text-primary'
            }`}
          >
            Tất cả ({jobs.length + watches.length})
          </button>
          <button
            onClick={() => setFilter('running')}
            className={`px-3 py-1 text-xs font-medium rounded-md transition-colors ${
              filter === 'running'
                ? 'bg-bg-card text-text-primary shadow-sm border border-border/80'
                : 'text-text-secondary hover:text-text-primary'
            }`}
          >
            Đang chạy ({runningJobs.length})
          </button>
          <button
            onClick={() => setFilter('completed')}
            className={`px-3 py-1 text-xs font-medium rounded-md transition-colors ${
              filter === 'completed'
                ? 'bg-bg-card text-text-primary shadow-sm border border-border/80'
                : 'text-text-secondary hover:text-text-primary'
            }`}
          >
            Lịch sử ({completedJobs.length})
          </button>
          <button
            onClick={() => setFilter('watches')}
            className={`px-3 py-1 text-xs font-medium rounded-md transition-colors ${
              filter === 'watches'
                ? 'bg-bg-card text-text-primary shadow-sm border border-border/80'
                : 'text-text-secondary hover:text-text-primary'
            }`}
          >
            Thư mục theo dõi ({watches.length})
          </button>
        </div>

        <Button
          size="sm"
          variant="secondary"
          onClick={onRefresh}
          disabled={isRefreshing}
          className="text-xs gap-1.5"
        >
          <RefreshCw className={`h-3.5 w-3.5 ${isRefreshing ? 'animate-spin text-accent' : ''}`} />
          Làm mới
        </Button>
      </div>

      {/* ── Active Jobs Section ───────────────────────────── */}
      {(filter === 'all' || filter === 'running') && (
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <ArrowLeftRight className="h-4 w-4 text-accent" />
            <h2 className="text-xs font-semibold uppercase tracking-wider text-text-secondary">
              Tác vụ đang xử lý ({runningJobs.length})
            </h2>
          </div>

          {runningJobs.length === 0 ? (
            <Card className="p-6 text-center text-text-muted space-y-1.5">
              <FolderSearch className="h-5 w-5 mx-auto text-text-muted stroke-[1.5]" />
              <p className="text-xs">Không có tác vụ clone nào đang diễn ra.</p>
            </Card>
          ) : (
            <div className="grid grid-cols-1 gap-3">
              {runningJobs.map((job) => (
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
      )}

      {/* ── Watches (Realtime Sync) Section ───────────────── */}
      {(filter === 'all' || filter === 'watches') && (
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <Radio className="h-4 w-4 text-accent" />
            <h2 className="text-xs font-semibold uppercase tracking-wider text-text-secondary">
              Theo dõi thư mục thời gian thực ({watches.length})
            </h2>
          </div>

          {watches.length === 0 ? (
            <Card className="p-6 text-center text-text-muted space-y-1.5">
              <Radio className="h-5 w-5 mx-auto text-text-muted stroke-[1.5]" />
              <p className="text-xs">Chưa có luồng theo dõi thư mục tự động nào.</p>
              <p className="text-[11px] text-text-muted">Dùng lệnh /watch trong bot Telegram để thiết lập đồng bộ liên tục.</p>
            </Card>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {watches.map((watch) => (
                <WatchCard
                  key={watch.id}
                  watch={watch}
                  onPause={onPauseWatch}
                  onResume={onResumeWatch}
                />
              ))}
            </div>
          )}
        </div>
      )}

      {/* ── Completed Jobs History Section ────────────────── */}
      {(filter === 'all' || filter === 'completed') && (
        <div className="space-y-3">
          <div className="flex items-center gap-2">
            <CheckCircle2 className="h-4 w-4 text-success" />
            <h2 className="text-xs font-semibold uppercase tracking-wider text-text-secondary">
              Lịch sử hoàn tất gần đây ({completedJobs.length})
            </h2>
          </div>

          {completedJobs.length === 0 ? (
            <Card className="p-6 text-center text-text-muted">
              <p className="text-xs">Chưa có lịch sử tác vụ hoàn tất.</p>
            </Card>
          ) : (
            <div className="grid grid-cols-1 gap-2.5">
              {completedJobs.map((job) => (
                <JobCard key={job.id} job={job} compact />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
