import React from 'react'
import { Card } from '@/components/ui/Card'
import { cn } from '@/lib/utils'
import { LucideIcon } from 'lucide-react'

export interface StatCardProps {
  label: string
  value: string | number
  subtext?: string
  icon?: LucideIcon
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
        <p className="text-xs font-normal text-text-secondary">{label}</p>
        <div className="flex items-baseline gap-1.5">
          <span className="text-lg font-semibold tracking-tight text-text-primary font-mono">{value}</span>
          {subtext && <span className="text-[0.6875rem] text-text-muted">{subtext}</span>}
        </div>
      </div>
      {Icon && (
        <div className="p-2 rounded-lg bg-accent/10 text-accent shrink-0">
          <Icon className="h-4 w-4 stroke-[1.75]" />
        </div>
      )}
    </Card>
  )
}
