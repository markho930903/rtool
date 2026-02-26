use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn extract_rust_command_keys(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter_map(|line| {
            let marker = "::commands::";
            let start = line.find(marker)?;
            let tail = &line[start + marker.len()..];
            let key: String = tail
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                .collect();
            if key.is_empty() { None } else { Some(key) }
        })
        .collect()
}

fn extract_ts_command_keys(source: &str) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let Some(start) = source.find("export type CommandKey =") else {
        return keys;
    };

    for line in source[start..].lines().skip(1) {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            if trimmed.ends_with(';') {
                break;
            }
            continue;
        }

        let Some(first_quote) = trimmed.find('"') else {
            continue;
        };
        let remain = &trimmed[first_quote + 1..];
        let Some(second_quote) = remain.find('"') else {
            continue;
        };
        let key = &remain[..second_quote];
        if !key.is_empty() {
            keys.insert(key.to_string());
        }

        if trimmed.ends_with(';') {
            break;
        }
    }

    keys
}

fn project_root_from_manifest_dir(manifest_dir: &Path) -> PathBuf {
    manifest_dir.join("..").join("..").join("..").to_path_buf()
}

#[test]
fn command_keys_should_match_typescript_contract() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let invoke_rs = manifest_dir.join("src").join("bootstrap").join("invoke.rs");
    let contracts_ts = project_root_from_manifest_dir(manifest_dir)
        .join("src")
        .join("contracts")
        .join("index.ts");

    let invoke_source = fs::read_to_string(&invoke_rs)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", invoke_rs.display()));
    let contracts_source = fs::read_to_string(&contracts_ts)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", contracts_ts.display()));

    let rust_keys = extract_rust_command_keys(&invoke_source);
    let ts_keys = extract_ts_command_keys(&contracts_source);

    assert!(
        !rust_keys.is_empty(),
        "no command key found in {}",
        invoke_rs.display()
    );
    assert!(
        !ts_keys.is_empty(),
        "no CommandKey union member found in {}",
        contracts_ts.display()
    );

    let only_rust: Vec<String> = rust_keys.difference(&ts_keys).cloned().collect();
    let only_ts: Vec<String> = ts_keys.difference(&rust_keys).cloned().collect();

    if !only_rust.is_empty() || !only_ts.is_empty() {
        panic!(
            "command contract mismatch\nonly in invoke.rs: {:?}\nonly in contracts/index.ts: {:?}",
            only_rust, only_ts
        );
    }
}
