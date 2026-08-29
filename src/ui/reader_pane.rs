//! The reading area: one word, pinned by its optimal recognition point.

use iced::widget::{Space, button, column, container, responsive, row, stack, text};
use iced::{Element, Fill, Font, Padding, Size};

use super::skin::{FocusMark, Skin};
use super::style;
use crate::app::{App, Message};
use crate::document::tokenize::pivot_of;
use crate::fonts;

/// Breathing room either side of the word.
const PADDING: f32 = 24.0;

/// However cramped the window gets, the word stays legible.
const MIN_SIZE: f32 = 14.0;

pub fn pane<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    let Some(word) = app.current_chunk() else {
        return empty_state(skin);
    };

    let requested = app.config.font_size as f32;
    let mark = app.config.focus_mark;

    // `responsive` is what makes the word aware of the room it actually has.
    // Without it the reading area cannot know that a side panel is open, and a
    // long word at a large size simply overruns its cell and draws across the
    // panel next to it.
    let body = responsive(move |available: Size| {
        let size = fitted_size(&word, requested, available.width);

        container(
            column![
                guide(size, skin),
                word_line(&word, size, skin, mark),
                guide(size, skin),
            ]
            .spacing(size * 0.22)
            .width(Fill),
        )
        .center(Fill)
        .into()
    });

    container(body)
        .center(Fill)
        .padding(PADDING)
        // A last line of defence: at MIN_SIZE a single enormous word can still
        // be wider than the pane, and it must be cut off rather than spill.
        .clip(true)
        .into()
}

/// The largest size at or below `requested` that keeps the word inside `available`.
///
/// The pivot is centred, so the word is only as wide as it needs the *wider* of
/// its two halves to be, mirrored: `2 * max(before, after) + pivot`. Rendered
/// widths scale linearly with font size, so the fitting size follows in one
/// step with no search.
fn fitted_size(word: &str, requested: f32, available: f32) -> f32 {
    // An unbounded parent hands `Responsive` an infinite width; there is
    // nothing to fit to in that case.
    if !available.is_finite() || available <= 1.0 {
        return requested;
    }

    let font = fonts::for_text(word);
    let (before, pivot, after) = split_at_pivot(word);

    let widest_half =
        fonts::text_width(&before, font, requested).max(fonts::text_width(&after, font, requested));
    let needed = 2.0 * widest_half + fonts::text_width(&pivot, font, requested);

    if needed <= available || needed <= 0.0 {
        return requested;
    }

    (requested * available / needed)
        .max(MIN_SIZE)
        .min(requested)
}

fn split_at_pivot(word: &str) -> (String, String, String) {
    let chars: Vec<char> = word.chars().collect();
    let index = pivot_of(&chars).min(chars.len().saturating_sub(1));

    (
        chars[..index].iter().collect(),
        chars.get(index).copied().into_iter().collect(),
        chars[(index + 1).min(chars.len())..].iter().collect(),
    )
}

/// Splits the word at its pivot and lays the three parts out so the pivot
/// always lands in the same column.
///
/// The row is `[Fill][Shrink][Fill]`, which is what keeps the pivot centred:
/// the two Fill cells split whatever is left over equally, so the middle cell
/// is always centred on the line no matter how long either side of the word is.
/// It must be `Shrink` rather than a fixed width - a fixed column sized from
/// the font size is narrower than a wide glyph like `m` or `W`, and the glyph
/// then spills out over the letters either side of it.
///
/// Everything here counts characters, never bytes: Georgian is three bytes per
/// character, and byte slicing would both misplace the pivot and panic.
fn word_line<'a>(word: &str, size: f32, skin: Skin, mark: FocusMark) -> Element<'a, Message> {
    let font = fonts::for_text(word);
    let (before, pivot, after) = split_at_pivot(word);

    row![
        container(part(before, size, font, skin.text)).align_right(Fill),
        pivot_cell(pivot, size, font, skin, mark),
        container(part(after, size, font, skin.text)).align_left(Fill),
    ]
    .align_y(iced::Center)
    .into()
}

fn part<'a>(
    content: String,
    size: f32,
    font: Font,
    color: iced::Color,
) -> text::Text<'a, iced::Theme, iced::Renderer> {
    text(content)
        .size(size)
        .font(font)
        .color(color)
        .shaping(text::Shaping::Advanced)
        .wrapping(text::Wrapping::None)
}

