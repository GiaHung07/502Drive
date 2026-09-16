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

export const ToastProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [toasts, setToasts] = useState<ToastMessage[]>([])

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id))
  }, [])

  const toast = useCallback(
    ({ title, description, variant = 'default', duration = 3500 }: Omit<ToastMessage, 'id'>) => {
      const id = Math.random().toString(36).substring(2, 9)
      const newToast: ToastMessage = { id, title, description, variant, duration }
      setToasts((prev) => [...prev, newToast])

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
        className="fixed top-4 right-4 z-50 flex flex-col gap-2 max-w-xs w-full pointer-events-none"
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
  const icons = {
    default: <Info className="h-4 w-4 text-info shrink-0 stroke-[1.75]" />,
    success: <CheckCircle2 className="h-4 w-4 text-accent shrink-0 stroke-[1.75]" />,
    warning: <AlertTriangle className="h-4 w-4 text-warning shrink-0 stroke-[1.75]" />,
    error: <AlertCircle className="h-4 w-4 text-error shrink-0 stroke-[1.75]" />,
  }

  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: -12, scale: 0.94 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, x: 50, scale: 0.94, transition: { duration: 0.15 } }}
      transition={{ duration: 0.2, ease: [0.16, 1, 0.3, 1] }}
      className={cn(
        'pointer-events-auto flex items-start gap-2.5 p-3 rounded-xl bg-bg-card/95 backdrop-blur-md border border-border/80 shadow-modal overflow-hidden relative select-none'
      )}
    >
      <div className="pt-0.5">{icons[toast.variant || 'default']}</div>
      <div className="flex-1 space-y-0.5 min-w-0">
        <p className="text-xs font-semibold text-text-primary leading-tight">{toast.title}</p>
        {toast.description && (
          <p className="text-[0.6875rem] text-text-secondary leading-normal truncate">{toast.description}</p>
        )}
      </div>
      <button
        onClick={onClose}
        aria-label="Đóng thông báo"
        className="p-1 rounded-md text-text-muted hover:text-text-primary hover:bg-bg-input transition-colors shrink-0 cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
      >
        <X className="h-3.5 w-3.5" />
      </button>

      {/* Subtle indicator bar */}
      <motion.div
        className="absolute bottom-0 left-0 right-0 h-[2px] bg-accent/60"
        initial={{ width: '100%' }}
        animate={{ width: '0%' }}
        transition={{ duration: (toast.duration || 3500) / 1000, ease: 'linear' }}
      />
    </motion.div>
  )
}
