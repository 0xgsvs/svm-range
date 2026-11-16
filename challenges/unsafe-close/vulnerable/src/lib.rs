//! A note program that closes a note by zeroing its lamports but leaves the
//! note's data in place, so the account can be funded again and claimed twice.
//!
//! Agave rejects owner reassignment to the system program while account data is
//! non-zero, so this program does not assign the note away; the stale note stays
//! owned by the program and is still readable after the close.

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process_instruction);

const OPEN: u8 = 0;
const CLAIM: u8 = 1;
const NOTE_LEN: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoteError {
    Unauthorized = 0,
    InsufficientFunds = 1,
    AlreadyOpen = 2,
}

impl From<NoteError> for ProgramError {
    fn from(error: NoteError) -> Self {
        ProgramError::Custom(error as u32)
    }
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.first() {
        Some(&OPEN) => open(program_id, accounts, instruction_data),
        Some(&CLAIM) => claim(program_id, accounts),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn open(program_id: &Pubkey, accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let mut accounts = accounts.iter();
    let note = next_account_info(&mut accounts)?;
    let authority = next_account_info(&mut accounts)?;

    if note.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (tag, credit) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;
    if *tag != OPEN || credit.len() != 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let credit = u64::from_le_bytes(
        credit
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    if credit == 0 {
        return Err(ProgramError::InvalidInstructionData);
    }

    {
        let note_data = note.try_borrow_data()?;
        if note_data.len() != NOTE_LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if note_data.iter().any(|byte| *byte != 0) {
            return Err(NoteError::AlreadyOpen.into());
        }
    }

    let mut note_data = note.try_borrow_mut_data()?;
    let (authority_bytes, credit_bytes) = note_data.split_at_mut(32);
    authority_bytes.copy_from_slice(authority.key.as_ref());
    credit_bytes.copy_from_slice(&credit.to_le_bytes());

    Ok(())
}

fn claim(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let mut accounts = accounts.iter();
    let note = next_account_info(&mut accounts)?;
    let authority = next_account_info(&mut accounts)?;
    let vault = next_account_info(&mut accounts)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if note.owner != program_id || vault.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    let credit = {
        let note_data = note.try_borrow_data()?;
        let (stored_authority, credit) = parse_note(&note_data)?;
        if stored_authority != authority.key.as_ref() {
            return Err(NoteError::Unauthorized.into());
        }
        credit
    };
    if credit == 0 {
        return Err(ProgramError::InvalidAccountData);
    }

    let vault_lamports = **vault.lamports.borrow();
    let remaining = vault_lamports
        .checked_sub(credit)
        .ok_or(NoteError::InsufficientFunds)?;
    let note_lamports = **note.lamports.borrow();
    let payout = credit
        .checked_add(note_lamports)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    let authority_lamports = authority.lamports();

    **vault.lamports.borrow_mut() = remaining;
    **note.lamports.borrow_mut() = 0;
    **authority.lamports.borrow_mut() = authority_lamports
        .checked_add(payout)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // VULNERABILITY: the close only zeroes lamports. The note keeps the
    // authority and credit that authorize the next claim.
    Ok(())
}

fn parse_note(data: &[u8]) -> Result<(&[u8], u64), ProgramError> {
    if data.len() != NOTE_LEN {
        return Err(ProgramError::InvalidAccountData);
    }
    let (authority, credit) = data.split_at(32);
    let credit: [u8; 8] = credit
        .try_into()
        .map_err(|_| ProgramError::InvalidAccountData)?;
    Ok((authority, u64::from_le_bytes(credit)))
}
