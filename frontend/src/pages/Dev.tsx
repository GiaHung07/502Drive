import React, { useState, useEffect, useRef } from 'react'
import { Card } from '@/components/ui/Card'
import { Button } from '@/components/ui/Button'
import { DoctorResult } from '@/lib/types'
import { useToast } from '@/components/primitives/Toast'
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
  const [error, setError] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)
  const terminalBottomRef = useRef<HTMLDivElement>(null)
  const { toast } = useToast()

  const handleRunDoctor = async () => {
    setIsRunningDoctor(true)
    try {
      const res = await onRunDoctor()
      setDoctorResult(res)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setIsRunningDoctor(false)
    }
  }

  const handleFetchLogs = async () => {
    try {
      const lines = await onGetLogs(80)
      setLogs(lines)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  useEffect(() => {
    handleRunDoctor()
    handleFetchLogs()
  }, [])

  useEffect(() => {
    if (activeSubTab === 'logs' && terminalBottomRef.current) {
      terminalBottomRef.current.scrollIntoView({ behavior: 'smooth' })
    }
  }, [logs, activeSubTab])

  const handleCopy = () => {
    const text = activeSubTab === 'doctor' ? doctorResult?.raw_output || '' : logs.join('\n')
    navigator.clipboard.writeText(text)
    setCopied(true)
    toast({
      title: 'Đã sao chép',
      description: activeSubTab === 'doctor' ? 'Đã sao chép kết quả chẩn đoán vào bộ nhớ tạm.' : `Đã sao chép ${logs.length} dòng nhật ký vào bộ nhớ tạm.`,
      variant: 'default',
      duration: 2500,
    })
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="space-y-4 max-w-4xl pb-10">
      {/* ── Sub-header Controls ──────────────────────────── */}
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-border/50 pb-2">
        {/* Segmented Pill */}
        <div className="flex items-center p-0.5 rounded-lg bg-bg-input">
          <button
            onClick={() => setActiveSubTab('doctor')}
            aria-pressed={activeSubTab === 'doctor'}
            className={`px-3 py-1 text-xs font-medium rounded-md transition-all flex items-center gap-1.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
              activeSubTab === 'doctor'
                ? 'bg-bg-elevated text-text-primary shadow-sm'
                : 'text-text-secondary hover:text-text-primary'
            }`}
          >
            <Stethoscope className="h-3.5 w-3.5 text-accent" />
            <span>Chẩn đoán hệ thống</span>
          </button>
          <button
            onClick={() => setActiveSubTab('logs')}
            aria-pressed={activeSubTab === 'logs'}
            className={`px-3 py-1 text-xs font-medium rounded-md transition-all flex items-center gap-1.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
              activeSubTab === 'logs'
                ? 'bg-bg-elevated text-text-primary shadow-sm'
                : 'text-text-secondary hover:text-text-primary'
            }`}
          >
            <Terminal className="h-3.5 w-3.5 text-info" />
            <span>Nhật ký vận hành</span>
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
              <span>Chạy lại kiểm tra</span>
            </Button>
          ) : (
            <Button
              size="sm"
              variant="secondary"
              onClick={handleFetchLogs}
              className="text-xs gap-1.5"
            >
              <RefreshCw className="h-3 w-3" />
              <span>Làm mới nhật ký</span>
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

      {/* ── Error Banner ─────────────────────────────────── */}
      {error && (
        <div
          role="alert"
          className="flex items-start gap-2.5 p-3 rounded-xl bg-error/10 border border-error/30"
        >
          <AlertCircle className="h-4 w-4 text-error shrink-0 mt-0.5" />
          <div className="space-y-0.5 min-w-0">
            <p className="text-xs font-semibold text-error">Đã xảy ra lỗi</p>
            <p className="text-[0.6875rem] text-text-secondary font-mono break-words">{error}</p>
          </div>
        </div>
      )}

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
                    <p className="text-xs font-medium text-text-primary">
                      {c.name}
                    </p>
                    <p className="text-[0.6875rem] text-text-muted truncate">{c.detail}</p>
                  </div>
                </Card>
              ))}
            </div>
          )}

          {/* Raw Terminal Emulator */}
          <div className="rounded-xl bg-bg-input/60 border border-border/60 overflow-hidden font-mono text-xs shadow-card">
            <div className="bg-bg-elevated px-4 py-2 border-b border-border/50 flex items-center justify-between text-text-muted text-[0.6875rem]">
              <div className="flex items-center gap-2">
                <span className="h-2.5 w-2.5 rounded-full bg-error/70" />
                <span className="h-2.5 w-2.5 rounded-full bg-warning/70" />
                <span className="h-2.5 w-2.5 rounded-full bg-accent/70" />
                <span className="ml-2 text-text-secondary text-[0.6875rem]">502drive-doctor output</span>
              </div>
              <span className="text-[0.625rem] text-text-muted">console</span>
            </div>
            <pre className="p-4 text-text-primary overflow-x-auto leading-relaxed max-h-[360px] select-text font-mono text-[0.6875rem]">
              {doctorResult?.raw_output || 'Đang thực thi chẩn đoán...'}
            </pre>
          </div>
        </div>
      )}

      {/* ── SubTab 2: Runtime Logs with Semantic Colored Left Border ──────── */}
      {activeSubTab === 'logs' && (
        <div className="rounded-xl bg-bg-input/60 border border-border/60 overflow-hidden font-mono text-xs shadow-card">
          <div className="bg-bg-elevated px-4 py-2 border-b border-border/50 flex items-center justify-between text-text-muted text-[0.6875rem]">
            <span className="text-text-secondary text-[0.6875rem]">~/.local/share/gdclone-bot/logs/502drive.log</span>
            <span className="text-[0.625rem] text-text-muted">{logs.length} dòng</span>
          </div>
          <div className="p-3 space-y-1 overflow-y-auto max-h-[460px] select-text text-[0.6875rem] leading-relaxed">
            {logs.length === 0 ? (
              <p className="text-text-muted p-2">Chưa có bản ghi nhật ký nào.</p>
            ) : (
              logs.map((log, i) => {
                const isError = log.includes('[ERROR]')
                const isWarn = log.includes('[WARN]')
                const isInfo = log.includes('[INFO]')

                return (
                  <div
                    key={i}
                    className={`font-mono py-0.5 rounded-r transition-colors ${
                      isError
                        ? 'border-l-2 border-error bg-error/5 pl-2.5 text-error'
                        : isWarn
                        ? 'border-l-2 border-warning bg-warning/5 pl-2.5 text-warning'
                        : isInfo
                        ? 'border-l-2 border-accent/60 pl-2.5 text-text-primary'
                        : 'border-l-2 border-transparent pl-2.5 text-text-muted'
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
