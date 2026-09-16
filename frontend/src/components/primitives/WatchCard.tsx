import React, { useState } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { Card } from '@/components/ui/Card'
import { Badge } from '@/components/ui/Badge'
import { Button } from '@/components/ui/Button'
import { WatchSummary, WatchPolicyKind } from '@/lib/types'
import { shortId, formatTimeAgo, cn } from '@/lib/utils'
import {
  Radio,
  Pause,
  Play,
  FolderSync,
  ArrowRight,
  MoreHorizontal,
  SlidersHorizontal,
  ChevronRight,
  ArrowLeft,
  Unlink,
} from 'lucide-react'

export interface WatchCardProps {
  watch: WatchSummary
  onPause?: (id: string) => void
  onResume?: (id: string) => void
  onSetWatchPolicy?: (id: string, policyKind: WatchPolicyKind, policyValue: string) => void
  onUnwatch?: (id: string) => void
}

interface PolicyOption {
  value: string
  label: string
}

const POLICY_OPTIONS: { kind: WatchPolicyKind; label: string; values: PolicyOption[] }[] = [
  {
    kind: 'content_update',
    label: 'Cập nhật nội dung',
    values: [
      { value: 'versioned_copy', label: 'Bản phiên bản' },
      { value: 'replace_copy', label: 'Ghi đè' },
      { value: 'manual_confirmation', label: 'Xác nhận tay' },
    ],
  },
  {
    kind: 'deletion',
    label: 'Tệp bị xóa',
    values: [
      { value: 'preserve_destination', label: 'Giữ bản đích' },
      { value: 'manual_confirmation', label: 'Xác nhận tay' },
    ],
  },
  {
    kind: 'move_out',
    label: 'Tệp bị di chuyển ra ngoài',
    values: [
      { value: 'detach', label: 'Tách theo dõi' },
      { value: 'keep_following', label: 'Theo vị trí mới' },
    ],
  },
]

type MenuView = 'menu' | 'policy' | 'unwatch-confirm'

