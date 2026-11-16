use crate::{
    artifact::ensure_artifact,
    error::RangeError,
    harness::Harness,
    outcome::{AccountDelta, ExploitOutcome},
    Mode,
};

pub const SLUG: &str = "__SLUG__";

pub const PROGRAM_ID: solana_address::Address = solana_address::Address::new_from_array([0x00; 32]);

pub fn program_id() -> solana_address::Address {
    PROGRAM_ID
}

pub fn run(mode: Mode) -> Result<ExploitOutcome, RangeError> {
    let artifact = ensure_artifact(SLUG, mode)?;
    let mut harness = Harness::deploy(PROGRAM_ID, &artifact)?;

    let _ = &mut harness;

    Err(RangeError::Broken {
        slug: SLUG,
        detail: "template challenge is not implemented".to_owned(),
    })
}
