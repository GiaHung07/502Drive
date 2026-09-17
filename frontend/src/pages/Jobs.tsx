import React, { useState, useMemo, useRef, useEffect } from "react"
import { motion, AnimatePresence } from "motion/react"
import { Card } from "@/components/ui/Card"
import { JobCard } from "@/components/primitives/JobCard"
import { WatchCard } from "@/components/primitives/WatchCard"
import { Button } from "@/components/ui/Button"
import { JobSummary, WatchSummary, WatchPolicyKind } from "@/lib/types"
import {
  ArrowLeftRight,
  Radio,
  CheckCircle2,
  RefreshCw,
  FolderSearch,
  Inbox,
  Search,
  X,
  Plus,
} from "lucide-react"

import { useI18n } from "@/hooks/useI18n"

export interface JobsProps {
  jobs: JobSummary[]
  watches: WatchSummary[]
  onPauseJob: (id: string) => void
  onResumeJob: (id: string) => void
  onCancelJob: (id: string) => void
  onPauseWatch: (id: string) => void
  onResumeWatch: (id: string) => void
  onRetryJob?: (id: string) => void
  onSetWatchPolicy?: (id: string, policyKind: WatchPolicyKind, policyValue: string) => void
  onUnwatch?: (id: string) => void
  onCreateWatch?: () => void
  onRefresh: () => void
  isRefreshing?: boolean
}

type FilterType = "all" | "running" | "completed" | "watches"

