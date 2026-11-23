//! Design tokens generated from `design/scripts/tokens.py`.
//!
//! Run `python3 design/scripts/sync_tokens.py` after updating token values.
#![allow(dead_code)]

use gpui_kit::{Rgba, rgb};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Page background
    pub canvas: u32,
    /// Card and panel surface
    pub surface: u32,
    /// Recessed surface inside a card
    pub surface_raised: u32,
    /// Column and track background
    pub surface_sunken: u32,
    /// Navigation surface
    pub surface_inverse: u32,
    /// Raised navigation surface
    pub surface_inverse_raised: u32,
    /// Selected navigation item
    pub surface_selected: u32,
    /// Default hairline border
    pub border: u32,
    /// Emphasised border
    pub border_strong: u32,
    /// Primary text
    pub text: u32,
    /// Secondary text
    pub text_secondary: u32,
    /// Muted text and labels
    pub text_muted: u32,
    /// Faintest text
    pub text_faint: u32,
    /// Text on dark surfaces
    pub text_inverse: u32,
    /// Text on the accent colour
    pub text_on_accent: u32,
    /// Primary accent
    pub accent: u32,
    /// Primary accent hover
    pub accent_hover: u32,
    /// Accent surface
    pub accent_muted: u32,
    /// Quiet accent surface
    pub accent_quiet: u32,
    /// Accent coloured text
    pub accent_text: u32,
    /// Secondary accent text
    pub accent_soft_text: u32,
    /// Needs attention
    pub warning: u32,
    /// Needs attention surface
    pub warning_muted: u32,
    /// Needs attention text
    pub warning_text: u32,
    /// Destructive action
    pub danger: u32,
    /// Destructive surface
    pub danger_muted: u32,
    /// Informational surface
    pub info: u32,
    /// Informational text
    pub info_text: u32,
    /// Secondary accent surface
    pub violet_muted: u32,
    /// Reserved time surface
    pub yellow_muted: u32,
    /// Default icon colour
    pub icon: u32,
    /// Muted icon colour
    pub icon_muted: u32,
    /// Navigation icon colour
    pub icon_inverse: u32,
    /// Active navigation icon colour
    pub icon_inverse_active: u32,
    /// Icon on the accent colour
    pub icon_on_accent: u32,
    /// Secondary text on dark surfaces
    pub icon_on_inverse: u32,
}

impl Palette {
    pub const fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => LIGHT,
            ThemeMode::Dark => DARK,
        }
    }
    pub fn canvas(&self) -> Rgba {
        rgb(self.canvas)
    }
    pub fn surface(&self) -> Rgba {
        rgb(self.surface)
    }
    pub fn surface_raised(&self) -> Rgba {
        rgb(self.surface_raised)
    }
    pub fn surface_sunken(&self) -> Rgba {
        rgb(self.surface_sunken)
    }
    pub fn surface_inverse(&self) -> Rgba {
        rgb(self.surface_inverse)
    }
    pub fn surface_inverse_raised(&self) -> Rgba {
        rgb(self.surface_inverse_raised)
    }
    pub fn surface_selected(&self) -> Rgba {
        rgb(self.surface_selected)
    }
    pub fn border(&self) -> Rgba {
        rgb(self.border)
    }
    pub fn border_strong(&self) -> Rgba {
        rgb(self.border_strong)
    }
    pub fn text(&self) -> Rgba {
        rgb(self.text)
    }
    pub fn text_secondary(&self) -> Rgba {
        rgb(self.text_secondary)
    }
    pub fn text_muted(&self) -> Rgba {
        rgb(self.text_muted)
    }
    pub fn text_faint(&self) -> Rgba {
        rgb(self.text_faint)
    }
    pub fn text_inverse(&self) -> Rgba {
        rgb(self.text_inverse)
    }
    pub fn text_on_accent(&self) -> Rgba {
        rgb(self.text_on_accent)
    }
    pub fn accent(&self) -> Rgba {
        rgb(self.accent)
    }
    pub fn accent_hover(&self) -> Rgba {
        rgb(self.accent_hover)
    }
    pub fn accent_muted(&self) -> Rgba {
        rgb(self.accent_muted)
    }
    pub fn accent_quiet(&self) -> Rgba {
        rgb(self.accent_quiet)
    }
    pub fn accent_text(&self) -> Rgba {
        rgb(self.accent_text)
    }
    pub fn accent_soft_text(&self) -> Rgba {
        rgb(self.accent_soft_text)
    }
    pub fn warning(&self) -> Rgba {
        rgb(self.warning)
    }
    pub fn warning_muted(&self) -> Rgba {
        rgb(self.warning_muted)
    }
    pub fn warning_text(&self) -> Rgba {
        rgb(self.warning_text)
    }
    pub fn danger(&self) -> Rgba {
        rgb(self.danger)
    }
    pub fn danger_muted(&self) -> Rgba {
        rgb(self.danger_muted)
    }
    pub fn info(&self) -> Rgba {
        rgb(self.info)
    }
    pub fn info_text(&self) -> Rgba {
        rgb(self.info_text)
    }
    pub fn violet_muted(&self) -> Rgba {
        rgb(self.violet_muted)
    }
    pub fn yellow_muted(&self) -> Rgba {
        rgb(self.yellow_muted)
    }
    pub fn icon(&self) -> Rgba {
        rgb(self.icon)
    }
    pub fn icon_muted(&self) -> Rgba {
        rgb(self.icon_muted)
    }
    pub fn icon_inverse(&self) -> Rgba {
        rgb(self.icon_inverse)
    }
    pub fn icon_inverse_active(&self) -> Rgba {
        rgb(self.icon_inverse_active)
    }
    pub fn icon_on_accent(&self) -> Rgba {
        rgb(self.icon_on_accent)
    }
    pub fn icon_on_inverse(&self) -> Rgba {
        rgb(self.icon_on_inverse)
    }
}

