use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use moyu_pal::config::{CursorConfig, CursorSource};
use moyu_pal::dir::assets_dir;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use cursor_icon::{CursorIcon, ParseError};

#[derive(Debug, Clone, Copy, PartialEq, TS)]
#[ts(
    export,
    type = "'hidden' | 'default' | 'context-menu' | 'help' | 'pointer' | 'progress' | 'wait' | 'cell' | 'crosshair' | 'text' | 'vertical-text' | 'alias' | 'copy' | 'move' | 'no-drop' | 'not-allowed' | 'grab' | 'grabbing' | 'e-resize' | 'n-resize' | 'ne-resize' | 'nw-resize' | 's-resize' | 'se-resize' | 'sw-resize' | 'w-resize' | 'ew-resize' | 'ns-resize' | 'nesw-resize' | 'nwse-resize' | 'col-resize' | 'row-resize' | 'all-scroll' | 'zoom-in' | 'zoom-out' | 'dnd-ask' | 'all-resize'"
)]
pub enum MoyuCursor {
    Visible(CursorIcon),
    Hidden,
}

impl Serialize for MoyuCursor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.name().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MoyuCursor {
    fn deserialize<D>(deserializer: D) -> Result<MoyuCursor, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        MoyuCursor::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Default for MoyuCursor {
    fn default() -> Self {
        Self::Visible(CursorIcon::Default)
    }
}

impl From<CursorIcon> for MoyuCursor {
    fn from(icon: CursorIcon) -> Self {
        Self::Visible(icon)
    }
}

impl FromStr for MoyuCursor {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "hidden" => Ok(Self::Hidden),
            _ => Ok(Self::Visible(CursorIcon::from_str(s)?)),
        }
    }
}

impl MoyuCursor {
    pub fn name(&self) -> &str {
        match self {
            Self::Visible(icon) => icon.name(),
            Self::Hidden => "hidden",
        }
    }
}

impl core::fmt::Display for MoyuCursor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.name())
    }
}

/// A decoded cursor image, ready for a window system to turn into a platform cursor.
///
/// The bytes are straight (not premultiplied) RGBA8 in row-major order, which is what
/// every backend needs; the hotspot is in image pixels.
///
/// Equality is by identity: a backend caches platform cursors per image, and comparing
/// the pixels again would mean hashing megabytes for one pointer move.
#[derive(Debug, Clone)]
pub struct CursorImage {
    pub width: u16,
    pub height: u16,
    pub hotspot_x: u16,
    pub hotspot_y: u16,
    pub rgba: Vec<u8>,
}

impl CursorImage {
    /// Hotspot inside the image, in image pixels.
    pub fn hotspot(&self) -> (u16, u16) {
        (self.hotspot_x, self.hotspot_y)
    }
}

/// Read and decode every configured image.
///
/// Entries that cannot be loaded are logged and left out, which makes them fall
/// back to `default` and then to the platform icon, exactly like a state that was
/// never configured.
pub async fn load_cursors(config: &CursorConfig) -> HashMap<CursorIcon, Arc<CursorImage>> {
    // Flatten the configuration into one entry per state.
    let entries = match config {
        CursorConfig::All(path) => vec![(CursorIcon::Default, CursorSource::Path(path.clone()))],
        CursorConfig::States(states) => {
            let mut entries = Vec::with_capacity(states.len());

            for (name, source) in states {
                let Some(source) = source else {
                    continue;
                };

                if name == "hidden" {
                    log::warn!(
                        "custom cursor 'hidden' is not a cursor state; use the `cursor` \
                         property of a node to hide the cursor"
                    );
                    continue;
                }

                match name.parse::<CursorIcon>() {
                    Ok(icon) => entries.push((icon, source.clone())),
                    Err(_) => {
                        log::warn!("ignoring custom cursor '{name}': not a known cursor state name")
                    }
                }
            }

            entries
        }
    };

    let mut images = HashMap::with_capacity(entries.len());

    // States sharing a path also share the decoded pixels; the hotspot is part of
    // the image, so entries that differ only in hotspot decode separately.
    let mut decoded: HashMap<String, Arc<CursorImage>> = HashMap::new();

    for (icon, source) in entries {
        let image = match decoded.get(source.src()) {
            Some(image) if image.hotspot() == source.hotspot() => image.clone(),
            _ => match load_image(&source).await {
                Ok(image) => {
                    let image = Arc::new(image);
                    decoded.insert(source.src().to_string(), image.clone());
                    image
                }
                Err(err) => {
                    log::warn!("custom cursor '{}' ({}): {err}", icon.name(), source.src());
                    continue;
                }
            },
        };

        images.insert(icon, image);
    }

    if !images.is_empty() {
        log::info!("loaded {} custom cursor image(s)", images.len());
    }

    images
}

/// The largest cursor image the engine will hand to a window system.
///
/// This is `winit`'s `MAX_CURSOR_SIZE`, and the only platform limit that can be
/// checked before the platform is involved. A larger image is rejected here, so no
/// unusable `CursorImage` is ever produced and backends never have to fall back.
const MAX_CURSOR_SIZE: u32 = 2048;

/// Read, decode and validate one cursor image.
async fn load_image(source: &CursorSource) -> anyhow::Result<CursorImage> {
    let src = source.src();
    let url = assets_dir().join(src)?;
    let bytes = moyu_pal::fs::read(&url).await?;
    let image = moyu_image::decode(&bytes)?;

    let (width, height) = (image.width(), image.height());
    if width > MAX_CURSOR_SIZE || height > MAX_CURSOR_SIZE {
        anyhow::bail!("{width}x{height} exceeds the {MAX_CURSOR_SIZE}px cursor limit");
    }

    // The limit above keeps both dimensions inside `u16`.
    let (width, height) = (width as u16, height as u16);

    let (hotspot_x, hotspot_y) = source.hotspot();
    if hotspot_x >= width || hotspot_y >= height {
        anyhow::bail!("hotspot ({hotspot_x}, {hotspot_y}) is outside the {width}x{height} image");
    }

    Ok(CursorImage {
        width,
        height,
        hotspot_x,
        hotspot_y,
        rgba: image.to_compact().into_owned(),
    })
}
