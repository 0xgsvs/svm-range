use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Vulnerable,
    Patched,
}

impl Mode {
    pub const ALL: [Self; 2] = [Self::Vulnerable, Self::Patched];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vulnerable => "vulnerable",
            Self::Patched => "patched",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "vulnerable" => Some(Self::Vulnerable),
            "patched" => Some(Self::Patched),
            _ => None,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
