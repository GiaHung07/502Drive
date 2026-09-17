//! Help text and reply-prompt texts.

use crate::telegram::handlers::ReplyPrompt;
use crate::telegram::i18n::TextKey as T;
use crate::telegram::keyboards;

pub(crate) fn render_help_text(lang: keyboards::UiLanguage, watch_enabled: bool) -> String {
    let mut text = match lang {
        keyboards::UiLanguage::Vi => {
            "LỆNH CHÍNH\n\
             ━━━━━━━━━\n\
             /start, /menu         Mở bảng điều khiển chính\n\
             /clone <url>           Kiểm tra nguồn, xem kế hoạch, rồi clone\n\
             /clone_here <url>      Clone ngay vào thư mục đích mặc định\n\
             /destination           Xem/đổi thư mục đích đã lưu\n\
             /set_destination <url> Đặt thư mục đích mặc định\n\
             /clear_destination     Xoá thư mục đích mặc định\n\
             /jobs                  Job đang chạy hoặc tạm dừng\n\
             /status <job_id>       Chi tiết một job\n\
             /pause <job_id>        Tạm dừng\n\
             /resume <job_id>       Tiếp tục\n\
             /cancel <job_id>       Huỷ\n\
             /retry <job_id>        Làm lại phần lỗi\n\
             /last_report           Gửi lại report job gần nhất\n\
             /preview               Bảng tổng quan realtime\n\
             /connect               Hướng dẫn đăng nhập Google\n\
             /account               Tài khoản Google\n\
             /disconnect            Ngắt kết nối Google Drive\n\
             /whoami                Telegram ID và quyền của bạn\n\
             \n"
        }
        keyboards::UiLanguage::En => {
            "MAIN COMMANDS\n\
             ━━━━━━━━━\n\
             /start, /menu         Open the main dashboard\n\
             /clone <url>           Check source, preview plan, then clone\n\
             /clone_here <url>      Clone to the default destination now\n\
             /destination           View/change saved destination\n\
             /set_destination <url> Set the default destination\n\
             /clear_destination     Clear the default destination\n\
             /jobs                  Running or paused jobs\n\
             /status <job_id>       Job detail\n\
             /pause <job_id>        Pause\n\
             /resume <job_id>       Resume\n\
             /cancel <job_id>       Cancel\n\
             /retry <job_id>        Retry failed items\n\
             /last_report           Send the latest job JSON/CSV report\n\
             /preview               Realtime overview\n\
             /connect               Google sign-in guide\n\
             /account               Google account\n\
             /disconnect            Disconnect Google Drive\n\
             /whoami                Your Telegram ID and role\n\
             \n"
        }
    }
    .to_string();

    text.push_str(&match (lang, watch_enabled) {
        (keyboards::UiLanguage::Vi, _) => {
            "WATCH/SYNC\n\
             ━━━━━━━━━━\n\
             /sync <nguồn> [đích]         Đồng bộ realtime (tự động lấy đích mặc định nếu bỏ qua đích)\n\
             /watch <nguồn> <đích>        Theo dõi thư mục nguồn sang thư mục đích\n\
             /watches                     Danh sách thư mục đang theo dõi\n\
             /watch_status <id>           Trạng thái đồng bộ, số thay đổi còn chờ, policy\n\
             /watch_pause <id>            Tạm dừng áp thay đổi, vẫn ghi nhận backlog\n\
             /watch_resume <id>           Tiếp tục áp thay đổi từ nguồn sang đích\n\
             /watch_policy <id> <policy>  Đổi cách xử lý khi nội dung file nguồn thay đổi\n\
                                          versioned_copy: tạo bản copy mới\n\
                                          replace_copy: copy mới rồi đưa bản cũ vào thùng rác\n\
                                          manual_confirmation: dừng để xác nhận thủ công\n\
             /watch_filter <id> list|add <glob>|remove <glob>|clear\n\
                                          Loại trừ file khớp glob khỏi đồng bộ (vd: *.tmp)\n\
             /unwatch <id>                Dừng theo dõi\n\
             \n"
                .to_string()
        }
        (keyboards::UiLanguage::En, _) => {
            "WATCH/SYNC\n\
             ━━━━━━━━━━\n\
             /sync <source> [dest]        Realtime sync (uses default destination if dest omitted)\n\
             /watch <source> <dest>       Source is the folder to preserve; destination receives copies\n\
             /watches                     Watched folders\n\
             /watch_status <id>           Sync status, pending changes, policy\n\
             /watch_pause <id>            Pause applying changes while backlog is still recorded\n\
             /watch_resume <id>           Resume applying source changes to destination\n\
             /watch_policy <id> <policy>  Change how source file updates are handled\n\
                                          versioned_copy: create a new copy\n\
                                          replace_copy: copy new, then trash old copy\n\
                                          manual_confirmation: stop for manual confirmation\n\
             /watch_filter <id> list|add <glob>|remove <glob>|clear\n\
                                          Exclude files matching a glob from sync (e.g. *.tmp)\n\
             /unwatch <id>                Stop watching\n\
             \n"
                .to_string()
        }
    });

    if !watch_enabled {
        text.push_str(&match lang {
            keyboards::UiLanguage::Vi => {
                "⚠️ Watch đang TẮT trong config: các lệnh WATCH/SYNC ở trên chỉ hoạt động sau khi bật watch.\n\n"
            }
            keyboards::UiLanguage::En => {
                "⚠️ Watch is DISABLED in config: the WATCH/SYNC commands above only work once watch is enabled.\n\n"
            }
        });
    }

    match lang {
        keyboards::UiLanguage::Vi => {
            text.push_str("Quản trị: /whoami /grant <user_id> /revoke <user_id> /disconnect")
        }
        keyboards::UiLanguage::En => {
            text.push_str("Admin: /whoami /grant <user_id> /revoke <user_id> /disconnect")
        }
    }
    text
}

