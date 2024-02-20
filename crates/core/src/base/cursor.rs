use std::str::FromStr;

use cursor_icon::{CursorIcon, ParseError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum HaiCursor {
    Visible(CursorIcon),
    Hidden,
}

impl Serialize for HaiCursor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.name().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HaiCursor {
    fn deserialize<D>(deserializer: D) -> Result<HaiCursor, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        HaiCursor::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Default for HaiCursor {
    fn default() -> Self {
        Self::Visible(CursorIcon::Default)
    }
}

impl From<CursorIcon> for HaiCursor {
    fn from(icon: CursorIcon) -> Self {
        Self::Visible(icon)
    }
}

impl FromStr for HaiCursor {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hidden" => Ok(Self::Hidden),
            _ => Ok(Self::Visible(CursorIcon::from_str(s)?)),
        }
    }
}

impl HaiCursor {
    pub fn name(&self) -> &str {
        match self {
            Self::Visible(icon) => icon.name(),
            Self::Hidden => "hidden",
        }
    }
}

impl core::fmt::Display for HaiCursor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name())
    }
}
