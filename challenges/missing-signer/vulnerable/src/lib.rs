//! A vault that trusts the stored authority pubkey but never checks that the
//! account actually signed the transaction.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

const WITHDRAW: u8 = 0;
const AUTHORITY_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VaultError {
    Unauthorized = 0,
    InsufficientFunds = 1,
}

impl From<VaultError> for ProgramError {
    fn from(error: VaultError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut accounts = accounts.iter();
    let authority = next_account_info(&mut accounts)?;
    let vault = next_account_info(&mut accounts)?;
    let destination = next_account_info(&mut accounts)?;

    if vault.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if instruction_data.len() != 9 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let (tag, amount) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;
    if *tag != WITHDRAW {
        return Err(ProgramError::InvalidInstructionData);
    }
    let amount = u64::from_le_bytes(
        amount
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    let vault_data = vault.try_borrow_data()?;
    let stored_authority = vault_data
        .get(..AUTHORITY_LEN)
        .ok_or(ProgramError::InvalidAccountData)?;
    if authority.key.as_ref() != stored_authority {
        return Err(VaultError::Unauthorized.into());
    }
    drop(vault_data);

    // VULNERABILITY: authority.is_signer is never checked.
    let vault_lamports = **vault.lamports.borrow();
    let remaining = vault_lamports
        .checked_sub(amount)
        .ok_or(VaultError::InsufficientFunds)?;
    **vault.lamports.borrow_mut() = remaining;

    let destination_lamports = destination.lamports();
    **destination.lamports.borrow_mut() = destination_lamports
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    Ok(())
}
