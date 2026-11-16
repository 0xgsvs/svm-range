# Unsafe close

**Difficulty:** Medium

## Root cause

`challenges/unsafe-close/vulnerable/src/lib.rs` at line 128 pays a note and then
"closes" it by zeroing its lamports:

```rust
**vault.lamports.borrow_mut() = remaining;
**note.lamports.borrow_mut() = 0;
**authority.lamports.borrow_mut() = authority_lamports
    .checked_add(payout)
    .ok_or(ProgramError::ArithmeticOverflow)?;

// VULNERABILITY: the close only zeroes lamports. The note keeps the
// authority and credit that authorize the next claim.
```

Zero lamports makes the account look closed, but its 40 bytes still contain the
authority pubkey and the paid credit. The program treats that data as proof of a
live note, so an attacker can bundle three instructions in one transaction:

1. `claim` pays the credit and zeroes the note's lamports.
2. A system transfer funds the zero-lamport note with its rent-exempt minimum.
3. `claim` again reads the stale data and pays the same credit a second time.

The account is never cleared, only made temporarily poor. Funding it again is
enough to resurrect the authorization.

### Why this variant

The textbook close also assigns the account to the system program. Agave 4.x
rejects that when the data is not zeroed: the runtime returns `ModifiedProgramId`
because owner reassignment requires zero-initialized data. The bug therefore
cannot be demonstrated by assigning a stale account away on this runtime. The
remaining real instance of the same root cause is a lamports-only close that
leaves the account program-owned with live data. It is the same missing state
clear, and it is exploitable without any runtime exemption.

A cross-transaction re-fund is not used because the SVM purges zero-lamport
accounts at the end of the transaction. Bundling the re-fund with the second
claim is realistic, deterministic, and keeps the stale write visible.

## Exploit path

`crates/range/src/challenges/unsafe_close.rs` runs this scenario:

1. Create a program-owned note with `authority = attacker` and `credit = 1 SOL`,
   funded with rent, plus a 10 SOL vault.
2. Submit one transaction with `claim`, a system transfer of the note's rent,
   and `claim` again.
3. The vulnerable program pays 1 SOL on the first claim, then accepts the same
   stale note data after the transfer and pays another 1 SOL.

The same transaction also returns the note's rent twice to the attacker. The
test proves the double payout by asserting the vault goes
`10_000_000_000 -> 8_000_000_000` instead of stopping at 9 SOL.

## Fix

`challenges/unsafe-close/patched/src/lib.rs` at line 128 destroys the state
before the account is closed:

```rust
// FIX: clear the data, shrink it to zero, and only then close the account.
{
    let mut note_data = note.try_borrow_mut_data()?;
    note_data.fill(0);
}
note.resize(0)?;
note.assign(&solana_program::system_program::ID);
**note.lamports.borrow_mut() = 0;
```

The note becomes a zero-lamport, zero-length system account. The transfer in the
bundle funds an empty system account, and the second claim fails with
`IncorrectProgramId`. Because the bundle is atomic, the whole transaction
reverts and the vault stays at 10 SOL.

## Run it

```sh
cargo run -- run unsafe-close --mode vulnerable
cargo run -- run unsafe-close --mode patched
```
