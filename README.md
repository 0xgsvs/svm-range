# svm-range

A small vulnerable Solana program range for security practice. Each challenge
ships a real SBF program with a bug, a deterministic LiteSVM exploit, a patched
program that fixes the root cause, and a short write-up.

## Problem

Reading about a missing signer check is not the same as writing the instruction
that drains a vault without a signature. This range keeps every bug end to end:
the vulnerable program, the exact transaction that exploits it, the state change
that proves the theft, and the minimal patch that stops it.

Three challenges are included:

| Challenge | Difficulty | Bug |
|---|---|---|
| `missing-signer` | Easy | A vault trusts a stored authority pubkey but never checks `is_signer`. |
| `missing-owner-check` | Easy | A treasury trusts a config account owned by an attacker program. |
| `unsafe-close` | Medium | A close zeroes lamports but leaves the note's data, so a funded ghost claims twice. |

## Demo

<!-- demo:start -->
```text
$ cargo run -- list
missing-signer       Missing signer check: A vault trusts a stored authority pubkey but never checks that it signed.
missing-owner-check  Missing owner check: A vault trusts a config account without checking that the program owns it.
unsafe-close         Unsafe close: A close zeroes lamports but leaves state, so a funded ghost account claims again.

$ cargo run -- run missing-signer --mode vulnerable
Challenge:  Missing signer check (missing-signer)
Mode:       vulnerable
Program id: 6URwbPipuA4MJLG7LCRRZuWnms3JZ9cRG3z9indXWz8G
Artifact:   target/sbf/missing-signer/vulnerable/missing_signer_vulnerable.so

  vault: 3000000000 -> 0 lamports
  attacker destination: 0 -> 3000000000 lamports
  note: authority DgmxzQX61DxkAMkAubrgHVJb637fYYTdh7ouVqZGnJrp was supplied with is_signer = false

the vault paid out for an authority account that never signed

EXPLOIT SUCCEEDED (vulnerable)

$ cargo run -- run missing-signer --mode patched
Challenge:  Missing signer check (missing-signer)
Mode:       patched
Program id: 6URwbPipuA4MJLG7LCRRZuWnms3JZ9cRG3z9indXWz8G
Artifact:   target/sbf/missing-signer/patched/missing_signer_patched.so

  vault: 3000000000 -> 3000000000 lamports
  attacker destination: 0 -> 0 lamports
  transaction error: InstructionError(0, MissingRequiredSignature)

the unsigned authority was rejected and the vault stayed funded

EXPLOIT BLOCKED (patched)

$ cargo run -- test
CHALLENGE              VULNERABLE           PATCHED              RESULT
missing-signer         EXPLOIT SUCCEEDED    EXPLOIT BLOCKED      PASS
missing-owner-check    EXPLOIT SUCCEEDED    EXPLOIT BLOCKED      PASS
unsafe-close           EXPLOIT SUCCEEDED    EXPLOIT BLOCKED      PASS

All 3 challenges behaved as expected.
```
<!-- demo:end -->

## Quickstart

Requirements:

- Rust 1.99.1, pinned by `rust-toolchain.toml`.
- The Agave/Solana CLI with the `cargo build-sbf` subcommand (4.x) and
  platform-tools v1.57.
- Linux or macOS. No Docker image is part of this MVP.

```sh
cargo run -- list
cargo run -- run missing-signer --mode vulnerable
cargo run -- run missing-signer --mode patched
cargo test
```

The first run of a mode compiles the challenge program with:

```sh
cargo build-sbf \
  --manifest-path challenges/<slug>/<mode>/Cargo.toml \
  --sbf-out-dir target/sbf/<slug>/<mode>
```

Artifacts are cached in `target/sbf`. The build helper reuses an artifact until
the program source or manifest is newer. Builds use `CARGO_BUILD_JOBS=3` and a
shared `target/sbf-build` directory, so the host stays within modest memory.

## Architecture

```text
.
├── crates/range/                 library, exploit runners, and `svm-range` binary
│   ├── src/artifact.rs           SBF build + freshness cache (OnceLock, build lock)
│   ├── src/harness.rs            fresh LiteSVM, deterministic payer, tx submission
│   ├── src/challenges/<slug>.rs  one deterministic exploit per challenge
│   └── tests/challenges.rs       vulnerable and patched assertions for every challenge
├── challenges/<slug>/vulnerable  standalone SBF program with the bug
├── challenges/<slug>/patched     standalone SBF program with the minimal fix
├── challenges/<slug>/README.md   root cause, exploit path, fix
└── challenges/_template/         skeleton copied by `svm-range new`
```

Program crates are standalone workspaces (`[workspace]`, edition 2021,
`solana-program` 2.x, `cdylib` + `lib`) so `cargo build-sbf` treats them as
on-chain programs and the host workspace stays on edition 2024 and the
`litesvm` 0.16 / solana-* 4.x types. The CLI passes `--sbf-out-dir`, so every
artifact lands at `target/sbf/<slug>/<mode>/<lib>.so`.

`svm-range run <slug> --mode vulnerable|patched` deploys that `.so` into a fresh
`LiteSVM`, creates the challenge accounts, submits the same exploit transaction,
and reports the account diff. The exit code is 0 only when the observed outcome
matches the requested mode: the exploit must succeed on `vulnerable` and be
blocked on `patched`.

## Adding a challenge

```sh
cargo run -- new my-challenge
```

This copies `challenges/_template` to `challenges/my-challenge` and substitutes
the slug, title, package names, and library names. Then:

1. Implement `challenges/my-challenge/vulnerable/src/lib.rs` and
   `challenges/my-challenge/patched/src/lib.rs`.
2. Copy the scaffold at `challenges/my-challenge/challenge.rs` to
   `crates/range/src/challenges/my_challenge.rs` and fill in the exploit.
3. Register the module in `crates/range/src/challenges/mod.rs`.
4. Add the root cause, exploit path, and fix to `challenges/my-challenge/README.md`.
5. Run both modes, `cargo test`, `cargo fmt --all -- --check`, and
   `cargo clippy --all-targets -- -D warnings`.

No workspace edit is required for the programs: each copied crate is a
standalone workspace and the build helper derives its paths from the slug.

## Limits

- LiteSVM is a fast simulator, not a validator. It does not model consensus,
  banking stages, or the full program-deployment pipeline.
- The programs are deliberately small teaching artifacts, not production
  templates.
- Program IDs and keypairs are fixed for reproducibility; none are real.
- The runner proves lamport and data state changes. It does not model compute
  budgets or fees beyond the fee payer.
- `unsafe-close` demonstrates the lamports-only close variant. Agave rejects
  owner reassignment to the system program while account data is non-zero, so
  the write-up documents that constraint and the exploit that remains real on
  this runtime.
- The Docker image and public index page are later milestones; this repository
  contains M1 plus three complete challenges.

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at
your option.
