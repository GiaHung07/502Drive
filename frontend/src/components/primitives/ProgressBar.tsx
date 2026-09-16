import React from 'react'
import { motion } from 'motion/react'
import { cn } from '@/lib/utils'

export interface ProgressBarProps {
  progress: number // 0 to 100
  isRunning?: boolean
  variant?: 'accent' | 'warning' | 'error' | 'success'
  height?: 'sm' | 'md' | 'lg'
  className?: string
}

export const ProgressBar: React.FC<ProgressBarProps> = ({
  progress,
  isRunning = false,
  variant = 'accent',
  height = 'md',
  className,
}) => {
  const clamped = Math.min(100, Math.max(0, progress))

  const heightStyles = {
    sm: 'h-1',
    md: 'h-1.5',
    lg: 'h-2',
  }

  const variantStyles = {
    accent: 'bg-accent',
    warning: 'bg-warning',
    error: 'bg-error',
    success: 'bg-success',
  }

  return (
    <div
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(clamped)}
      aria-label="Tiến độ"
      className={cn(
        'w-full rounded-full bg-bg-input overflow-hidden relative',
        heightStyles[height],
        className
      )}
    >
      <motion.div
        className={cn('h-full rounded-full', variantStyles[variant], isRunning && 'shimmer-active')}
        initial={{ width: 0 }}
        animate={{ width: `${clamped}%` }}
        transition={{ duration: 0.35, ease: [0.16, 1, 0.3, 1] }}
      />
    </div>
  )
}
