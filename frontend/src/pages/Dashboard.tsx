import React from "react"
import { StatCard } from "@/components/primitives/StatCard"
import { JobCard } from "@/components/primitives/JobCard"
import { Card, CardHeader, CardTitle, CardContent } from "@/components/ui/Card"
import { Skeleton } from "@/components/ui/Skeleton"
import { Badge } from "@/components/ui/Badge"
import { Button } from "@/components/ui/Button"
import {
  HardDrive,
  Send,
  FolderSync,
  Play,
  Files,
  Database,
  ExternalLink,
  RotateCcw,
  Sparkles,
  LogOut,
  LogIn,
  Layers,
  ArrowRight,
  Copy,
} from "lucide-react"
import { SystemStatus, JobSummary } from "@/lib/types"
import { formatBytes } from "@/lib/utils"

export interface DashboardProps {
  status: SystemStatus | null
  recentJobs: JobSummary[]
  onTriggerLogin: () => void
  onTriggerRevoke: () => void
  onOpenBot: () => void
  onRestartService: () => void
  onNavigateToJobs: () => void
  onPauseJob?: (id: string) => void
  onResumeJob?: (id: string) => void
  onCancelJob?: (id: string) => void
  onOpenWizard?: () => void
  onOpenQuickClone?: () => void
}

