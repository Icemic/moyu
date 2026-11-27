//! Embedded Lucide SVG icon set and GPUI `AssetSource`.

use std::borrow::Cow;

use gpui::{AssetSource, Hsla, Result, SharedString, Svg, prelude::*, px, svg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum IconName {
    Minus,
    Square,
    Copy,
    X,
    ChevronDown,
    ChevronUp,
    ChevronRight,
    Check,
    Folder,
    FolderOpen,
    Plus,
    Settings,
    Code,
    Terminal,
    Sparkles,
    Play,
}

impl IconName {
    pub const fn path(self) -> &'static str {
        match self {
            Self::Minus => "icons/minus.svg",
            Self::Square => "icons/square.svg",
            Self::Copy => "icons/copy.svg",
            Self::X => "icons/x.svg",
            Self::ChevronDown => "icons/chevron-down.svg",
            Self::ChevronUp => "icons/chevron-up.svg",
            Self::ChevronRight => "icons/chevron-right.svg",
            Self::Check => "icons/check.svg",
            Self::Folder => "icons/folder.svg",
            Self::FolderOpen => "icons/folder-open.svg",
            Self::Plus => "icons/plus.svg",
            Self::Settings => "icons/settings.svg",
            Self::Code => "icons/code.svg",
            Self::Terminal => "icons/terminal.svg",
            Self::Sparkles => "icons/sparkles.svg",
            Self::Play => "icons/play.svg",
        }
    }
}

pub fn icon(name: IconName, size_px: f32, color: Hsla) -> Svg {
    svg()
        .path(name.path())
        .size(px(size_px))
        .text_color(color)
        .flex_shrink_0()
}

pub struct DevtoolAssets;

impl AssetSource for DevtoolAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let bytes: Option<&'static [u8]> = match path {
            "icons/minus.svg" => Some(include_bytes!("../../../assets/icons/minus.svg")),
            "icons/square.svg" => Some(include_bytes!("../../../assets/icons/square.svg")),
            "icons/copy.svg" => Some(include_bytes!("../../../assets/icons/copy.svg")),
            "icons/x.svg" => Some(include_bytes!("../../../assets/icons/x.svg")),
            "icons/chevron-down.svg" => {
                Some(include_bytes!("../../../assets/icons/chevron-down.svg"))
            }
            "icons/chevron-up.svg" => Some(include_bytes!("../../../assets/icons/chevron-up.svg")),
            "icons/chevron-right.svg" => {
                Some(include_bytes!("../../../assets/icons/chevron-right.svg"))
            }
            "icons/check.svg" => Some(include_bytes!("../../../assets/icons/check.svg")),
            "icons/folder.svg" => Some(include_bytes!("../../../assets/icons/folder.svg")),
            "icons/folder-open.svg" => {
                Some(include_bytes!("../../../assets/icons/folder-open.svg"))
            }
            "icons/plus.svg" => Some(include_bytes!("../../../assets/icons/plus.svg")),
            "icons/settings.svg" => Some(include_bytes!("../../../assets/icons/settings.svg")),
            "icons/code.svg" => Some(include_bytes!("../../../assets/icons/code.svg")),
            "icons/terminal.svg" => Some(include_bytes!("../../../assets/icons/terminal.svg")),
            "icons/sparkles.svg" => Some(include_bytes!("../../../assets/icons/sparkles.svg")),
            "icons/play.svg" => Some(include_bytes!("../../../assets/icons/play.svg")),
            _ => None,
        };
        Ok(bytes.map(Cow::Borrowed))
    }

    fn list(&self, _path: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}
