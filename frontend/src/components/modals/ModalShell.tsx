import React from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { X } from 'lucide-react'

export interface ModalShellProps {
  isOpen: boolean
  onClose: () => void
  icon: React.ReactNode
  title: string
  subtitle?: string
  children: React.ReactNode
  footer?: React.ReactNode
}

/**
 * Shared modal frame matching the SetupWizardModal design language:
 * fixed inset overlay with backdrop blur, motion scale/opacity entry,
 * hairline header/footer on bg-elevated, rounded-2xl corners.
 */
export const ModalShell: React.FC<ModalShellProps> = ({
  isOpen,
  onClose,
  icon,
  title,
  subtitle,
  children,
  footer,
}) => {
  return (
    <AnimatePresence>
      {isOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs">
          <motion.div
            initial={{ opacity: 0, scale: 0.96, y: 8 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.96, y: 8 }}
            transition={{ duration: 0.15, ease: 'easeOut' }}
            role="dialog"
            aria-modal="true"
            aria-label={title}
            className="w-full max-w-xl bg-bg-elevated border border-border/80 rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]"
          >
            {/* Modal Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-border/60 bg-bg-card/50">
              <div className="flex items-center gap-2.5">
                <div className="p-2 rounded-xl bg-accent/10 text-accent">{icon}</div>
                <div>
                  <h2 className="text-sm font-semibold text-text-primary">{title}</h2>
                  {subtitle && (
                    <p className="text-[0.6875rem] text-text-secondary">{subtitle}</p>
                  )}
                </div>
              </div>
              <button
                onClick={onClose}
                aria-label="Đóng cửa sổ"
                className="p-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-input/60 transition-colors cursor-pointer"
              >
                <X className="h-4 w-4" />
              </button>
            </div>

            {/* Modal Body */}
            <div className="p-6 overflow-y-auto space-y-4 flex-1">{children}</div>

            {/* Modal Footer */}
            {footer && (
              <div className="flex items-center justify-end gap-2 px-6 py-4 border-t border-border/60 bg-bg-card/50">
                {footer}
              </div>
            )}
          </motion.div>
        </div>
      )}
    </AnimatePresence>
  )
}
