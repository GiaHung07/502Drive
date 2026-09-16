import React, { useState, useEffect, useRef } from 'react'
import { Card } from '@/components/ui/Card'
import { Button } from '@/components/ui/Button'
import { DoctorResult } from '@/lib/types'
import { Terminal, Stethoscope, CheckCircle2, AlertCircle, RefreshCw, Copy, Check } from 'lucide-react'

export interface DevProps {
  onRunDoctor: () => Promise<DoctorResult>
  onGetLogs: (lines?: number) => Promise<string[]>
}

export const Dev: React.FC<DevProps> = ({ onRunDoctor, onGetLogs }) => {
  const [activeSubTab, setActiveSubTab] = useState<'doctor' | 'logs'>('doctor')
  const [doctorResult, setDoctorResult] = useState<DoctorResult | null>(null)
  const [logs, setLogs] = useState<string[]>([])
  const [isRunningDoctor, setIsRunningDoctor] = useState(false)
  const [copied, setCopied] = useState(false)
  const terminalBottomRef = useRef<HTMLDivElement>(null)

  const handleRunDoctor = async () => {
    setIsRunningDoctor(true)
    try {
      const res = await onRunDoctor()
      setDoctorResult(res)
    } finally {
      setIsRunningDoctor(false)
    }
  }

  const handleFetchLogs = async () => {
    const lines = await onGetLogs(60)
    setLogs(lines)
  }

  useEffect(() => {
    handleRunDoctor()
    handleFetchLogs()
  }, [])

  const handleCopy = () => {
    const text = activeSubTab === 'doctor' ? doctorResult?.raw_output || '' : logs.join('\n')
    navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="space-y-4 max-w-4xl pb-10">
      {/* ── Sub-header Controls ──────────────────────────── */}
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border/60 pb-2">
        <div className="flex items-center gap-1.5 p-1 rounded-lg bg-bg-elevated border border-border">
          <button
            onClick={() => setActiveSubTab('doctor')}
            className={`px-3 py-1 text-xs font-medium rounded-md transition-colors flex items-center gap-1.5 ${
              activeSubTab === 'doctor'
                ? 'bg-bg-card text-text-primary shadow-sm border border-border/80'
                : 'text-text-secondary hover:text-text-primary'
            }`}
          >
            <Stethoscope className="h-3.5 w-3.5 text-accent" />
            Chẩn đoán hệ thống (Doctor)
          </button>
          <button
            onClick={() => setActiveSubTab('logs')}
            className={`px-3 py-1 text-xs font-medium rounded-md transition-colors flex items-center gap-1.5 ${
              activeSubTab === 'logs'
                ? 'bg-bg-card text-text-primary shadow-sm border border-border/80'
                : 'text-text-secondary hover:text-text-primary'
            }`}
          >
            <Terminal className="h-3.5 w-3.5 text-info" />
            Nhật ký runtime (Logs)
          </button>
        </div>

        <div className="flex items-center gap-2">
          {activeSubTab === 'doctor' ? (
            <Button
              size="sm"
              variant="secondary"
              onClick={handleRunDoctor}
              disabled={isRunningDoctor}
              className="text-xs gap-1.5"
            >
              <RefreshCw className={`h-3 w-3 ${isRunningDoctor ? 'animate-spin text-accent' : ''}`} />
              Chạy lại Doctor
            </Button>
          ) : (
            <Button
              size="sm"
              variant="secondary"
              onClick={handleFetchLogs}
              className="text-xs gap-1.5"
            >
              <RefreshCw className="h-3 w-3" />
              Tải log mới nhất
            </Button>
          )}

          <Button
            size="sm"
            variant="ghost"
            onClick={handleCopy}
            className="text-xs gap-1"
            title="Sao chép nội dung"
          >
            {copied ? <Check className="h-3.5 w-3.5 text-accent" /> : <Copy className="h-3.5 w-3.5" />}
            <span>{copied ? 'Đã sao chép' : 'Sao chép'}</span>
          </Button>
        </div>
      </div>

      {/* ── SubTab 1: Doctor Check Matrix ───────────────── */}
      {activeSubTab === 'doctor' && (
        <div className="space-y-3">
          {doctorResult && (
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-2.5">
              {doctorResult.checks.map((c) => (
                <Card key={c.name} className="p-3 flex items-start gap-2.5">
                  {c.passed ? (
                    <CheckCircle2 className="h-4 w-4 text-accent shrink-0 mt-0.5" />
                  ) : (
                    <AlertCircle className="h-4 w-4 text-error shrink-0 mt-0.5" />
                  )}
                  <div className="space-y-0.5 overflow-hidden">
                    <p className="text-xs font-semibold text-text-primary uppercase tracking-wide font-mono">
                      {c.name}
                    </p>
                    <p className="text-[11px] text-text-muted truncate">{c.detail}</p>
                  </div>
                </Card>
              ))}
            </div>
          )}

          {/* Raw Terminal Emulator */}
          <div className="rounded-lg bg-bg-base border border-border overflow-hidden font-mono text-xs shadow-card">
            <div className="bg-bg-elevated px-4 py-2 border-b border-border flex items-center justify-between text-text-muted text-[11px]">
              <div className="flex items-center gap-2">
                <span className="h-2.5 w-2.5 rounded-full bg-error/60" />
                <span className="h-2.5 w-2.5 rounded-full bg-warning/60" />
                <span className="h-2.5 w-2.5 rounded-full bg-accent/60" />
                <span className="ml-2 text-text-secondary">502drive doctor console</span>
              </div>
              <span>bash</span>
            </div>
            <pre className="p-4 text-text-primary overflow-x-auto leading-relaxed max-h-[380px] select-text">
              {doctorResult?.raw_output || 'Đang thực thi chẩn đoán...'}
            </pre>
          </div>
        </div>
      )}

      {/* ── SubTab 2: Runtime Logs ──────────────────────── */}
      {activeSubTab === 'logs' && (
        <div className="rounded-lg bg-bg-base border border-border overflow-hidden font-mono text-xs shadow-card">
          <div className="bg-bg-elevated px-4 py-2 border-b border-border flex items-center justify-between text-text-muted text-[11px]">
            <span className="text-text-secondary">~/.local/share/gdclone-bot/logs/502drive.log</span>
            <span className="text-[10px]">{logs.length} dòng hiển thị</span>
          </div>
          <div className="p-4 space-y-1 overflow-y-auto max-h-[460px] select-text text-[11px] leading-relaxed">
            {logs.length === 0 ? (
              <p className="text-text-muted">Chưa có bản ghi nhật ký nào.</p>
            ) : (
              logs.map((log, i) => {
                const isError = log.includes('[ERROR]')
                const isWarn = log.includes('[WARN]')
                const isInfo = log.includes('[INFO]')

                return (
                  <div
                    key={i}
                    className={`font-mono ${
                      isError
                        ? 'text-error'
                        : isWarn
                        ? 'text-warning'
                        : isInfo
                        ? 'text-text-secondary'
                        : 'text-text-muted'
                    }`}
                  >
                    {log}
                  </div>
                )
              })
            )}
            <div ref={terminalBottomRef} />
          </div>
        </div>
      )}
    </div>
  )
}