export const WatchCard: React.FC<WatchCardProps> = ({
  watch,
  onPause,
  onResume,
  onSetWatchPolicy,
  onUnwatch,
}) => {
  const isActive = watch.status === 'active'
  const isPaused = watch.status === 'paused'
  const [isMenuOpen, setIsMenuOpen] = useState(false)
  const [menuView, setMenuView] = useState<MenuView>('menu')

  const excludeGlobs = watch.exclude_globs ?? []

  const closeMenu = () => {
    setIsMenuOpen(false)
    setMenuView('menu')
  }

  const handlePolicySelect = (kind: WatchPolicyKind, value: string) => {
    onSetWatchPolicy?.(watch.id, kind, value)
    closeMenu()
  }

  const handleUnwatch = () => {
    onUnwatch?.(watch.id)
    closeMenu()
  }

  return (
    <Card className="p-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <FolderSync className="h-4 w-4 text-accent stroke-[1.6]" />
          <Badge variant={isActive ? 'active' : isPaused ? 'paused' : 'default'} pulse={isActive}>
            {isActive ? 'Đang theo dõi' : isPaused ? 'Tạm dừng' : watch.status}
          </Badge>
          <span className="text-xs font-mono text-text-muted">#{watch.short_id || shortId(watch.id)}</span>
        </div>

        <div className="flex items-center gap-1.5">
          {isActive && onPause && (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => onPause(watch.id)}
              className="h-6.5 px-2 text-[0.6875rem] gap-1"
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
              className="h-6.5 px-2 text-[0.6875rem] gap-1 text-accent hover:bg-accent/10"
            >
              <Play className="h-3 w-3" />
              Tiếp tục
            </Button>
          )}

          {/* Action menu (⋯) */}
          {(onSetWatchPolicy || onUnwatch) && (
            <div className="relative">
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setIsMenuOpen((o) => !o)}
                aria-label="Hành động theo dõi"
                aria-expanded={isMenuOpen}
                title="Hành động"
                className="h-6.5 w-6.5 p-0"
              >
                <MoreHorizontal className="h-3.5 w-3.5" />
              </Button>

              <AnimatePresence>
                {isMenuOpen && (
                  <>
                    {/* Click-away layer */}
                    <div
                      className="fixed inset-0 z-20"
                      onClick={closeMenu}
                      aria-hidden="true"
                    />
                    <motion.div
                      initial={{ opacity: 0, scale: 0.96, y: -4 }}
                      animate={{ opacity: 1, scale: 1, y: 0 }}
                      exit={{ opacity: 0, scale: 0.96, y: -4 }}
                      transition={{ type: 'spring', stiffness: 450, damping: 35 }}
                      className="absolute right-0 top-full mt-1.5 z-30 w-64 rounded-xl border border-border/80 bg-bg-elevated shadow-modal p-1.5"
                    >
                      {menuView === 'menu' && (
                        <div className="space-y-0.5">
                          {onSetWatchPolicy && (
                            <button
                              type="button"
                              onClick={() => setMenuView('policy')}
                              className="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg text-xs font-medium text-text-primary hover:bg-bg-input/70 cursor-pointer transition-colors text-left"
                            >
                              <SlidersHorizontal className="h-3.5 w-3.5 text-accent" />
                              <span className="flex-1">Đổi chính sách</span>
                              <ChevronRight className="h-3 w-3 text-text-muted" />
                            </button>
                          )}
                          {onUnwatch && (
                            <button
                              type="button"
                              onClick={() => setMenuView('unwatch-confirm')}
                              className="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg text-xs font-medium text-error hover:bg-error/10 cursor-pointer transition-colors text-left"
                            >
                              <Unlink className="h-3.5 w-3.5" />
                              <span className="flex-1">Ngừng theo dõi</span>
                            </button>
                          )}
                        </div>
                      )}

                      {menuView === 'policy' && (
                        <div className="space-y-1">
                          <button
                            type="button"
                            onClick={() => setMenuView('menu')}
                            className="w-full flex items-center gap-1.5 px-2 py-1.5 rounded-lg text-[0.6875rem] font-semibold text-text-secondary hover:text-text-primary hover:bg-bg-input/70 cursor-pointer transition-colors text-left"
                          >
                            <ArrowLeft className="h-3 w-3" />
                            Chính sách đồng bộ
                          </button>
                          {POLICY_OPTIONS.map((group) => (
                            <div
                              key={group.kind}
                              className="px-2 py-1.5 rounded-lg space-y-1"
                            >
                              <p className="text-[0.625rem] font-medium text-text-muted uppercase tracking-wider">
                                {group.label}
                              </p>
                              <div className="flex items-center gap-1">
                                {group.values.map((opt) => (
                                  <button
                                    key={opt.value}
                                    type="button"
                                    onClick={() => handlePolicySelect(group.kind, opt.value)}
                                    className="flex-1 px-1.5 py-1 rounded-md text-[0.625rem] font-medium text-text-secondary bg-bg-input/70 border border-border/50 hover:border-accent/50 hover:text-accent hover:bg-accent/10 cursor-pointer transition-colors"
                                  >
                                    {opt.label}
                                  </button>
                                ))}
                              </div>
                            </div>
                          ))}
                        </div>
                      )}

                      {menuView === 'unwatch-confirm' && (
                        <div className="p-2 space-y-2.5">
                          <p className="text-xs font-semibold text-text-primary leading-snug">
                            Ngừng theo dõi thư mục này?
                          </p>
                          <p className="text-[0.6875rem] text-text-secondary leading-relaxed">
                            Daemon sẽ dừng xử lý sự kiện thay đổi. Dữ liệu đã sao chép được giữ nguyên.
                          </p>
                          <div className="flex items-center justify-end gap-1.5">
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={() => setMenuView('menu')}
                              className="h-7 px-2.5 text-[0.6875rem] rounded-lg"
                            >
                              Hủy
                            </Button>
                            <Button
                              size="sm"
                              variant="danger"
                              onClick={handleUnwatch}
                              className="h-7 px-2.5 text-[0.6875rem] gap-1 rounded-lg font-semibold"
                            >
                              <Unlink className="h-3 w-3" />
                              Ngừng theo dõi
                            </Button>
                          </div>
                        </div>
                      )}
                    </motion.div>
                  </>
                )}
              </AnimatePresence>
            </div>
          )}
        </div>
      </div>

      <div className="flex items-center gap-2 text-xs bg-bg-input/60 p-2 rounded-lg font-mono text-text-secondary overflow-hidden">
        <span className="text-text-muted truncate max-w-[40%]">src: {watch.source_root_id}</span>
        <ArrowRight className="h-3 w-3 shrink-0 text-text-muted" />
        <span className="text-text-primary truncate max-w-[40%]">dst: {watch.destination_root_id}</span>
      </div>

      {excludeGlobs.length > 0 && (
        <div className="flex items-center flex-wrap gap-1.5">
          <span className="text-[0.625rem] font-medium text-text-muted shrink-0">Bỏ qua:</span>
          {excludeGlobs.slice(0, 3).map((g) => (
            <span
              key={g}
              className="px-2 py-0.5 rounded-full bg-bg-input/80 border border-border/40 text-[0.625rem] font-mono text-text-secondary whitespace-nowrap"
            >
              {g}
            </span>
          ))}
          {excludeGlobs.length > 3 && (
            <span
              className={cn(
                'px-2 py-0.5 rounded-full bg-bg-input/80 border border-border/40',
                'text-[0.625rem] font-mono text-text-muted'
              )}
              title={excludeGlobs.slice(3).join(', ')}
            >
              +{excludeGlobs.length - 3}
            </span>
          )}
        </div>
      )}

      <div className="flex items-center justify-between text-[0.6875rem] text-text-muted pt-1">
        <div className="flex items-center gap-1.5">
          <Radio className="h-3 w-3 text-accent" />
          <span>Hàng đợi: <span className="font-mono text-text-primary">{watch.backlog_count}</span> sự kiện</span>
        </div>
        <span>Cập nhật {formatTimeAgo(watch.updated_at_ms)}</span>
      </div>
    </Card>
  )
}
