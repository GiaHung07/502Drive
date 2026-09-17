import React, { useState } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { Calendar, ArrowUpCircle, Loader2, ShieldCheck, Sparkles } from 'lucide-react'
import { LogoMark } from '@/components/brand/Logo'
import { Button } from '@/components/ui/Button'
import { RemoteUpdateInfo } from '@/lib/types'
import { api, getErrorMessage } from '@/lib/ipc'
import { useToast } from '@/components/primitives/Toast'
import { useI18n } from '@/hooks/useI18n'

export interface UpdateModalProps {
  isOpen: boolean
  onClose: () => void
  updateInfo: RemoteUpdateInfo | null
  onUpdateCompleted?: () => void
}

export const UpdateModal: React.FC<UpdateModalProps> = ({
  isOpen,
  onClose,
  updateInfo,
  onUpdateCompleted,
}) => {
  const { t } = useI18n()
  const [isApplying, setIsApplying] = useState(false)
  const { toast } = useToast()

  if (!isOpen || !updateInfo) return null

  const handleApplyUpdate = async () => {
    setIsApplying(true)
    try {
      toast({
        title: t('update_modal.updating'),
        variant: 'default',
        duration: 8000,
      })
      const msg = await api.applyRemoteUpdate()
      toast({
        title: t('update_modal.update_success'),
        description: msg,
        variant: 'success',
        duration: 8000,
      })
      onUpdateCompleted?.()
      onClose()
    } catch (err) {
      toast({
        title: t('common.error'),
        description: getErrorMessage(err),
        variant: 'error',
        duration: 8000,
      })
    } finally {
      setIsApplying(false)
    }
  }

  const today = new Date().toLocaleDateString('vi-VN', {
    month: 'long',
    day: 'numeric',
    year: 'numeric',
  })

  return (
    <AnimatePresence>
      <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
        {/* Backdrop */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          onClick={isApplying ? undefined : onClose}
          className="fixed inset-0 bg-black/65 backdrop-blur-xs"
        />

        {/* Modal Window */}
        <motion.div
          initial={{ opacity: 0, scale: 0.95, y: 8 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.95, y: 8 }}
          transition={{ type: 'spring', stiffness: 450, damping: 32 }}
          className="relative w-full max-w-md rounded-2xl bg-bg-surface border border-border/80 shadow-2xl overflow-hidden z-10"
        >
          {/* Mac-style Window Topbar */}
          <div className="flex items-center justify-between px-4 py-2.5 bg-bg-elevated/70 border-b border-border/50 select-none">
            <span className="text-xs font-semibold text-text-primary tracking-wide flex items-center gap-1.5">
              <span>502Drive</span>
            </span>
            <div className="flex items-center gap-1.5">
              <span className="h-2.5 w-2.5 rounded-full bg-amber-500/80 cursor-pointer" onClick={onClose} />
              <span className="h-2.5 w-2.5 rounded-full bg-emerald-500/80 cursor-pointer" />
              <span className="h-2.5 w-2.5 rounded-full bg-rose-500/80 cursor-pointer" onClick={onClose} />
            </div>
          </div>

          {/* Modal Content */}
          <div className="p-6 space-y-5">
            <div className="flex items-start gap-4">
              {/* Logo Badge */}
              <LogoMark size="lg" />

              <div className="space-y-1 min-w-0">
                <h3 className="text-base font-bold text-text-primary flex items-center gap-2">
                  <span className="font-mono text-accent">{updateInfo.latest_version}</span>
                  <span>{t('update_modal.ready_suffix')}</span>
                </h3>
                <p className="text-xs text-text-muted flex items-center gap-1.5">
                  <Calendar className="h-3.5 w-3.5" />
                  <span>{today}</span>
                </p>
              </div>
            </div>

            {/* Changelog & Highlights */}
            <div className="space-y-2 rounded-xl bg-bg-input/60 border border-border/60 p-3.5 text-xs">
              <p className="font-semibold text-text-secondary flex items-center gap-1.5 text-[0.6875rem] uppercase tracking-wider">
                <Sparkles className="h-3 w-3 text-accent" />
                <span>{t('update_modal.changelog_title')}</span>
              </p>
              <p className="text-xs text-text-primary leading-relaxed whitespace-pre-line font-mono select-text">
                {updateInfo.changelog || 'Bản cập nhật tối ưu hóa hiệu năng, giao diện và mở rộng tính năng mới.'}
              </p>
              <div className="flex items-center gap-1.5 text-[0.6875rem] text-accent font-medium pt-1">
                <ShieldCheck className="h-3.5 w-3.5" />
                <span>Tự động tạo bản sao lưu snapshot trước khi nâng cấp</span>
              </div>
            </div>

            {/* Actions */}
            <div className="flex items-center justify-end gap-2.5 pt-1">
              <Button
                size="sm"
                variant="ghost"
                onClick={onClose}
                disabled={isApplying}
                className="text-xs rounded-xl px-4 text-text-secondary hover:text-text-primary"
              >
                {t('update_modal.btn_later')}
              </Button>
              <Button
                size="sm"
                variant="primary"
                onClick={handleApplyUpdate}
                disabled={isApplying}
                className="text-xs rounded-xl px-4 gap-1.5 bg-accent hover:bg-accent-hover text-white font-semibold shadow-xs"
              >
                {isApplying ? (
                  <>
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    <span>Đang cập nhật...</span>
                  </>
                ) : (
                  <>
                    <ArrowUpCircle className="h-3.5 w-3.5" />
                    <span>{t('update_modal.btn_update_now')}</span>
                  </>
                )}
              </Button>
            </div>
          </div>
        </motion.div>
      </div>
    </AnimatePresence>
  )
}
