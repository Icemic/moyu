//! FishFlow-aligned theme tokens (Light, Dark, Solarized Light, One Dark) for GPUI.

use gpui::{App, Global, Hsla, WindowAppearance, rgb, rgba};

use crate::config::ThemePreference;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeKind {
    Light,
    Dark,
    SolarizedLight,
    OneDark,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub kind: ThemeKind,
    pub background: Hsla,
    pub foreground: Hsla,
    pub card: Hsla,
    pub card_foreground: Hsla,
    pub popover: Hsla,
    pub popover_foreground: Hsla,
    pub primary: Hsla,
    pub primary_hover: Hsla,
    pub primary_foreground: Hsla,
    pub secondary: Hsla,
    pub secondary_hover: Hsla,
    pub secondary_foreground: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub meta: Hsla,
    pub accent: Hsla,
    pub accent_foreground: Hsla,
    pub destructive: Hsla,
    pub destructive_hover: Hsla,
    pub destructive_foreground: Hsla,
    pub border: Hsla,
    pub rule: Hsla,
    pub input: Hsla,
    pub ring: Hsla,
    pub success: Hsla,
    pub warning: Hsla,
    pub info: Hsla,
    pub selection_bg: Hsla,
    pub backdrop: Hsla,
}

impl Global for Theme {}

impl Theme {
    pub fn light() -> Self {
        Self {
            kind: ThemeKind::Light,
            background: rgb(0xFFFFFF).into(),
            foreground: rgb(0x0A0A0A).into(),
            card: rgb(0xFFFFFF).into(),
            card_foreground: rgb(0x0A0A0A).into(),
            popover: rgb(0xFFFFFF).into(),
            popover_foreground: rgb(0x0A0A0A).into(),
            primary: rgb(0x171717).into(),
            primary_hover: rgb(0x2E2E2E).into(),
            primary_foreground: rgb(0xFAFAFA).into(),
            secondary: rgb(0xE8E8E8).into(),
            secondary_hover: rgb(0xDCDCDC).into(),
            secondary_foreground: rgb(0x171717).into(),
            muted: rgb(0xEBEBEB).into(),
            muted_foreground: rgb(0x737373).into(),
            meta: rgb(0x5A5A5A).into(),
            accent: rgb(0xE5E5E5).into(),
            accent_foreground: rgb(0x171717).into(),
            destructive: rgb(0xDC2626).into(),
            destructive_hover: rgb(0xB91C1C).into(),
            destructive_foreground: rgb(0xFFFFFF).into(),
            border: rgb(0xE0E0E0).into(),
            rule: rgb(0xD8D8D8).into(),
            input: rgb(0xDCDCDC).into(),
            ring: rgb(0x737373).into(),
            success: rgb(0x10B981).into(),
            warning: rgb(0xCA8A04).into(),
            info: rgb(0x7C3AED).into(),
            selection_bg: rgba(0x3B82F633).into(),
            backdrop: rgba(0x00000044).into(),
        }
    }

    pub fn dark() -> Self {
        Self {
            kind: ThemeKind::Dark,
            background: rgb(0x0A0A0A).into(),
            foreground: rgb(0xFAFAFA).into(),
            card: rgb(0x171717).into(),
            card_foreground: rgb(0xFAFAFA).into(),
            popover: rgb(0x171717).into(),
            popover_foreground: rgb(0xFAFAFA).into(),
            primary: rgb(0xE5E5E5).into(),
            primary_hover: rgb(0xCCCCCC).into(),
            primary_foreground: rgb(0x171717).into(),
            secondary: rgb(0x262626).into(),
            secondary_hover: rgb(0x323232).into(),
            secondary_foreground: rgb(0xFAFAFA).into(),
            muted: rgb(0x222222).into(),
            muted_foreground: rgb(0xA1A1A1).into(),
            meta: rgb(0xB5B5B5).into(),
            accent: rgb(0x2A2A2A).into(),
            accent_foreground: rgb(0xFAFAFA).into(),
            destructive: rgb(0xEF4444).into(),
            destructive_hover: rgb(0xDC2626).into(),
            destructive_foreground: rgb(0xFFFFFF).into(),
            border: rgb(0x262626).into(),
            rule: rgb(0x2C2C2C).into(),
            input: rgb(0x333333).into(),
            ring: rgb(0x737373).into(),
            success: rgb(0x10B981).into(),
            warning: rgb(0xEAB308).into(),
            info: rgb(0x8B5CF6).into(),
            selection_bg: rgba(0x528BFF44).into(),
            backdrop: rgba(0x00000077).into(),
        }
    }

