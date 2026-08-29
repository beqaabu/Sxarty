//! The view layer.

pub mod chrome;
pub mod context;
pub mod reader_pane;
pub mod skin;
pub mod style;

use iced::widget::{Space, column, container, row, text};
use iced::{Element, Fill, FillPortion};

use crate::app::{App, Message};
use skin::Skin;

pub fn view(app: &App) -> Element<'_, Message> {
    let skin = app.skin();

    let body = if app.zen {
        zen(app, skin)
    } else {
        full(app, skin)
    };

    container(body)
        .width(Fill)
        .height(Fill)
        .style(style::ground(skin))
        .into()
}

/// The normal layout: chrome top and bottom, reading area in the middle,
/// optional rails either side.
///
/// Context sits on the left because it is about position in the document;
/// settings and recents sit on the right because they are about the session.
/// Both can be open at once without either displacing the reading area.
fn full<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    let mut middle = row![].height(Fill);

    if app.config.show_context {
        middle = middle.push(context::rail(app, skin));
        middle = middle.push(vertical_divider(skin));
    }

    middle = middle.push(
        column![
            reader_pane::pane(app, skin),
            horizontal_divider(skin),
            chrome::footer(app, skin),
        ]
        .width(Fill)
        .height(Fill),
    );

    if let Some(panel) = app.panel {
        middle = middle.push(vertical_divider(skin));
        middle = middle.push(chrome::panel(app, skin, panel));
    }

    let mut root = column![chrome::header(app, skin), horizontal_divider(skin)];

    if let Some(error) = &app.error {
        root = root.push(container(chrome::error_strip(error, skin)).padding([8, 16]));
    }

    root.push(middle).width(Fill).height(Fill).into()
}

/// Zen mode: the word, a hairline of progress, and nothing else.
fn zen<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    let progress = app.reader.progress(app.len());

    // FillPortion is integral, so scale up for a smooth-looking bar.
    let done = (progress * 1000.0).round().clamp(0.0, 1000.0) as u16;
    let left = 1000u16.saturating_sub(done);

    let bar = row![
        container(Space::new().height(2.0))
            .width(FillPortion(done.max(1)))
            .style(style::accent_block(skin, 1.0)),
        container(Space::new().height(2.0))
            .width(FillPortion(left.max(1)))
            .style(style::faint_block(skin)),
    ];

    column![
        container(text("esc").size(10).color(skin.faint))
            .width(Fill)
            .align_right(Fill)
            .padding([10, 14]),
        reader_pane::pane(app, skin),
        bar,
    ]
    .width(Fill)
    .height(Fill)
    .into()
}

fn horizontal_divider<'a>(skin: Skin) -> Element<'a, Message> {
    container(Space::new().height(1.0))
        .width(Fill)
        .style(style::divider(skin))
        .into()
}

fn vertical_divider<'a>(skin: Skin) -> Element<'a, Message> {
    container(Space::new().width(1.0))
        .height(Fill)
        .style(style::divider(skin))
        .into()
}
