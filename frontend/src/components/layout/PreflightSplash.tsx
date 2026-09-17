import React, { useState, useEffect } from 'react'
import { motion, AnimatePresence } from 'motion/react'
import { CheckCircle2, AlertCircle, RefreshCw, LogIn, ArrowRight, Sparkles, Wrench } from 'lucide-react'
import { LogoMark } from '@/components/brand/Logo'
import { Button } from '@/components/ui/Button'
import { ProgressBar } from '@/components/primitives/ProgressBar'
import { api, getErrorMessage } from '@/lib/ipc'
import { PreflightReport } from '@/lib/types'

export interface PreflightSplashProps {
  onComplete: () => void
  onTriggerLogin: () => void
  onStartService: () => void
}

export const PreflightSplash: React.FC<PreflightSplashProps> = ({
  onComplete,
  onTriggerLogin,
  onStartService,
}) => {
  const [report, setReport] = useState<PreflightReport | null>(null)
  const [currentStepIndex, setCurrentStepIndex] = useState(0)
  const [progress, setProgress] = useState(15)
  const [isDone, setIsDone] = useState(false)
  const [isFixing, setIsFixing] = useState<string | null>(null)
  const [loadError, setLoadError] = useState<string | null>(null)

  const runCheck = async () => {
    setLoadError(null)
    setIsDone(false)
    setReport(null)
    setCurrentStepIndex(0)
    setProgress(20)
    try {
      const res = await api.runPreflightCheck()
      setReport(res)

      // Reveal the real per-step results with a short stagger for readability
      for (let i = 0; i < res.steps.length; i++) {
        setCurrentStepIndex(i)
        setProgress(20 + Math.round(((i + 1) / res.steps.length) * 75))
        await new Promise((r) => setTimeout(r, 220))
      }

      setProgress(100)
      setIsDone(true)

      // If all passed without setup requirements, auto-advance after brief delay
      if (res.all_passed && !res.needs_setup) {
        setTimeout(() => {
          onComplete()
        }, 600)
      }
    } catch (err) {
      setLoadError(getErrorMessage(err))
      setProgress(100)
      setIsDone(true)
    }
  }

  useEffect(() => {
    runCheck()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const handleFixAction = async (action: string) => {
    setIsFixing(action)
    try {
      if (action === 'login') {
        await onTriggerLogin()
      } else if (action === 'start_service') {
        await onStartService()
      }
      // Re-run check after action
      setTimeout(() => {
        runCheck()
        setIsFixing(null)
      }, 1000)
    } catch (err) {
      console.error(err)
      setIsFixing(null)
    }
  }

  return (
    <div className="fixed inset-0 z-50 bg-bg-base flex flex-col items-center justify-center p-6 select-none font-sans">
      <motion.div
        initial={{ opacity: 0, scale: 0.95, y: 10 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        transition={{ duration: 0.3, ease: [0.16, 1, 0.3, 1] }}
        className="w-full max-w-lg space-y-6"
      >
        {/* Brand Header */}
        <div className="text-center space-y-3">
          <motion.div
            animate={{ scale: [1, 1.05, 1] }}
            transition={{ repeat: Infinity, duration: 3, ease: 'easeInOut' }}
            className="flex items-center justify-center mx-auto"
          >
            <LogoMark size="xl" />
          </motion.div>
          <div className="space-y-1">
            <h1 className="text-xl font-bold text-text-primary tracking-tight">
              <span className="text-accent font-extrabold">502</span>
              <span>Drive</span>
            </h1>
            <p className="text-xs text-text-secondary">
              Khởi tạo môi trường & tự động kiểm tra hệ thống
            </p>
          </div>
        </div>

        {/* Progress bar */}
        <div className="space-y-1.5 px-1">
          <div className="flex items-center justify-between text-[11px] text-text-muted">
            <span>
              {isDone
                ? report?.needs_setup
                  ? 'Cần hoàn thiện thiết lập ban đầu'
                  : 'Hệ thống đã sẵn sàng'
                : 'Đang kiểm tra các mục cấu hình...'}
            </span>
            <span className="font-mono">{progress}%</span>
          </div>
          <ProgressBar progress={progress} isRunning={!isDone} />
        </div>

        {/* Checklist Box */}
        {loadError ? (
          <div
            role="alert"
            className="rounded-xl bg-error/10 border border-error/30 shadow-card p-4 space-y-3"
          >
            <div className="flex items-start gap-2.5 text-xs">
              <AlertCircle className="h-4 w-4 text-error shrink-0 mt-0.5" />
              <div className="space-y-1">
                <p className="font-semibold text-text-primary">
                  Không thể kiểm tra hệ thống
                </p>
                <p className="text-text-secondary break-words font-mono">
                  {loadError}
                </p>
              </div>
            </div>
            <Button
              size="sm"
              variant="secondary"
              onClick={runCheck}
              className="text-xs gap-1.5 w-full"
            >
              <RefreshCw className="h-3.5 w-3.5" />
              <span>Thử lại</span>
            </Button>
          </div>
        ) : (
        <div className="rounded-xl bg-bg-card border border-border/80 shadow-card p-4 space-y-2.5 max-h-[260px] overflow-y-auto">
          {report?.steps.map((step, idx) => {
            const isCurrent = idx === currentStepIndex && !isDone
            const isPassed = step.status === 'passed'
            const isWarning = step.status === 'warning'
            const isFailed = step.status === 'failed'
            const fixable = isDone && (isWarning || isFailed) && step.fix_action

            return (
              <motion.div
                key={step.id}
                initial={{ opacity: 0, x: -6 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: idx * 0.05 }}
                className="flex items-start justify-between gap-3 text-xs"
              >
                <div className="flex items-start gap-2.5 min-w-0">
                  <div className="pt-0.5 shrink-0">
                    {isCurrent ? (
                      <RefreshCw className="h-3.5 w-3.5 text-accent animate-spin" />
                    ) : isPassed ? (
                      <CheckCircle2 className="h-3.5 w-3.5 text-accent" />
                    ) : isWarning ? (
                      <AlertCircle className="h-3.5 w-3.5 text-warning" />
                    ) : isFailed ? (
                      <AlertCircle className="h-3.5 w-3.5 text-error" />
                    ) : (
                      <div className="h-3.5 w-3.5 rounded-full border border-border" />
                    )}
                  </div>
                  <div className="space-y-0.5 truncate">
                    <p className="font-medium text-text-primary truncate">{step.title}</p>
                    <p className="text-[0.6875rem] text-text-muted truncate">{step.message}</p>
                  </div>
                </div>

                {fixable && (step.fix_action === 'login' || step.fix_action === 'start_service') && (
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => handleFixAction(step.fix_action as string)}
                    disabled={isFixing !== null}
                    className="text-[0.6875rem] h-6 px-2 gap-1 rounded-lg shrink-0"
                  >
                    {isFixing === step.fix_action ? (
                      <RefreshCw className="h-3 w-3 animate-spin" />
                    ) : (
                      <Wrench className="h-3 w-3" />
                    )}
                    <span>Sửa</span>
                  </Button>
                )}

                {step.auto_fixed && (
                  <span className="text-[0.625rem] px-1.5 py-0.5 rounded bg-accent/10 text-accent font-medium shrink-0">
                    Tự động tạo
                  </span>
                )}
              </motion.div>
            )
          })}
        </div>
        )}

        {/* Setup Resolution Action Card (If needed) */}
        <AnimatePresence>
          {isDone && report?.needs_setup && (
            <motion.div
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              className="p-4 rounded-2xl bg-accent/10 border border-accent/25 flex flex-col sm:flex-row sm:items-center justify-between gap-4 shadow-sm"
            >
              <div className="flex items-start gap-3 min-w-0 flex-1">
                <div className="p-2 rounded-xl bg-accent/15 text-accent shrink-0 mt-0.5">
                  <Sparkles className="h-4 w-4" />
                </div>
                <div className="space-y-0.5 min-w-0">
                  <p className="text-xs font-bold text-text-primary">Thiết lập kết nối</p>
                  <p className="text-[11px] text-text-secondary leading-relaxed">
                    Đăng nhập tài khoản Google Drive để bắt đầu sao chép
                  </p>
                </div>
              </div>

              <div className="flex items-center gap-2.5 shrink-0 self-end sm:self-center">
                <Button
                  size="sm"
                  variant="primary"
                  onClick={() => handleFixAction('login')}
                  disabled={isFixing === 'login'}
                  className="text-xs gap-2 px-4 py-2 font-semibold rounded-xl bg-accent hover:bg-accent-hover text-white shadow-xs"
                >
                  <LogIn className="h-3.5 w-3.5" />
                  <span>Đăng nhập</span>
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={onComplete}
                  className="text-xs gap-1.5 px-3 py-2 rounded-xl"
                >
                  <span>Bỏ qua</span>
                  <ArrowRight className="h-3 w-3" />
                </Button>
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Continue Button if done without setup prompt */}
        {isDone && !loadError && !report?.needs_setup && (
          <div className="text-center pt-1">
            <Button
              size="md"
              variant="primary"
              onClick={onComplete}
              className="w-full text-xs gap-1.5 font-medium"
            >
              <span>Vào ứng dụng ngay</span>
              <ArrowRight className="h-3.5 w-3.5" />
            </Button>
          </div>
        )}
      </motion.div>
    </div>
  )
}
