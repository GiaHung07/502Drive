import React, { createContext, useContext, useState, useCallback } from 'react'
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

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id))
  }, [])

  const toast = useCallback(
    ({ title, description, variant = 'default', duration = 3500 }: Omit<ToastMessage, 'id'>) => {
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
        className="fixed top-5 left-1/2 -translate-x-1/2 z-50 flex flex-col items-center gap-2 max-w-sm sm:max-w-md w-full px-4 pointer-events-none"
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
      card: 'border-border/80 shadow-modal hover:border-accent/40',
      badge: 'bg-accent/15 text-accent border border-accent/25',
      icon: <Info className="h-4 w-4 stroke-[2.2]" />,
      bar: 'bg-accent',
    },
    success: {
      card: 'border-success/35 shadow-[0_12px_36px_-6px_rgba(34,197,94,0.18)] hover:border-success/50',
      badge: 'bg-success/15 text-success border border-success/30',
      icon: <CheckCircle2 className="h-4 w-4 stroke-[2.2]" />,
      bar: 'bg-success',
    },
    warning: {
      card: 'border-warning/35 shadow-[0_12px_36px_-6px_rgba(234,179,8,0.18)] hover:border-warning/50',
      badge: 'bg-warning/15 text-warning border border-warning/30',
      icon: <AlertTriangle className="h-4 w-4 stroke-[2.2]" />,
      bar: 'bg-warning',
    },
    error: {
      card: 'border-error/35 shadow-[0_12px_36px_-6px_rgba(239,68,68,0.18)] hover:border-error/50',
      badge: 'bg-error/15 text-error border border-error/30',
      icon: <AlertCircle className="h-4 w-4 stroke-[2.2]" />,
      bar: 'bg-error',
    },
  }

  const current = variantStyles[variant]

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: -24, scale: 0.92, filter: 'blur(3px)' }}
      animate={{ opacity: 1, y: 0, scale: 1, filter: 'blur(0px)' }}
      exit={{ opacity: 0, y: -16, scale: 0.94, filter: 'blur(3px)', transition: { duration: 0.16 } }}
      transition={{ type: 'spring', stiffness: 450, damping: 30, mass: 0.8 }}
      className={cn(
        'pointer-events-auto flex items-start gap-3 p-3.5 rounded-2xl bg-bg-elevated/95 backdrop-blur-2xl border overflow-hidden relative select-none w-full max-w-sm sm:max-w-md transition-all',
        current.card
      )}
    >
      <div className={cn('p-1.5 rounded-xl shrink-0 mt-0.5', current.badge)}>
        {current.icon}
      </div>

      <div className="flex-1 space-y-0.5 min-w-0 pr-1">
        <p className="text-xs font-semibold text-text-primary leading-tight tracking-tight">
          {toast.title}
        </p>
        {toast.description && (
          <p className="text-[0.6875rem] text-text-secondary leading-relaxed line-clamp-2">
            {toast.description}
          </p>
        )}
      </div>

      <button
        onClick={onClose}
        aria-label="Đóng thông báo"
        className="p-1 rounded-lg text-text-muted hover:text-text-primary hover:bg-bg-input/80 transition-colors shrink-0 cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
      >
        <X className="h-3.5 w-3.5" />
      </button>

      {/* Synchronized progress bar indicator matching variant */}
      {toast.duration !== 0 && (
        <motion.div
          className={cn('absolute bottom-0 left-0 right-0 h-[2px] opacity-75', current.bar)}
          initial={{ width: '100%' }}
          animate={{ width: '0%' }}
          transition={{ duration: (toast.duration || 3500) / 1000, ease: 'linear' }}
        />
      )}
    </motion.div>
  )
}
