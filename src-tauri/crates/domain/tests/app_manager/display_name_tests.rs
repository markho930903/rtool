use super::*;

#[test]
fn resolve_display_name_prefers_readable_stem_over_short_alias() {
    let path = Path::new("/Applications/Visual Studio Code.app");
    let mut candidates = Vec::new();
    push_display_name_candidate(&mut candidates, Some("Code".to_string()), 90);
    push_display_name_candidate(&mut candidates, Some("Visual Studio Code".to_string()), 85);

    let name =
        resolve_application_display_name(path, "/Applications/Visual Studio Code.app", candidates);
    assert_eq!(name, "Visual Studio Code");
}

#[test]
fn resolve_display_name_keeps_short_name_when_stem_is_also_short() {
    let path = Path::new("/Applications/Code.app");
    let mut candidates = Vec::new();
    push_display_name_candidate(&mut candidates, Some("Code".to_string()), 90);
    let name = resolve_application_display_name(path, "/Applications/Code.app", candidates);
    assert_eq!(name, "Code");
}

#[test]
fn resolve_display_name_prefers_windows_registry_display_name() {
    let path = Path::new("/Program Files/Foo/foo.exe");
    let mut candidates = Vec::new();
    push_display_name_candidate(&mut candidates, Some("Foo Enterprise".to_string()), 90);
    push_display_name_candidate(&mut candidates, Some("foo".to_string()), 80);
    let name = resolve_application_display_name(path, "/Program Files/Foo/foo.exe", candidates);
    assert_eq!(name, "Foo Enterprise");
}
