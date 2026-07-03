use teloxide::utils::command::BotCommands;

#[derive(Debug, Clone, BotCommands)]
#[command(rename_rule = "snake_case", description = "Các lệnh:")]
pub enum Command {
    #[command(description = "khởi động bot và xem hướng dẫn")]
    Start,
    #[command(description = "hướng dẫn đăng nhập Google trên máy chạy bot")]
    Connect,
    #[command(description = "xem trạng thái tài khoản Google")]
    Account,
    #[command(description = "xem danh sách lệnh")]
    Help,
    #[command(description = "xem bảng trạng thái realtime trong Telegram")]
    Preview,
    #[command(description = "thu hồi token và ngắt Google Drive")]
    Disconnect,
    #[command(description = "clone Drive URL hoặc ID")]
    Clone(String),
    #[command(description = "clone ngay vào thư mục đích mặc định")]
    CloneHere(String),
    #[command(description = "liệt kê job đang chạy/tạm dừng")]
    Jobs,
    #[command(description = "xem trạng thái một job")]
    Status(String),
    #[command(description = "tạm dừng job")]
    Pause(String),
    #[command(description = "tiếp tục job đã tạm dừng")]
    Resume(String),
    #[command(description = "hủy job")]
    Cancel(String),
    #[command(description = "thử lại các item lỗi")]
    Retry(String),
    #[command(description = "xem Telegram user id và quyền hiện tại")]
    Whoami,
    #[command(description = "đặt thư mục Drive đích mặc định")]
    SetDestination(String),
    #[command(description = "xem thư mục đích mặc định")]
    Destination,
    #[command(description = "xóa thư mục đích mặc định")]
    ClearDestination,
    #[command(description = "cấp quyền dùng bot cho user id")]
    Grant(String),
    #[command(description = "thu hồi quyền dùng bot của user id")]
    Revoke(String),
    // ── Watch commands ────────────────────────────────────────────────────────
    #[command(description = "theo dõi folder Drive: /watch <nguồn> <đích>")]
    Watch(String),
    #[command(description = "liệt kê watch subscription")]
    Watches,
    #[command(description = "xem chi tiết watch subscription")]
    WatchStatus(String),
    #[command(description = "tạm dừng watch subscription")]
    WatchPause(String),
    #[command(description = "tiếp tục watch subscription")]
    WatchResume(String),
    /// Policy string: "<watch_id> <content_update_policy>"
    /// valid policies: versioned_copy | replace_copy | manual_confirmation
    #[command(description = "đặt policy cập nhật nội dung cho watch")]
    WatchPolicy(String),
    #[command(description = "dừng và xóa watch subscription")]
    Unwatch(String),
}
