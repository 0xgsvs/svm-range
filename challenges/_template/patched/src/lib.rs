//! TODO: describe the patched program in one sentence.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

const BUMP: u8 = 0;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let mut accounts = accounts.iter();
    let counter = next_account_info(&mut accounts)?;
    let authority = next_account_info(&mut accounts)?;

    if counter.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if instruction_data != [BUMP] {
        return Err(ProgramError::InvalidInstructionData);
    }

    // FIX: the caller must prove it controls the authority account.
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut counter_data = counter.try_borrow_mut_data()?;
    let slot = counter_data
        .first_mut()
        .ok_or(ProgramError::InvalidAccountData)?;
    *slot = slot
        .checked_add(1)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    Ok(())
}
