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

#[test]
fn detects_all_drive_reference_formats() {
    use gdclone_bot::drive::id_parser::detect_drive_reference;

    // Folder URL
    let r1 = detect_drive_reference("https://drive.google.com/drive/folders/1Abc_DEF-ghi234567890")
        .unwrap();
    assert_eq!(r1.file_id, "1Abc_DEF-ghi234567890");

    // File URL with query params
    let r2 = detect_drive_reference(
        "https://drive.google.com/file/d/1AbcDEFghi234567890123456/view?usp=sharing",
    )
    .unwrap();
    assert_eq!(r2.file_id, "1AbcDEFghi234567890123456");

    // Embedded in conversation text
    let r3 = detect_drive_reference("Here is the source folder https://drive.google.com/drive/folders/1XYZ_9876543210abcdefgh please clone it").unwrap();
    assert_eq!(r3.file_id, "1XYZ_9876543210abcdefgh");

    // Bare ID (>= 20 chars)
    let r4 = detect_drive_reference("1AbcDEFghi234567890123456").unwrap();
    assert_eq!(r4.file_id, "1AbcDEFghi234567890123456");

    // Query param id
    let r5 = detect_drive_reference("https://drive.google.com/open?id=1AbcDEFghi234567890123456")
        .unwrap();
    assert_eq!(r5.file_id, "1AbcDEFghi234567890123456");

    // User path folder URL
    let r6 = detect_drive_reference(
        "https://drive.google.com/drive/u/0/folders/1Abc_DEF-ghi2345678901234",
    )
    .unwrap();
    assert_eq!(r6.file_id, "1Abc_DEF-ghi2345678901234");

    // False positives rejected:
    assert!(detect_drive_reference("cancel").is_none());
    assert!(detect_drive_reference("status").is_none());
    assert!(detect_drive_reference("short_string_123").is_none());
    assert!(detect_drive_reference("https://example.com/folders/1Abc_DEF-ghi234567890").is_none());
}
