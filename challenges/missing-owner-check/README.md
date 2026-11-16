# Missing owner check

**Difficulty:** Easy

## Root cause

`challenges/missing-owner-check/vulnerable/src/lib.rs` at line 55 reads the
treasury admin from a caller-supplied config account but never checks who owns
that account:

```rust
// VULNERABILITY: config.owner is never compared to program_id. Anyone can
// pass an account they own, containing their own pubkey as the admin.
let config_data = config.try_borrow_data()?;
let stored_admin = config_data
    .get(..ADMIN_LEN)
    .ok_or(ProgramError::InvalidAccountData)?;
if admin.key.as_ref() != stored_admin {
    return Err(TreasuryError::Unauthorized.into());
}
```

Account data is arbitrary bytes supplied by whatever program owns the account.
An attacker can create an account owned by their own program, write their own
pubkey into the first 32 bytes, and pass it as `config`. The admin comparison and
the `is_signer` check both pass, because the attacker's key really does sign and
really is the forged admin.

## Exploit path

`crates/range/src/challenges/missing_owner_check.rs` runs this scenario:

1. Create a legitimate config account owned by the treasury, plus a 4 SOL vault.
2. Create a forged config account owned by an attacker program whose data is the
   attacker's pubkey.
3. Call `withdraw(4 SOL)` with the forged config, the attacker as the signing
   admin, the vault, and an attacker destination.
4. The vulnerable program never compares `config.owner` to `program_id`, so it
   trusts the forged admin and empties the vault.

The test proves the theft: vault lamports go `4_000_000_000 -> 0` and the
attacker destination goes `0 -> 4_000_000_000`.

## Fix

`challenges/missing-owner-check/patched/src/lib.rs` at line 54 rejects config
accounts that the treasury does not own before reading their data:

```rust
// FIX: reject accounts this program does not own before trusting their data.
if config.owner != program_id {
    return Err(ProgramError::IncorrectProgramId);
}
```

The forged account is rejected with `IncorrectProgramId` and the vault stays
funded.

## Run it

```sh
cargo run -- run missing-owner-check --mode vulnerable
cargo run -- run missing-owner-check --mode patched
```
