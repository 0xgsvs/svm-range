use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountDelta {
    pub label: &'static str,
    pub before: u64,
    pub after: u64,
}

impl AccountDelta {
    pub const fn new(label: &'static str, before: u64, after: u64) -> Self {
        Self {
            label,
            before,
            after,
        }
    }

    pub const fn changed(&self) -> bool {
        self.before != self.after
    }
}

impl fmt::Display for AccountDelta {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {} -> {} lamports",
            self.label, self.before, self.after
        )
    }
}

#[derive(Clone, Debug)]
pub struct ExploitOutcome {
    pub succeeded: bool,
    pub summary: String,
    pub error: Option<String>,
    pub changes: Vec<AccountDelta>,
    pub notes: Vec<String>,
}

impl ExploitOutcome {
    pub fn succeeded(summary: impl Into<String>, changes: Vec<AccountDelta>) -> Self {
        Self {
            succeeded: true,
            summary: summary.into(),
            error: None,
            changes,
            notes: Vec::new(),
        }
    }

    pub fn blocked(
        summary: impl Into<String>,
        error: impl Into<String>,
        changes: Vec<AccountDelta>,
    ) -> Self {
        Self {
            succeeded: false,
            summary: summary.into(),
            error: Some(error.into()),
            changes,
            notes: Vec::new(),
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}
