//! Header, footer and the right-hand panels.

use std::time::Duration;

use iced::widget::{Space, button, column, container, pick_list, row, slider, text, text_input};
use iced::{Element, Fill, Length};

use super::skin::{Accent, FocusMark, Skin, SkinId};
use super::style;
use crate::app::{App, Message, Panel};
use crate::config::{CHUNK_RANGE, FONT_RANGE, WPM_RANGE};

pub const PANEL_WIDTH: f32 = 288.0;

const LABEL: f32 = 11.0;
const BODY: f32 = 13.0;

/// Common speeds, offered as one-tap chips so nobody has to drag a slider to
/// get back to a familiar pace.
const WPM_PRESETS: [u32; 4] = [200, 300, 450, 600];

// ---------------------------------------------------------------- header ---

pub fn header<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    let title = match &app.document {
        Some(document) => document.title.clone(),
        None => "No document".to_string(),
    };

    let title_color = if app.document.is_some() {
        skin.text
    } else {
        skin.faint
    };

    let mark = container(Space::new().width(7.0).height(7.0)).style(style::accent_block(skin, 1.0));

    // The title takes the leftover width and is clipped, so a long file name
    // cannot push the buttons off the edge of the window.
    let left = row![
        container(mark).padding([0, 2]),
        text("Sxarty").size(BODY).color(skin.dim),
        container(Space::new().width(1.0).height(14.0)).style(style::faint_block(skin)),
        container(
            text(title)
                .size(BODY)
                .color(title_color)
                .shaping(text::Shaping::Advanced)
                .wrapping(text::Wrapping::None),
        )
        .width(Fill)
        .clip(true),
    ]
    .spacing(10)
    .align_y(iced::Center);

    let actions = row![
        tool(app, skin, "Open", Message::OpenDialog, false),
        tool(app, skin, "Paste", Message::PasteRequested, false),
        tool(
            app,
            skin,
            "Recent",
            Message::TogglePanel(Panel::Recents),
            app.panel == Some(Panel::Recents),
        ),
        tool(
            app,
            skin,
            "Context",
            Message::ToggleContext,
            app.config.show_context
        ),
        tool(
            app,
            skin,
            "Settings",
            Message::TogglePanel(Panel::Settings),
            app.panel == Some(Panel::Settings),
        ),
        tool(app, skin, "Zen", Message::ToggleZen, app.zen),
    ]
    .spacing(4)
    .align_y(iced::Center);

    container(row![left, actions].align_y(iced::Center).spacing(12))
        .width(Fill)
        .padding([10, 16])
        .style(style::bar(skin))
        .into()
}

fn tool<'a>(
    _app: &App,
    skin: Skin,
    label: &'a str,
    message: Message,
    active: bool,
) -> Element<'a, Message> {
    let button = button(text(label).size(12))
        .padding([6, 10])
        .on_press(message);

    if active {
        button.style(style::ghost_active(skin)).into()
    } else {
        button.style(style::ghost(skin)).into()
    }
}

// ---------------------------------------------------------------- footer ---

pub fn footer<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    let total = app.len();
    let last = total.saturating_sub(1);

    let scrubber = slider(
        0..=last.max(1) as u32,
        app.reader.index.min(last) as u32,
        Message::Seek,
    )
    .step(1u32)
    .style(style::scrubber(skin));

    let position = if total == 0 {
        "-".to_string()
    } else {
        format!(
            "{} / {}  ·  {}%",
            thousands(app.reader.index + 1),
            thousands(total),
            (app.reader.progress(total) * 100.0).round() as u32
        )
    };

    let remaining = if total == 0 {
        String::new()
    } else {
        format!("{} left", duration(app.remaining()))
    };

    let stats = column![
        text(position).size(LABEL).color(skin.dim),
        text(remaining).size(LABEL).color(skin.faint),
    ]
    .spacing(2);

    let speed = row![
        text("wpm").size(LABEL).color(skin.faint),
        slider(WPM_RANGE, app.config.wpm, Message::WpmChanged)
            .step(5u32)
            .width(Length::Fixed(120.0))
            .style(style::control_slider(skin)),
        text(app.config.wpm.to_string()).size(BODY).color(skin.text),
    ]
    .spacing(8)
    .align_y(iced::Center);

    let bar = row![
        container(stats).width(Fill).align_left(Fill),
        transport(app, skin),
        container(speed).width(Fill).align_right(Fill),
    ]
    .align_y(iced::Center)
    .spacing(12);

    container(column![scrubber, bar].spacing(10))
        .width(Fill)
        .padding([12, 16])
        .style(style::bar(skin))
        .into()
}