export const Dashboard: React.FC<DashboardProps> = ({
  status,
  recentJobs,
  onTriggerLogin,
  onTriggerRevoke,
  onOpenBot,
  onRestartService,
  onNavigateToJobs,
  onPauseJob,
  onResumeJob,
  onCancelJob,
  onOpenWizard,
  onOpenQuickClone,
}) => {
  const isAccountConnected = status?.account_status === "connected"
  const isServiceActive = status?.service_active ?? false
  const isLoading = status === null
  const jobsToDisplay = recentJobs.slice(0, 4)

  return (
    <div className="space-y-6 lg:space-y-8 w-full pb-12">
      {/* ── Banner: Setup Wizard thông minh cho người dùng mới ── */}
      {!isAccountConnected && (
        <div className="p-5 sm:p-6 rounded-2xl bg-accent/10 border border-accent/25 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-5 shadow-xs">
          <div className="flex items-start gap-3.5">
            <div className="p-2.5 rounded-xl bg-accent text-white shadow-sm shrink-0 mt-0.5">
              <Sparkles className="h-6 w-6" />
            </div>
            <div className="space-y-1">
              <h3 className="text-base sm:text-lg font-bold text-text-primary">
                Bắt đầu cấu hình 502Drive trong 3 phút
              </h3>
              <p className="text-xs sm:text-sm text-text-secondary leading-relaxed">
                Chưa kết nối tài khoản Google Drive. Mở Trình hướng dẫn thiết lập từng bước để điền thông số và load xác nhận tức thì.
              </p>
            </div>
          </div>
          <Button
            size="sm"
            variant="primary"
            onClick={onOpenWizard}
            className="text-xs gap-1.5 rounded-xl shrink-0 font-semibold shadow-xs"
          >
            <Sparkles className="h-3.5 w-3.5" />
            <span>Mở Setup Wizard</span>
            <ArrowRight className="h-3.5 w-3.5" />
          </Button>
        </div>
      )}
      {/* ── Quick action: Sao chép nhanh (khi tài khoản đã kết nối) ── */}
      {isAccountConnected && onOpenQuickClone && (
        <div className="p-5 sm:p-6 rounded-2xl bg-accent/10 border border-accent/25 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-5 shadow-xs">
          <div className="flex items-start gap-3.5">
            <div className="p-2.5 rounded-xl bg-accent text-white shadow-sm shrink-0 mt-0.5">
              <Copy className="h-6 w-6" />
            </div>
            <div className="space-y-1">
              <h3 className="text-base sm:text-lg font-bold text-text-primary">
                Sao chép nhanh thư mục Drive
              </h3>
              <p className="text-xs sm:text-sm text-text-secondary leading-relaxed">
                Dán liên kết thư mục Google Drive để gửi yêu cầu sao chép ngay lập tức — daemon sẽ xác nhận và bắt đầu clone trong nền.
              </p>
            </div>
          </div>
          <Button
            size="sm"
            variant="primary"
            onClick={onOpenQuickClone}
            className="text-xs gap-1.5 rounded-xl shrink-0 font-semibold shadow-xs"
          >
            <Copy className="h-3.5 w-3.5" />
            <span>Sao chép nhanh</span>
            <ArrowRight className="h-3.5 w-3.5" />
          </Button>
        </div>
      )}
      {/* ── Section: Số liệu tổng quan (Apple 4-Column Adaptive Grid) ── */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3.5 sm:gap-4">
        {isLoading
          ? Array.from({ length: 4 }).map((_, i) => (
              <div
                key={i}
                className="p-4 sm:p-5 rounded-2xl bg-bg-card border border-border/70 shadow-xs space-y-3"
              >
                <Skeleton className="h-9 w-9 rounded-xl" />
                <Skeleton className="h-6 w-16" />
                <Skeleton className="h-3 w-24" />
              </div>
            ))
          : (
        <>
        <StatCard
          label="Tác vụ đang chạy"
          value={status?.stats?.active_jobs ?? 0}
          subtext={`/ ${status?.stats?.total_jobs ?? 0} tổng`}
          icon={Play}
        />
        <StatCard
          label="Tệp đã sao chép"
          value={status?.stats?.total_cloned_files ?? 0}
          icon={Files}
        />
        <StatCard
          label="Dung lượng đã sao chép"
          value={formatBytes(status?.stats?.total_cloned_bytes ?? 0)}
          icon={HardDrive}
        />
        <StatCard
          label="Toàn vẹn cơ sở dữ liệu"
          value={status?.db_integrity === "ok" ? "Bình thường" : "Cần kiểm tra"}
          icon={Database}
        />
        </>
          )}
      </div>

      {/* ── Section: Tài khoản & Kết nối (3-Column Balanced Grid) ── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between px-1">
          <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
            Tài khoản & Kết nối
          </h2>
        </div>

        {isLoading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3.5 sm:gap-4">
            {Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="p-4 rounded-2xl bg-bg-card border border-border/70 shadow-xs space-y-3">
                <div className="flex items-center justify-between">
                  <Skeleton className="h-7 w-28 rounded-lg" />
                  <Skeleton className="h-5 w-20 rounded-full" />
                </div>
                <Skeleton className="h-14 w-full rounded-xl" />
                <Skeleton className="h-8 w-full rounded-xl" />
              </div>
            ))}
          </div>
        ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3.5 sm:gap-4">
          {/* Card 1: Google Account */}
          <Card className="flex flex-col justify-between rounded-2xl shadow-xs border border-border/70 p-4">
            <CardHeader className="p-0 pb-3">
              <div className="flex items-center justify-between">
                <CardTitle className="text-sm font-semibold flex items-center gap-2">
                  <div className="p-1.5 rounded-lg bg-accent/10 text-accent">
                    <HardDrive className="h-4 w-4" />
                  </div>
                  <span>Google Drive</span>
                </CardTitle>
                <Badge variant={isAccountConnected ? "connected" : "disconnected"}>
                  {isAccountConnected ? "Đã kết nối" : "Chưa kết nối"}
                </Badge>
              </div>
            </CardHeader>
            <CardContent className="p-0 space-y-3">
              <div className="p-3 rounded-xl bg-bg-input/60 border border-border/40">
                <p className="text-xs font-mono text-text-primary truncate font-medium">
                  {status?.google_account || "Chưa đăng nhập Google"}
                </p>
                <p className="text-[0.6875rem] text-text-muted mt-0.5">
                  {isAccountConnected ? "OAuth token hoạt động ổn định" : "Cần liên kết để sao chép dữ liệu"}
                </p>
              </div>

              <div className="flex items-center gap-2 pt-1">
                {isAccountConnected ? (
                  <>
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={onTriggerLogin}
                      className="flex-1 text-xs rounded-xl"
                    >
                      Đổi tài khoản
                    </Button>
                    <Button
                      size="sm"
                      variant="danger"
                      onClick={onTriggerRevoke}
                      className="h-8 px-2.5 text-xs gap-1 rounded-xl"
                      title="Ngắt kết nối tài khoản"
                    >
                      <LogOut className="h-3.5 w-3.5" />
                      <span>Ngắt</span>
                    </Button>
                  </>
                ) : (
                  <Button
                    size="sm"
                    variant="primary"
                    onClick={onTriggerLogin}
                    className="w-full text-xs gap-1.5 rounded-xl"
                  >
                    <LogIn className="h-3.5 w-3.5" />
                    Đăng nhập Google
                  </Button>
                )}
              </div>
            </CardContent>
          </Card>

          {/* Card 2: Telegram Bot */}
          <Card className="flex flex-col justify-between rounded-2xl shadow-xs border border-border/70 p-4">
            <CardHeader className="p-0 pb-3">
              <div className="flex items-center justify-between">
                <CardTitle className="text-sm font-semibold flex items-center gap-2">
                  <div className="p-1.5 rounded-lg bg-accent/10 text-accent">
                    <Send className="h-4 w-4" />
                  </div>
                  <span>Telegram Bot</span>
                </CardTitle>
                <Badge variant={isServiceActive ? "active" : "paused"} pulse={isServiceActive}>
                  {isServiceActive ? "Đang chạy" : "Đã dừng"}
                </Badge>
              </div>
            </CardHeader>
            <CardContent className="p-0 space-y-3">
              <div className="p-3 rounded-xl bg-bg-input/60 border border-border/40">
                <p className="text-xs font-mono text-text-primary truncate font-medium">
                  @{status?.bot_username || "Drive502_Bot"}
                </p>
                <p className="text-[0.6875rem] text-text-muted mt-0.5">
                  {isServiceActive ? "Long polling đang sẵn sàng nhận lệnh" : "Service chạy nền chưa khởi động"}
                </p>
              </div>

              <div className="flex items-center gap-2 pt-1">
                <Button
                  size="sm"
                  variant="primary"
                  onClick={onOpenBot}
                  className="flex-1 text-xs gap-1.5 rounded-xl"
                >
                  <ExternalLink className="h-3.5 w-3.5" />
                  Mở Telegram Bot
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={onRestartService}
                  className="h-8 w-8 p-0 rounded-xl"
                  title="Khởi động lại service"
                >
                  <RotateCcw className="h-3.5 w-3.5" />
                </Button>
              </div>
            </CardContent>
          </Card>

          {/* Card 3: Destination Folder */}
          <Card className="flex flex-col justify-between rounded-2xl shadow-xs border border-border/70 p-4">
            <CardHeader className="p-0 pb-3">
              <div className="flex items-center justify-between">
                <CardTitle className="text-sm font-semibold flex items-center gap-2">
                  <div className="p-1.5 rounded-lg bg-accent/10 text-accent">
                    <FolderSync className="h-4 w-4" />
                  </div>
                  <span>Thư mục đích</span>
                </CardTitle>
                <Badge variant="default">Mặc định</Badge>
              </div>
            </CardHeader>
            <CardContent className="p-0 space-y-3">
              <div className="p-3 rounded-xl bg-bg-input/60 border border-border/40">
                <p className="text-xs font-medium text-text-primary truncate">
                  {status?.destination_label || "My Drive / Backup 502"}
                </p>
                <p className="text-[0.625rem] font-mono text-text-muted mt-0.5 truncate">
                  ID: {status?.destination_id || "1aBcDeFgHiJkLmNoPqRsTuVwXyZ01234"}
                </p>
              </div>

              <div className="flex items-center justify-between text-xs text-text-secondary pt-1">
                <span className="text-[0.6875rem] text-text-muted">Vị trí lưu trữ dữ liệu nhân bản từ bot</span>
              </div>
            </CardContent>
          </Card>
        </div>
        )}
      </div>

      {/* ── Section: Tiến trình công việc gần đây ─────────── */}
      <div className="space-y-3">
        <div className="flex items-center justify-between px-1">
          <div className="flex items-center gap-2">
            <Layers className="h-4 w-4 text-accent" />
            <h2 className="text-xs font-semibold text-text-secondary uppercase tracking-wider">
              Tiến trình công việc gần đây
            </h2>
          </div>
          <Button
            size="sm"
            variant="ghost"
            onClick={onNavigateToJobs}
            className="text-xs text-text-secondary hover:text-text-primary h-7 px-2.5 rounded-lg gap-1"
          >
            <span>Tất cả tác vụ</span>
            <ArrowRight className="h-3 w-3" />
          </Button>
        </div>

        {isLoading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3.5">
            {Array.from({ length: 2 }).map((_, i) => (
              <div key={i} className="p-4 rounded-2xl bg-bg-card border border-border/70 shadow-xs space-y-3">
                <Skeleton className="h-5 w-40 rounded-lg" />
                <Skeleton className="h-2.5 w-full rounded-full" />
                <Skeleton className="h-3 w-2/3" />
              </div>
            ))}
          </div>
        ) : jobsToDisplay.length === 0 ? (
          <Card className="p-8 text-center space-y-2 rounded-2xl border border-border/70 shadow-xs">
            <div className="w-11 h-11 rounded-2xl bg-accent/10 text-accent flex items-center justify-center mx-auto shadow-xs">
              <Sparkles className="h-5 w-5 stroke-[1.75]" />
            </div>
            <p className="text-xs font-semibold text-text-primary">Chưa có tác vụ sao chép nào</p>
            <p className="text-[0.6875rem] text-text-muted max-w-sm mx-auto leading-relaxed">
              Gửi liên kết thư mục Google Drive vào bot Telegram hoặc dùng phím tắt ⌘K để tạo lệnh clone nhanh.
            </p>
          </Card>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3.5">
            {jobsToDisplay.map((job) => (
              <JobCard
                key={job.id}
                job={job}
                onPause={onPauseJob}
                onResume={onResumeJob}
                onCancel={onCancelJob}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
