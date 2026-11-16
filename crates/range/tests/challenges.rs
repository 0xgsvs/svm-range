use svm_range::{AccountDelta, ExploitOutcome, Mode, run_challenge};

fn delta<'a>(outcome: &'a ExploitOutcome, label: &str) -> &'a AccountDelta {
    outcome
        .changes
        .iter()
        .find(|change| change.label == label)
        .expect("account delta")
}

#[test]
fn missing_signer_vulnerable_drains_vault() {
    let outcome = run_challenge("missing-signer", Mode::Vulnerable).expect("vulnerable run");
    assert!(outcome.succeeded, "{}", outcome.summary);
    assert_eq!(delta(&outcome, "vault").after, 0);
    assert_eq!(delta(&outcome, "attacker destination").after, 3_000_000_000);
}

#[test]
fn missing_signer_patched_rejects_unsigned_authority() {
    let outcome = run_challenge("missing-signer", Mode::Patched).expect("patched run");
    assert!(!outcome.succeeded, "{}", outcome.summary);
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|error| error.contains("MissingRequiredSignature")),
        "unexpected error: {:?}",
        outcome.error
    );
    assert_eq!(delta(&outcome, "vault").after, 3_000_000_000);
    assert_eq!(delta(&outcome, "attacker destination").after, 0);
}

#[test]
fn missing_owner_check_vulnerable_trusts_forged_config() {
    let outcome = run_challenge("missing-owner-check", Mode::Vulnerable).expect("vulnerable run");
    assert!(outcome.succeeded, "{}", outcome.summary);
    assert_eq!(delta(&outcome, "vault").after, 0);
    assert_eq!(delta(&outcome, "attacker destination").after, 4_000_000_000);
}

#[test]
fn missing_owner_check_patched_rejects_forged_config() {
    let outcome = run_challenge("missing-owner-check", Mode::Patched).expect("patched run");
    assert!(!outcome.succeeded, "{}", outcome.summary);
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|error| error.contains("IncorrectProgramId")),
        "unexpected error: {:?}",
        outcome.error
    );
    assert_eq!(delta(&outcome, "vault").after, 4_000_000_000);
    assert_eq!(delta(&outcome, "attacker destination").after, 0);
}

#[test]
fn unsafe_close_vulnerable_claims_twice() {
    let outcome = run_challenge("unsafe-close", Mode::Vulnerable).expect("vulnerable run");
    assert!(outcome.succeeded, "{}", outcome.summary);
    assert_eq!(delta(&outcome, "vault").before, 10_000_000_000);
    assert_eq!(delta(&outcome, "vault").after, 8_000_000_000);
    assert!(
        outcome
            .notes
            .iter()
            .any(|note| note.contains("stale") || note.contains("untouched")),
        "missing stale-state evidence: {:?}",
        outcome.notes
    );
}

#[test]
fn unsafe_close_patched_blocks_second_claim() {
    let outcome = run_challenge("unsafe-close", Mode::Patched).expect("patched run");
    assert!(!outcome.succeeded, "{}", outcome.summary);
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|error| error.contains("IncorrectProgramId")),
        "unexpected error: {:?}",
        outcome.error
    );
    assert_eq!(delta(&outcome, "vault").before, 10_000_000_000);
    assert_eq!(delta(&outcome, "vault").after, 10_000_000_000);
}
