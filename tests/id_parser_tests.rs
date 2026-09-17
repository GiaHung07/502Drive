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

    // Google Docs URL
    let r7 = detect_drive_reference(
        "https://docs.google.com/document/d/1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms/edit",
    )
    .unwrap();
    assert_eq!(r7.file_id, "1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgvE2upms");

    // Google Sheets URL
    let r8 = detect_drive_reference(
        "https://docs.google.com/spreadsheets/d/1qpyC0XzvTcKT6EISywVQESXEt8S0H6GiL-VNkiMmPrs/edit#gid=0",
    )
    .unwrap();
    assert_eq!(r8.file_id, "1qpyC0XzvTcKT6EISywVQESXEt8S0H6GiL-VNkiMmPrs");

    // Google Slides URL with trailing query
    let r9 = detect_drive_reference(
        "https://docs.google.com/presentation/d/1234567890abcdefghij1234567890/edit?usp=sharing",
    )
    .unwrap();
    assert_eq!(r9.file_id, "1234567890abcdefghij1234567890");

    // Folder URL with resource key and trailing slash
    let r10 = detect_drive_reference(
        "https://drive.google.com/drive/folders/1Abc_DEF-ghi2345678901234/?resourcekey=0-abcdef12345",
    )
    .unwrap();
    assert_eq!(r10.file_id, "1Abc_DEF-ghi2345678901234");
    assert_eq!(r10.resource_key.as_deref(), Some("0-abcdef12345"));

    // False positives rejected:
    assert!(detect_drive_reference("cancel").is_none());
    assert!(detect_drive_reference("status").is_none());
    assert!(detect_drive_reference("short_string_123").is_none());
    assert!(detect_drive_reference("https://example.com/folders/1Abc_DEF-ghi234567890").is_none());
    assert!(detect_drive_reference("https://drive.google.com/").is_none());
}
