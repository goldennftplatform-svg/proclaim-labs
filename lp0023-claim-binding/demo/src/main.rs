// SPDX-License-Identifier: MIT AND Apache-2.0

//! Local rejection demo for the deployer-claim binding MODEL.
//!
//! Not real Ed25519, not on-chain, no LEZ/IDL/deployment, not criterion complete.

use claim_core::{
    claim_digest, Claim, ClaimRegistry, DeployerId, ProgramId, Signature, VerificationResult,
};
use signer::{KeyPair, TrustedDeployerDirectory};

struct Scenario {
    index: u8,
    title: &'static str,
    deployer_label: &'static str,
    claim: Claim,
    signature: Signature,
    result: VerificationResult,
    entries_after: usize,
}

struct DemoOutcome {
    scenarios: Vec<Scenario>,
    by_program_count: usize,
    by_deployer_count: usize,
}

fn run_scenarios() -> DemoOutcome {
    let alice = KeyPair::from_seed(1);
    let bob = KeyPair::from_seed(2);
    let alice_id = DeployerId::from_label("alice");
    let bob_id = DeployerId::from_label("bob");
    let mallory_id = DeployerId::from_label("mallory-unknown");

    let mut directory = TrustedDeployerDirectory::new();
    directory.insert(alice_id.clone(), alice.public_key());
    directory.insert(bob_id, bob.public_key());

    let mut registry = ClaimRegistry::new(directory);

    let program_alpha = ProgramId::from_label("program-alpha");
    let program_beta = ProgramId::from_label("program-beta");
    let program_gamma = ProgramId::from_label("program-gamma");

    // (a) Honest claim, signed by the key the directory trusts for alice.
    let valid_claim = Claim::new(
        program_alpha.clone(),
        alice_id.clone(),
        "I deployed program-alpha (alice).",
        1_700_000_000_000,
    );
    let valid_signature = alice.sign(&claim_digest(&valid_claim));
    let valid_result = registry.submit(&valid_claim, &valid_signature);
    let valid_entries = registry.len();

    // (b) Claim says alice, but the signature was made with bob's key.
    let forged_claim = Claim::new(
        program_beta,
        alice_id.clone(),
        "I deployed program-beta (alice) -- signed with BOB key.",
        1_700_000_000_001,
    );
    let forged_signature = bob.sign(&claim_digest(&forged_claim));
    let forged_result = registry.submit(&forged_claim, &forged_signature);
    let forged_entries = registry.len();

    // (c) Deployer is not present in the trusted directory at all.
    let unknown_claim = Claim::new(
        program_gamma,
        mallory_id.clone(),
        "I deployed program-gamma (mallory, not in directory).",
        1_700_000_000_002,
    );
    let unknown_signature = bob.sign(&claim_digest(&unknown_claim));
    let unknown_result = registry.submit(&unknown_claim, &unknown_signature);
    let unknown_entries = registry.len();

    // (d) Same (program_id, deployer) pair as scenario 1, different statement
    // and timestamp, with a valid signature: still a duplicate.
    let duplicate_claim = Claim::new(
        program_alpha.clone(),
        alice_id.clone(),
        "RE-SUBMIT: I deployed program-alpha (alice).",
        1_700_000_000_009,
    );
    let duplicate_signature = alice.sign(&claim_digest(&duplicate_claim));
    let duplicate_result = registry.submit(&duplicate_claim, &duplicate_signature);
    let duplicate_entries = registry.len();

    let by_program_count = registry.by_program(&program_alpha).len();
    let by_deployer_count = registry.by_deployer(&alice_id).len();

    DemoOutcome {
        scenarios: vec![
            Scenario {
                index: 1,
                title: "VALID CLAIM (alice signs with her trusted key)",
                deployer_label: "alice (trusted)",
                claim: valid_claim,
                signature: valid_signature,
                result: valid_result,
                entries_after: valid_entries,
            },
            Scenario {
                index: 2,
                title: "FORGED SIGNATURE (claims alice, signed with bob's key)",
                deployer_label: "alice (trusted)",
                claim: forged_claim,
                signature: forged_signature,
                result: forged_result,
                entries_after: forged_entries,
            },
            Scenario {
                index: 3,
                title: "UNKNOWN DEPLOYER (mallory is not in the directory)",
                deployer_label: "mallory (unknown)",
                claim: unknown_claim,
                signature: unknown_signature,
                result: unknown_result,
                entries_after: unknown_entries,
            },
            Scenario {
                index: 4,
                title: "DUPLICATE (same program_id + deployer as scenario 1)",
                deployer_label: "alice (trusted)",
                claim: duplicate_claim,
                signature: duplicate_signature,
                result: duplicate_result,
                entries_after: duplicate_entries,
            },
        ],
        by_program_count,
        by_deployer_count,
    }
}

