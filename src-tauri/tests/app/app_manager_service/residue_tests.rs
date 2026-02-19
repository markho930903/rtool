use super::*;

fn candidate_with_confidence(confidence: &str, evidence_len: usize) -> ResidueCandidate {
    ResidueCandidate {
        path: PathBuf::from("/tmp/a"),
        scope: "user".to_string(),
        kind: "cache".to_string(),
        exists: true,
        filesystem: true,
        match_reason: "test".to_string(),
        confidence: confidence.to_string(),
        evidence: (0..evidence_len).map(|idx| format!("e{idx}")).collect(),
        risk_level: "low".to_string(),
        recommended: true,
        readonly_reason_code: None,
    }
}

#[test]
fn residue_candidate_replace_prefers_higher_confidence() {
    let current = candidate_with_confidence("high", 1);
    let next = candidate_with_confidence("exact", 1);
    assert!(should_replace_residue_candidate(&current, &next));
    assert!(!should_replace_residue_candidate(&next, &current));
}

#[test]
fn residue_candidate_replace_prefers_more_evidence_when_confidence_equal() {
    let current = candidate_with_confidence("high", 1);
    let next = candidate_with_confidence("high", 2);
    assert!(should_replace_residue_candidate(&current, &next));
}
