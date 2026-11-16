mod missing_owner_check;
mod missing_signer;
mod unsafe_close;

use crate::{Mode, challenge::Challenge, error::RangeError, outcome::ExploitOutcome};

pub const ALL: &[Challenge] = &[
    Challenge::new(
        missing_signer::SLUG,
        "Missing signer check",
        "A vault trusts a stored authority pubkey but never checks that it signed.",
        missing_signer::program_id,
        missing_signer::run,
    ),
    Challenge::new(
        missing_owner_check::SLUG,
        "Missing owner check",
        "A vault trusts a config account without checking that the program owns it.",
        missing_owner_check::program_id,
        missing_owner_check::run,
    ),
    Challenge::new(
        unsafe_close::SLUG,
        "Unsafe close",
        "A close zeroes lamports but leaves state, so a funded ghost account claims again.",
        unsafe_close::program_id,
        unsafe_close::run,
    ),
];

pub fn find(slug: &str) -> Option<&'static Challenge> {
    ALL.iter().find(|challenge| challenge.slug == slug)
}

pub fn run(slug: &str, mode: Mode) -> Result<ExploitOutcome, RangeError> {
    find(slug)
        .ok_or_else(|| RangeError::UnknownChallenge(slug.to_owned()))?
        .run(mode)
}
