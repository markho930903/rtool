use super::*;

#[test]
fn collect_sources_should_reject_empty() {
    let result = collect_sources(&[]);
    assert!(result.is_err());
}
