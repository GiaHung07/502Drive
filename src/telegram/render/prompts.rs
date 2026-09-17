//! Help text and reply-prompt texts.

use crate::telegram::handlers::ReplyPrompt;
use crate::telegram::i18n::TextKey as T;
use crate::telegram::keyboards;

pub(crate) fn render_help_text(lang: keyboards::UiLanguage) -> String {
    lang.text(T::HelpText).to_string()
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
