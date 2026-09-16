import React, { useState } from 'react'
import { Radio, Loader2, CheckCircle2, FolderOpen, X, Tag } from 'lucide-react'
import { ModalShell } from '@/components/modals/ModalShell'
import { Button } from '@/components/ui/Button'
import { FolderPickerPanel } from '@/components/primitives/FolderPicker'
import { useToast } from '@/components/primitives/Toast'
import { api, getErrorMessage } from '@/lib/ipc'
import { DriveItemRef } from '@/lib/types'
import { isValidDriveSource, cn } from '@/lib/utils'

export interface CreateWatchModalProps {
  isOpen: boolean
  onClose: () => void
  onRefreshData: () => Promise<void>
}

/**
 * "Tạo theo dõi" modal — queues a watch request via the GUI ↔ engine bridge
 * (`create_watch_request`) and polls the daemon's decision.
 */
export const CreateWatchModal: React.FC<CreateWatchModalProps> = ({
  isOpen,
  onClose,
  onRefreshData,
}) => {
  const [sourceUrl, setSourceUrl] = useState('')
  const [destination, setDestination] = useState<DriveItemRef | null>(null)
  const [isPickerOpen, setIsPickerOpen] = useState(false)
  const [globs, setGlobs] = useState<string[]>([])
  const [globInput, setGlobInput] = useState('')
  const [isWaiting, setIsWaiting] = useState(false)

  const { toast } = useToast()

  const isSourceValid = isValidDriveSource(sourceUrl)
  const canSubmit = isSourceValid && !isWaiting

  const resetForm = () => {
    setSourceUrl('')
    setDestination(null)
    setIsPickerOpen(false)
    setGlobs([])
    setGlobInput('')
  }

  const handleClose = () => {
    if (isWaiting) return
    resetForm()
    onClose()
  }

  const addGlob = () => {
    const pattern = globInput.trim()
    if (!pattern) return
    if (!globs.includes(pattern)) {
      setGlobs((g) => [...g, pattern])
    }
    setGlobInput('')
  }

  const removeGlob = (pattern: string) => {
    setGlobs((g) => g.filter((x) => x !== pattern))
  }

  const handleGlobKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault()
      addGlob()
    } else if (e.key === 'Backspace' && !globInput && globs.length > 0) {
      setGlobs((g) => g.slice(0, -1))
    }
  }

  const handleSubmit = async () => {
    if (!canSubmit) return
    setIsWaiting(true)
    try {
      const { request_id } = await api.createWatchRequest({
        sourceUrl: sourceUrl.trim(),
        destinationUrl: destination?.id,
        excludeGlobs: globs,
      })
      const result = await api.trackUiRequest(request_id)
      if (result.status === 'accepted') {
        toast({
          title: 'Đã tạo thư mục theo dõi',
          description: result.note || 'Daemon đã chấp nhận và thiết lập theo dõi thời gian thực.',
          variant: 'success',
          duration: 6000,
        })
        await onRefreshData()
        setIsWaiting(false)
        resetForm()
        onClose()
      } else {
        toast({
          title: 'Yêu cầu theo dõi bị từ chối',
          description: result.note || 'Daemon từ chối yêu cầu. Vui lòng kiểm tra lại liên kết.',
          variant: 'error',
          duration: 6000,
        })
        setIsWaiting(false)
      }
    } catch (err) {
      toast({
        title: 'Lỗi tạo thư mục theo dõi',
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
      icon={<Radio className="h-4 w-4 stroke-[2]" />}
      title="Tạo theo dõi thời gian thực (Watch)"
      subtitle="Daemon sẽ tự động đồng bộ thay đổi từ thư mục nguồn sang đích"
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
            <Radio className="h-3.5 w-3.5" />
            <span>Gửi yêu cầu theo dõi</span>
          </Button>
        </>
      }
    >
      {isWaiting ? (
        <div className="flex flex-col items-center justify-center py-10 space-y-3">
          <Loader2 className="h-8 w-8 animate-spin text-accent" />
          <p className="text-sm font-semibold text-text-primary">Đang chờ daemon xác nhận…</p>
          <p className="text-xs text-text-muted text-center max-w-xs leading-relaxed">
            Daemon sẽ kiểm tra nguồn, chạy bản clone đầu tiên và kích hoạt luồng sự kiện thay đổi.
          </p>
        </div>
      ) : (
        <>
          {/* Source link */}
          <div className="space-y-1.5">
            <label htmlFor="create-watch-source" className="text-xs font-medium text-text-secondary">
              Liên kết thư mục Drive nguồn <span className="text-error">*</span>
            </label>
            <input
              id="create-watch-source"
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

          {/* Optional destination picker */}
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
          </div>

          {/* Exclusion globs chip editor */}
          <div className="space-y-1.5">
            <label
              htmlFor="create-watch-glob"
              className="text-xs font-medium text-text-secondary flex items-center gap-1.5"
            >
              <Tag className="h-3 w-3 text-accent" />
              Mẫu loại trừ (exclude globs)
              <span className="text-text-muted font-normal">— tùy chọn</span>
            </label>
            {globs.length > 0 && (
              <div className="flex flex-wrap items-center gap-1.5">
                {globs.map((g) => (
                  <span
                    key={g}
                    className="inline-flex items-center gap-1 pl-2.5 pr-1 py-0.5 rounded-full bg-accent/10 border border-accent/25 text-[0.6875rem] font-mono text-accent"
                  >
                    {g}
                    <button
                      type="button"
                      onClick={() => removeGlob(g)}
                      aria-label={`Xóa mẫu ${g}`}
                      title={`Xóa mẫu ${g}`}
                      className="p-0.5 rounded-full hover:bg-accent/20 cursor-pointer transition-colors"
                    >
                      <X className="h-2.5 w-2.5" />
                    </button>
                  </span>
                ))}
              </div>
            )}
            <input
              id="create-watch-glob"
              type="text"
              value={globInput}
              onChange={(e) => setGlobInput(e.target.value)}
              onKeyDown={handleGlobKeyDown}
              placeholder={globs.length === 0 ? 'VD: *.mp4 rồi nhấn Enter' : 'Thêm mẫu khác…'}
              className="w-full px-3 py-2 text-xs rounded-xl bg-bg-input border border-border/80 text-text-primary focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/20 font-mono transition-colors"
            />
            <p className="text-[0.6875rem] text-text-muted leading-relaxed">
              Nhập mẫu rồi nhấn <kbd className="px-1 py-0.5 rounded bg-bg-input border border-border/40 font-mono text-[0.625rem]">Enter</kbd> để thêm, nhấn <kbd className="px-1 py-0.5 rounded bg-bg-input border border-border/40 font-mono text-[0.625rem]">X</kbd> trên chip để xóa. Ví dụ: <span className="font-mono text-text-secondary">*.mp4</span>, <span className="font-mono text-text-secondary">**/temp/**</span>
            </p>
          </div>
        </>
      )}
    </ModalShell>
  )
}
