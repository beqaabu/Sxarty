//! Font handling.
//!
//! # Why this module exists
//!
//! `cosmic-text` (the shaper behind iced) keeps a per-platform table mapping a
//! Unicode script to the system font that can render it. Its **unix** table has
//! `Script::Georgian => "Noto Sans Georgian"`; its **macOS** table has no
//! Georgian entry at all, and macOS ships Georgian only inside the private
//! `.SF Georgian` family, which is never reached by the generic fallback.
//!
//! That is the whole reason Georgian text renders on Linux and turns into
//! blank boxes on macOS. Rather than depend on the shaper's fallback chain, we
//! embed Noto Sans Georgian in the binary and select it explicitly for any text
//! that contains Georgian codepoints. Deterministic on every platform.

use iced::Font;

/// Noto Sans Georgian, SIL Open Font License 1.1. See `assets/fonts/LICENSE`.
pub const GEORGIAN_REGULAR: &[u8] = include_bytes!("../assets/fonts/NotoSansGeorgian-Regular.ttf");
pub const GEORGIAN_BOLD: &[u8] = include_bytes!("../assets/fonts/NotoSansGeorgian-Bold.ttf");

/// The embedded Georgian family. Registered at startup in `main`.
pub const GEORGIAN: Font = Font::with_name("Noto Sans Georgian");

/// The platform's own sans-serif, used for Latin and for UI chrome.
pub const UI: Font = Font::DEFAULT;

/// True for the Georgian blocks: Mkhedruli/Asomtavruli, Mtavruli, and Nuskhuri.
pub fn is_georgian(c: char) -> bool {
    matches!(
        c as u32,
        0x10A0..=0x10FF   // Georgian
        | 0x1C90..=0x1CBF // Georgian Extended (Mtavruli)
        | 0x2D00..=0x2D2F // Georgian Supplement (Nuskhuri)
    )
}

/// Picks a font that can actually draw `text`.
///
/// Noto Sans Georgian carries only 182 glyphs - Georgian and a little
/// punctuation, no Latin and no digits - so it must never be the default font.
/// We hand it out only when the string actually needs it, and let advanced
/// shaping fill any remaining gaps from the system.
pub fn for_text(text: &str) -> Font {
    if text.chars().any(is_georgian) {
        GEORGIAN
    } else {
        UI
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_georgian_string_picks_the_embedded_family() {
        assert_eq!(for_text("\u{10E5}\u{10D0}\u{10E0}"), GEORGIAN);
        assert_eq!(for_text("plain latin"), UI);
        // Mixed text still needs the Georgian family; shaping fills in the rest.
        assert_eq!(
            for_text("Rustaveli \u{10E8}\u{10DD}\u{10D7}\u{10D0}"),
            GEORGIAN
        );
    }

    /// The regression test for the macOS bug.
    ///
    /// Shapes Georgian through the very font system iced renders with. Glyph id
    /// 0 is `.notdef` - the empty box - so a run of zeroes here is exactly what
    /// the user saw on macOS before the font was embedded.
    #[test]
    fn georgian_shapes_to_real_glyphs_through_iceds_font_system() {
        use iced::advanced::graphics::text::{cosmic_text, font_system};

        // Same registration `main` performs at startup.
        font_system()
            .write()
            .expect("font system")
            .load_font(GEORGIAN_REGULAR.into());

        let mut guard = font_system().write().expect("font system");
        let raw = guard.raw();

        let mut buffer = cosmic_text::Buffer::new(raw, cosmic_text::Metrics::new(48.0, 60.0));
        let attrs =
            cosmic_text::Attrs::new().family(cosmic_text::Family::Name("Noto Sans Georgian"));

        // "\u{10E5}\u{10D0}\u{10E0}\u{10D7}\u{10E3}\u{10DA}\u{10D8}" - "Georgian".
        buffer.set_text(
            raw,
            "\u{10E5}\u{10D0}\u{10E0}\u{10D7}\u{10E3}\u{10DA}\u{10D8}",
            &attrs,
            cosmic_text::Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(raw, false);

        let glyphs: Vec<u16> = buffer
            .layout_runs()
            .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.glyph_id))
            .collect();

        assert_eq!(glyphs.len(), 7, "expected one glyph per character");
        assert!(
            glyphs.iter().all(|id| *id != 0),
            "Georgian fell back to .notdef: {glyphs:?}"
        );
    }
}
