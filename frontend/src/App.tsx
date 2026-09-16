import React, { useState, useEffect, useCallback } from 'react'
import { Shell } from '@/components/layout/Shell'
import { TabId } from '@/components/layout/Sidebar'
import { Dashboard } from '@/pages/Dashboard'
import { Jobs } from '@/pages/Jobs'
import { Settings } from '@/pages/Settings'
import { Dev } from '@/pages/Dev'
import { PreflightSplash } from '@/components/layout/PreflightSplash'
import { SetupWizardModal } from '@/components/setup/SetupWizardModal'
import { ToastProvider, useToast } from '@/components/primitives/Toast'
import { ThemeProvider } from '@/hooks/useTheme'
import { api, getErrorMessage } from '@/lib/ipc'
import { SystemStatus, JobSummary, WatchSummary, ConfigSummary } from '@/lib/types'

const AppContent: React.FC = () => {
  const [isPreflightDone, setIsPreflightDone] = useState(false)
  const [activeTab, setActiveTab] = useState<TabId>('dashboard')
  const [status, setStatus] = useState<SystemStatus | null>(null)
  const [jobs, setJobs] = useState<JobSummary[]>([])
  const [watches, setWatches] = useState<WatchSummary[]>([])
  const [config, setConfig] = useState<ConfigSummary | null>(null)
  const [isRefreshing, setIsRefreshing] = useState(false)
  const [isWizardOpen, setIsWizardOpen] = useState(false)

  const { toast } = useToast()

  const refreshData = useCallback(async () => {
    setIsRefreshing(true)
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
    } catch (err) {
      console.error('Lỗi tải dữ liệu 502Drive:', err)
    } finally {
      setIsRefreshing(false)
    }
  }, [])

  useEffect(() => {
    refreshData()
    const interval = setInterval(() => {
      // Pause background polling while the wizard owns the screen.
      if (!isWizardOpen) refreshData()
    }, 3000)
    return () => clearInterval(interval)
  }, [refreshData, isWizardOpen])

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
