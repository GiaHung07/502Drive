use gdclone_bot::drive::client::app_property_copy_key_query;

#[test]
fn app_property_query_escapes_values() {
    let query = app_property_copy_key_query("parent'id", "copy\\key'id");

    assert!(query.contains("'parent\\'id' in parents"));
    assert!(query.contains("key='gdclone_copy_key'"));
    assert!(query.contains("value='copy\\\\key\\'id'"));
    assert!(query.contains("trashed=false"));
}
