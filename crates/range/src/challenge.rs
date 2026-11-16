use solana_address::Address;

use crate::{Mode, error::RangeError, outcome::ExploitOutcome};

pub struct Challenge {
    pub slug: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub program_id: fn() -> Address,
    run: fn(Mode) -> Result<ExploitOutcome, RangeError>,
}

impl Challenge {
    pub const fn new(
        slug: &'static str,
        title: &'static str,
        description: &'static str,
        program_id: fn() -> Address,
        run: fn(Mode) -> Result<ExploitOutcome, RangeError>,
    ) -> Self {
        Self {
            slug,
            title,
            description,
            program_id,
            run,
        }
    }

    pub fn run(&self, mode: Mode) -> Result<ExploitOutcome, RangeError> {
        (self.run)(mode)
    }

    pub fn program_id(&self) -> Address {
        (self.program_id)()
    }
}
