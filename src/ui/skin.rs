//! The colour system.
//!
//! Every surface, line and label in the app comes from a [`Skin`], so a theme
//! is one struct rather than a hunt through the view code. The reading accent
//! is kept separate from the skin: the pivot colour is a personal choice and
//! people want to change it without changing the whole background.

use iced::theme::Palette;
use iced::{Color, Theme};
use serde::{Deserialize, Serialize};

fn rgb(hex: u32) -> Color {
    Color {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a: 1.0,
    }
}

/// A colour with a given alpha, for hairlines and hover states.
pub fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SkinId {
    #[default]
    Midnight,
    Slate,
    Paper,
    Sepia,
}

impl SkinId {
    pub const ALL: [SkinId; 4] = [
        SkinId::Midnight,
        SkinId::Slate,
        SkinId::Paper,
        SkinId::Sepia,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SkinId::Midnight => "Midnight",
            SkinId::Slate => "Slate",
            SkinId::Paper => "Paper",
            SkinId::Sepia => "Sepia",
        }
    }
}

impl std::fmt::Display for SkinId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// The pivot / progress colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Accent {
    #[default]
    Crimson,
    Amber,
    Teal,
    Violet,
    Sky,
}

impl Accent {
    pub const ALL: [Accent; 5] = [
        Accent::Crimson,
        Accent::Amber,
        Accent::Teal,
        Accent::Violet,
        Accent::Sky,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Accent::Crimson => "Crimson",
            Accent::Amber => "Amber",
            Accent::Teal => "Teal",
            Accent::Violet => "Violet",
            Accent::Sky => "Sky",
        }
    }

    /// Light backgrounds need a darker, denser accent to keep contrast.
    pub fn color(self, is_dark: bool) -> Color {
        let hex = match (self, is_dark) {
            (Accent::Crimson, true) => 0xFF5C5C,
            (Accent::Crimson, false) => 0xC9282D,
            (Accent::Amber, true) => 0xF0B429,
            (Accent::Amber, false) => 0xA9700F,
            (Accent::Teal, true) => 0x34D1BF,
            (Accent::Teal, false) => 0x0E7A70,
            (Accent::Violet, true) => 0xA78BFA,
            (Accent::Violet, false) => 0x6538C4,
            (Accent::Sky, true) => 0x60A5FA,
            (Accent::Sky, false) => 0x1660C4,
        };
        rgb(hex)
    }
}

impl std::fmt::Display for Accent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// How the pivot character is marked.
///
/// Recolouring the pivot is the conventional RSVP treatment, but it does split
/// the word into three visual pieces, which some readers find harder to take in
/// than the whole word at once. The alignment is what does the real work, so
/// the mark is a preference rather than a requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FocusMark {
    /// The pivot in the accent colour.
    #[default]
    Letter,
    /// The whole word in one colour, with a rule under the pivot.
    Underline,
    /// No mark at all - just the guide ticks and the alignment.
    Off,
}

impl FocusMark {
    pub const ALL: [FocusMark; 3] = [FocusMark::Letter, FocusMark::Underline, FocusMark::Off];

    pub fn label(self) -> &'static str {
        match self {
            FocusMark::Letter => "Letter",
            FocusMark::Underline => "Underline",
            FocusMark::Off => "Off",
        }
    }
}

/// A fully resolved palette. Cheap to copy, so view functions take it by value.
#[derive(Debug, Clone, Copy)]
pub struct Skin {
    /// The window ground.
    pub bg: Color,
    /// Panels and bars sitting on the ground.
    pub surface: Color,
    /// Controls sitting on a panel.
    pub raised: Color,
    /// Hairlines.
    pub border: Color,
    /// Primary reading colour.
    pub text: Color,
    /// Labels and secondary information.
    pub dim: Color,
    /// Guides, disabled states, the far edges of the context panel.
    pub faint: Color,
    /// The pivot letter, the progress fill, the active control.
    pub accent: Color,
    pub is_dark: bool,
}

impl Skin {
    pub fn new(id: SkinId, accent: Accent) -> Self {
        let (bg, surface, raised, border, text, dim, faint, is_dark) = match id {
            SkinId::Midnight => (
                0x0B0E14, 0x121722, 0x1B2130, 0x252C3B, 0xE4E8F0, 0x8D97AB, 0x49526A, true,
            ),
            SkinId::Slate => (
                0x17191C, 0x1E2125, 0x272B31, 0x33383F, 0xE6E7E9, 0x9A9EA6, 0x565C66, true,
            ),
            SkinId::Paper => (
                0xFAFAF8, 0xFFFFFF, 0xF1F1EE, 0xE1E1DB, 0x1B1D20, 0x63676E, 0xAFB3B9, false,
            ),
            SkinId::Sepia => (
                0xF4ECD8, 0xFBF5E6, 0xEDE2C8, 0xDCCFB0, 0x3A322A, 0x7A6E5E, 0xB7AA93, false,
            ),
        };

        Skin {
            bg: rgb(bg),
            surface: rgb(surface),
            raised: rgb(raised),
            border: rgb(border),
            text: rgb(text),
            dim: rgb(dim),
            faint: rgb(faint),
            accent: accent.color(is_dark),
            is_dark,
        }
    }

    /// The iced [`Theme`] to hand the runtime, so built-in widget defaults and
    /// the window chrome agree with the skin.
    pub fn theme(&self, id: SkinId) -> Theme {
        Theme::custom(
            id.label(),
            Palette {
                background: self.bg,
                text: self.text,
                primary: self.accent,
                success: self.accent,
                warning: self.accent,
                danger: rgb(if self.is_dark { 0xFF6B6B } else { 0xC02A2A }),
            },
        )
    }

    /// The colour for text that must be readable but must not pull the eye,
    /// used by the context panel.
    pub fn muted(&self) -> Color {
        alpha(self.dim, 0.75)
    }
}
