//! Moyu Gallery-aligned Light & Dark theme tokens for GPUI.

use gpui::{App, Global, Hsla, WindowAppearance, rgb};

use crate::config::ThemePreference;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub mode: ThemeMode,
    pub background: Hsla,
    pub surface: Hsla,
    pub surface_muted: Hsla,
    pub text: Hsla,
    pub text_secondary: Hsla,
    pub text_tertiary: Hsla,
    pub control_idle: Hsla,
    pub control_hover: Hsla,
    pub control_press: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub accent_press: Hsla,
    pub on_accent: Hsla,
    pub accent_soft: Hsla,
    pub track: Hsla,
    pub border: Hsla,
    pub success: Hsla,
    pub warn: Hsla,
    pub danger: Hsla,
    pub danger_hover: Hsla,
    pub on_danger: Hsla,
}

impl Global for Theme {}

impl Theme {
    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            background: rgb(0xECEFF0).into(),
            surface: rgb(0xFFFFFF).into(),
            surface_muted: rgb(0xF6F8F8).into(),
            text: rgb(0x1E2826).into(),
            text_secondary: rgb(0x4F5D5B).into(),
            text_tertiary: rgb(0x8B9694).into(),
            control_idle: rgb(0xF0F3F3).into(),
            control_hover: rgb(0xE5EAE9).into(),
            control_press: rgb(0xDAE1DF).into(),
            accent: rgb(0x1E6D65).into(),
            accent_hover: rgb(0x175D57).into(),
            accent_press: rgb(0x124E49).into(),
            on_accent: rgb(0xFAFCFC).into(),
            accent_soft: rgb(0xD8EEEB).into(),
            track: rgb(0xE3E9E8).into(),
            border: rgb(0xC4CFCD).into(),
            success: rgb(0x5F9B72).into(),
            warn: rgb(0xB18448).into(),
            danger: rgb(0xA66B75).into(),
            danger_hover: rgb(0x945B65).into(),
            on_danger: rgb(0xFAFCFC).into(),
        }
    }

    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            background: rgb(0x121212).into(),
            surface: rgb(0x1F1F1F).into(),
            surface_muted: rgb(0x262626).into(),
            text: rgb(0xEDEDED).into(),
            text_secondary: rgb(0xB0B0B0).into(),
            text_tertiary: rgb(0x7C7C7C).into(),
            control_idle: rgb(0x2C2C2C).into(),
            control_hover: rgb(0x363636).into(),
            control_press: rgb(0x252525).into(),
            accent: rgb(0x8AC9C1).into(),
            accent_hover: rgb(0x9ED6CE).into(),
            accent_press: rgb(0x74B4AC).into(),
            on_accent: rgb(0x13211F).into(),
            accent_soft: rgb(0x263B39).into(),
            track: rgb(0x333333).into(),
            border: rgb(0x3B3B3B).into(),
            success: rgb(0x78B88C).into(),
            warn: rgb(0xD4A66A).into(),
            danger: rgb(0xCF8B96).into(),
            danger_hover: rgb(0xDC9CA6).into(),
            on_danger: rgb(0x1C1415).into(),
        }
    }

    pub fn resolve(pref: ThemePreference, appearance: WindowAppearance) -> Self {
        match pref {
            ThemePreference::Light => Self::light(),
            ThemePreference::Dark => Self::dark(),
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
