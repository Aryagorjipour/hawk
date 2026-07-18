use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Locale {
    #[default]
    En,
    Fa,
}

impl Locale {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fa => "fa",
        }
    }

    pub fn parse(s: &str) -> Self {
        let base = s.split(['-', '_']).next().unwrap_or(s);
        match base.to_ascii_lowercase().as_str() {
            "fa" | "per" | "pes" => Self::Fa,
            _ => Self::En,
        }
    }

    pub fn from_telegram_language_code(code: Option<&str>) -> Self {
        match code {
            Some(c) => Self::parse(c),
            None => Self::En,
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_persian_variants() {
        assert_eq!(Locale::parse("fa"), Locale::Fa);
        assert_eq!(Locale::parse("fa-IR"), Locale::Fa);
        assert_eq!(Locale::parse("en-US"), Locale::En);
    }
}
