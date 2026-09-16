import React, { useState } from 'react'
import { motion } from 'motion/react'
import { Copy, Loader2, FolderOpen, CheckCircle2, FolderPlus } from 'lucide-react'
import { ModalShell } from '@/components/modals/ModalShell'
import { Button } from '@/components/ui/Button'
import { FolderPickerPanel } from '@/components/primitives/FolderPicker'
import { useToast } from '@/components/primitives/Toast'
import { api, getErrorMessage } from '@/lib/ipc'
import { DriveItemRef, CloneDuplicatePolicy } from '@/lib/types'
import { isValidDriveSource, cn } from '@/lib/utils'

export interface QuickCloneModalProps {
  isOpen: boolean
  onClose: () => void
  onRefreshData: () => Promise<void>
}

const DUPLICATE_POLICIES: { value: CloneDuplicatePolicy; label: string }[] = [
  { value: 'keep_both', label: 'Giữ cả hai' },
  { value: 'skip_same_source', label: 'Bỏ qua nếu trùng' },
  { value: 'replace_safe', label: 'Thay thế an toàn' },
]

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
  const [sourceUrl, setSourceUrl] = useState('')
  const [nameOverride, setNameOverride] = useState('')
  const [destination, setDestination] = useState<DriveItemRef | null>(null)
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
      const { request_id } = await api.createCloneRequest({
        sourceUrl: sourceUrl.trim(),
        nameOverride: nameOverride.trim() || undefined,
        destinationParentId: destination?.id,
        duplicatePolicy,
      })
      const result = await api.trackUiRequest(request_id)
      if (result.status === 'accepted') {
        toast({
          title: 'Đã tạo tác vụ sao chép',
          description: result.note || 'Daemon đã chấp nhận yêu cầu sao chép.',
          variant: 'success',
          duration: 6000,
        })
        await onRefreshData()
        setIsWaiting(false)
        resetForm()
        onClose()
      } else {
        toast({
          title: 'Yêu cầu sao chép bị từ chối',
          description: result.note || 'Daemon từ chối yêu cầu. Vui lòng kiểm tra lại liên kết.',
          variant: 'error',
          duration: 6000,
        })
        setIsWaiting(false)
      }
    } catch (err) {
      toast({
        title: 'Lỗi tạo tác vụ sao chép',
        description: getErrorMessage(err),
        variant: 'error',
        duration: 6000,
      })
      setIsWaiting(false)
    }
  }

  return (
    <ModalShell
      isOpen={isOpen}
      onClose={handleClose}
      icon={<Copy className="h-4 w-4 stroke-[2]" />}
      title="Sao chép nhanh (Quick Clone)"
      subtitle="Gửi yêu cầu sao chép thư mục Drive trực tiếp từ giao diện, không cần qua bot Telegram"
      footer={
        <>
          <Button size="sm" variant="secondary" onClick={handleClose} className="text-xs rounded-xl">
            Đóng
          </Button>
          <Button
            size="sm"
            variant="primary"
            disabled={!canSubmit}
            onClick={handleSubmit}
            title={isSourceValid ? undefined : 'Nhập liên kết hoặc ID thư mục Drive hợp lệ'}
            className="text-xs gap-1.5 rounded-xl shadow-xs bg-accent hover:bg-accent-hover text-white font-semibold"
          >
            <Copy className="h-3.5 w-3.5" />
            <span>Gửi yêu cầu sao chép</span>
          </Button>
        </>
      }
    >
      {isWaiting ? (
        <div className="flex flex-col items-center justify-center py-10 space-y-3">
          <Loader2 className="h-8 w-8 animate-spin text-accent" />
          <p className="text-sm font-semibold text-text-primary">Đang chờ daemon xác nhận…</p>
          <p className="text-xs text-text-muted text-center max-w-xs leading-relaxed">
            Yêu cầu đã được ghi vào hàng đợi. Daemon sẽ kiểm tra nguồn và xác nhận trong giây lát.
          </p>
        </div>
      ) : (
        <>
          {/* Source link */}
          <div className="space-y-1.5">
            <label htmlFor="quick-clone-source" className="text-xs font-medium text-text-secondary">
              Liên kết thư mục Drive nguồn <span className="text-error">*</span>
            </label>
            <input
              id="quick-clone-source"
              type="text"
              value={sourceUrl}
              onChange={(e) => setSourceUrl(e.target.value)}
              placeholder="VD: https://drive.google.com/drive/folders/1AbC... hoặc dán ID trực tiếp"
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
                Liên kết không hợp lệ. Hãy dùng link drive.google.com, link thư mục (folders/…) hoặc ID dài tối thiểu 20 ký tự.
              </p>
            )}
          </div>

          {/* Optional destination name override */}
          <div className="space-y-1.5">
            <label htmlFor="quick-clone-name" className="text-xs font-medium text-text-secondary">
              Tên thư mục đích <span className="text-text-muted font-normal">(tùy chọn)</span>
            </label>
            <input
              id="quick-clone-name"
              type="text"
              value={nameOverride}
              onChange={(e) => setNameOverride(e.target.value)}
              placeholder="Để trống để giữ nguyên tên thư mục gốc"
              className="w-full px-3 py-2 text-xs rounded-xl bg-bg-input border border-border/80 text-text-primary focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 transition-colors"
            />
          </div>

          {/* Destination folder picker */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-text-secondary">
                Thư mục đích <span className="text-text-muted font-normal">(tùy chọn)</span>
              </label>
              {destination && (
                <button
                  type="button"
                  onClick={() => setDestination(null)}
                  className="text-[0.6875rem] text-text-muted hover:text-error cursor-pointer transition-colors"
                >
                  Xóa lựa chọn
                </button>
              )}
            </div>
            {destination ? (
              <div className="flex items-center gap-2.5 p-3 rounded-xl bg-accent/5 border border-accent/25">
                <div className="p-1.5 rounded-lg bg-accent/10 text-accent shrink-0">
                  <CheckCircle2 className="h-4 w-4" />
                </div>
                <div className="min-w-0 flex-1">
                  <p className="text-xs font-semibold text-text-primary truncate">{destination.name}</p>
                  <p className="text-[0.625rem] font-mono text-text-muted truncate">{destination.id}</p>
                </div>
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => setIsPickerOpen((o) => !o)}
                  className="text-[0.6875rem] h-7 px-2 rounded-lg shrink-0"
                >
                  Đổi
                </Button>
              </div>
            ) : (
              <Button
                size="sm"
                variant="secondary"
                onClick={() => setIsPickerOpen((o) => !o)}
                aria-expanded={isPickerOpen}
                className="w-full h-9 text-xs gap-1.5 rounded-xl justify-start text-text-secondary"
              >
                <FolderOpen className="h-3.5 w-3.5 text-accent" />
                <span>{isPickerOpen ? 'Ẩn danh sách thư mục' : 'Chọn thư mục đích từ Drive…'}</span>
              </Button>
            )}
            {isPickerOpen && (
              <FolderPickerPanel
                selectedId={destination?.id ?? null}
                onSelect={(item) => {
                  setDestination(item)
                  setIsPickerOpen(false)
                }}
              />
            )}
            {!destination && !isPickerOpen && (
              <p className="text-[0.6875rem] text-text-muted">
                Nếu bỏ trống, daemon sẽ lưu vào thư mục đích mặc định của hệ thống.
              </p>
            )}
          </div>

          {/* Duplicate policy segmented control */}
          <div className="space-y-1.5">
            <span className="text-xs font-medium text-text-secondary block">
              Cách xử lý khi trùng lặp
            </span>
            <div className="flex items-center p-1 rounded-xl bg-bg-input/70 border border-border/50 select-none">
              {DUPLICATE_POLICIES.map((p) => {
                const isSelected = duplicatePolicy === p.value
                return (
                  <button
                    key={p.value}
                    type="button"
                    onClick={() => setDuplicatePolicy(p.value)}
                    aria-pressed={isSelected}
                    className="relative flex-1 px-2 py-1.5 text-[0.6875rem] font-medium rounded-lg transition-colors cursor-pointer text-text-secondary hover:text-text-primary"
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
                        'relative z-10',
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
              {duplicatePolicy === 'keep_both' && 'Tạo bản sao mới bên cạnh bản trùng tên (tên đuôi "(1)").'}
              {duplicatePolicy === 'skip_same_source' && 'Bỏ qua mục đã được sao chép trước đó từ cùng nguồn.'}
              {duplicatePolicy === 'replace_safe' && 'Ghi đè bản đích trùng tên một cách an toàn (kiểm tra nội dung).'}
            </p>
          </div>
        </>
      )}
    </ModalShell>
  )
}