fn hex_prefix(bytes: &[u8], count: usize) -> String {
    bytes
        .iter()
        .take(count)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn print_scenario(scenario: &Scenario) {
    println!("[{}] {}", scenario.index, scenario.title);
    println!("    program_id            = {}", scenario.claim.program_id);
    println!(
        "    deployer              = {} ({})",
        scenario.claim.deployer, scenario.deployer_label
    );
    println!("    statement             = {:?}", scenario.claim.statement);
    println!(
        "    timestamp_ms          = {}",
        scenario.claim.timestamp_ms
    );
    println!(
        "    claim_digest          = {}",
        claim_digest(&scenario.claim)
    );
    println!(
        "    signature (first 8 B) = {}",
        hex_prefix(scenario.signature.as_bytes(), 8)
    );
    println!("    registry result       = {}", scenario.result);
    println!(
        "    rejected              = {}",
        scenario.result != VerificationResult::Valid
    );
    println!("    entries after         = {}", scenario.entries_after);
    println!();
}

fn main() {
    let outcome = run_scenarios();
    let accepted = outcome
        .scenarios
        .iter()
        .filter(|scenario| scenario.result == VerificationResult::Valid)
        .count();
    let rejected = outcome
        .scenarios
        .iter()
        .filter(|scenario| scenario.result != VerificationResult::Valid)
        .count();

    println!("=== LP-0023 deployer-claim binding: LOCAL MODEL REJECTION DEMO ===");
    println!("WARNING: toy signature MODEL, NOT real Ed25519; no chain, no LEZ, no IDL.");
    println!("WARNING: local demonstration only; NOT criterion complete.");
    println!();

    for scenario in &outcome.scenarios {
        print_scenario(scenario);
    }

    println!("=== SUMMARY ===");
    println!("accepted                  = {accepted}");
    println!("rejected                  = {rejected}");
    println!("queries: by program_alpha = {}", outcome.by_program_count);
    println!("queries: by alice         = {}", outcome.by_deployer_count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_produces_all_four_expected_results() {
        let outcome = run_scenarios();
        let results: Vec<VerificationResult> = outcome
            .scenarios
            .iter()
            .map(|scenario| scenario.result)
            .collect();
        assert_eq!(
            results,
            vec![
                VerificationResult::Valid,
                VerificationResult::InvalidSignature,
                VerificationResult::UnknownDeployer,
                VerificationResult::Duplicate,
            ]
        );
    }

    #[test]
    fn demo_rejections_do_not_grow_the_registry() {
        let outcome = run_scenarios();
        let counts: Vec<usize> = outcome
            .scenarios
            .iter()
            .map(|scenario| scenario.entries_after)
            .collect();
        assert_eq!(counts, vec![1, 1, 1, 1]);
        assert_eq!(outcome.by_program_count, 1);
        assert_eq!(outcome.by_deployer_count, 1);
    }

    #[test]
    fn demo_duplicate_differs_in_digest_but_not_in_binding_pair() {
        let outcome = run_scenarios();
        let first = &outcome.scenarios[0];
        let duplicate = &outcome.scenarios[3];
        assert_eq!(first.claim.program_id, duplicate.claim.program_id);
        assert_eq!(first.claim.deployer, duplicate.claim.deployer);
        assert_ne!(first.claim.statement, duplicate.claim.statement);
        assert_ne!(claim_digest(&first.claim), claim_digest(&duplicate.claim));
    }
}
