use super::*;
#[cfg(target_os = "macos")]
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn should_map_common_file_extensions() {
    assert_eq!(file_extension_icon("pdf"), "i-noto:page-facing-up");
    assert_eq!(file_extension_icon("rs"), "i-noto:desktop-computer");
    assert_eq!(file_extension_icon("plist"), "i-noto:scroll");
    assert_eq!(file_extension_icon("zip"), "i-noto:file-folder");
    assert_eq!(file_extension_icon("unknown"), FALLBACK_FILE_ICON);
}

#[cfg(target_os = "macos")]
fn create_temp_app_dir(app_name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!("rtool-icon-test-{app_name}-{nonce}"));
    let app_dir = root.join(format!("{app_name}.app"));
    fs::create_dir_all(app_dir.join("Contents").join("Resources"))
        .expect("failed to create app resources dir");
    app_dir
}

#[cfg(target_os = "macos")]
fn write_info_plist(app_dir: &Path, content: &str) {
    let info_plist = app_dir.join("Contents").join("Info.plist");
    fs::write(info_plist, content).expect("failed to write Info.plist");
}

#[cfg(target_os = "macos")]
#[test]
fn resolve_macos_icon_source_prefers_plist_declared_icon() {
    let app_dir = create_temp_app_dir("Visual Studio Code");
    let resources = app_dir.join("Contents").join("Resources");
    fs::write(resources.join("Code.icns"), b"code").expect("failed to write Code.icns");
    fs::write(resources.join("javascript.icns"), b"javascript")
        .expect("failed to write javascript.icns");
    write_info_plist(
        app_dir.as_path(),
        r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
  <key>CFBundleIconFile</key>
  <string>Code.icns</string>
</dict>
</plist>"#,
    );

    let source = resolve_macos_icon_source(app_dir.as_path()).expect("source should exist");
    let icon_name = source
        .icon_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    assert_eq!(icon_name, "Code.icns");

    let parent = app_dir
        .parent()
        .expect("app dir parent should exist")
        .to_path_buf();
    fs::remove_dir_all(parent).expect("failed to cleanup temp dir");
}

#[cfg(target_os = "macos")]
#[test]
fn resolve_macos_icon_source_fallback_prefers_app_icon_over_document() {
    let app_dir = create_temp_app_dir("Zed");
    let resources = app_dir.join("Contents").join("Resources");
    fs::write(resources.join("Document.icns"), b"document").expect("failed to write Document.icns");
    fs::write(resources.join("Zed.icns"), b"zed").expect("failed to write Zed.icns");

    let source = resolve_macos_icon_source(app_dir.as_path()).expect("source should exist");
    let icon_name = source
        .icon_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    assert_eq!(icon_name, "Zed.icns");

    let parent = app_dir
        .parent()
        .expect("app dir parent should exist")
        .to_path_buf();
    fs::remove_dir_all(parent).expect("failed to cleanup temp dir");
}