fn pivot_cell<'a>(
    pivot: String,
    size: f32,
    font: Font,
    skin: Skin,
    mark: FocusMark,
) -> Element<'a, Message> {
    let color = match mark {
        FocusMark::Letter => skin.accent,
        FocusMark::Underline | FocusMark::Off => skin.text,
    };

    let glyph = part(pivot, size, font, color);

    match mark {
        FocusMark::Underline => {
            let rule = container(Space::new().height((size * 0.05).max(2.0)))
                .width(Fill)
                .style(style::accent_block(skin, 1.0));

            // A stack takes its size from the base layer, so the rule is drawn
            // over the glyph's own box and cannot change the line's height or
            // knock the word off its baseline.
            stack(vec![
                glyph.into(),
                container(rule)
                    .width(Fill)
                    .align_bottom(Fill)
                    .padding(Padding {
                        bottom: size * 0.1,
                        ..Padding::default()
                    })
                    .into(),
            ])
            .into()
        }
        FocusMark::Letter | FocusMark::Off => glyph.into(),
    }
}

/// A tick mark above and below the pivot column.
///
/// Laid out with the same `[Fill][_][Fill]` split as the word, so the ticks and
/// the pivot always share a centre line.
fn guide<'a>(size: f32, skin: Skin) -> Element<'a, Message> {
    let tick = container(Space::new().width(2.0).height((size * 0.18).max(6.0)))
        .style(style::accent_block(skin, 0.5));

    row![Space::new().width(Fill), tick, Space::new().width(Fill),].into()
}

fn empty_state<'a>(skin: Skin) -> Element<'a, Message> {
    let content = column![
        text("Drop a document here")
            .size(28)
            .color(skin.text)
            .shaping(text::Shaping::Advanced),
        text("txt, md, pdf, docx and epub").size(14).color(skin.dim),
        Space::new().height(18.0),
        row![
            button(text("Open a file").size(14))
                .padding([9, 18])
                .style(style::primary(skin))
                .on_press(Message::OpenDialog),
            button(text("Paste text").size(14))
                .padding([9, 18])
                .style(style::ghost(skin))
                .on_press(Message::PasteRequested),
        ]
        .spacing(10),
    ]
    .spacing(8)
    .align_x(iced::Center);

    container(content).center(Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
        use iced::advanced::graphics::text::font_system;
        font_system()
            .write()
            .expect("font system")
            .load_font(crate::fonts::GEORGIAN_REGULAR.into());
    }

    #[test]
    fn the_pivot_splits_on_characters_not_bytes() {
        // Georgian is three bytes per character; byte slicing would panic here.
        let (before, pivot, after) = split_at_pivot("\u{10E5}\u{10D0}\u{10E0}\u{10D7}");
        assert_eq!(before.chars().count(), 1);
        assert_eq!(pivot.chars().count(), 1);
        assert_eq!(after.chars().count(), 2);
    }

    #[test]
    fn a_word_that_fits_keeps_the_size_it_was_asked_for() {
        setup();
        assert_eq!(fitted_size("cat", 64.0, 2000.0), 64.0);
    }

    #[test]
    fn a_word_too_wide_for_the_pane_is_scaled_down_to_fit() {
        setup();

        let available = 260.0;
        let word = "incomprehensibilities";
        let size = fitted_size(word, 96.0, available);

        assert!(size < 96.0, "expected shrinking, got {size}");

        // And the scaled result genuinely fits: the pivot is centred, so the
        // word needs twice its wider half plus the pivot glyph.
        let (before, pivot, after) = split_at_pivot(word);
        let font = crate::fonts::for_text(word);
        let needed = 2.0
            * crate::fonts::text_width(&before, font, size)
                .max(crate::fonts::text_width(&after, font, size))
            + crate::fonts::text_width(&pivot, font, size);

        assert!(needed <= available + 1.0, "{needed} > {available}");
    }

    #[test]
    fn fitting_never_drops_below_the_legible_minimum() {
        setup();
        assert!(fitted_size("incomprehensibilities", 96.0, 4.0) >= MIN_SIZE);
    }

    #[test]
    fn an_unbounded_pane_leaves_the_size_alone() {
        setup();
        assert_eq!(fitted_size("anything", 64.0, f32::INFINITY), 64.0);
    }
}
