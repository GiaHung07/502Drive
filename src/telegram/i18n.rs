#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UiLanguage {
    Vi,
    En,
}

impl UiLanguage {
    pub fn from_code(code: &str) -> Self {
        match code {
            "en" => Self::En,
            _ => Self::Vi,
        }
    }

    pub fn text(self, key: TextKey) -> &'static str {
        key.text(self)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TextKey {
    Account,
    BackHome,
    BadgeDefault,
    BadgeSharedDrive,
    BrowseMyDrive,
    BrowseSharedDrive,
    ButtonSetDefault,
    Cancel,
    Clone,
    CloneAgain,
    CloneCancelledTitle,
    CloneCompletedTitle,
    CloneHere,
    CloneIncompleteTitle,
    CloneNow,
    CloneToDestination,
    ConfirmCancelJobBody,
    ConfirmCancelJobTitle,
    ConfirmUnwatchBody,
    ConfirmUnwatchTitle,
    ConfirmSetDestinationPrompt,
    CommandAccount,
    CommandCancel,
    CommandClearDestination,
    CommandClone,
    CommandCloneHere,
    CommandConnect,
    CommandDestination,
    CommandDisconnect,
    CommandHelp,
    CommandJobs,
    CommandLastReport,
    CommandMenu,
    CommandPause,
    CommandPreview,
    CommandResume,
    CommandRetry,
    CommandSetDestination,
    CommandStart,
    CommandStatus,
    CommandSync,
    CommandUnwatch,
    CommandWatch,
    CommandWatchFilter,
    CommandWatchPause,
    CommandWatchPolicy,
    CommandWatchResume,
    CommandWatchStatus,
    CommandWatches,
    CommandWhoami,
    Destination,
    HelpText,
    HomeBotLabel,
    HomeBotReady,
    HomeDefaultDestination,
    HomeDriveLabel,
    HomeHint,
    HomeJobsRunning,
    HomeNeedsAttention,
    HomeNoDestination,
    HomeStatusConnected,
    HomeStatusNotConnected,
    HomeStatusReconnect,
    HomeWatching,
    Jobs,
    KeepJob,
    KeepWatch,
    Manual,
    MenuClone,
    MenuJobs,
    MenuSettings,
    MenuWatch,
    NewWatch,
    NextPage,
    Pause,
    PreviousPage,
    PreviewDestinationMissing,
    PreviewFooter,
    PreviewNoRunning,
    PreviewRunningHeader,
    PreviewTitle,
    PreviewTotalJobs,
    PromptClone,
    PromptCloneHere,
    PromptMarkerCancel,
    PromptMarkerClone,
    PromptMarkerCloneHere,
    PromptMarkerGrant,
    PromptMarkerPause,
    PromptMarkerResume,
    PromptMarkerRetry,
    PromptMarkerRevoke,
    PromptMarkerSetDestination,
    PromptMarkerStatus,
    PromptMarkerUnwatch,
    PromptMarkerWatch,
    PromptMarkerWatchPause,
    PromptMarkerWatchPolicy,
    PromptMarkerWatchResume,
    PromptMarkerWatchStatus,
    PromptPlaceholderDestinationFolder,
    PromptPlaceholderDriveSource,
    PromptPlaceholderJobId,
    PromptPlaceholderTelegramUserId,
    PromptPlaceholderWatchId,
    PromptPlaceholderWatchLinks,
    PromptPlaceholderWatchPolicy,
    PromptSetDestination,
    PromptWatch,
    PromptWatchPolicy,
    Refresh,
    Replace,
    Report,
    ReportCompleted,
    ReportFailed,
    ReportHintClean,
    ReportHintRetry,
    ReportHintSkipped,
    ReportJobTitle,
    ReportLatestTitle,
    ReportScanned,
    ReportSkipped,
    ReportStatus,
    Resume,
    Retry,
    RetryFailed,
    SelectThisFolder,
    StopWatching,
    UpOneLevel,
    Versioned,
    ViewErrors,
    ViewReport,
    Watches,
    ConfirmWatchTitle,
    WatchDirectionOneWay,
    StartWatchingButton,
    WatchOptionsButton,
    WatchOptionsTitle,
    WatchOptionsHelp,
    WatchCreatedTitle,
    WatchCreatedSummary,
    WatchLastSynced,
    WatchMappedFiles,
    WatchConflictTitle,
    WatchConflictCurrent,
    WatchConflictNew,
    WatchConflictRemaining,
    WatchActionNewVersion,
    WatchActionReplace,
    WatchActionSkip,
    WatchResolveConflictButton,
}

impl TextKey {
    pub const ALL: &'static [Self] = &[
        Self::Account,
        Self::BackHome,
        Self::BadgeDefault,
        Self::BadgeSharedDrive,
        Self::BrowseMyDrive,
        Self::BrowseSharedDrive,
        Self::ButtonSetDefault,
        Self::Cancel,
        Self::Clone,
        Self::CloneAgain,
        Self::CloneCancelledTitle,
        Self::CloneCompletedTitle,
        Self::CloneHere,
        Self::CloneIncompleteTitle,
        Self::CloneNow,
        Self::CloneToDestination,
        Self::ConfirmCancelJobBody,
        Self::ConfirmCancelJobTitle,
        Self::ConfirmUnwatchBody,
        Self::ConfirmUnwatchTitle,
        Self::ConfirmSetDestinationPrompt,
        Self::CommandAccount,
        Self::CommandCancel,
        Self::CommandClearDestination,
        Self::CommandClone,
        Self::CommandCloneHere,
        Self::CommandConnect,
        Self::CommandDestination,
        Self::CommandDisconnect,
        Self::CommandHelp,
        Self::CommandJobs,
        Self::CommandLastReport,
        Self::CommandMenu,
        Self::CommandPause,
        Self::CommandPreview,
        Self::CommandResume,
        Self::CommandRetry,
        Self::CommandSetDestination,
        Self::CommandStart,
        Self::CommandStatus,
        Self::CommandSync,
        Self::CommandUnwatch,
        Self::CommandWatch,
        Self::CommandWatchFilter,
        Self::CommandWatchPause,
        Self::CommandWatchPolicy,
        Self::CommandWatchResume,
        Self::CommandWatchStatus,
        Self::CommandWatches,
        Self::CommandWhoami,
        Self::Destination,
        Self::HelpText,
        Self::HomeBotLabel,
        Self::HomeBotReady,
        Self::HomeDefaultDestination,
        Self::HomeDriveLabel,
        Self::HomeHint,
        Self::HomeJobsRunning,
        Self::HomeNeedsAttention,
        Self::HomeNoDestination,
        Self::HomeStatusConnected,
        Self::HomeStatusNotConnected,
        Self::HomeStatusReconnect,
        Self::HomeWatching,
        Self::Jobs,
        Self::KeepJob,
        Self::KeepWatch,
        Self::Manual,
        Self::MenuClone,
        Self::MenuJobs,
        Self::MenuSettings,
        Self::MenuWatch,
        Self::NewWatch,
        Self::NextPage,
        Self::Pause,
        Self::PreviousPage,
        Self::PreviewDestinationMissing,
        Self::PreviewFooter,
        Self::PreviewNoRunning,
        Self::PreviewRunningHeader,
        Self::PreviewTitle,
        Self::PreviewTotalJobs,
        Self::PromptClone,
        Self::PromptCloneHere,
        Self::PromptMarkerCancel,
        Self::PromptMarkerClone,
        Self::PromptMarkerCloneHere,
        Self::PromptMarkerGrant,
        Self::PromptMarkerPause,
        Self::PromptMarkerResume,
        Self::PromptMarkerRetry,
        Self::PromptMarkerRevoke,
        Self::PromptMarkerSetDestination,
        Self::PromptMarkerStatus,
        Self::PromptMarkerUnwatch,
        Self::PromptMarkerWatch,
        Self::PromptMarkerWatchPause,
        Self::PromptMarkerWatchPolicy,
        Self::PromptMarkerWatchResume,
        Self::PromptMarkerWatchStatus,
        Self::PromptPlaceholderDestinationFolder,
        Self::PromptPlaceholderDriveSource,
        Self::PromptPlaceholderJobId,
        Self::PromptPlaceholderTelegramUserId,
        Self::PromptPlaceholderWatchId,
        Self::PromptPlaceholderWatchLinks,
        Self::PromptPlaceholderWatchPolicy,
        Self::PromptSetDestination,
        Self::PromptWatch,
        Self::PromptWatchPolicy,
        Self::Refresh,
        Self::Replace,
        Self::Report,
        Self::ReportCompleted,
        Self::ReportFailed,
        Self::ReportHintClean,
        Self::ReportHintRetry,
        Self::ReportHintSkipped,
        Self::ReportJobTitle,
        Self::ReportLatestTitle,
        Self::ReportScanned,
        Self::ReportSkipped,
        Self::ReportStatus,
        Self::Resume,
        Self::Retry,
        Self::RetryFailed,
        Self::SelectThisFolder,
        Self::StopWatching,
        Self::UpOneLevel,
        Self::Versioned,
        Self::ViewErrors,
        Self::ViewReport,
        Self::Watches,
        Self::ConfirmWatchTitle,
        Self::WatchDirectionOneWay,
        Self::StartWatchingButton,
        Self::WatchOptionsButton,
        Self::WatchOptionsTitle,
        Self::WatchOptionsHelp,
        Self::WatchCreatedTitle,
        Self::WatchCreatedSummary,
        Self::WatchLastSynced,
        Self::WatchMappedFiles,
        Self::WatchConflictTitle,
        Self::WatchConflictCurrent,
        Self::WatchConflictNew,
        Self::WatchConflictRemaining,
        Self::WatchActionNewVersion,
        Self::WatchActionReplace,
        Self::WatchActionSkip,
        Self::WatchResolveConflictButton,
    ];

