use gdclone_bot::drive::id_parser::{DriveResourceKind, ParseDriveIdError, parse};

#[test]
fn parses_folder_url() {
    let parsed = parse("https://drive.google.com/drive/folders/1Abc_DEF-ghi234567890").unwrap();
    assert_eq!(parsed.id, "1Abc_DEF-ghi234567890");
    assert_eq!(parsed.kind, DriveResourceKind::Folder);
}

#[test]
fn parses_file_url() {
    let parsed =
        parse("https://drive.google.com/file/d/1AbcDEFghi234567890/view?usp=sharing").unwrap();
    assert_eq!(parsed.id, "1AbcDEFghi234567890");
    assert_eq!(parsed.kind, DriveResourceKind::File);
}

#[test]
fn parses_open_id_url() {
    let parsed = parse("https://drive.google.com/open?id=1AbcDEFghi234567890").unwrap();
    assert_eq!(parsed.id, "1AbcDEFghi234567890");
    assert_eq!(parsed.kind, DriveResourceKind::Unknown);
}

#[test]
fn rejects_non_drive_url() {
    let err = parse("https://example.com/file/d/1AbcDEFghi234567890").unwrap_err();
    assert_eq!(err, ParseDriveIdError::Invalid);
}
