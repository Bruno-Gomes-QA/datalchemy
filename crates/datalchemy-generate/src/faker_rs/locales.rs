use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocaleKey {
    EnUs,
    PtBr,
}

impl LocaleKey {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "en_US" => Some(Self::EnUs),
            "pt_BR" => Some(Self::PtBr),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::EnUs => "en_US",
            Self::PtBr => "pt_BR",
        }
    }
}

impl fmt::Display for LocaleKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
