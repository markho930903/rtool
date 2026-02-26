use std::fs;
use std::path::{Path, PathBuf};

fn command_files(features_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(features_dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let file = path.join("commands.rs");
        if file.is_file() {
            files.push(file);
        }
    }
    files.sort();
    files
}

#[test]
fn command_adapters_should_not_call_domain_crates_directly() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let features_dir = manifest_dir.join("src").join("features");
    let files = command_files(features_dir.as_path());
    assert!(
        !files.is_empty(),
        "no command files found under {}",
        features_dir.display()
    );

    let forbidden = [
        "use app_launcher_app::",
        "use app_transfer::service::",
        "use app_clipboard::service::",
        "use app_resource_monitor::",
        "use app_infra::logging::",
    ];

    let mut violations = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", file.display()));
        for marker in forbidden {
            if source.contains(marker) {
                violations.push(format!("{} contains `{}`", file.display(), marker));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "found forbidden direct domain dependency in command adapters:\\n{}",
        violations.join("\\n")
    );
}