fn transport<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    let loaded = app.document.is_some();
    let playing = app.reader.playing;

    let step = move |label: &'a str, message: Message| {
        let mut b = button(text(label).size(15))
            .padding([6, 10])
            .style(style::ghost(skin));
        if loaded {
            b = b.on_press(message);
        }
        b
    };

    let mut play = button(
        text(if playing { "Pause" } else { "Play" })
            .size(13)
            .align_x(iced::Center),
    )
    .padding([8, 0])
    .width(Length::Fixed(84.0))
    .style(style::primary(skin));

    if loaded {
        play = play.on_press(Message::TogglePlay);
    }

    row![
        step("«", Message::ParagraphStep(-1)),
        step("‹", Message::Step(-1)),
        play,
        step("›", Message::Step(1)),
        step("»", Message::ParagraphStep(1)),
    ]
    .spacing(6)
    .align_y(iced::Center)
    .into()
}

// ----------------------------------------------------------------- panel ---

pub fn panel<'a>(app: &App, skin: Skin, which: Panel) -> Element<'a, Message> {
    let body = match which {
        Panel::Settings => settings(app, skin),
        Panel::Recents => recents(app, skin),
    };

    container(body)
        .width(Length::Fixed(PANEL_WIDTH))
        .height(Fill)
        .style(style::bar(skin))
        .clip(true)
        .into()
}

fn section<'a>(title: &'a str, skin: Skin, body: Element<'a, Message>) -> Element<'a, Message> {
    column![text(title).size(LABEL).color(skin.faint), body,]
        .spacing(8)
        .into()
}

fn settings<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    let speed = column![
        row![
            slider(WPM_RANGE, app.config.wpm, Message::WpmChanged)
                .step(5u32)
                .style(style::control_slider(skin)),
            text_input("300", &app.wpm_input)
                .on_input(Message::WpmInput)
                .on_submit(Message::WpmSubmit)
                .size(BODY)
                .padding([5, 8])
                .width(Length::Fixed(62.0))
                .style(style::field(skin)),
        ]
        .spacing(10)
        .align_y(iced::Center),
        row(WPM_PRESETS.into_iter().map(|preset| {
            chip(
                skin,
                preset.to_string(),
                Message::WpmChanged(preset),
                app.config.wpm == preset,
            )
        }))
        .spacing(6),
    ]
    .spacing(8);

    let font = row![
        slider(FONT_RANGE, app.config.font_size, Message::FontChanged)
            .step(2u32)
            .style(style::control_slider(skin)),
        text_input("64", &app.font_input)
            .on_input(Message::FontInput)
            .on_submit(Message::FontSubmit)
            .size(BODY)
            .padding([5, 8])
            .width(Length::Fixed(62.0))
            .style(style::field(skin)),
    ]
    .spacing(10)
    .align_y(iced::Center);

    let chunk = row(CHUNK_RANGE.map(|size| {
        chip(
            skin,
            size.to_string(),
            Message::ChunkChanged(size as u32),
            app.config.chunk == size,
        )
    }))
    .spacing(6);

    let pacing = column![
        chip(
            skin,
            if app.config.smart_pacing { "On" } else { "Off" }.to_string(),
            Message::SmartPacingToggled(!app.config.smart_pacing),
            app.config.smart_pacing,
        ),
        text("Holds longer on commas, full stops and long words.")
            .size(11)
            .color(skin.faint),
    ]
    .spacing(6);

    let focus = column![
        row(FocusMark::ALL.into_iter().map(|mark| {
            chip(
                skin,
                mark.label().to_string(),
                Message::FocusMarkChanged(mark),
                app.config.focus_mark == mark,
            )
        }))
        .spacing(6),
        text("The alignment does the work; the mark is a preference.")
            .size(11)
            .color(skin.faint),
    ]
    .spacing(6);

    let theme = column![
        pick_list(SkinId::ALL, Some(app.config.skin), Message::SkinChanged)
            .width(Fill)
            .padding([5, 8])
            .text_size(BODY)
            .style(style::dropdown(skin))
            .menu_style(style::menu(skin)),
        row(Accent::ALL
            .into_iter()
            .map(|accent| { swatch(skin, accent, app.config.accent == accent) }))
        .spacing(6),
    ]
    .spacing(8);

    scrollable(
        column![
            section("Speed", skin, speed.into()),
            section("Font size", skin, font.into()),
            section("Words at a time", skin, chunk.into()),
            section("Smart pacing", skin, pacing.into()),
            section("Focus mark", skin, focus.into()),
            section("Appearance", skin, theme.into()),
            container(Space::new().height(1.0))
                .width(Fill)
                .style(style::divider(skin)),
            shortcuts(skin),
        ]
        .spacing(18)
        .padding([16, 16]),
        skin,
    )
}

fn scrollable<'a>(content: impl Into<Element<'a, Message>>, skin: Skin) -> Element<'a, Message> {
    iced::widget::scrollable(content)
        .height(Fill)
        .style(style::scroll(skin))
        .into()
}