    pub fn solarized_light() -> Self {
        Self {
            kind: ThemeKind::SolarizedLight,
            background: rgb(0xFDF6E3).into(),
            foreground: rgb(0x657B83).into(),
            card: rgb(0xEEE8D5).into(),
            card_foreground: rgb(0x657B83).into(),
            popover: rgb(0xFDF6E3).into(),
            popover_foreground: rgb(0x657B83).into(),
            primary: rgb(0x586E75).into(),
            primary_hover: rgb(0x475B62).into(),
            primary_foreground: rgb(0xFDF6E3).into(),
            secondary: rgb(0xE6DFCB).into(),
            secondary_hover: rgb(0xDDD5BF).into(),
            secondary_foreground: rgb(0x586E75).into(),
            muted: rgb(0xEEE8D5).into(),
            muted_foreground: rgb(0x93A1A1).into(),
            meta: rgb(0x7F8E91).into(),
            accent: rgb(0xE3DCC6).into(),
            accent_foreground: rgb(0x586E75).into(),
            destructive: rgb(0xDC322F).into(),
            destructive_hover: rgb(0xC12926).into(),
            destructive_foreground: rgb(0xFDF6E3).into(),
            border: rgb(0xDDD6C1).into(),
            rule: rgb(0xD6CEB8).into(),
            input: rgb(0xD6CEB8).into(),
            ring: rgb(0x268BD2).into(),
            success: rgb(0x859900).into(),
            warning: rgb(0xB58900).into(),
            info: rgb(0x268BD2).into(),
            selection_bg: rgba(0x268BD233).into(),
            backdrop: rgba(0x00000033).into(),
        }
    }

    pub fn one_dark() -> Self {
        Self {
            kind: ThemeKind::OneDark,
            background: rgb(0x282C34).into(),
            foreground: rgb(0xABB2BF).into(),
            card: rgb(0x21252B).into(),
            card_foreground: rgb(0xABB2BF).into(),
            popover: rgb(0x21252B).into(),
            popover_foreground: rgb(0xABB2BF).into(),
            primary: rgb(0x404754).into(),
            primary_hover: rgb(0x4C5464).into(),
            primary_foreground: rgb(0xF8FAFD).into(),
            secondary: rgb(0x30333D).into(),
            secondary_hover: rgb(0x393E4A).into(),
            secondary_foreground: rgb(0xC0BDBD).into(),
            muted: rgb(0x2C313C).into(),
            muted_foreground: rgb(0x7F848E).into(),
            meta: rgb(0x8E94A0).into(),
            accent: rgb(0x323842).into(),
            accent_foreground: rgb(0xD7DAE0).into(),
            destructive: rgb(0xC24038).into(),
            destructive_hover: rgb(0xD14B43).into(),
            destructive_foreground: rgb(0xFFFFFF).into(),
            border: rgba(0x00000000).into(),
            rule: rgb(0x323741).into(),
            input: rgb(0x1D1F23).into(),
            ring: rgb(0x528BFF).into(),
            success: rgb(0x98C379).into(),
            warning: rgb(0xD19A66).into(),
            info: rgb(0x61AFEF).into(),
            selection_bg: rgba(0x528BFF44).into(),
            backdrop: rgba(0x00000066).into(),
        }
    }

    pub fn resolve(pref: ThemePreference, appearance: WindowAppearance) -> Self {
        match pref {
            ThemePreference::Light => Self::light(),
            ThemePreference::Dark => Self::dark(),
            ThemePreference::SolarizedLight => Self::solarized_light(),
            ThemePreference::OneDark => Self::one_dark(),
            ThemePreference::System => match appearance {
                WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
                _ => Self::light(),
            },
        }
    }

    pub fn global(cx: &App) -> Self {
        *cx.global::<Self>()
    }
}
