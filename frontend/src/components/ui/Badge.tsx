import React from 'react'
import { cn } from '@/lib/utils'

export type BadgeVariant =
  | 'running'
  | 'paused'
  | 'done'
  | 'completed'
  | 'failed'
  | 'queued'
  | 'active'
  | 'initializing'
  | 'connected'
  | 'disconnected'
  | 'default'

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  variant?: BadgeVariant
  pulse?: boolean
}

export const Badge: React.FC<BadgeProps> = ({
  className,
  variant = 'default',
  pulse = false,
  children,
  ...props
}) => {
  const variantStyles: Record<BadgeVariant, string> = {
    running: 'bg-accent/12 text-accent',
    active: 'bg-accent/12 text-accent',
    connected: 'bg-accent/15 text-accent',
    done: 'bg-success/12 text-success',
    completed: 'bg-success/12 text-success',
    paused: 'bg-warning/12 text-warning',
    failed: 'bg-error/12 text-error',
    disconnected: 'bg-error/12 text-error',
    queued: 'bg-info/12 text-info',
    initializing: 'bg-info/12 text-info',
    default: 'bg-bg-input text-text-secondary',
  }

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[0.6875rem] font-medium whitespace-nowrap transition-colors select-none max-w-full min-w-0',
        variantStyles[variant] || variantStyles.default,
        className
      )}
      {...props}
    >
      {pulse && (
        <span className="relative flex h-1.5 w-1.5 shrink-0">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-current opacity-60"></span>
          <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-current"></span>
        </span>
      )}
      <span className="truncate">{children}</span>
    </span>
  )
}
