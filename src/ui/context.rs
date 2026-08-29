//! The context rail: the surrounding text, with your position marked.
//!
//! RSVP hides structure. You cannot see that a paragraph is ending, you cannot
//! glance back at the clause you just read, and you lose all sense of where you
//! are on the page. This panel gives that back without pulling the eye: the
//! surrounding text is rendered in a muted colour at a small size, and only the
//! word currently on screen is lit.
//!
//! The text is paged rather than scrolled with every word. Reflowing the whole
//! panel 300 times a minute would be unreadable, so the window only turns over
//! when the reader crosses a page boundary.
//!
//! Every word is also a jump target. Each one carries its token index as a rich
//! text link, so clicking it moves the reader there. That makes the panel a way
//! to navigate and not only a way to look, which matters most in exactly the
//! case RSVP is worst at: you half-caught a clause and want to go back to it.

use iced::advanced::widget::Id;
use iced::widget::operation::{RelativeOffset, snap_to};
use iced::widget::{Space, column, container, rich_text, row, scrollable, span, text};
use iced::{Element, Fill, Length, Task};

use super::skin::{Skin, alpha};
use super::style;
use crate::app::{App, Message};
use crate::document::Document;
use crate::fonts;

pub const RAIL_WIDTH: f32 = 300.0;

/// Tokens per page. Large enough to hold a paragraph or two, small enough that
/// the highlighted word is never far from the middle.
const PAGE: usize = 90;

const BODY_SIZE: f32 = 13.0;

/// Names the rail's scrollable so the update loop can scroll it.
const SCROLL: Id = Id::new("context-scroll");

/// Which page of the document the given token falls on.
pub fn page_of(index: usize) -> usize {
    index / PAGE
}

/// The token range the rail renders around `index`.
///
/// A page of lead-in and a page of look-ahead, so the reader can see both where
/// the sentence came from and where it is going.
fn range_of(index: usize, len: usize) -> (usize, usize) {
    let page = page_of(index);
    (page.saturating_sub(1) * PAGE, ((page + 2) * PAGE).min(len))
}

/// Scrolls the rail so the token at `index` is on screen.
///
/// Setting the scroll fraction to the token's fraction through the rendered
/// range is enough to guarantee that. With content height `H` and viewport `V`,
/// a scroll of `y` shows `[y(H-V), y(H-V)+V]`; putting `y = f` places the token
/// at `f*V` from the top of the viewport, which is inside it for every `f` in
/// `0..=1`. So the word drifts from the top of the panel to the bottom as you
/// work through a page, and never leaves it.
pub fn follow(index: usize, len: usize) -> Task<crate::app::Message> {
    snap_to(
        SCROLL,
        RelativeOffset {
            // Leave any horizontal scroll alone.
            x: None,
            y: Some(scroll_fraction(index, len)),
        },
    )
}

/// How far through the rendered range the token at `index` sits.
fn scroll_fraction(index: usize, len: usize) -> f32 {
    let (start, end) = range_of(index, len);
    let span = end.saturating_sub(start).max(1) as f32;

    (index.saturating_sub(start) as f32 / span).clamp(0.0, 1.0)
}

pub fn rail<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    let body: Element<'a, Message> = match &app.document {
        Some(document) if !document.is_empty() => flow(document, app.reader.index, skin),
        _ => container(
            text("Open a document to see where you are in it")
                .size(12)
                .color(skin.faint),
        )
        .padding(4)
        .into(),
    };

    let panel = column![
        heading(app, skin),
        container(Space::new().height(1.0))
            .width(Fill)
            .style(style::divider(skin)),
        scrollable(container(body).padding([12, 14]))
            .id(SCROLL)
            .height(Fill)
            .style(style::scroll(skin)),
    ];

    container(panel)
        .width(Length::Fixed(RAIL_WIDTH))
        .height(Fill)
        .style(style::bar(skin))
        .clip(true)
        .into()
}

fn heading<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    let position = match &app.document {
        Some(document) => {
            let paragraph = document.paragraph_of(app.reader.index).unwrap_or(0) + 1;
            format!(
                "Paragraph {paragraph} of {}",
                document.paragraphs.len().max(1)
            )
        }
        None => String::new(),
    };

    container(
        row![
            text("Context").size(11).color(skin.dim),
            Space::new().width(Fill),
            text(position).size(11).color(skin.faint),
        ]
        .align_y(iced::Center),
    )
    .padding([10, 14])
    .width(Fill)
    .into()
}

/// Renders the page around `index` as one flowing block of rich text.
fn flow<'a>(document: &Document, index: usize, skin: Skin) -> Element<'a, Message> {
    let (start, end) = range_of(index, document.len());

    let mut spans = Vec::with_capacity((end - start) * 2);
    let mut previous_paragraph = document.tokens[start].paragraph;

    for (offset, token) in document.tokens[start..end].iter().enumerate() {
        let position = start + offset;

        if token.paragraph != previous_paragraph {
            spans.push(span("\n\n"));
            previous_paragraph = token.paragraph;
        } else if position != start {
            spans.push(span(" "));
        }

        let current = position == index;
        let read = position < index;

        let color = if current {
            skin.accent
        } else if read {
            // Text already consumed fades back; the eye should fall forward.
            alpha(skin.dim, 0.55)
        } else {
            skin.muted()
        };

        // The link payload is the token index; clicking it seeks there.
        // iced draws hovered links underlined and shows a pointer cursor, so
        // the whole panel advertises itself without any extra chrome.
        let mut fragment = span(token.text.clone())
            .font(fonts::for_text(&token.text))
            .color(color)
            .link(position);

        if current {
            fragment = fragment
                .background(alpha(skin.accent, 0.16))
                .padding([1, 2]);
        }

        spans.push(fragment);
    }

    rich_text(spans)
        .size(BODY_SIZE)
        .line_height(1.75)
        .on_link_click(Message::JumpTo)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rendered_range_always_contains_the_reading_position() {
        let len = 1000;
        for index in [0, 1, PAGE - 1, PAGE, PAGE + 5, 500, len - 1] {
            let (start, end) = range_of(index, len);
            assert!(
                start <= index && index < end,
                "{index} outside {start}..{end}"
            );
            assert!(end <= len, "{end} past the end of {len}");
        }
    }

    #[test]
    fn a_short_document_renders_entirely() {
        let (start, end) = range_of(3, 10);
        assert_eq!((start, end), (0, 10));
    }

    #[test]
    fn the_scroll_fraction_stays_on_screen() {
        // y = f keeps the token inside the viewport for every f in 0..=1, so
        // the only thing to guarantee here is that f stays in range.
        for index in [0, 1, PAGE, PAGE * 3 + 7, 999] {
            let f = scroll_fraction(index, 1000);
            assert!((0.0..=1.0).contains(&f), "{index} gave {f}");
        }
    }

    #[test]
    fn the_position_advances_monotonically_within_a_page() {
        let a = scroll_fraction(PAGE + 10, 1000);
        let b = scroll_fraction(PAGE + 40, 1000);
        assert!(b > a, "{b} should be past {a}");
    }
}