pub(crate) fn clone_prompt(lang: keyboards::UiLanguage) -> &'static str {
    lang.text(T::PromptClone)
}

pub(crate) fn clone_here_prompt(lang: keyboards::UiLanguage) -> &'static str {
    lang.text(T::PromptCloneHere)
}

pub(crate) fn set_destination_prompt(lang: keyboards::UiLanguage) -> &'static str {
    lang.text(T::PromptSetDestination)
}

pub(crate) fn watch_prompt(lang: keyboards::UiLanguage) -> &'static str {
    lang.text(T::PromptWatch)
}

pub(crate) fn watch_id_prompt(command: &str, lang: keyboards::UiLanguage) -> String {
    let marker = match command {
        "/watch_status" => ReplyPrompt::WatchStatus.marker(lang),
        "/watch_pause" => ReplyPrompt::WatchPause.marker(lang),
        "/watch_resume" => ReplyPrompt::WatchResume.marker(lang),
        "/unwatch" => ReplyPrompt::Unwatch.marker(lang),
        _ => "",
    };
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Dán ID watch vào ô trả lời tin nhắn này.\n\
             Lệnh: {command} <id_watch>\n\
             \n\
             Dùng /watches để bấm chọn, không cần nhớ ID.\n\
             {marker}"
        ),
        keyboards::UiLanguage::En => format!(
            "Paste the watch ID in reply to this message.\n\
             Command: {command} <watch_id>\n\
             \n\
             Use /watches to select from list without remembering ID.\n\
             {marker}"
        ),
    }
}

pub(crate) fn job_id_prompt(command: &str, lang: keyboards::UiLanguage) -> String {
    let marker = match command {
        "/status" => ReplyPrompt::Status.marker(lang),
        "/pause" => ReplyPrompt::Pause.marker(lang),
        "/resume" => ReplyPrompt::Resume.marker(lang),
        "/cancel" => ReplyPrompt::Cancel.marker(lang),
        "/retry" => ReplyPrompt::Retry.marker(lang),
        _ => "",
    };
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Dán job ID vào ô trả lời tin nhắn này.\n\
             Lệnh: {command} <job_id>\n\
             \n\
             Dùng /jobs để xem job đang chạy.\n\
             {marker}"
        ),
        keyboards::UiLanguage::En => format!(
            "Paste the job ID in reply to this message.\n\
             Command: {command} <job_id>\n\
             \n\
             Use /jobs to see running jobs.\n\
             {marker}"
        ),
    }
}

pub(crate) fn user_id_prompt(command: &str, lang: keyboards::UiLanguage) -> String {
    let marker = match command {
        "/grant" => ReplyPrompt::Grant.marker(lang),
        "/revoke" => ReplyPrompt::Revoke.marker(lang),
        _ => "",
    };
    match lang {
        keyboards::UiLanguage::Vi => format!(
            "Dán Telegram user ID vào ô trả lời tin nhắn này.\n\
             Lệnh: {command} <telegram_user_id>\n\
             \n\
             User có thể dùng /whoami để xem ID.\n\
             {marker}"
        ),
        keyboards::UiLanguage::En => format!(
            "Paste the Telegram user ID in reply to this message.\n\
             Command: {command} <telegram_user_id>\n\
             \n\
             Users can run /whoami to see their ID.\n\
             {marker}"
        ),
    }
}

pub(crate) fn watch_policy_prompt(lang: keyboards::UiLanguage) -> &'static str {
    lang.text(T::PromptWatchPolicy)
}

pub(crate) fn prompt_text(prompt: ReplyPrompt, lang: keyboards::UiLanguage) -> String {
    match prompt {
        ReplyPrompt::Clone => clone_prompt(lang).to_string(),
        ReplyPrompt::CloneHere => clone_here_prompt(lang).to_string(),
        ReplyPrompt::SetDestination => set_destination_prompt(lang).to_string(),
        ReplyPrompt::Status => job_id_prompt("/status", lang),
        ReplyPrompt::Pause => job_id_prompt("/pause", lang),
        ReplyPrompt::Resume => job_id_prompt("/resume", lang),
        ReplyPrompt::Cancel => job_id_prompt("/cancel", lang),
        ReplyPrompt::Retry => job_id_prompt("/retry", lang),
        ReplyPrompt::Grant => user_id_prompt("/grant", lang),
        ReplyPrompt::Revoke => user_id_prompt("/revoke", lang),
        ReplyPrompt::Watch => watch_prompt(lang).to_string(),
        ReplyPrompt::WatchStatus => watch_id_prompt("/watch_status", lang),
        ReplyPrompt::WatchPause => watch_id_prompt("/watch_pause", lang),
        ReplyPrompt::WatchResume => watch_id_prompt("/watch_resume", lang),
        ReplyPrompt::WatchPolicy => watch_policy_prompt(lang).to_string(),
        ReplyPrompt::Unwatch => watch_id_prompt("/unwatch", lang),
    }
}
