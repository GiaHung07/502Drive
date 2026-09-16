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
      'inline-flex items-center justify-center font-medium rounded-lg transition-all duration-120 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 disabled:opacity-45 disabled:pointer-events-none cursor-pointer select-none text-xs'

    const sizeStyles = {
      sm: 'h-8 px-3 text-xs sm:text-sm gap-1.5',
      md: 'h-9 px-4 text-xs sm:text-sm gap-2',
      lg: 'h-10.5 px-5 text-sm sm:text-base gap-2.5',
      icon: 'h-9 w-9 p-0',
    }

    const variantStyles = {
      primary:
        'bg-accent text-bg-base font-semibold hover:bg-accent-hover active:scale-[0.98] shadow-sm',
      secondary:
        'bg-bg-input/70 border border-border/70 text-text-primary hover:bg-bg-input hover:border-border active:scale-[0.98]',
      ghost:
        'text-text-secondary hover:text-text-primary hover:bg-bg-input/50 active:bg-bg-input/80',
      outline:
        'border border-border/80 text-text-primary hover:border-border hover:bg-bg-input/40',
      danger:
        'text-error border border-error/25 bg-transparent hover:bg-error/10 hover:border-error/45 active:scale-[0.98]',
    }

    return (
      <motion.button
        ref={ref}
        whileTap={{ scale: disabled || isLoading ? 1 : 0.98 }}
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
