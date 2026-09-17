import React, { useState, useEffect, useCallback } from 'react'
import { Shell } from '@/components/layout/Shell'
import { TabId } from '@/components/layout/Sidebar'
import { Dashboard } from '@/pages/Dashboard'
import { Jobs } from '@/pages/Jobs'
import { Settings } from '@/pages/Settings'
import { Dev } from '@/pages/Dev'
import { PreflightSplash } from '@/components/layout/PreflightSplash'
import { SetupWizardModal } from '@/components/setup/SetupWizardModal'
import { QuickCloneModal } from '@/components/modals/QuickCloneModal'
import { CreateWatchModal } from '@/components/modals/CreateWatchModal'
import { ToastProvider, useToast } from '@/components/primitives/Toast'
import { ThemeProvider } from '@/hooks/useTheme'
import { api, getErrorMessage } from '@/lib/ipc'
import { SystemStatus, JobSummary, WatchSummary, ConfigSummary, WatchPolicyKind } from '@/lib/types'

const AppContent: React.FC = () => {
  const [isPreflightDone, setIsPreflightDone] = useState(false)
  const [activeTab, setActiveTab] = useState<TabId>('dashboard')
  const [status, setStatus] = useState<SystemStatus | null>(null)
  const [jobs, setJobs] = useState<JobSummary[]>([])
  const [watches, setWatches] = useState<WatchSummary[]>([])
  const [config, setConfig] = useState<ConfigSummary | null>(null)
  const [isRefreshing, setIsRefreshing] = useState(false)
  const [isWizardOpen, setIsWizardOpen] = useState(false)
  const [isCloneOpen, setIsCloneOpen] = useState(false)
  const [isWatchOpen, setIsWatchOpen] = useState(false)
  // Optimistic language state — immediately reflects user toggle without waiting for round-trip
  const [currentLang, setCurrentLang] = useState<string>('vi')

  const { toast } = useToast()

  const refreshData = useCallback(async (silent = false) => {
    if (!silent) setIsRefreshing(true)
    try {
      const [newStatus, newJobs, newWatches, newConfig] = await Promise.all([
        api.getSystemStatus(),
        api.listJobs(30),
        api.listWatches(),
        api.getConfig(),
      ])
      setStatus(newStatus)
      setJobs(newJobs)
      setWatches(newWatches)
      setConfig(newConfig)
      // Sync language state when config refreshes (avoid stomping an in-flight optimistic update)
      if (newConfig?.language) {
        setCurrentLang(newConfig.language)
      }
    } catch (err) {
      console.error('Lỗi tải dữ liệu 502Drive:', err)
    } finally {
      if (!silent) setIsRefreshing(false)
    }
  }, [])

  useEffect(() => {
    refreshData()
    const interval = setInterval(() => {
      // Background polling is silent — no spinner flicker every 3 seconds.
      if (!isWizardOpen) refreshData(true)
    }, 3000)
    return () => clearInterval(interval)
  }, [refreshData, isWizardOpen])

  // Sync currentLang from config on initial load
  useEffect(() => {
    if (config?.language) {
      setCurrentLang(config.language)
    }
  }, [config?.language])

  const handleChangeLang = useCallback(async (lang: string) => {
    // Optimistic: update UI immediately before backend round-trip
    setCurrentLang(lang)
    try {
      await api.updateConfig('language', lang)
      toast({
        title: lang === 'vi' ? 'Đã chuyển sang Tiếng Việt' : 'Switched to English',
        variant: 'success',
        duration: 1500,
      })
      await refreshData(true)
    } catch (err) {
      // Revert on error
      setCurrentLang(config?.language || 'vi')
      toast({ title: 'Lỗi đổi ngôn ngữ', description: getErrorMessage(err), variant: 'error' })
    }
  }, [config?.language, refreshData, toast])

  const handleStartService = async () => {
    try {
      await api.startService()
      toast({
        title: 'Đã kích hoạt service',
        description: 'Dịch vụ nền gdclone-bot đã được bật.',
        variant: 'success',
      })
      await refreshData()
    } catch (err) {
      toast({
        title: 'Lỗi kích hoạt service',
        description: getErrorMessage(err),
        variant: 'error',
      })
    }
  }

  const handleRestartService = async () => {
    try {
      await api.restartService()
      toast({
        title: 'Khởi động lại service',
        description: 'Đã gửi lệnh systemctl restart gdclone-bot thành công.',
        variant: 'success',
      })
      await refreshData()
    } catch (err) {
      toast({
        title: 'Lỗi khởi động service',
        description: getErrorMessage(err),
        variant: 'error',
      })
    }
  }

  const handleOpenBot = async () => {
    try {
      await api.openTelegramBot()
    } catch (err) {
      toast({
        title: 'Không thể mở Telegram',
        description: getErrorMessage(err),
        variant: 'error',
      })
    }
  }

  const handleTriggerLogin = async () => {
    try {
      await api.triggerLogin()
      toast({
        title: 'Đăng nhập Google OAuth',
        description: 'Vui lòng xác nhận quyền truy cập trên trình duyệt web.',
        variant: 'default',
      })
    } catch (err) {
      toast({
        title: 'Lỗi đăng nhập',
        description: getErrorMessage(err),
        variant: 'error',
      })
    }
  }

  const handleTriggerRevoke = async () => {
    try {
      await api.triggerRevoke()
      toast({
        title: 'Đã ngắt kết nối Google',
        description: 'Token truy cập đã được hủy an toàn.',
        variant: 'warning',
      })
      await refreshData()
    } catch (err) {
      toast({
        title: 'Lỗi ngắt kết nối',
        description: getErrorMessage(err),
        variant: 'error',
      })
    }
  }

  const handlePauseJob = async (jobId: string) => {
    try {
      await api.pauseJob(jobId)
      toast({
        title: 'Tác vụ đã tạm dừng',
        description: `Job ${jobId.slice(0, 8)} đã được dừng an toàn.`,
        variant: 'default',
      })
      await refreshData()
    } catch (err) {
      toast({ title: 'Không thể dừng job', description: getErrorMessage(err), variant: 'error' })
    }
  }

  const handleResumeJob = async (jobId: string) => {
    try {
      await api.resumeJob(jobId)
      toast({
        title: 'Tiếp tục tác vụ',
        description: `Job ${jobId.slice(0, 8)} đang được tiếp tục xử lý.`,
        variant: 'success',
      })
      await refreshData()
    } catch (err) {
      toast({ title: 'Không thể tiếp tục job', description: getErrorMessage(err), variant: 'error' })
    }
  }

  const handleCancelJob = async (jobId: string) => {
    try {
      await api.cancelJob(jobId)
      toast({
        title: 'Đã hủy tác vụ',
        description: `Job ${jobId.slice(0, 8)} đã được hủy.`,
        variant: 'warning',
      })
      await refreshData()
    } catch (err) {
      toast({ title: 'Không thể hủy job', description: getErrorMessage(err), variant: 'error' })
    }
  }

  const handlePauseWatch = async (watchId: string) => {
    try {
      await api.pauseWatch(watchId)
      toast({ title: 'Tạm dừng theo dõi', variant: 'default' })
      await refreshData()
    } catch (err) {
      toast({ title: 'Lỗi', description: getErrorMessage(err), variant: 'error' })
    }
  }

  const handleResumeWatch = async (watchId: string) => {
    try {
      await api.resumeWatch(watchId)
      toast({ title: 'Tiếp tục theo dõi', variant: 'success' })
      await refreshData()
    } catch (err) {
      toast({ title: 'Lỗi', description: getErrorMessage(err), variant: 'error' })
    }
  }

  const handleRetryJob = async (jobId: string) => {
    try {
      const { request_id } = await api.retryJob(jobId)
      toast({
        title: 'Đã gửi yêu cầu thử lại',
        description: 'Đang chờ daemon xác nhận…',
        variant: 'default',
      })
      const result = await api.trackUiRequest(request_id)
      if (result.status === 'accepted') {
        toast({
          title: 'Thử lại đã được chấp nhận',
          description: result.note || `Job ${jobId.slice(0, 8)} đã được lên lịch chạy lại.`,
          variant: 'success',
          duration: 6000,
        })
        await refreshData()
      } else {
        toast({
          title: 'Yêu cầu thử lại bị từ chối',
          description: result.note || 'Daemon từ chối yêu cầu thử lại.',
          variant: 'error',
          duration: 6000,
        })
      }
    } catch (err) {
      toast({ title: 'Lỗi thử lại job', description: getErrorMessage(err), variant: 'error' })
    }
  }

  const handleSetWatchPolicy = async (
    watchId: string,
    policyKind: WatchPolicyKind,
    policyValue: string
  ) => {
    try {
      await api.setWatchPolicy(watchId, policyKind, policyValue)
      toast({
        title: 'Đã cập nhật chính sách',
        description: `Chính sách "${policyKind}" của watch ${watchId.slice(0, 8)} đã được áp dụng.`,
        variant: 'success',
        duration: 3000,
      })
      await refreshData()
    } catch (err) {
      toast({ title: 'Lỗi cập nhật chính sách', description: getErrorMessage(err), variant: 'error' })
    }
  }

  const handleUnwatch = async (watchId: string) => {
    try {
      const stopped = await api.unwatch(watchId)
      if (stopped) {
        toast({
          title: 'Đã ngừng theo dõi',
          description: `Watch ${watchId.slice(0, 8)} đã được dừng. Dữ liệu đã sao chép được giữ nguyên.`,
          variant: 'warning',
        })
      } else {
        toast({
          title: 'Không thể ngừng theo dõi',
          description: 'Watch không tồn tại hoặc đã dừng trước đó.',
          variant: 'error',
        })
      }
      await refreshData()
    } catch (err) {
      toast({ title: 'Lỗi ngừng theo dõi', description: getErrorMessage(err), variant: 'error' })
    }
  }

  const handleUpdateConfig = async (field: string, value: unknown) => {
    try {
      await api.updateConfig(field, value)
      toast({
        title: 'Đã lưu cấu hình',
        description: `Đã cập nhật ${field}.`,
        variant: 'success',
        duration: 2000,
      })
      await refreshData()
    } catch (err) {
      toast({ title: 'Lỗi lưu cấu hình', description: getErrorMessage(err), variant: 'error' })
    }
  }

  const handleCheckRemoteUpdate = async () => {
    try {
      const info = await api.checkRemoteUpdate()
      if (info.update_available) {
        toast({
          title: `Bản cập nhật mới ${info.latest_version}`,
          description: info.changelog,
          variant: 'success',
          duration: 6000,
        })
      } else {
        toast({
          title: 'Đã cập nhật mới nhất',
          description: `Bạn đang sử dụng phiên bản ${info.current_version}. Hệ thống đã đồng bộ.`,
          variant: 'default',
        })
      }
    } catch (err) {
      toast({ title: 'Lỗi kiểm tra cập nhật', description: getErrorMessage(err), variant: 'error' })
    }
  }

  return (
    <>
      {!isPreflightDone ? (
        <PreflightSplash
          onComplete={() => setIsPreflightDone(true)}
          onTriggerLogin={handleTriggerLogin}
          onStartService={handleStartService}
        />
      ) : (
        <Shell
          activeTab={activeTab}
          onSelectTab={setActiveTab}
          status={status}
          isRefreshing={isRefreshing}
          onRefresh={refreshData}
          onRestartService={handleRestartService}
          onOpenBot={handleOpenBot}
          onTriggerLogin={handleTriggerLogin}
          currentLang={currentLang}
          onChangeLang={handleChangeLang}
        >
          {activeTab === 'dashboard' && (
            <Dashboard
              status={status}
              recentJobs={jobs}
              onOpenBot={handleOpenBot}
              onTriggerLogin={handleTriggerLogin}
              onTriggerRevoke={handleTriggerRevoke}
              onRestartService={handleRestartService}
              onPauseJob={handlePauseJob}
              onResumeJob={handleResumeJob}
              onCancelJob={handleCancelJob}
              onNavigateToJobs={() => setActiveTab('jobs')}
              onOpenWizard={() => setIsWizardOpen(true)}
              onOpenQuickClone={() => setIsCloneOpen(true)}
            />
          )}

          {activeTab === 'jobs' && (
            <Jobs
              jobs={jobs}
              watches={watches}
              onPauseJob={handlePauseJob}
              onResumeJob={handleResumeJob}
              onCancelJob={handleCancelJob}
              onPauseWatch={handlePauseWatch}
              onResumeWatch={handleResumeWatch}
              onRetryJob={handleRetryJob}
              onSetWatchPolicy={handleSetWatchPolicy}
              onUnwatch={handleUnwatch}
              onCreateWatch={() => setIsWatchOpen(true)}
              onRefresh={refreshData}
              isRefreshing={isRefreshing}
            />
          )}

          {activeTab === 'settings' && (
            <Settings
              config={config}
              status={status}
              onUpdateConfig={handleUpdateConfig}
              onTriggerLogin={handleTriggerLogin}
              onTriggerRevoke={handleTriggerRevoke}
              onRestartService={handleRestartService}
              onCheckUpdate={handleCheckRemoteUpdate}
              onOpenWizard={() => setIsWizardOpen(true)}
            />
          )}

          {activeTab === 'dev' && (
            <Dev
              onRunDoctor={api.runDoctor}
              onGetLogs={api.getLogs}
            />
          )}
        </Shell>
      )}

      <SetupWizardModal
        isOpen={isWizardOpen}
        onClose={() => setIsWizardOpen(false)}
        config={config}
        status={status}
        onTriggerLogin={handleTriggerLogin}
        onRefreshData={refreshData}
        onRestartService={handleRestartService}
      />

      <QuickCloneModal
        isOpen={isCloneOpen}
        onClose={() => setIsCloneOpen(false)}
        onRefreshData={refreshData}
      />

      <CreateWatchModal
        isOpen={isWatchOpen}
        onClose={() => setIsWatchOpen(false)}
        onRefreshData={refreshData}
      />
    </>
  )
}

export default function App() {
  return (
    <ThemeProvider>
      <ToastProvider>
        <AppContent />
      </ToastProvider>
    </ThemeProvider>
  )
}
