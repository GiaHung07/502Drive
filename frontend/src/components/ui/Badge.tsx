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

export const Badge: React.FC<BadgeProps> = ({ className, variant = 'default', pulse = false, children, ...props }) => {
  const variantStyles: Record<BadgeVariant, string> = {
    running: 'bg-accent/12 text-accent border-accent/30',
    active: 'bg-accent/12 text-accent border-accent/30',
    connected: 'bg-accent/12 text-accent border-accent/30',
    done: 'bg-success/12 text-success border-success/30',
    completed: 'bg-success/12 text-success border-success/30',
    paused: 'bg-warning/12 text-warning border-warning/30',
    failed: 'bg-error/12 text-error border-error/30',
    disconnected: 'bg-error/12 text-error border-error/30',
    queued: 'bg-info/12 text-info border-info/30',
    initializing: 'bg-info/12 text-info border-info/30',
    default: 'bg-bg-elevated text-text-secondary border-border',
  }

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium border tracking-wide uppercase font-mono text-[10px]',
        variantStyles[variant] || variantStyles.default,
        className
      )}
      {...props}
    >
      {pulse && (
        <span className="relative flex h-1.5 w-1.5">
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-current opacity-75"></span>
          <span className="relative inline-flex rounded-full h-1.5 w-1.5 bg-current"></span>
        </span>
      )}
      {children}
    </span>
  )
}
