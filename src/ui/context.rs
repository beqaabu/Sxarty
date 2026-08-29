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

use iced::widget::{Space, column, container, rich_text, row, scrollable, span, text};
use iced::{Element, Fill, Length};

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
    let page = index / PAGE;
    // One page of lead-in and one of look-ahead, so the reader can see both
    // where the sentence came from and where it is going.
    let start = page.saturating_sub(1) * PAGE;
    let end = ((page + 2) * PAGE).min(document.len());

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

        let mut fragment = span(token.text.clone())
            .font(fonts::for_text(&token.text))
            .color(color);

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
        .on_link_click(iced::never)
        .into()
}
