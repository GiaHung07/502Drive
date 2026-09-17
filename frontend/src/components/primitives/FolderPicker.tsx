import React, { useState, useEffect, useCallback } from 'react'
import { ChevronRight, CornerLeftUp, Folder, Loader2, Check, AlertCircle } from 'lucide-react'
import { api, getErrorMessage } from '@/lib/ipc'
import { DriveItemRef } from '@/lib/types'
import { cn } from '@/lib/utils'

import { useI18n } from '@/hooks/useI18n'
import { Button } from '@/components/ui/Button'
import { LogIn } from 'lucide-react'

export interface FolderPickerPanelProps {
  /** Currently selected folder id (highlighted row). */
  selectedId?: string | null
  /** Called when the user picks a folder (row-level "Chọn" button). */
  onSelect: (item: DriveItemRef) => void
  className?: string
}

/**
 * Reusable Drive folder picker panel used by the clone + watch modals.
 * Browses via `browse_drive_children` with breadcrumb + back navigation.
 * Row click drills into a folder; the "Chọn" button picks it as destination.
 */
export const FolderPickerPanel: React.FC<FolderPickerPanelProps> = ({
  selectedId,
  onSelect,
  className,
}) => {
  const { t } = useI18n()
  // Path of folders drilled into; empty array = root level (shared drives + My Drive).
  const [path, setPath] = useState<DriveItemRef[]>([])
  const [items, setItems] = useState<DriveItemRef[]>([])
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // Shared-drive context to pass back as `drive_id` when listing children.
  const currentDriveId = useCallback(
    (stack: DriveItemRef[]): string | null => {
      for (let i = stack.length - 1; i >= 0; i--) {
        if (stack[i].drive_id) return stack[i].drive_id ?? null
      }
      return null
    },
    []
  )

  useEffect(() => {
    let cancelled = false
    const load = async () => {
      setIsLoading(true)
      setError(null)
      try {
        const parent = path.length > 0 ? path[path.length - 1] : undefined
        const nextDriveId = currentDriveId(path)
        const children = await api.browseDriveChildren(parent?.id, nextDriveId ?? undefined)
        if (!cancelled) setItems(children)
      } catch (err) {
        if (!cancelled) setError(getErrorMessage(err))
      } finally {
        if (!cancelled) setIsLoading(false)
      }
    }
    load()
    return () => {
      cancelled = true
    }
  }, [path, currentDriveId])

  const enterFolder = (item: DriveItemRef) => {
    setPath((p) => [...p, item])
  }

  const truncateTo = (index: number) => {
    setPath((p) => (index < 0 ? [] : p.slice(0, index + 1)))
  }

  const goBack = () => {
    setPath((p) => p.slice(0, -1))
  }

  const handleQuickReconnect = async () => {
    try {
      await api.triggerLogin()
    } catch (e) {
      console.error(e)
    }
  }

  return (
    <div
      className={cn(
        'rounded-xl border border-border/70 bg-bg-input/40 overflow-hidden shadow-xs',
        className
      )}
    >
      {/* Breadcrumb + back navigation */}
      <div className="flex items-center gap-1 px-2.5 py-2 border-b border-border/50 bg-bg-card/60 overflow-x-auto">
        <button
          type="button"
          onClick={goBack}
          disabled={path.length === 0}
          aria-label={t('folder_picker.back')}
          title={t('folder_picker.back')}
          className="p-1 rounded-md text-text-secondary hover:text-text-primary hover:bg-bg-input/70 disabled:opacity-35 disabled:pointer-events-none cursor-pointer transition-colors shrink-0"
        >
          <CornerLeftUp className="h-3.5 w-3.5" />
        </button>
        <div className="flex items-center gap-0.5 text-[0.6875rem] min-w-0">
          <button
            type="button"
            onClick={() => truncateTo(-1)}
            className="px-1.5 py-0.5 rounded-md font-medium text-text-secondary hover:text-text-primary hover:bg-bg-input/70 cursor-pointer transition-colors shrink-0"
          >
            {t('folder_picker.root')}
          </button>
          {path.map((p, idx) => (
            <React.Fragment key={p.id}>
              <ChevronRight className="h-3 w-3 text-text-muted shrink-0" />
              <button
                type="button"
                onClick={() => truncateTo(idx)}
                className={cn(
                  'px-1.5 py-0.5 rounded-md font-medium cursor-pointer transition-colors truncate max-w-[10rem]',
                  idx === path.length - 1
                    ? 'text-text-primary'
                    : 'text-text-secondary hover:text-text-primary hover:bg-bg-input/70'
                )}
              >
                {p.name}
              </button>
            </React.Fragment>
          ))}
        </div>
      </div>

      {/* Folder list */}
      <div className="max-h-56 overflow-y-auto">
        {isLoading ? (
          <div className="flex items-center justify-center gap-2 py-6 text-xs text-text-secondary">
            <Loader2 className="h-4 w-4 animate-spin text-accent" />
            <span>{t('folder_picker.loading')}</span>
          </div>
        ) : error ? (
          <div className="flex flex-col gap-2.5 py-4 px-3 text-xs text-error">
            <div className="flex items-start gap-2">
              <AlertCircle className="h-4 w-4 shrink-0 mt-0.5" />
              <span className="font-medium leading-relaxed">
                {error.includes('401') || error.includes('unauthorized_client')
                  ? t('folder_picker.auth_error_title')
                  : error}
              </span>
            </div>
            {(error.includes('401') || error.includes('unauthorized_client')) && (
              <div className="ml-6 space-y-2 text-[0.6875rem] text-text-secondary bg-bg-card rounded-lg p-2.5 border border-border/60">
                <p className="font-semibold text-text-primary">{t('folder_picker.auth_fix_title')}</p>
                <ol className="list-decimal list-inside space-y-1 leading-relaxed">
                  <li>{t('folder_picker.auth_fix_step1')}</li>
                  <li>{t('folder_picker.auth_fix_step2')}</li>
                  <li>{t('folder_picker.auth_fix_step3')}</li>
                </ol>
                <Button
                  size="sm"
                  variant="primary"
                  onClick={handleQuickReconnect}
                  className="text-xs h-7 gap-1.5 rounded-lg mt-1 w-full bg-accent hover:bg-accent/90 text-white font-semibold"
                >
                  <LogIn className="h-3 w-3" />
                  <span>{t('folder_picker.btn_reconnect')}</span>
                </Button>
              </div>
            )}
          </div>
        ) : items.length === 0 ? (
          <p className="py-6 text-center text-xs text-text-muted">{t('folder_picker.empty')}</p>
        ) : (
          <ul className="p-1.5 space-y-0.5">
            {items.map((item) => {
              const isSelected = selectedId === item.id
              return (
                <li key={item.id}>
                  <div
                    className={cn(
                      'group flex items-center gap-2 rounded-lg px-2 py-1.5 transition-colors',
                      isSelected ? 'bg-accent/10' : 'hover:bg-bg-input/70'
                    )}
                  >
                    <button
                      type="button"
                      onClick={() => enterFolder(item)}
                      className="flex items-center gap-2 flex-1 min-w-0 text-left cursor-pointer"
                      aria-label={`Mở thư mục ${item.name}`}
                    >
                      <Folder
                        className={cn(
                          'h-4 w-4 shrink-0 stroke-[1.6]',
                          isSelected ? 'text-accent' : 'text-text-muted'
                        )}
                      />
                      <span
                        className={cn(
                          'text-xs truncate',
                          isSelected ? 'text-accent font-semibold' : 'text-text-primary'
                        )}
                      >
                        {item.name}
                      </span>
                    </button>
                    <button
                      type="button"
                      onClick={() => onSelect(item)}
                      aria-label={`Chọn ${item.name} làm thư mục đích`}
                      title={`Chọn "${item.name}"`}
                      className={cn(
                        'p-1 rounded-md cursor-pointer transition-all shrink-0',
                        isSelected
                          ? 'text-accent'
                          : 'text-text-muted opacity-0 group-hover:opacity-100 hover:text-accent focus-visible:opacity-100'
                      )}
                    >
                      <Check className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </li>
              )
            })}
          </ul>
        )}
      </div>
    </div>
  )
}
