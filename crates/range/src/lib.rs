pub mod artifact;
pub mod challenge;
pub mod challenges;
pub mod error;
pub mod harness;
pub mod new;
pub mod outcome;

mod mode;

pub use challenge::Challenge;
pub use error::{BuildError, RangeError};
pub use mode::Mode;
pub use new::{NewChallenge, create_from_template};
pub use outcome::{AccountDelta, ExploitOutcome};

pub fn all_challenges() -> &'static [Challenge] {
    challenges::ALL
}

pub fn find_challenge(slug: &str) -> Option<&'static Challenge> {
    challenges::find(slug)
}

pub fn run_challenge(slug: &str, mode: Mode) -> Result<ExploitOutcome, RangeError> {
    find_challenge(slug)
        .ok_or_else(|| RangeError::UnknownChallenge(slug.to_owned()))?
        .run(mode)
}
