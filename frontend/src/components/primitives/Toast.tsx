import React, { createContext, useContext, useState, useCallback, useRef } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { CheckCircle2, AlertCircle, AlertTriangle, Info, X } from 'lucide-react'
import { cn } from '@/lib/utils'
import { ToastMessage } from '@/lib/types'

interface ToastContextType {
  toast: (msg: Omit<ToastMessage, 'id'>) => void
  removeToast: (id: string) => void
}

const ToastContext = createContext<ToastContextType | undefined>(undefined)

const MAX_TOASTS = 3

export const ToastProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [toasts, setToasts] = useState<ToastMessage[]>([])
  // Dedupe guard: identical title+description fired while still visible
  // refreshes the existing toast instead of stacking another copy.
  const lastSignatureRef = useRef<{ signature: string; at: number } | null>(null)

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id))
  }, [])

  const toast = useCallback(
    ({ title, description, variant = 'default', duration = 3500 }: Omit<ToastMessage, 'id'>) => {
      const now = Date.now()
      const signature = `${variant}::${title}::${description ?? ''}`
      if (
        lastSignatureRef.current &&
        lastSignatureRef.current.signature === signature &&
        now - lastSignatureRef.current.at < 1500
      ) {
        lastSignatureRef.current.at = now
        return
      }
      lastSignatureRef.current = { signature, at: now }

      const id = Math.random().toString(36).substring(2, 9)
      const newToast: ToastMessage = { id, title, description, variant, duration }
      setToasts((prev) => {
        const next = [...prev, newToast]
        if (next.length > MAX_TOASTS) {
          return next.slice(next.length - MAX_TOASTS)
        }
        return next
      })

      if (duration > 0) {
        setTimeout(() => {
          removeToast(id)
        }, duration)
      }
    },
    [removeToast]
  )

  return (
    <ToastContext.Provider value={{ toast, removeToast }}>
      {children}
      <div
        role="region"
        aria-label="Notifications"
        aria-live="polite"
        className="fixed top-4 right-4 z-50 flex flex-col items-end gap-2 w-full max-w-xs pointer-events-none"
      >
        <AnimatePresence mode="popLayout">
          {toasts.map((t) => (
            <ToastItem key={t.id} toast={t} onClose={() => removeToast(t.id)} />
          ))}
        </AnimatePresence>
      </div>
    </ToastContext.Provider>
  )
}

export const useToast = () => {
  const context = useContext(ToastContext)
  if (!context) {
    throw new Error('useToast must be used within ToastProvider')
  }
  return context
}

const ToastItem: React.FC<{ toast: ToastMessage; onClose: () => void }> = ({ toast, onClose }) => {
  const variant = toast.variant || 'default'

  const variantStyles = {
    default: {
      card: 'border-border/80 shadow-float hover:border-accent/40',
      badge: 'bg-accent/12 text-accent',
      icon: <Info className="h-3.5 w-3.5 stroke-[2.2]" />,
      bar: 'bg-accent',
    },
    success: {
      card: 'border-success/30 shadow-float hover:border-success/45',
      badge: 'bg-success/12 text-success',
      icon: <CheckCircle2 className="h-3.5 w-3.5 stroke-[2.2]" />,
      bar: 'bg-success',
    },
    warning: {
      card: 'border-warning/30 shadow-float hover:border-warning/45',
      badge: 'bg-warning/12 text-warning',
      icon: <AlertTriangle className="h-3.5 w-3.5 stroke-[2.2]" />,
      bar: 'bg-warning',
    },
    error: {
      card: 'border-error/30 shadow-float hover:border-error/45',
      badge: 'bg-error/12 text-error',
      icon: <AlertCircle className="h-3.5 w-3.5 stroke-[2.2]" />,
      bar: 'bg-error',
    },
  }

  const current = variantStyles[variant]

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: -10, scale: 0.97 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, x: 12, scale: 0.97, transition: { duration: 0.14 } }}
      transition={{ type: 'spring', stiffness: 420, damping: 32 }}
      className={cn(
        'pointer-events-auto flex items-center gap-2.5 p-3 rounded-xl bg-bg-elevated/90 backdrop-blur-md border overflow-hidden relative select-none w-full shadow-float transition-colors',
        current.card
      )}
    >
      <div className={cn('p-1 rounded-lg shrink-0', current.badge)}>
        {current.icon}
      </div>

      <div className="flex-1 min-w-0 pr-1 space-y-px">
        <p className="text-[0.8125rem] font-medium text-text-primary leading-snug">
          {toast.title}
        </p>
        {toast.description && (
          <p className="text-xs text-text-secondary leading-snug line-clamp-2">
            {toast.description}
          </p>
        )}
      </div>

      <button
        onClick={onClose}
        aria-label="Đóng thông báo"
        className="p-1 rounded-md text-text-muted/80 hover:text-text-primary hover:bg-bg-input/70 transition-colors shrink-0 cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
      >
        <X className="h-3 w-3" />
      </button>

      {/* Synchronized progress bar indicator matching variant */}
      {toast.duration !== 0 && (
        <motion.div
          className={cn('absolute bottom-0 left-0 right-0 h-[2px] opacity-50', current.bar)}
          initial={{ width: '100%' }}
          animate={{ width: '0%' }}
          transition={{ duration: (toast.duration || 3500) / 1000, ease: 'linear' }}
        />
      )}
    </motion.div>
  )
}