pub const LIGHT: Palette = Palette {
    canvas: 0xF6F7F5,
    surface: 0xFFFFFF,
    surface_raised: 0xF5F8F5,
    surface_sunken: 0xEFF2F0,
    surface_inverse: 0x162929,
    surface_inverse_raised: 0x243B3A,
    surface_selected: 0x335A4B,
    border: 0xE4E9E6,
    border_strong: 0xD5DDD8,
    text: 0x19292A,
    text_secondary: 0x314245,
    text_muted: 0x647779,
    text_faint: 0x8E9E9D,
    text_inverse: 0xFFFFFF,
    text_on_accent: 0xFFFFFF,
    accent: 0x266C56,
    accent_hover: 0x1E5B48,
    accent_muted: 0xDDEEE5,
    accent_quiet: 0xB7D7C8,
    accent_text: 0x266C56,
    accent_soft_text: 0x3B9571,
    warning: 0xC16A4C,
    warning_muted: 0xF5E8DD,
    warning_text: 0xC16A4C,
    danger: 0xA64F47,
    danger_muted: 0xF9F1EF,
    info: 0xDFEBF2,
    info_text: 0x314245,
    violet_muted: 0xEAE6F4,
    yellow_muted: 0xF3E8C8,
    icon: 0x19292A,
    icon_muted: 0x8E9E9D,
    icon_inverse: 0x7CA69A,
    icon_inverse_active: 0xECF7F1,
    icon_on_accent: 0xFFFFFF,
    icon_on_inverse: 0xA8BFB7,
};

pub const DARK: Palette = Palette {
    canvas: 0x112124,
    surface: 0x1C3132,
    surface_raised: 0x203638,
    surface_sunken: 0x182B2C,
    surface_inverse: 0x0B191C,
    surface_inverse_raised: 0x122A2C,
    surface_selected: 0x274F45,
    border: 0x334747,
    border_strong: 0x415756,
    text: 0xEDF6F0,
    text_secondary: 0xD5E5DC,
    text_muted: 0xA4B9B0,
    text_faint: 0x849D92,
    text_inverse: 0xF1F8F3,
    text_on_accent: 0x0E1F1B,
    accent: 0x419677,
    accent_hover: 0x57A88A,
    accent_muted: 0x2B5448,
    accent_quiet: 0x3D7364,
    accent_text: 0x75B49C,
    accent_soft_text: 0x8FC4AE,
    warning: 0xECAA89,
    warning_muted: 0x4C3830,
    warning_text: 0xECAA89,
    danger: 0xE08C82,
    danger_muted: 0x4A3230,
    info: 0x294653,
    info_text: 0xA9CBD8,
    violet_muted: 0x3B3552,
    yellow_muted: 0x4A4128,
    icon: 0xEDF6F0,
    icon_muted: 0x849D92,
    icon_inverse: 0x8FB3A5,
    icon_inverse_active: 0xE7F6ED,
    icon_on_accent: 0x0E1F1B,
    icon_on_inverse: 0x9FBBB0,
};

/// Small control
pub const RADIUS_SM: f32 = 6.0;
/// Button and chip
pub const RADIUS_MD: f32 = 9.0;
/// Card
pub const RADIUS_LG: f32 = 13.0;
/// Dialog
pub const RADIUS_XL: f32 = 17.0;
/// Fully rounded
pub const RADIUS_PILL: f32 = 999.0;

/// Screen title
pub const TEXT_DISPLAY: f32 = 32.0;
/// Dialog and section title
pub const TEXT_TITLE: f32 = 24.0;
/// Card title
pub const TEXT_HEADING: f32 = 19.0;
/// Default body
pub const TEXT_BODY: f32 = 13.0;
/// Emphasised body
pub const TEXT_BODY_STRONG: f32 = 13.0;
/// Form and table label
pub const TEXT_LABEL: f32 = 11.0;
/// Supporting caption
pub const TEXT_CAPTION: f32 = 10.0;
/// Uppercase section eyebrow
pub const TEXT_EYEBROW: f32 = 10.0;
/// Terminal output
pub const TEXT_MONO: f32 = 12.0;

/// Tight gap
pub const SPACE_1: f32 = 4.0;
/// Icon to label
pub const SPACE_2: f32 = 8.0;
/// Inner padding
pub const SPACE_3: f32 = 12.0;
/// Card padding
pub const SPACE_4: f32 = 16.0;
/// Card padding wide
pub const SPACE_5: f32 = 20.0;
/// Section gap
pub const SPACE_6: f32 = 24.0;
/// Section gap wide
pub const SPACE_7: f32 = 32.0;
/// Page gutter
pub const SPACE_8: f32 = 40.0;

pub const CANVAS_WIDTH: f32 = 1440.0;
pub const CANVAS_HEIGHT: f32 = 900.0;
pub const SIDEBAR_WIDTH: f32 = 252.0;
pub const CONTENT_GUTTER: f32 = 38.0;
pub const CARD_GAP: f32 = 16.0;
