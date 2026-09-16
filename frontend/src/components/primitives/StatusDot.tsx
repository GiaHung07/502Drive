import React from 'react'
import { motion } from 'motion/react'
import { cn } from '@/lib/utils'

export type StatusDotVariant = 'active' | 'idle' | 'error' | 'warning'

export interface StatusDotProps {
  variant?: StatusDotVariant
  size?: 'sm' | 'md' | 'lg'
  pulse?: boolean
  className?: string
}

export const StatusDot: React.FC<StatusDotProps> = ({
  variant = 'active',
  size = 'md',
  pulse = true,
  className,
}) => {
  const sizeMap = {
    sm: 'h-1.5 w-1.5',
    md: 'h-2 w-2',
    lg: 'h-2.5 w-2.5',
  }

  const colorMap = {
    active: 'bg-accent',
    idle: 'bg-text-muted',
    error: 'bg-error',
    warning: 'bg-warning',
  }

  return (
    <span className={cn('relative inline-flex items-center justify-center', className)}>
      {pulse && (variant === 'active' || variant === 'warning') && (
        <motion.span
          animate={{ scale: [1, 1.8, 1], opacity: [0.8, 0, 0.8] }}
          transition={{ duration: 2.2, repeat: Infinity, ease: 'easeInOut' }}
          className={cn('absolute rounded-full opacity-75', sizeMap[size], colorMap[variant])}
        />
      )}
      <span className={cn('relative rounded-full', sizeMap[size], colorMap[variant])} />
    </span>
  )
}
