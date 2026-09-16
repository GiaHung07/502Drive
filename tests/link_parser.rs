use gdclone_bot::drive::links::{DriveItemKindHint, parse_drive_reference};

#[test]
fn parses_resource_key() {
    let parsed = parse_drive_reference(
        "https://drive.google.com/file/d/1AbcDEFghi234567890/view?resourcekey=abc123",
    )
    .unwrap();
    assert_eq!(parsed.file_id, "1AbcDEFghi234567890");
    assert_eq!(parsed.resource_key.as_deref(), Some("abc123"));
    assert_eq!(parsed.hinted_kind, Some(DriveItemKindHint::File));
    assert_eq!(
        parsed.resource_key_header_value().as_deref(),
        Some("1AbcDEFghi234567890/abc123")
    );
}

#[test]
fn parses_docs_url() {
    let parsed =
        parse_drive_reference("https://docs.google.com/document/d/1AbcDEFghi234567890/edit")
            .unwrap();
    assert_eq!(parsed.hinted_kind, Some(DriveItemKindHint::GoogleDocument));
}

#[test]
fn parses_bare_drive_folder_url() {
    let parsed = parse_drive_reference(
        "drive.google.com/drive/u/6/folders/1ECfxZCXFzI6ylHoyYaFWS2dVNSIoyFw8",
    )
    .unwrap();
    assert_eq!(parsed.file_id, "1ECfxZCXFzI6ylHoyYaFWS2dVNSIoyFw8");
    assert_eq!(parsed.hinted_kind, Some(DriveItemKindHint::Folder));
}
