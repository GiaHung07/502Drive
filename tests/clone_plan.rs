use gdclone_bot::engine::{
    copy::{copy_file_request_json, create_folder_request_json, create_shortcut_request_json},
    operation::{AppProperties, ScopeType},
};
use serde_json::Value;

#[test]
fn create_folder_request_contains_idempotency_app_properties() {
    let props = AppProperties::new(ScopeType::Job, "job-1", "src-folder", "dst-parent", 1);
    let body: Value = serde_json::from_str(&create_folder_request_json(
        "Course Materials",
        "dst-parent",
        &props,
    ))
    .unwrap();

    assert_eq!(body["name"], "Course Materials");
    assert_eq!(body["mimeType"], "application/vnd.google-apps.folder");
    assert_eq!(body["parents"][0], "dst-parent");
    assert_eq!(body["appProperties"]["gdclone_source_id"], "src-folder");
    assert_eq!(
        body["appProperties"]["gdclone_copy_key"],
        props.gdclone_copy_key
    );
}

#[test]
fn copy_file_request_contains_idempotency_app_properties() {
    let props = AppProperties::new(ScopeType::Job, "job-1", "src-file", "dst-parent", 1);
    let body: Value =
        serde_json::from_str(&copy_file_request_json("notes.pdf", "dst-parent", &props)).unwrap();

    assert_eq!(body["name"], "notes.pdf");
    assert_eq!(body["parents"][0], "dst-parent");
    assert_eq!(body["appProperties"]["gdclone_generation"], "1");
    assert_eq!(body["appProperties"]["gdclone_scope_id"], "job-1");
}

#[test]
fn create_shortcut_request_preserves_target() {
    let props = AppProperties::new(ScopeType::Job, "job-1", "shortcut-src", "dst-parent", 1);
    let body: Value = serde_json::from_str(&create_shortcut_request_json(
        "Course link",
        "dst-parent",
        "target-file",
        Some("target-resource-key"),
        &props,
    ))
    .unwrap();

    assert_eq!(body["name"], "Course link");
    assert_eq!(body["mimeType"], "application/vnd.google-apps.shortcut");
    assert_eq!(body["parents"][0], "dst-parent");
    assert_eq!(body["shortcutDetails"]["targetId"], "target-file");
    assert_eq!(
        body["shortcutDetails"]["targetResourceKey"],
        "target-resource-key"
    );
    assert_eq!(body["appProperties"]["gdclone_source_id"], "shortcut-src");
}
