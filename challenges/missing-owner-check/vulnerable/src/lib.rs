//! A treasury that reads its admin from a config account without checking
//! that the config account belongs to this program.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

const WITHDRAW: u8 = 0;
const ADMIN_LEN: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TreasuryError {
    Unauthorized = 0,
    InsufficientFunds = 1,
}

impl From<TreasuryError> for ProgramError {
    fn from(error: TreasuryError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut accounts = accounts.iter();
    let config = next_account_info(&mut accounts)?;
    let admin = next_account_info(&mut accounts)?;
    let vault = next_account_info(&mut accounts)?;
    let destination = next_account_info(&mut accounts)?;

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

    // VULNERABILITY: config.owner is never compared to program_id. Anyone can
    // pass an account they own, containing their own pubkey as the admin.
    let config_data = config.try_borrow_data()?;
    let stored_admin = config_data
        .get(..ADMIN_LEN)
        .ok_or(ProgramError::InvalidAccountData)?;
    if admin.key.as_ref() != stored_admin {
        return Err(TreasuryError::Unauthorized.into());
    }
    drop(config_data);

    if !admin.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if vault.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let vault_lamports = **vault.lamports.borrow();
    let remaining = vault_lamports
        .checked_sub(amount)
        .ok_or(TreasuryError::InsufficientFunds)?;
    **vault.lamports.borrow_mut() = remaining;

    let destination_lamports = destination.lamports();
    **destination.lamports.borrow_mut() = destination_lamports
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    Ok(())
}
