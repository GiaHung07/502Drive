import React from 'react'

export interface LogoProps {
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl'
  showText?: boolean
  className?: string
}

export const LogoMark: React.FC<{ size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl'; className?: string }> = ({
  size = 'md',
  className = '',
}) => {
  const dimensions = {
    xs: 'w-5 h-5 rounded-lg',
    sm: 'w-7 h-7 rounded-xl',
    md: 'w-8 h-8 rounded-xl',
    lg: 'w-10 h-10 rounded-2xl',
    xl: 'w-14 h-14 rounded-3xl',
  }[size]

  return (
    <div className={`relative shrink-0 flex items-center justify-center select-none ${className}`}>
      {/* Subtle Ambient Glow */}
      <div className={`absolute inset-0 ${dimensions} bg-accent/30 blur-[6px] opacity-60`} />

      {/* Official 502Drive App Icon */}
      <img
        src="/logo.png"
        alt="502Drive"
        className={`relative z-10 ${dimensions} object-cover shadow-sm border border-white/10 dark:border-white/5 transition-transform hover:scale-105`}
      />
    </div>
  )
}

export const Logo: React.FC<LogoProps> = ({
  size = 'md',
  showText = true,
  className = '',
}) => {
  return (
    <div className={`flex items-center gap-2.5 select-none ${className}`}>
      <LogoMark size={size} />
      {showText && (
        <div className="flex items-center gap-1.5 leading-none">
          <span className="font-bold tracking-tight text-text-primary text-base font-sans">
            <span className="text-accent font-extrabold">502</span>
            <span>Drive</span>
          </span>
          <span className="px-1.5 py-0.5 text-[0.625rem] font-bold tracking-wider rounded-md bg-accent/15 text-accent border border-accent/25 uppercase">
            CORE
          </span>
        </div>
      )}
    </div>
  )
}

export default Logo
