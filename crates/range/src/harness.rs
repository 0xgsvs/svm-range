use std::path::Path;

use litesvm::{LiteSVM, types::TransactionMetadata};
use solana_account::Account;
use solana_address::Address;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_message::Message;
use solana_signer::Signer;
use solana_transaction::Transaction;
use solana_transaction_error::TransactionError;

use crate::error::RangeError;

const PAYER_SEED: [u8; 32] = [0x77; 32];
const AIRDROP_LAMPORTS: u64 = 100 * 1_000_000_000;

pub struct Harness {
    pub svm: LiteSVM,
    pub payer: Keypair,
    pub payer_address: Address,
}

impl Harness {
    pub fn deploy(program_id: Address, artifact: &Path) -> Result<Self, RangeError> {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(program_id, artifact)
            .map_err(|error| {
                RangeError::Svm(format!(
                    "failed to deploy {} at {program_id}: {error}",
                    artifact.display()
                ))
            })?;

        let payer = Keypair::new_from_array(PAYER_SEED);
        let payer_address = payer.pubkey();
        svm.airdrop(&payer_address, AIRDROP_LAMPORTS)
            .map_err(|error| RangeError::Svm(format!("airdrop failed: {:?}", error.err)))?;

        Ok(Self {
            svm,
            payer,
            payer_address,
        })
    }

    pub fn keypair(seed: u8) -> Keypair {
        Keypair::new_from_array([seed; 32])
    }

    pub fn set_account(
        &mut self,
        address: Address,
        lamports: u64,
        data: Vec<u8>,
        owner: Address,
    ) -> Result<(), RangeError> {
        self.svm
            .set_account(
                address,
                Account {
                    lamports,
                    data,
                    owner,
                    ..Account::default()
                },
            )
            .map_err(|error| {
                RangeError::Svm(format!("failed to create account {address}: {error}"))
            })
    }

    pub fn execute(
        &mut self,
        instructions: &[Instruction],
    ) -> Result<TransactionMetadata, TransactionError> {
        self.execute_signed(instructions, &[])
    }

    pub fn execute_signed(
        &mut self,
        instructions: &[Instruction],
        signers: &[&Keypair],
    ) -> Result<TransactionMetadata, TransactionError> {
        let mut all_signers: Vec<&Keypair> = Vec::with_capacity(signers.len() + 1);
        all_signers.push(&self.payer);
        all_signers.extend_from_slice(signers);

        let blockhash = self.svm.latest_blockhash();
        let message =
            Message::new_with_blockhash(instructions, Some(&self.payer_address), &blockhash);
        let transaction = Transaction::new(all_signers.as_slice(), message, blockhash);
        self.svm
            .send_transaction(transaction)
            .map_err(|failed| failed.err)
    }

    pub fn lamports(&self, address: &Address) -> u64 {
        self.svm
            .get_account(address)
            .map_or(0, |account| account.lamports)
    }

    pub fn rent_exemption(&self, data_len: usize) -> u64 {
        self.svm.minimum_balance_for_rent_exemption(data_len)
    }
}
