# __TITLE__

**Difficulty:** TODO

## Root cause

`__SLUG__/vulnerable/src/lib.rs`:

```rust
// Quote the exact vulnerable lines here.
```

Explain which runtime guarantee the program trusts without verifying.

## Exploit path

Describe the accounts and instruction data the attacker controls, and the state
change that proves the exploit. The runner for this challenge lives in
`crates/range/src/challenges/__SLUG_SNAKE__.rs`; it must be deterministic.

## Fix

`__SLUG__/patched/src/lib.rs`:

```rust
// Quote the exact fix here.
```

## Run it

```sh
cargo run -- run __SLUG__ --mode vulnerable
cargo run -- run __SLUG__ --mode patched
```
