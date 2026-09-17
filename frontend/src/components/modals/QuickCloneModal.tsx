import React, { useState } from 'react'
import { motion } from 'motion/react'
import { Copy, Loader2, FolderOpen, FolderPlus } from 'lucide-react'
import { ModalShell } from '@/components/modals/ModalShell'
import { Button } from '@/components/ui/Button'
import { FolderPickerPanel } from '@/components/primitives/FolderPicker'
import { useToast } from '@/components/primitives/Toast'
import { api, getErrorMessage } from '@/lib/ipc'
import { DriveItemRef, CloneDuplicatePolicy } from '@/lib/types'
import { isValidDriveSource, cn } from '@/lib/utils'
import { useI18n } from '@/hooks/useI18n'

export interface QuickCloneModalProps {
  isOpen: boolean
  onClose: () => void
  onRefreshData: () => Promise<void>
}

/**
 * "Sao chép nhanh" modal — queues a one-shot clone request via the GUI ↔
 * engine bridge (`create_clone_request`) and polls the daemon's decision
 * (`get_ui_request_status` every 2s, max 45s).
 */
export const QuickCloneModal: React.FC<QuickCloneModalProps> = ({
  isOpen,
  onClose,
  onRefreshData,
}) => {
  const { t } = useI18n()
  const [sourceUrl, setSourceUrl] = useState('')
  const [nameOverride, setNameOverride] = useState('')
  const [destination, setDestination] = useState<DriveItemRef | null>(null)
  const [manualDestId, setManualDestId] = useState('')
  const [isPickerOpen, setIsPickerOpen] = useState(false)
  const [duplicatePolicy, setDuplicatePolicy] = useState<CloneDuplicatePolicy>('keep_both')
  const [isWaiting, setIsWaiting] = useState(false)

  const { toast } = useToast()

  const isSourceValid = isValidDriveSource(sourceUrl)
  const canSubmit = isSourceValid && !isWaiting

  const resetForm = () => {
    setSourceUrl('')
    setNameOverride('')
    setDestination(null)
    setManualDestId('')
    setIsPickerOpen(false)
    setDuplicatePolicy('keep_both')
  }

  const handleClose = () => {
    if (isWaiting) return
    resetForm()
    onClose()
  }

  const handleSubmit = async () => {
    if (!canSubmit) return
    setIsWaiting(true)
    try {
      const finalDestId = destination?.id || (manualDestId.trim() ? manualDestId.trim() : undefined)
      const { request_id } = await api.createCloneRequest({
        sourceUrl: sourceUrl.trim(),
        nameOverride: nameOverride.trim() || undefined,
        destinationParentId: finalDestId,
        duplicatePolicy,
      })
      const result = await api.trackUiRequest(request_id)
      if (result.status === 'accepted') {
        toast({
          title: t('common.success'),
          description: result.note || t('quick_clone.waiting_daemon_desc'),
          variant: 'success',
          duration: 6000,
        })
        await onRefreshData()
        setIsWaiting(false)
        resetForm()
        onClose()
      } else {
        toast({
          title: t('common.error'),
          description: result.note || t('quick_clone.source_error'),
          variant: 'error',
          duration: 6000,
        })
        setIsWaiting(false)
      }
    } catch (err) {
      toast({
        title: t('common.error'),
        description: getErrorMessage(err),
        variant: 'error',
        duration: 6000,
      })
      setIsWaiting(false)
    }
  }

  const duplicatePolicies: { value: CloneDuplicatePolicy; label: string }[] = [
    { value: 'keep_both', label: t('quick_clone.policy_keep_both') },
    { value: 'skip_same_source', label: t('quick_clone.policy_skip_same_source') },
    { value: 'replace_safe', label: t('quick_clone.policy_replace_safe') },
  ]

  return (
    <ModalShell
      isOpen={isOpen}
      onClose={handleClose}
      icon={<Copy className="h-4 w-4 stroke-[2]" />}
      title={t('quick_clone.modal_title')}
      subtitle={t('quick_clone.modal_subtitle')}
      footer={
        <>
          <Button size="sm" variant="secondary" onClick={handleClose} className="text-xs rounded-xl">
            {t('common.close')}
          </Button>
          <Button
            size="sm"
            variant="primary"
            disabled={!canSubmit}
            onClick={handleSubmit}
            title={isSourceValid ? undefined : t('quick_clone.source_error')}
            className="text-xs gap-1.5 rounded-xl shadow-xs bg-accent hover:bg-accent-hover text-white font-semibold"
          >
            <Copy className="h-3.5 w-3.5" />
            <span>{t('quick_clone.btn_submit')}</span>
          </Button>
        </>
      }
    >
      {isWaiting ? (
        <div className="flex flex-col items-center justify-center py-10 space-y-3">
          <Loader2 className="h-8 w-8 animate-spin text-accent" />
          <p className="text-sm font-semibold text-text-primary">{t('quick_clone.waiting_daemon')}</p>
          <p className="text-xs text-text-muted text-center max-w-xs leading-relaxed">
            {t('quick_clone.waiting_daemon_desc')}
          </p>
        </div>
      ) : (
        <>
          {/* Source link */}
          <div className="space-y-1.5">
            <label htmlFor="quick-clone-source" className="text-xs font-medium text-text-secondary">
              {t('quick_clone.source_label')} <span className="text-error">*</span>
            </label>
            <input
              id="quick-clone-source"
              type="text"
              value={sourceUrl}
              onChange={(e) => setSourceUrl(e.target.value)}
              placeholder={t('quick_clone.source_placeholder')}
              aria-invalid={!isSourceValid && sourceUrl.trim().length > 0}
              className={cn(
                'w-full px-3 py-2 text-xs rounded-xl bg-bg-input border text-text-primary focus:outline-none focus:border-accent font-mono transition-colors',
                sourceUrl.trim() && !isSourceValid
                  ? 'border-error/60'
                  : 'border-border/80 focus:ring-2 focus:ring-accent/20'
              )}
            />
            {sourceUrl.trim() && !isSourceValid && (
              <p className="text-[0.6875rem] text-error">
                {t('quick_clone.source_error')}
              </p>
            )}
          </div>

          {/* Optional destination name override */}
          <div className="space-y-1.5">
            <label htmlFor="quick-clone-name" className="text-xs font-medium text-text-secondary">
              {t('quick_clone.dest_name_label')}
            </label>
            <input
              id="quick-clone-name"
              type="text"
              value={nameOverride}
              onChange={(e) => setNameOverride(e.target.value)}
              placeholder={t('quick_clone.dest_name_placeholder')}
              className="w-full px-3 py-2 text-xs rounded-xl bg-bg-input border border-border/80 text-text-primary focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-colors"
            />
          </div>

          {/* Destination folder picker */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-text-secondary">
                {t('quick_clone.dest_folder_label')}
              </label>
              {(destination || manualDestId) && (
                <button
                  type="button"
                  onClick={() => {
                    setDestination(null)
                    setManualDestId('')
                  }}
                  className="text-[0.6875rem] text-text-muted hover:text-error cursor-pointer transition-colors"
                >
                  {t('quick_clone.btn_clear_dest')}
                </button>
              )}
            </div>
            {destination ? (
              <div className="flex items-center gap-3 p-3.5 rounded-2xl bg-accent/10 border border-accent/30 shadow-xs">
                <div className="p-2 rounded-xl bg-accent text-white shadow-xs shrink-0">
                  <FolderOpen className="h-4.5 w-4.5" />
                </div>
                <div className="min-w-0 flex-1">
                  <p className="text-xs font-bold text-text-primary truncate">{destination.name}</p>
                  <p className="text-[0.6875rem] font-mono text-accent truncate">ID: {destination.id}</p>
                </div>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => setIsPickerOpen((o) => !o)}
                  className="text-xs h-8 px-3.5 font-semibold rounded-xl shrink-0 border-accent/40 text-accent hover:bg-accent/15"
                >
                  {isPickerOpen ? t('quick_clone.btn_hide_folder') : 'Đổi thư mục'}
                </Button>
              </div>
            ) : (
              <div className="space-y-2">
                <div className="flex gap-2.5 items-center">
                  <input
                    type="text"
                    value={manualDestId}
                    onChange={(e) => setManualDestId(e.target.value)}
                    placeholder={t('quick_clone.dest_manual_placeholder')}
                    className="flex-1 h-10 px-3.5 py-2 text-xs rounded-xl bg-bg-input border border-border/80 text-text-primary font-mono focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-all shadow-xs"
                  />
                  <Button
                    size="md"
                    variant={isPickerOpen ? "secondary" : "outline"}
                    onClick={() => setIsPickerOpen((o) => !o)}
                    aria-expanded={isPickerOpen}
                    className="h-10 px-4 text-xs font-semibold gap-2 rounded-xl shrink-0 bg-accent/10 hover:bg-accent/20 text-accent border border-accent/30 shadow-xs transition-all"
                  >
                    <FolderOpen className="h-4 w-4 stroke-[2]" />
                    <span>{isPickerOpen ? t('quick_clone.btn_hide_folder') : t('quick_clone.btn_pick_folder')}</span>
                  </Button>
                </div>
              </div>
            )}
            {isPickerOpen && (
              <FolderPickerPanel
                selectedId={destination?.id ?? null}
                onSelect={(item) => {
                  setDestination(item)
                  setManualDestId(item.id)
                  setIsPickerOpen(false)
                }}
              />
            )}
            {!destination && !manualDestId && !isPickerOpen && (
              <p className="text-[0.6875rem] text-text-muted">
                {t('quick_clone.dest_default_hint')}
              </p>
            )}
          </div>

          {/* Duplicate policy segmented control */}
          <div className="space-y-1.5">
            <span className="text-xs font-medium text-text-secondary block">
              {t('quick_clone.duplicate_policy_label')}
            </span>
            <div className="grid grid-cols-3 gap-1 p-1 rounded-xl bg-bg-input/70 border border-border/50 select-none">
              {duplicatePolicies.map((p) => {
                const isSelected = duplicatePolicy === p.value
                return (
                  <button
                    key={p.value}
                    type="button"
                    onClick={() => setDuplicatePolicy(p.value)}
                    aria-pressed={isSelected}
                    title={p.label}
                    className="relative w-full min-w-0 px-2 py-1.5 text-[0.6875rem] font-medium rounded-lg transition-colors cursor-pointer text-text-secondary hover:text-text-primary text-center"
                  >
                    {isSelected && (
                      <motion.div
                        layoutId="quickClonePolicySegment"
                        className="absolute inset-0 rounded-lg bg-bg-elevated border border-border/70 shadow-xs"
                        transition={{ type: 'spring', stiffness: 450, damping: 35 }}
                      />
                    )}
                    <span
                      className={cn(
                        'relative z-10 block truncate',
                        isSelected ? 'font-semibold text-text-primary' : ''
                      )}
                    >
                      {p.label}
                    </span>
                  </button>
                )
              })}
            </div>
            <p className="text-[0.6875rem] text-text-muted flex items-start gap-1.5">
              <FolderPlus className="h-3 w-3 shrink-0 mt-0.5 text-accent" />
              {duplicatePolicy === 'keep_both' && t('quick_clone.policy_keep_both_desc')}
              {duplicatePolicy === 'skip_same_source' && t('quick_clone.policy_skip_desc')}
              {duplicatePolicy === 'replace_safe' && t('quick_clone.policy_replace_desc')}
            </p>
          </div>
        </>
      )}
    </ModalShell>
  )
}
