//! The reading area: one word, pinned by its optimal recognition point.

use iced::widget::{Space, button, column, container, row, stack, text};
use iced::{Element, Fill, Font, Padding};

use super::skin::{FocusMark, Skin};
use super::style;
use crate::app::{App, Message};
use crate::document::tokenize::pivot_of;
use crate::fonts;

pub fn pane<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    let Some(word) = app.current_chunk() else {
        return empty_state(skin);
    };

    let size = app.config.font_size as f32;

    let content = column![
        guide(size, skin),
        word_line(word, size, skin, app.config.focus_mark),
        guide(size, skin),
    ]
    .spacing(size * 0.22)
    .width(Fill);

    container(content).center(Fill).padding(24).into()
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
fn word_line<'a>(word: String, size: f32, skin: Skin, mark: FocusMark) -> Element<'a, Message> {
    let font = fonts::for_text(&word);
    let chars: Vec<char> = word.chars().collect();
    let index = pivot_of(&chars).min(chars.len().saturating_sub(1));

    let before: String = chars[..index].iter().collect();
    let pivot: String = chars.get(index).copied().into_iter().collect();
    let after: String = chars[(index + 1).min(chars.len())..].iter().collect();

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
