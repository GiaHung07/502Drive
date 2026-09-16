import React from 'react'
import { Card } from '@/components/ui/Card'
import { cn } from '@/lib/utils'
import { LucideIcon } from 'lucide-react'

export interface StatCardProps {
  label: string
  value: string | number
  subtext?: string
  icon?: LucideIcon
  trend?: string
  className?: string
}

export const StatCard: React.FC<StatCardProps> = ({
  label,
  value,
  subtext,
  icon: Icon,
  className,
}) => {
  return (
    <Card className={cn('p-3.5 flex items-center justify-between', className)}>
      <div className="space-y-1">
        <p className="text-xs font-medium text-text-secondary">{label}</p>
        <div className="flex items-baseline gap-2">
          <span className="text-xl font-bold tracking-tight text-text-primary font-mono">{value}</span>
          {subtext && <span className="text-[11px] text-text-muted">{subtext}</span>}
        </div>
      </div>
      {Icon && (
        <div className="p-2.5 rounded-md bg-bg-elevated border border-border/50 text-text-secondary">
          <Icon className="h-4 w-4 stroke-[1.6]" />
        </div>
      )}
    </Card>
  )
}