fn chip<'a>(skin: Skin, label: String, message: Message, active: bool) -> Element<'a, Message> {
    let button = button(text(label).size(12).align_x(iced::Center))
        .padding([5, 10])
        .on_press(message);

    if active {
        button.style(style::ghost_active(skin)).into()
    } else {
        button.style(style::ghost(skin)).into()
    }
}

/// A colour swatch for picking the pivot accent.
fn swatch<'a>(skin: Skin, accent: Accent, active: bool) -> Element<'a, Message> {
    let color = accent.color(skin.is_dark);
    let dot = container(Space::new().width(16.0).height(16.0)).style(move |_| {
        iced::widget::container::Style {
            background: Some(color.into()),
            border: iced::Border {
                radius: 8.0.into(),
                width: if active { 2.0 } else { 0.0 },
                color: skin.text,
            },
            ..Default::default()
        }
    });

    button(dot)
        .padding(3)
        .style(style::ghost(skin))
        .on_press(Message::AccentChanged(accent))
        .into()
}

fn recents<'a>(app: &App, skin: Skin) -> Element<'a, Message> {
    if app.config.recents.is_empty() {
        return container(
            text("Documents you open show up here, with the place you stopped.")
                .size(12)
                .color(skin.faint),
        )
        .padding(16)
        .into();
    }

    let entries = app.config.recents.iter().map(|recent| {
        let percent = if recent.total > 1 {
            (recent.index as f32 / (recent.total - 1) as f32 * 100.0).round() as u32
        } else {
            0
        };

        button(
            column![
                text(recent.title.clone())
                    .size(BODY)
                    .color(skin.text)
                    .shaping(text::Shaping::Advanced)
                    .wrapping(text::Wrapping::None),
                text(format!("{percent}% · {} words", thousands(recent.total)))
                    .size(11)
                    .color(skin.faint),
            ]
            .spacing(2),
        )
        .width(Fill)
        .padding([8, 10])
        .style(style::ghost(skin))
        .on_press(Message::OpenPath(recent.path.clone()))
        .into()
    });

    scrollable(
        column![
            column(entries).spacing(4),
            button(text("Clear list").size(11))
                .padding([5, 10])
                .style(style::ghost(skin))
                .on_press(Message::ForgetRecents),
        ]
        .spacing(14)
        .padding(16),
        skin,
    )
}

fn shortcuts<'a>(skin: Skin) -> Element<'a, Message> {
    const KEYS: [(&str, &str); 10] = [
        ("Space", "Play / pause"),
        ("← →", "One word"),
        ("Shift ← →", "One paragraph"),
        ("↑ ↓", "Speed ±25"),
        ("+ -", "Font size"),
        ("1 2 3", "Words at a time"),
        ("F", "Focus mark"),
        ("C", "Context panel"),
        ("Z", "Zen mode"),
        ("O", "Open a file"),
    ];

    let rows = KEYS.into_iter().map(|(key, what)| {
        row![
            container(text(key).size(11).color(skin.dim)).width(Length::Fixed(76.0)),
            text(what).size(11).color(skin.faint),
        ]
        .spacing(8)
        .into()
    });

    column![
        text("Shortcuts").size(LABEL).color(skin.faint),
        column(rows).spacing(5),
    ]
    .spacing(8)
    .into()
}

// ------------------------------------------------------------- messaging ---

/// A dismissible error strip. Failures used to go to stderr, where a
/// double-clicked app has no terminal to show them in.
pub fn error_strip<'a>(message: &str, skin: Skin) -> Element<'a, Message> {
    container(
        row![
            text(message.to_string()).size(12).width(Fill),
            button(text("Dismiss").size(11))
                .padding([4, 8])
                .style(style::ghost(skin))
                .on_press(Message::DismissError),
        ]
        .spacing(10)
        .align_y(iced::Center),
    )
    .width(Fill)
    .padding([8, 12])
    .style(style::alert(skin))
    .into()
}

// ------------------------------------------------------------ formatting ---

fn thousands(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);

    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }

    out
}

fn duration(value: Duration) -> String {
    let seconds = value.as_secs();
    match seconds {
        0..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m", seconds / 60),
        _ => format!("{}h {:02}m", seconds / 3600, (seconds % 3600) / 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_groups_from_the_right() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(1234567), "1,234,567");
    }

    #[test]
    fn duration_switches_units_at_the_right_thresholds() {
        assert_eq!(duration(Duration::from_secs(45)), "45s");
        assert_eq!(duration(Duration::from_secs(90)), "1m");
        assert_eq!(duration(Duration::from_secs(3600)), "1h 00m");
        assert_eq!(duration(Duration::from_secs(3600 * 2 + 300)), "2h 05m");
    }
}
