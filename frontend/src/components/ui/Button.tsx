import React, { forwardRef } from 'react'
import { motion, HTMLMotionProps } from 'motion/react'
import { cn } from '@/lib/utils'

export interface ButtonProps extends Omit<HTMLMotionProps<'button'>, 'ref' | 'children'> {
  variant?: 'primary' | 'ghost' | 'danger' | 'outline' | 'secondary'
  size?: 'sm' | 'md' | 'lg' | 'icon'
  isLoading?: boolean
  children?: React.ReactNode
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'primary', size = 'md', isLoading, disabled, children, ...props }, ref) => {
    const baseStyles =
      'inline-flex items-center justify-center font-medium rounded-md transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/50 disabled:opacity-50 disabled:pointer-events-none cursor-pointer'

    const sizeStyles = {
      sm: 'h-7 px-2.5 text-xs gap-1.5',
      md: 'h-8.5 px-3.5 text-sm gap-2',
      lg: 'h-10 px-5 text-sm gap-2.5',
      icon: 'h-8 w-8 p-0',
    }

    const variantStyles = {
      primary: 'bg-accent text-bg-base font-semibold hover:bg-accent-hover shadow-sm',
      secondary: 'bg-bg-elevated border border-border text-text-primary hover:bg-bg-card-hover hover:border-border-strong',
      ghost: 'text-text-secondary hover:text-text-primary hover:bg-white/5 active:bg-white/10',
      outline: 'border border-border text-text-primary hover:border-border-strong hover:bg-bg-elevated',
      danger: 'text-error border border-error/20 bg-error/5 hover:bg-error/15 hover:border-error/40',
    }

    return (
      <motion.button
        ref={ref}
        whileTap={{ scale: disabled || isLoading ? 1 : 0.97 }}
        transition={{ duration: 0.08, ease: 'easeOut' }}
        disabled={disabled || isLoading}
        className={cn(baseStyles, sizeStyles[size], variantStyles[variant], className)}
        {...props}
      >
        {isLoading && (
          <svg className="animate-spin -ml-0.5 h-3.5 w-3.5 text-current" fill="none" viewBox="0 0 24 24">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
        )}
        {children}
      </motion.button>
    )
  }
)
Button.displayName = 'Button'
