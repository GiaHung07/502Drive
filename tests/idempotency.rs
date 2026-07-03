use gdclone_bot::engine::operation::{AppProperties, ScopeType, idempotency_key};

#[test]
fn idempotency_key_is_deterministic_and_scope_sensitive() {
    let a = idempotency_key(ScopeType::Job, "scope", "source", "parent", 1);
    let b = idempotency_key(ScopeType::Job, "scope", "source", "parent", 1);
    let c = idempotency_key(ScopeType::Watch, "scope", "source", "parent", 1);

    assert_eq!(a, b);
    assert_ne!(a, c);
    assert!(a.starts_with("gdclone_"));
}

#[test]
fn app_properties_use_same_copy_key() {
    let props = AppProperties::new(ScopeType::Job, "job-1", "src-1", "dst-parent", 2);
    assert_eq!(props.gdclone_source_id, "src-1");
    assert_eq!(props.gdclone_scope_id, "job-1");
    assert_eq!(props.gdclone_generation, "2");
    assert_eq!(
        props.gdclone_copy_key,
        idempotency_key(ScopeType::Job, "job-1", "src-1", "dst-parent", 2)
    );
}