export const Jobs: React.FC<JobsProps> = ({
  jobs,
  watches,
  onPauseJob,
  onResumeJob,
  onCancelJob,
  onPauseWatch,
  onResumeWatch,
  onRetryJob,
  onSetWatchPolicy,
  onUnwatch,
  onCreateWatch,
  onRefresh,
  isRefreshing,
}) => {
  const { t } = useI18n()
  const [filter, setFilter] = useState<FilterType>("all")
  const [searchQuery, setSearchQuery] = useState("")
  const [isSearchFocused, setIsSearchFocused] = useState(false)
  const searchInputRef = useRef<HTMLInputElement>(null)

  // Hotkey "/" to focus search
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "/" && document.activeElement !== searchInputRef.current) {
        e.preventDefault()
        searchInputRef.current?.focus()
      }
    }
    window.addEventListener("keydown", handleKeyDown)
    return () => window.removeEventListener("keydown", handleKeyDown)
  }, [])

  // Filter jobs by search query
  const query = searchQuery.trim().toLowerCase()

  const filteredJobs = useMemo(() => {
    if (!query) return jobs
    return jobs.filter(
      (j) =>
        j.id.toLowerCase().includes(query) ||
        (j.short_id && j.short_id.toLowerCase().includes(query)) ||
        j.status.toLowerCase().includes(query)
    )
  }, [jobs, query])

  const filteredWatches = useMemo(() => {
    if (!query) return watches
    return watches.filter(
      (w) =>
        w.id.toLowerCase().includes(query) ||
        (w.short_id && w.short_id.toLowerCase().includes(query)) ||
        w.source_root_id.toLowerCase().includes(query) ||
        w.destination_root_id.toLowerCase().includes(query) ||
        w.status.toLowerCase().includes(query)
    )
  }, [watches, query])

  const runningJobs = useMemo(
    () =>
      filteredJobs.filter(
        (j) => j.status === "running" || j.status === "discovering" || j.status === "paused"
      ),
    [filteredJobs]
  )

  const completedJobs = useMemo(
    () =>
      filteredJobs.filter(
        (j) => j.status === "completed" || j.status === "failed" || j.status === "cancelled"
      ),
    [filteredJobs]
  )

  const tabs: { id: FilterType; label: string; count: number }[] = [
    { id: "all", label: t('jobs_page.filter_all'), count: filteredJobs.length + filteredWatches.length },
    { id: "running", label: t('jobs_page.filter_running'), count: runningJobs.length },
    { id: "completed", label: t('jobs_page.filter_completed'), count: completedJobs.length },
    { id: "watches", label: t('jobs_page.filter_watches'), count: filteredWatches.length },
  ]

  return (
    <div className="space-y-6 pb-12 w-full">
      {/* ── Control Bar: Animated Search & Segmented Filter ──────────────── */}
      <div className="flex flex-col md:flex-row items-stretch md:items-center justify-between gap-4 pb-4 border-b border-border/60">
        {/* Apple-style Animated Segmented Filter Tabs */}
        <div className="flex items-center p-1 rounded-2xl bg-bg-input/70 border border-border/50 self-start select-none">
          {tabs.map((tab) => {
            const isSelected = filter === tab.id
            return (
              <button
                key={tab.id}
                onClick={() => setFilter(tab.id)}
                className="relative px-3.5 sm:px-4 py-1.5 text-xs sm:text-sm font-medium rounded-xl transition-colors cursor-pointer text-text-secondary hover:text-text-primary flex items-center gap-1.5"
              >
                {isSelected && (
                  <motion.div
                    layoutId="activeJobFilter"
                    className="absolute inset-0 rounded-xl bg-bg-elevated border border-border/70 shadow-xs"
                    transition={{ type: "spring", stiffness: 450, damping: 35 }}
                  />
                )}
                <span className={`relative z-10 ${isSelected ? "font-semibold text-text-primary" : ""}`}>
                  {tab.label}
                </span>
                <span
                  className={`relative z-10 text-[0.625rem] sm:text-xs font-mono px-1.5 py-0.5 rounded-full ${
                    isSelected
                      ? "bg-accent/15 text-accent font-semibold"
                      : "bg-bg-card text-text-muted"
                  }`}
                >
                  {tab.count}
                </span>
              </button>
            )
          })}
        </div>

        {/* Right tools: Animated Search Input & Refresh Button */}
        <div className="flex items-center gap-2.5">
          <motion.div
            animate={{ width: isSearchFocused || searchQuery ? 280 : 210 }}
            transition={{ type: "spring", stiffness: 450, damping: 35 }}
            className="relative flex items-center"
            onBlur={(e) => {
              if (!e.currentTarget.contains(e.relatedTarget as Node)) setIsSearchFocused(false)
            }}
          >
            <Search className="absolute left-3 h-4 w-4 text-text-muted pointer-events-none" />
            <input
              ref={searchInputRef}
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onFocus={() => setIsSearchFocused(true)}
              placeholder={t('jobs_page.search_placeholder')}
              aria-label={t('jobs_page.search_placeholder')}
              className="w-full h-9 pl-9 pr-8 text-xs sm:text-sm rounded-xl bg-bg-input/70 border border-border/50 text-text-primary placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-accent/30 focus:border-accent transition-all shadow-xs"
            />
            <AnimatePresence>
              {searchQuery && (
                <motion.button
                  initial={{ opacity: 0, scale: 0.8 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.8 }}
                  onClick={() => setSearchQuery("")}
                  aria-label={t('common.cancel')}
                  className="absolute right-2.5 p-1 rounded-md text-text-muted hover:text-text-primary hover:bg-bg-card transition-colors cursor-pointer"
                  title={t('common.cancel')}
                >
                  <X className="h-3.5 w-3.5" />
                </motion.button>
              )}
            </AnimatePresence>
          </motion.div>

          <Button
            size="md"
            variant="secondary"
            onClick={onRefresh}
            disabled={isRefreshing}
            className="gap-2 shrink-0 font-medium"
          >
            <RefreshCw className={`h-4 w-4 ${isRefreshing ? "animate-spin text-accent" : ""}`} />
            <span className="hidden sm:inline">{t('common.refresh')}</span>
          </Button>
        </div>
      </div>

      {/* ── Active Jobs Section ───────────────────────────── */}
      {(filter === "all" || filter === "running") && (
        <motion.div layout className="space-y-3.5">
          <div className="flex items-center justify-between px-1">
            <div className="flex items-center gap-2.5">
              <div className="p-1.5 rounded-lg bg-accent/10 text-accent">
                <ArrowLeftRight className="h-4 w-4" />
              </div>
              <h2 className="text-sm font-semibold text-text-primary">
                {t('jobs_page.filter_running')} ({runningJobs.length})
              </h2>
            </div>
          </div>

          {runningJobs.length === 0 ? (
            <Card className="p-8 text-center space-y-2.5 border-dashed border-border/80">
              <div className="w-12 h-12 rounded-2xl bg-bg-input/80 text-text-muted flex items-center justify-center mx-auto shadow-xs">
                <FolderSearch className="h-6 w-6 stroke-[1.5]" />
              </div>
              <p className="text-sm font-semibold text-text-primary">
                {query ? `${t('jobs_page.empty_title')} "${query}"` : t('jobs_page.empty_title')}
              </p>
              <p className="text-xs text-text-muted max-w-sm mx-auto">
                {t('jobs_page.empty_desc')}
              </p>
            </Card>
          ) : (
            <div className="grid grid-cols-1 gap-3.5">
              <AnimatePresence>
                {runningJobs.map((job) => (
                  <motion.div
                    key={job.id}
                    layout
                    initial={{ opacity: 0, y: 8 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, scale: 0.98 }}
                    transition={{ duration: 0.15 }}
                  >
                    <JobCard
                      job={job}
                      onPause={onPauseJob}
                      onResume={onResumeJob}
                      onCancel={onCancelJob}
                    />
                  </motion.div>
                ))}
              </AnimatePresence>
            </div>
          )}
        </motion.div>
      )}

      {/* ── Watches (Realtime Sync) Section ───────────────── */}
      {(filter === "all" || filter === "watches") && (
        <motion.div layout className="space-y-3.5 pt-2">
          <div className="flex items-center justify-between px-1">
            <div className="flex items-center gap-2.5">
              <div className="p-1.5 rounded-lg bg-accent/10 text-accent">
                <Radio className="h-4 w-4" />
              </div>
              <h2 className="text-sm font-semibold text-text-primary">
                {t('watches.card_title')} ({filteredWatches.length})
              </h2>
            </div>
            {onCreateWatch && (
              <Button
                size="sm"
                variant="secondary"
                onClick={onCreateWatch}
                className="text-xs gap-1.5 rounded-xl font-medium shrink-0"
                title={t('jobs_page.btn_new_watch')}
              >
                <Plus className="h-3.5 w-3.5 text-accent" />
                {t('jobs_page.btn_new_watch')}
              </Button>
            )}
          </div>

          {filteredWatches.length === 0 ? (
            <Card className="p-8 text-center space-y-2.5 border-dashed border-border/80">
              <div className="w-12 h-12 rounded-2xl bg-bg-input/80 text-text-muted flex items-center justify-center mx-auto shadow-xs">
                <Radio className="h-6 w-6 stroke-[1.5]" />
              </div>
              <p className="text-sm font-semibold text-text-primary">
                {t('watches.no_watches_title')}
              </p>
              <p className="text-xs text-text-muted max-w-sm mx-auto">
                {t('watches.no_watches_desc')}
              </p>
            </Card>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <AnimatePresence>
                {filteredWatches.map((watch) => (
                  <motion.div
                    key={watch.id}
                    layout
                    initial={{ opacity: 0, y: 8 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, scale: 0.98 }}
                    transition={{ duration: 0.15 }}
                  >
                    <WatchCard
                      watch={watch}
                      onPause={onPauseWatch}
                      onResume={onResumeWatch}
                      onSetWatchPolicy={onSetWatchPolicy}
                      onUnwatch={onUnwatch}
                    />
                  </motion.div>
                ))}
              </AnimatePresence>
            </div>
          )}
        </motion.div>
      )}

      {/* ── Completed Jobs History Section ────────────────── */}
      {(filter === "all" || filter === "completed") && (
        <motion.div layout className="space-y-3.5 pt-2">
          <div className="flex items-center justify-between px-1">
            <div className="flex items-center gap-2.5">
              <div className="p-1.5 rounded-lg bg-success/10 text-success">
                <CheckCircle2 className="h-4 w-4" />
              </div>
              <h2 className="text-sm font-semibold text-text-primary">
                {t('jobs_page.filter_completed')} ({completedJobs.length})
              </h2>
            </div>
          </div>

          {completedJobs.length === 0 ? (
            <Card className="p-8 text-center space-y-2.5 border-dashed border-border/80">
              <div className="w-12 h-12 rounded-2xl bg-bg-input/80 text-text-muted flex items-center justify-center mx-auto shadow-xs">
                <Inbox className="h-6 w-6 stroke-[1.5]" />
              </div>
              <p className="text-sm font-semibold text-text-primary">
                {t('jobs_page.empty_title')}
              </p>
            </Card>
          ) : (
            <div className="grid grid-cols-1 gap-3">
              <AnimatePresence>
                {completedJobs.map((job) => (
                  <motion.div
                    key={job.id}
                    layout
                    initial={{ opacity: 0, y: 8 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, scale: 0.98 }}
                    transition={{ duration: 0.15 }}
                  >
                    <JobCard key={job.id} job={job} compact onRetry={onRetryJob} />
                  </motion.div>
                ))}
              </AnimatePresence>
            </div>
          )}
        </motion.div>
      )}
    </div>
  )
}
