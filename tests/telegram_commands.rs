use gdclone_bot::telegram::commands::Command;
use teloxide::utils::command::BotCommands;

#[test]
fn parses_snake_case_commands_used_in_help_text() {
    assert!(matches!(
        Command::parse("/set_destination abc", "gdclone_bot").unwrap(),
        Command::SetDestination(value) if value == "abc"
    ));
    assert!(matches!(
        Command::parse("/clone_here abc", "gdclone_bot").unwrap(),
        Command::CloneHere(value) if value == "abc"
    ));
    assert!(matches!(
        Command::parse("/watch_status abc", "gdclone_bot").unwrap(),
        Command::WatchStatus(value) if value == "abc"
    ));
    assert!(matches!(
        Command::parse("/preview", "gdclone_bot").unwrap(),
        Command::Preview
    ));
    assert!(matches!(
        Command::parse("/menu", "gdclone_bot").unwrap(),
        Command::Menu
    ));
    assert!(matches!(
        Command::parse("/last_report", "gdclone_bot").unwrap(),
        Command::LastReport
    ));
}

#[test]
fn parses_sync_command() {
    assert!(matches!(
        Command::parse("/sync https://drive.google.com/drive/folders/SRC https://drive.google.com/drive/folders/DST", "gdclone_bot").unwrap(),
        Command::Sync(val) if val.contains("SRC") && val.contains("DST")
    ));
    assert!(matches!(
        Command::parse("/sync https://drive.google.com/drive/folders/SRC", "gdclone_bot").unwrap(),
        Command::Sync(val) if val == "https://drive.google.com/drive/folders/SRC"
    ));
}
