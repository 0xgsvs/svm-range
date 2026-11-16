# Missing signer check

**Difficulty:** Easy

## Root cause

`challenges/missing-signer/vulnerable/src/lib.rs` at line 66 authenticates the
withdrawal by comparing the supplied `authority` account to the pubkey stored in
the vault:

```rust
// VULNERABILITY: authority.is_signer is never checked.
if authority.key.as_ref() != stored_authority {
    return Err(VaultError::Unauthorized.into());
}
```

A pubkey comparison proves which account was passed, not that its key signed the
transaction. Solana lets any transaction list any account as a read-only account
with `is_signer = false`. The stored authority's key is public, so an attacker
can pass it without possessing the private key.

## Exploit path

`crates/range/src/challenges/missing_signer.rs` runs this scenario:

1. Create a program-owned vault holding 3 SOL, with the victim authority stored
   in its 32-byte data.
2. Build `withdraw(3 SOL)` with the victim authority as a **read-only,
   non-signer** account, the vault as writable, and the attacker as destination.
3. Sign only with the fee payer. The victim never signs.
4. The vulnerable program checks the stored pubkey, skips `is_signer`, and
   credits the attacker.

The test proves the theft, not just transaction success: vault lamports go
`3_000_000_000 -> 0` and the attacker destination goes `0 -> 3_000_000_000`.

## Fix

`challenges/missing-signer/patched/src/lib.rs` at line 65 requires the authority
to sign before it can authorize a payout:

```rust
// FIX: the stored authority must sign, not merely appear in the account list.
if !authority.is_signer {
    return Err(ProgramError::MissingRequiredSignature);
}
```

The unsigned transaction now fails with `MissingRequiredSignature` and the vault
balance is unchanged.

## Run it

```sh
cargo run -- run missing-signer --mode vulnerable
cargo run -- run missing-signer --mode patched
```