    fn text(self, lang: UiLanguage) -> &'static str {
        match (lang, self) {
            (UiLanguage::Vi, Self::Account) => "Tài khoản",
            (UiLanguage::En, Self::Account) => "Account",
            (UiLanguage::Vi, Self::BackHome) => "Trang chính",
            (UiLanguage::En, Self::BackHome) => "Home",
            (UiLanguage::Vi, Self::BadgeDefault) => "[mặc định] ",
            (UiLanguage::En, Self::BadgeDefault) => "[default] ",
            (UiLanguage::Vi, Self::BadgeSharedDrive) => "[SD]",
            (UiLanguage::En, Self::BadgeSharedDrive) => "[SD]",
            (UiLanguage::Vi, Self::BrowseMyDrive) => "Duyệt My Drive",
            (UiLanguage::En, Self::BrowseMyDrive) => "Browse My Drive",
            (UiLanguage::Vi, Self::BrowseSharedDrive) => "Duyệt Shared Drive",
            (UiLanguage::En, Self::BrowseSharedDrive) => "Browse Shared Drive",
            (UiLanguage::Vi, Self::ButtonSetDefault) => "✓ Đặt",
            (UiLanguage::En, Self::ButtonSetDefault) => "✓ Set",
            (UiLanguage::Vi, Self::Cancel) => "Huỷ",
            (UiLanguage::En, Self::Cancel) => "Cancel",
            (UiLanguage::Vi, Self::Clone) => "Clone",
            (UiLanguage::En, Self::Clone) => "Clone",
            (UiLanguage::Vi, Self::CloneAgain) => "Sao chép lại",
            (UiLanguage::En, Self::CloneAgain) => "Clone again",
            (UiLanguage::Vi, Self::CloneCancelledTitle) => "⚠ Sao chép đã huỷ",
            (UiLanguage::En, Self::CloneCancelledTitle) => "⚠ Clone cancelled",
            (UiLanguage::Vi, Self::CloneCompletedTitle) => "✓ Đã sao chép",
            (UiLanguage::En, Self::CloneCompletedTitle) => "✓ Clone completed",
            (UiLanguage::Vi, Self::CloneHere) => "Clone vào đích",
            (UiLanguage::En, Self::CloneHere) => "Clone here",
            (UiLanguage::Vi, Self::CloneIncompleteTitle) => "⚠ Sao chép chưa hoàn tất",
            (UiLanguage::En, Self::CloneIncompleteTitle) => "⚠ Clone incomplete",
            (UiLanguage::Vi, Self::CloneNow) => "Clone ngay",
            (UiLanguage::En, Self::CloneNow) => "Clone now",
            (UiLanguage::Vi, Self::CloneToDestination) => "Clone vào đích",
            (UiLanguage::En, Self::CloneToDestination) => "Clone to destination",
            (UiLanguage::Vi, Self::ConfirmCancelJobBody) => {
                "Job đang chạy sẽ dừng ở checkpoint gần nhất. Bạn chắc chắn muốn huỷ?"
            }
            (UiLanguage::En, Self::ConfirmCancelJobBody) => {
                "The running job will stop at the nearest checkpoint. Are you sure?"
            }
            (UiLanguage::Vi, Self::ConfirmCancelJobTitle) => "XÁC NHẬN HUỶ JOB",
            (UiLanguage::En, Self::ConfirmCancelJobTitle) => "CONFIRM JOB CANCEL",
            (UiLanguage::Vi, Self::ConfirmUnwatchBody) => {
                "Bot sẽ ngừng theo dõi thư mục này. Bản copy ở đích vẫn được giữ lại."
            }
            (UiLanguage::En, Self::ConfirmUnwatchBody) => {
                "The bot will stop watching this folder. Destination copies are kept."
            }
            (UiLanguage::Vi, Self::ConfirmUnwatchTitle) => "XÁC NHẬN DỪNG WATCH",
            (UiLanguage::En, Self::ConfirmUnwatchTitle) => "CONFIRM STOP WATCH",
            (UiLanguage::Vi, Self::ConfirmSetDestinationPrompt) => {
                "Đặt đây làm thư mục đích mặc định?"
            }
            (UiLanguage::En, Self::ConfirmSetDestinationPrompt) => {
                "Set this as default destination?"
            }
            (UiLanguage::Vi, Self::CommandAccount) => "Xem tài khoản Google",
            (UiLanguage::En, Self::CommandAccount) => "Show Google account",
            (UiLanguage::Vi, Self::CommandCancel) => "Huỷ job",
            (UiLanguage::En, Self::CommandCancel) => "Cancel job",
            (UiLanguage::Vi, Self::CommandClearDestination) => "Xoá thư mục đích mặc định",
            (UiLanguage::En, Self::CommandClearDestination) => "Clear default destination",
            (UiLanguage::Vi, Self::CommandClone) => "Kiểm tra và clone Drive URL",
            (UiLanguage::En, Self::CommandClone) => "Check and clone Drive URL",
            (UiLanguage::Vi, Self::CommandCloneHere) => "Clone ngay vào đích mặc định",
            (UiLanguage::En, Self::CommandCloneHere) => "Clone to default destination",
            (UiLanguage::Vi, Self::CommandConnect) => {
                "Hướng dẫn đăng nhập Google trên máy chạy bot"
            }
            (UiLanguage::En, Self::CommandConnect) => {
                "Guide to sign in to Google on the bot machine"
            }
            (UiLanguage::Vi, Self::CommandDestination) => "Xem/đổi thư mục đích",
            (UiLanguage::En, Self::CommandDestination) => "View/change destination",
            (UiLanguage::Vi, Self::CommandDisconnect) => "Ngắt kết nối Google Drive",
            (UiLanguage::En, Self::CommandDisconnect) => "Disconnect Google Drive",
            (UiLanguage::Vi, Self::CommandHelp) => "Xem các lệnh chính",
            (UiLanguage::En, Self::CommandHelp) => "Show commands",
            (UiLanguage::Vi, Self::CommandJobs) => "Xem job đang chạy",
            (UiLanguage::En, Self::CommandJobs) => "Show active jobs",
            (UiLanguage::Vi, Self::CommandLastReport) => "Gửi lại report gần nhất",
            (UiLanguage::En, Self::CommandLastReport) => "Send latest report",
            (UiLanguage::Vi, Self::CommandMenu | Self::CommandStart) => "Mở bảng điều khiển",
            (UiLanguage::En, Self::CommandMenu | Self::CommandStart) => "Open control center",
            (UiLanguage::Vi, Self::CommandPause) => "Tạm dừng job",
            (UiLanguage::En, Self::CommandPause) => "Pause job",
            (UiLanguage::Vi, Self::CommandPreview) => "Bảng tổng quan realtime",
            (UiLanguage::En, Self::CommandPreview) => "Realtime overview",
            (UiLanguage::Vi, Self::CommandResume) => "Tiếp tục job",
            (UiLanguage::En, Self::CommandResume) => "Resume job",
            (UiLanguage::Vi, Self::CommandRetry) => "Làm lại item lỗi",
            (UiLanguage::En, Self::CommandRetry) => "Retry failed items",
            (UiLanguage::Vi, Self::CommandSetDestination) => "Đặt thư mục đích mặc định",
            (UiLanguage::En, Self::CommandSetDestination) => "Set default destination",
            (UiLanguage::Vi, Self::CommandStatus) => "Xem chi tiết job",
            (UiLanguage::En, Self::CommandStatus) => "Show job details",
            (UiLanguage::Vi, Self::CommandSync) => {
                "Đồng bộ realtime từ nguồn sang đích: /sync <nguon> [dich]"
            }
            (UiLanguage::En, Self::CommandSync) => {
                "Realtime sync from source to dest: /sync <source> [dest]"
            }
            (UiLanguage::Vi, Self::CommandUnwatch) => "Dừng theo dõi thư mục",
            (UiLanguage::En, Self::CommandUnwatch) => "Stop watching folder",
            (UiLanguage::Vi, Self::CommandWatch) => "Theo dõi thư mục nguồn sang thư mục đích",
            (UiLanguage::En, Self::CommandWatch) => "Watch source folder into destination",
            (UiLanguage::Vi, Self::CommandWatchFilter) => "Quản lý glob loại trừ file của watch",
            (UiLanguage::En, Self::CommandWatchFilter) => "Manage per-watch file exclude globs",
            (UiLanguage::Vi, Self::CommandWatchPause) => "Tạm dừng đồng bộ",
            (UiLanguage::En, Self::CommandWatchPause) => "Pause watch",
            (UiLanguage::Vi, Self::CommandWatchPolicy) => "Đổi cách xử lý khi file nguồn thay đổi",
            (UiLanguage::En, Self::CommandWatchPolicy) => "Change source update policy",
            (UiLanguage::Vi, Self::CommandWatchResume) => "Tiếp tục đồng bộ",
            (UiLanguage::En, Self::CommandWatchResume) => "Resume watch",
            (UiLanguage::Vi, Self::CommandWatchStatus) => "Chi tiết đồng bộ của một watch",
            (UiLanguage::En, Self::CommandWatchStatus) => "Show watch details",
            (UiLanguage::Vi, Self::CommandWatches) => "Danh sách thư mục đang đồng bộ",
            (UiLanguage::En, Self::CommandWatches) => "Show synced folders",
            (UiLanguage::Vi, Self::CommandWhoami) => "Xem Telegram user id và quyền hiện tại",
            (UiLanguage::En, Self::CommandWhoami) => "Show your Telegram user id and role",
            (UiLanguage::Vi, Self::Destination) => "Thư mục đích",
            (UiLanguage::En, Self::Destination) => "Destination",
            (UiLanguage::Vi, Self::HelpText) => {
                "502Drive\n\
                 \n\
                 Bạn có thể:\n\
                 • Dán link Google Drive để sao chép hoặc theo dõi\n\
                 • Mở /menu để điều khiển\n\
                 • Dùng lệnh nhanh nếu muốn\n\
                 \n\
                 Lệnh: /clone /clone_here /sync /watch /watches /jobs /status\n\
                 Đích: /destination /set_destination /clear_destination\n\
                 Tài khoản: /account /connect /disconnect /whoami\n\
                 Nâng cao: /watch_status /watch_pause /watch_resume /watch_policy /watch_filter /unwatch /retry /last_report\n\
                 Quản trị: /grant /revoke (chỉ owner)"
            }
            (UiLanguage::En, Self::HelpText) => {
                "502Drive\n\
                 \n\
                 You can:\n\
                 • Paste a Google Drive link to clone or watch\n\
                 • Open /menu to control the bot\n\
                 • Use quick commands if you prefer\n\
                 \n\
                 Commands: /clone /clone_here /sync /watch /watches /jobs /status\n\
                 Destination: /destination /set_destination /clear_destination\n\
                 Account: /account /connect /disconnect /whoami\n\
                 Advanced: /watch_status /watch_pause /watch_resume /watch_policy /watch_filter /unwatch /retry /last_report\n\
                 Admin: /grant /revoke (owner only)"
            }
            (UiLanguage::Vi, Self::HomeBotLabel) => "Bot",
            (UiLanguage::En, Self::HomeBotLabel) => "Bot",
            (UiLanguage::Vi, Self::HomeBotReady) => "Sẵn sàng",
            (UiLanguage::En, Self::HomeBotReady) => "Ready",
            (UiLanguage::Vi, Self::HomeDefaultDestination) => "Đích mặc định",
            (UiLanguage::En, Self::HomeDefaultDestination) => "Default destination",
            (UiLanguage::Vi, Self::HomeDriveLabel) => "Google Drive",
            (UiLanguage::En, Self::HomeDriveLabel) => "Google Drive",
            (UiLanguage::Vi, Self::HomeHint) => "Gửi link Google Drive để bắt đầu nhanh.",
            (UiLanguage::En, Self::HomeHint) => "Send a Google Drive link to get started quickly.",
            (UiLanguage::Vi, Self::HomeJobsRunning) => "Đang chạy",
            (UiLanguage::En, Self::HomeJobsRunning) => "Running",
            (UiLanguage::Vi, Self::HomeNeedsAttention) => "Cần chú ý",
            (UiLanguage::En, Self::HomeNeedsAttention) => "Needs attention",
            (UiLanguage::Vi, Self::HomeNoDestination) => "—",
            (UiLanguage::En, Self::HomeNoDestination) => "—",
            (UiLanguage::Vi, Self::HomeStatusConnected) => "Đã kết nối",
            (UiLanguage::En, Self::HomeStatusConnected) => "Connected",
            (UiLanguage::Vi, Self::HomeStatusNotConnected) => "Chưa kết nối",
            (UiLanguage::En, Self::HomeStatusNotConnected) => "Not connected",
            (UiLanguage::Vi, Self::HomeStatusReconnect) => "Cần kết nối lại",
            (UiLanguage::En, Self::HomeStatusReconnect) => "Reconnect needed",
            (UiLanguage::Vi, Self::HomeWatching) => "Đang theo dõi",
            (UiLanguage::En, Self::HomeWatching) => "Watching",
            (UiLanguage::Vi, Self::Jobs) => "Jobs",
            (UiLanguage::En, Self::Jobs) => "Jobs",
            (UiLanguage::Vi, Self::KeepJob) => "Giữ job",
            (UiLanguage::En, Self::KeepJob) => "Keep job",
            (UiLanguage::Vi, Self::KeepWatch) => "Giữ watch",
            (UiLanguage::En, Self::KeepWatch) => "Keep watch",
            (UiLanguage::Vi, Self::Manual) => "Xác nhận tay",
            (UiLanguage::En, Self::Manual) => "Manual",
            (UiLanguage::Vi, Self::MenuClone) => "＋ Sao chép",
            (UiLanguage::En, Self::MenuClone) => "＋ Clone",
            (UiLanguage::Vi, Self::MenuJobs) => "Công việc",
            (UiLanguage::En, Self::MenuJobs) => "Jobs",
            (UiLanguage::Vi, Self::MenuSettings) => "Cài đặt",
            (UiLanguage::En, Self::MenuSettings) => "Settings",
            (UiLanguage::Vi, Self::MenuWatch) => "⟳ Theo dõi",
            (UiLanguage::En, Self::MenuWatch) => "⟳ Watch",
            (UiLanguage::Vi, Self::NewWatch) => "Đồng bộ mới",
            (UiLanguage::En, Self::NewWatch) => "New sync",
            (UiLanguage::Vi, Self::NextPage) => "Trang sau",
            (UiLanguage::En, Self::NextPage) => "Next",
            (UiLanguage::Vi, Self::Pause) => "Tạm dừng",
            (UiLanguage::En, Self::Pause) => "Pause",
            (UiLanguage::Vi, Self::PreviousPage) => "Trang trước",
            (UiLanguage::En, Self::PreviousPage) => "Previous",
            (UiLanguage::Vi, Self::PreviewDestinationMissing) => "Chưa đặt - dùng /set_destination",
            (UiLanguage::En, Self::PreviewDestinationMissing) => "Not set - use /set_destination",
            (UiLanguage::Vi, Self::PreviewFooter) => "(Tự cập nhật 30 giây - /preview để mở lại)",
            (UiLanguage::En, Self::PreviewFooter) => {
                "(Auto-updates every 30 seconds - use /preview to reopen)"
            }
            (UiLanguage::Vi, Self::PreviewNoRunning) => "Đang chạy: Không có",
            (UiLanguage::En, Self::PreviewNoRunning) => "Running: None",
            (UiLanguage::Vi, Self::PreviewRunningHeader) => "ĐANG CHẠY",
            (UiLanguage::En, Self::PreviewRunningHeader) => "RUNNING",
            (UiLanguage::Vi, Self::PreviewTitle) => "BẢNG TỔNG QUAN",
            (UiLanguage::En, Self::PreviewTitle) => "OVERVIEW",
            (UiLanguage::Vi, Self::PreviewTotalJobs) => "Tổng job",
            (UiLanguage::En, Self::PreviewTotalJobs) => "Total jobs",
            (UiLanguage::Vi, Self::PromptClone) => {
                "Dán link nguồn Drive vào ô trả lời tin nhắn này.\n\
                 Bot sẽ kiểm tra nguồn và mở màn xác nhận clone.\n\
                 \n\
                 Ví dụ:\n\
                 drive.google.com/drive/folders/NGUON\n\
                 \n\
                 Mẹo: bạn cũng có thể dán thẳng link Drive vào chat, không cần gõ /clone."
            }
            (UiLanguage::En, Self::PromptClone) => {
                "Paste the source Drive link in reply to this message.\n\
                 The bot will check the source and open the clone confirmation.\n\
                 \n\
                 Example:\n\
                 drive.google.com/drive/folders/SOURCE\n\
                 \n\
                 Tip: you can also paste a Drive link directly into chat without typing /clone."
            }
            (UiLanguage::Vi, Self::PromptCloneHere) => {
                "Dán link nguồn Drive vào ô trả lời tin nhắn này.\n\
                 Bot sẽ clone ngay vào thư mục đích mặc định.\n\
                 \n\
                 Ví dụ:\n\
                 drive.google.com/drive/folders/NGUON\n\
                 \n\
                 Chưa có đích mặc định thì dùng /destination hoặc /set_destination trước."
            }
            (UiLanguage::En, Self::PromptCloneHere) => {
                "Paste the source Drive link in reply to this message.\n\
                 The bot will clone it to the default destination folder.\n\
                 \n\
                 Example:\n\
                 drive.google.com/drive/folders/SOURCE\n\
                 \n\
                 If no default destination exists, use /destination or /set_destination first."
            }
            (UiLanguage::Vi, Self::PromptMarkerCancel) => "Lệnh: /cancel <job_id>",
            (UiLanguage::En, Self::PromptMarkerCancel) => "Command: /cancel <job_id>",
            (UiLanguage::Vi, Self::PromptMarkerClone) => {
                "Bot sẽ kiểm tra nguồn và mở màn xác nhận clone."
            }
            (UiLanguage::En, Self::PromptMarkerClone) => {
                "The bot will check the source and open the clone confirmation."
            }
            (UiLanguage::Vi, Self::PromptMarkerCloneHere) => {
                "Bot sẽ clone ngay vào thư mục đích mặc định."
            }
            (UiLanguage::En, Self::PromptMarkerCloneHere) => {
                "The bot will clone to the default destination folder."
            }
            (UiLanguage::Vi, Self::PromptMarkerGrant) => "Lệnh: /grant <telegram_user_id>",
            (UiLanguage::En, Self::PromptMarkerGrant) => "Command: /grant <telegram_user_id>",
            (UiLanguage::Vi, Self::PromptMarkerPause) => "Lệnh: /pause <job_id>",
            (UiLanguage::En, Self::PromptMarkerPause) => "Command: /pause <job_id>",
            (UiLanguage::Vi, Self::PromptMarkerResume) => "Lệnh: /resume <job_id>",
            (UiLanguage::En, Self::PromptMarkerResume) => "Command: /resume <job_id>",
            (UiLanguage::Vi, Self::PromptMarkerRetry) => "Lệnh: /retry <job_id>",
            (UiLanguage::En, Self::PromptMarkerRetry) => "Command: /retry <job_id>",
            (UiLanguage::Vi, Self::PromptMarkerRevoke) => "Lệnh: /revoke <telegram_user_id>",
            (UiLanguage::En, Self::PromptMarkerRevoke) => "Command: /revoke <telegram_user_id>",
            (UiLanguage::Vi, Self::PromptMarkerSetDestination) => {
                "Đích là folder sẽ nhận bản copy."
            }
            (UiLanguage::En, Self::PromptMarkerSetDestination) => {
                "The destination is the folder that will receive the copy."
            }
            (UiLanguage::Vi, Self::PromptMarkerStatus) => "Lệnh: /status <job_id>",
            (UiLanguage::En, Self::PromptMarkerStatus) => "Command: /status <job_id>",
            (UiLanguage::Vi, Self::PromptMarkerUnwatch) => "Lệnh: /unwatch <id_watch>",
            (UiLanguage::En, Self::PromptMarkerUnwatch) => "Command: /unwatch <watch_id>",
            (UiLanguage::Vi, Self::PromptMarkerWatch) => {
                "Dán 2 link vào ô trả lời tin nhắn này: nguồn rồi đích."
            }
            (UiLanguage::En, Self::PromptMarkerWatch) => {
                "Paste 2 links in reply to this message: source then destination."
            }
            (UiLanguage::Vi, Self::PromptMarkerWatchPause) => "Lệnh: /watch_pause <id_watch>",
            (UiLanguage::En, Self::PromptMarkerWatchPause) => "Command: /watch_pause <watch_id>",
            (UiLanguage::Vi, Self::PromptMarkerWatchPolicy) => {
                "Dán ID watch và policy vào ô trả lời tin nhắn này."
            }
            (UiLanguage::En, Self::PromptMarkerWatchPolicy) => {
                "Paste the watch ID and policy in reply to this message."
            }
            (UiLanguage::Vi, Self::PromptMarkerWatchResume) => "Lệnh: /watch_resume <id_watch>",
            (UiLanguage::En, Self::PromptMarkerWatchResume) => "Command: /watch_resume <watch_id>",
            (UiLanguage::Vi, Self::PromptMarkerWatchStatus) => "Lệnh: /watch_status <id_watch>",
            (UiLanguage::En, Self::PromptMarkerWatchStatus) => "Command: /watch_status <watch_id>",
            (UiLanguage::Vi, Self::PromptPlaceholderDestinationFolder) => "Dán link thư mục đích",
            (UiLanguage::En, Self::PromptPlaceholderDestinationFolder) => {
                "Paste destination folder link"
            }
            (UiLanguage::Vi, Self::PromptPlaceholderDriveSource) => "Dán link nguồn Drive",
            (UiLanguage::En, Self::PromptPlaceholderDriveSource) => "Paste source Drive link",
            (UiLanguage::Vi, Self::PromptPlaceholderJobId) => "Dán job id",
            (UiLanguage::En, Self::PromptPlaceholderJobId) => "Paste job id",
            (UiLanguage::Vi, Self::PromptPlaceholderTelegramUserId) => "Dán Telegram user id",
            (UiLanguage::En, Self::PromptPlaceholderTelegramUserId) => "Paste Telegram user id",
            (UiLanguage::Vi, Self::PromptPlaceholderWatchId) => "Dán watch id",
            (UiLanguage::En, Self::PromptPlaceholderWatchId) => "Paste watch id",
            (UiLanguage::Vi, Self::PromptPlaceholderWatchLinks) => "Dán: link nguồn link đích",
            (UiLanguage::En, Self::PromptPlaceholderWatchLinks) => {
                "Paste: source link destination link"
            }
            (UiLanguage::Vi, Self::PromptPlaceholderWatchPolicy) => "Dán: watch_id policy",
            (UiLanguage::En, Self::PromptPlaceholderWatchPolicy) => "Paste: watch_id policy",
            (UiLanguage::Vi, Self::PromptSetDestination) => {
                "Dán link thư mục đích vào ô trả lời tin nhắn này.\n\
                 Đích là folder sẽ nhận bản copy.\n\
                 \n\
                 Ví dụ:\n\
                 drive.google.com/drive/folders/DICH"
            }
            (UiLanguage::En, Self::PromptSetDestination) => {
                "Paste the destination folder link in reply to this message.\n\
                 The destination is the folder that will receive the copy.\n\
                 \n\
                 Example:\n\
                 drive.google.com/drive/folders/DESTINATION"
            }
            (UiLanguage::Vi, Self::PromptWatch) => {
                "Dán 2 link vào ô trả lời tin nhắn này: nguồn rồi đích.\n\
                 Nguồn là folder cần theo dõi. Đích là folder nhận bản copy.\n\
                 \n\
                 Ví dụ:\n\
                 drive.google.com/drive/folders/NGUON drive.google.com/drive/folders/DICH"
            }
            (UiLanguage::En, Self::PromptWatch) => {
                "Paste 2 links in reply to this message: source then destination.\n\
                 The source is the folder to watch. The destination receives the copies.\n\
                 \n\
                 Example:\n\
                 drive.google.com/drive/folders/SOURCE drive.google.com/drive/folders/DESTINATION"
            }
            (UiLanguage::Vi, Self::PromptWatchPolicy) => {
                "Dán ID watch và policy vào ô trả lời tin nhắn này.\n\
                 Policy hợp lệ:\n\
                 • versioned_copy: nguồn đổi nội dung thì tạo bản copy mới.\n\
                 • replace_copy: copy mới rồi đưa bản cũ vào thùng rác.\n\
                 • manual_confirmation: dừng lại để xác nhận thủ công.\n\
                 \n\
                 Ví dụ:\n\
                 abc12345 versioned_copy"
            }
            (UiLanguage::En, Self::PromptWatchPolicy) => {
                "Paste the watch ID and policy in reply to this message.\n\
                 Valid policies:\n\
                 • versioned_copy: create a new copy when source content changes.\n\
                 • replace_copy: create a new copy, then trash the old one.\n\
                 • manual_confirmation: stop and wait for manual confirmation.\n\
                 \n\
                 Example:\n\
                 abc12345 versioned_copy"
            }
            (UiLanguage::Vi, Self::Refresh) => "Làm mới",
            (UiLanguage::En, Self::Refresh) => "Refresh",
            (UiLanguage::Vi, Self::Replace) => "Thay bản cũ",
            (UiLanguage::En, Self::Replace) => "Replace",
            (UiLanguage::Vi, Self::Report) => "Report",
            (UiLanguage::En, Self::Report) => "Report",
            (UiLanguage::Vi, Self::ReportCompleted) => "Hoàn tất",
            (UiLanguage::En, Self::ReportCompleted) => "Completed",
            (UiLanguage::Vi, Self::ReportFailed) => "Lỗi",
            (UiLanguage::En, Self::ReportFailed) => "Failed",
            (UiLanguage::Vi, Self::ReportHintClean) => {
                "Job đã sạch lỗi. Có thể lưu JSON/CSV nếu cần đối soát."
            }
            (UiLanguage::En, Self::ReportHintClean) => {
                "This job has no failed items. Keep the JSON/CSV if you need an audit trail."
            }
            (UiLanguage::Vi, Self::ReportHintRetry) => {
                "Bấm Retry lỗi trong chi tiết job để chạy lại phần lỗi."
            }
            (UiLanguage::En, Self::ReportHintRetry) => {
                "Tap Retry failed in the job detail to rerun failed items."
            }
            (UiLanguage::Vi, Self::ReportHintSkipped) => {
                "Mở file JSON/CSV để xem các mục đã bỏ qua."
            }
            (UiLanguage::En, Self::ReportHintSkipped) => {
                "Open the JSON/CSV files to inspect skipped items."
            }
            (UiLanguage::Vi, Self::ReportJobTitle) => "REPORT JOB",
            (UiLanguage::En, Self::ReportJobTitle) => "JOB REPORT",
            (UiLanguage::Vi, Self::ReportLatestTitle) => "REPORT GẦN NHẤT",
            (UiLanguage::En, Self::ReportLatestTitle) => "LATEST REPORT",
            (UiLanguage::Vi, Self::ReportScanned) => "Đã quét",
            (UiLanguage::En, Self::ReportScanned) => "Scanned",
            (UiLanguage::Vi, Self::ReportSkipped) => "Bỏ qua",
            (UiLanguage::En, Self::ReportSkipped) => "Skipped",
            (UiLanguage::Vi, Self::ReportStatus) => "Trạng thái",
            (UiLanguage::En, Self::ReportStatus) => "Status",
            (UiLanguage::Vi, Self::Resume) => "Tiếp tục",
            (UiLanguage::En, Self::Resume) => "Resume",
            (UiLanguage::Vi, Self::Retry) => "Thử lại",
            (UiLanguage::En, Self::Retry) => "Retry",
            (UiLanguage::Vi, Self::RetryFailed) => "Retry lỗi",
            (UiLanguage::En, Self::RetryFailed) => "Retry failed",
            (UiLanguage::Vi, Self::SelectThisFolder) => "Chọn thư mục này",
            (UiLanguage::En, Self::SelectThisFolder) => "Select this folder",
            (UiLanguage::Vi, Self::StopWatching) => "Dừng theo dõi",
            (UiLanguage::En, Self::StopWatching) => "Stop watching",
            (UiLanguage::Vi, Self::UpOneLevel) => "Lên một cấp",
            (UiLanguage::En, Self::UpOneLevel) => "Up one level",
            (UiLanguage::Vi, Self::Versioned) => "Tạo bản mới",
            (UiLanguage::En, Self::Versioned) => "Versioned",
            (UiLanguage::Vi, Self::ViewErrors) => "Xem lỗi",
            (UiLanguage::En, Self::ViewErrors) => "View errors",
            (UiLanguage::Vi, Self::ViewReport) => "Xem báo cáo",
            (UiLanguage::En, Self::ViewReport) => "View report",
            (UiLanguage::Vi, Self::Watches) => "Đồng bộ (Sync)",
            (UiLanguage::En, Self::Watches) => "Sync",
            (UiLanguage::Vi, Self::ConfirmWatchTitle) => "XÁC NHẬN THEO DÕI REALTIME",
            (UiLanguage::En, Self::ConfirmWatchTitle) => "CONFIRM REALTIME WATCH",
            (UiLanguage::Vi, Self::WatchDirectionOneWay) => {
                "Chiều đồng bộ: Nguồn ↓ một chiều → Đích"
            }
            (UiLanguage::En, Self::WatchDirectionOneWay) => {
                "Direction: Source ↓ one-way → Destination"
            }
            (UiLanguage::Vi, Self::StartWatchingButton) => "✓ Bắt đầu theo dõi",
            (UiLanguage::En, Self::StartWatchingButton) => "✓ Start watching",
            (UiLanguage::Vi, Self::WatchOptionsButton) => "⚙ Tuỳ chọn",
            (UiLanguage::En, Self::WatchOptionsButton) => "⚙ Options",
            (UiLanguage::Vi, Self::WatchOptionsTitle) => "TÙY CHỌN THEO DÕI REALTIME",
            (UiLanguage::En, Self::WatchOptionsTitle) => "REALTIME WATCH OPTIONS",
            (UiLanguage::Vi, Self::WatchOptionsHelp) => {
                "Chọn chính sách khi nội dung tệp ở nguồn thay đổi:"
            }
            (UiLanguage::En, Self::WatchOptionsHelp) => {
                "Choose policy when source file content changes:"
            }
            (UiLanguage::Vi, Self::WatchCreatedTitle) => "✓ ĐÃ BẬT THEO DÕI REALTIME",
            (UiLanguage::En, Self::WatchCreatedTitle) => "✓ REALTIME WATCH ENABLED",
            (UiLanguage::Vi, Self::WatchCreatedSummary) => {
                "Đã thiết lập theo dõi một chiều. Engine sẽ tự động đồng bộ khi nguồn có thay đổi."
            }
            (UiLanguage::En, Self::WatchCreatedSummary) => {
                "One-way watch configured. Engine will automatically sync changes from source."
            }
            (UiLanguage::Vi, Self::WatchLastSynced) => "Đồng bộ",
            (UiLanguage::En, Self::WatchLastSynced) => "Sync status",
            (UiLanguage::Vi, Self::WatchMappedFiles) => "Đã ánh xạ",
            (UiLanguage::En, Self::WatchMappedFiles) => "Mapped files",
            (UiLanguage::Vi, Self::WatchConflictTitle) => "⚠ Có phiên bản mới",
            (UiLanguage::En, Self::WatchConflictTitle) => "⚠ New version detected",
            (UiLanguage::Vi, Self::WatchConflictCurrent) => "Bản hiện tại",
            (UiLanguage::En, Self::WatchConflictCurrent) => "Current copy",
            (UiLanguage::Vi, Self::WatchConflictNew) => "Bản nguồn mới",
            (UiLanguage::En, Self::WatchConflictNew) => "New source copy",
            (UiLanguage::Vi, Self::WatchConflictRemaining) => "Còn lại",
            (UiLanguage::En, Self::WatchConflictRemaining) => "Remaining",
            (UiLanguage::Vi, Self::WatchActionNewVersion) => "Tạo bản mới",
            (UiLanguage::En, Self::WatchActionNewVersion) => "New version",
            (UiLanguage::Vi, Self::WatchActionReplace) => "Thay thế",
            (UiLanguage::En, Self::WatchActionReplace) => "Replace",
            (UiLanguage::Vi, Self::WatchActionSkip) => "Bỏ qua",
            (UiLanguage::En, Self::WatchActionSkip) => "Skip",
            (UiLanguage::Vi, Self::WatchResolveConflictButton) => "⚠ Giải quyết xung đột",
            (UiLanguage::En, Self::WatchResolveConflictButton) => "⚠ Resolve conflict",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TextKey, UiLanguage};

    #[test]
    fn all_catalog_keys_have_vietnamese_and_english_text() {
        for key in TextKey::ALL {
            assert!(!UiLanguage::Vi.text(*key).trim().is_empty());
            assert!(!UiLanguage::En.text(*key).trim().is_empty());
        }
    }

    #[test]
    fn destructive_confirmation_copy_is_localized() {
        assert_eq!(
            UiLanguage::Vi.text(TextKey::ConfirmCancelJobTitle),
            "XÁC NHẬN HUỶ JOB"
        );
        assert_eq!(
            UiLanguage::En.text(TextKey::ConfirmCancelJobTitle),
            "CONFIRM JOB CANCEL"
        );
        assert!(
            UiLanguage::En
                .text(TextKey::ConfirmUnwatchBody)
                .contains("Destination copies")
        );
    }
}
