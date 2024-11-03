use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub struct HaiLogicalSize(u32, u32);

impl HaiLogicalSize {
    pub fn width(&self) -> u32 {
        self.0
    }

    pub fn height(&self) -> u32 {
        self.1
    }

    pub fn as_tuple(&self) -> (u32, u32) {
        (self.0, self.1)
    }
}

impl std::str::FromStr for HaiLogicalSize {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('x');
        let x = parts
            .next()
            .map(|s| s.parse())
            .ok_or(anyhow::anyhow!("expected format: <width>x<height>"))??;
        let y = parts
            .next()
            .map(|s| s.parse())
            .ok_or(anyhow::anyhow!("expected format: <width>x<height>"))??;
        Ok(HaiLogicalSize(x, y))
    }
}

impl std::fmt::Display for HaiLogicalSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.0, self.1)
    }
}

impl Serialize for HaiLogicalSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HaiLogicalSize {
    fn deserialize<D>(deserializer: D) -> Result<HaiLogicalSize, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}
