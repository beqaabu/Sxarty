//! Widget styling derived from the active [`Skin`].
//!
//! iced hands style closures a `&Theme`, which cannot express everything a skin
//! needs, so each helper closes over a copy of the [`Skin`] instead. `Skin` is
//! `Copy`, so this stays allocation-free.

use iced::widget::{button, container, pick_list, scrollable, slider, text_input};
use iced::{Background, Border, Color, Shadow, Vector};

use super::skin::{Skin, alpha};

pub const RADIUS: f32 = 8.0;
pub const HAIRLINE: f32 = 1.0;

/// The window ground.
pub fn ground(skin: Skin) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(skin.bg)),
        text_color: Some(skin.text),
        ..Default::default()
    }
}

/// A panel edge-to-edge against the window, so only one border shows.
pub fn bar(skin: Skin) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(skin.surface)),
        text_color: Some(skin.text),
        ..Default::default()
    }
}

/// A one-pixel divider.
pub fn divider(skin: Skin) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(skin.border)),
        ..Default::default()
    }
}

/// A solid block in the accent colour: the reading guides and the zen progress
/// line.
pub fn accent_block(skin: Skin, opacity: f32) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(alpha(skin.accent, opacity))),
        ..Default::default()
    }
}

pub fn faint_block(skin: Skin) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(skin.faint)),
        ..Default::default()
    }
}

/// An inline error message.
pub fn alert(_skin: Skin) -> impl Fn(&iced::Theme) -> container::Style {
    move |theme: &iced::Theme| {
        let danger = theme.palette().danger;
        container::Style {
            background: Some(Background::Color(alpha(danger, 0.12))),
            text_color: Some(danger),
            border: Border {
                color: alpha(danger, 0.35),
                width: HAIRLINE,
                radius: RADIUS.into(),
            },
            ..Default::default()
        }
    }
}

fn button_shape(background: Color, text_color: Color, border: Color) -> button::Style {
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: Border {
            color: border,
            width: HAIRLINE,
            radius: RADIUS.into(),
        },
        shadow: Shadow::default(),
        ..Default::default()
    }
}

/// A quiet button: chrome that should not compete with the word on screen.
pub fn ghost(skin: Skin) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| match status {
        button::Status::Active => button_shape(Color::TRANSPARENT, skin.dim, Color::TRANSPARENT),
        button::Status::Hovered => button_shape(skin.raised, skin.text, skin.border),
        button::Status::Pressed => button_shape(
            alpha(skin.accent, 0.16),
            skin.accent,
            alpha(skin.accent, 0.4),
        ),
        button::Status::Disabled => {
            button_shape(Color::TRANSPARENT, skin.faint, Color::TRANSPARENT)
        }
    }
}

/// A ghost button that is currently switched on.
pub fn ghost_active(skin: Skin) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let base = button_shape(
            alpha(skin.accent, 0.16),
            skin.accent,
            alpha(skin.accent, 0.45),
        );
        match status {
            button::Status::Hovered => button::Style {
                background: Some(Background::Color(alpha(skin.accent, 0.24))),
                ..base
            },
            _ => base,
        }
    }
}

/// The one prominent action on screen.
pub fn primary(skin: Skin) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let on_accent = if skin.is_dark && skin.accent.r + skin.accent.g + skin.accent.b > 1.8 {
            skin.bg
        } else {
            Color::WHITE
        };

        match status {
            button::Status::Hovered => {
                button_shape(alpha(skin.accent, 0.88), on_accent, Color::TRANSPARENT)
            }
            button::Status::Pressed => {
                button_shape(alpha(skin.accent, 0.72), on_accent, Color::TRANSPARENT)
            }
            button::Status::Disabled => button_shape(skin.raised, skin.faint, Color::TRANSPARENT),
            button::Status::Active => button_shape(skin.accent, on_accent, Color::TRANSPARENT),
        }
    }
}

/// A control that sits on a panel: numeric fields and dropdowns.
pub fn field(skin: Skin) -> impl Fn(&iced::Theme, text_input::Status) -> text_input::Style {
    move |_, status| {
        let focused = matches!(status, text_input::Status::Focused { .. });
        text_input::Style {
            background: Background::Color(skin.raised),
            border: Border {
                color: if focused { skin.accent } else { skin.border },
                width: HAIRLINE,
                radius: (RADIUS - 2.0).into(),
            },
            icon: skin.dim,
            placeholder: skin.faint,
            value: skin.text,
            selection: alpha(skin.accent, 0.35),
        }
    }
}

pub fn dropdown(skin: Skin) -> impl Fn(&iced::Theme, pick_list::Status) -> pick_list::Style {
    move |_, status| pick_list::Style {
        text_color: skin.text,
        placeholder_color: skin.faint,
        handle_color: skin.dim,
        background: Background::Color(skin.raised),
        border: Border {
            color: match status {
                pick_list::Status::Hovered | pick_list::Status::Opened { .. } => skin.accent,
                _ => skin.border,
            },
            width: HAIRLINE,
            radius: (RADIUS - 2.0).into(),
        },
    }
}

pub fn menu(skin: Skin) -> impl Fn(&iced::Theme) -> iced::widget::overlay::menu::Style {
    move |_| iced::widget::overlay::menu::Style {
        background: Background::Color(skin.surface),
        border: Border {
            color: skin.border,
            width: HAIRLINE,
            radius: (RADIUS - 2.0).into(),
        },
        text_color: skin.text,
        selected_text_color: skin.text,
        selected_background: Background::Color(alpha(skin.accent, 0.22)),
        shadow: float_shadow(skin),
    }
}

/// A settings slider: thin rail, small handle, accent fill on the left.
pub fn control_slider(skin: Skin) -> impl Fn(&iced::Theme, slider::Status) -> slider::Style {
    move |_, status| slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(skin.accent),
                Background::Color(skin.raised),
            ),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 7.0 },
            background: Background::Color(match status {
                slider::Status::Active => skin.accent,
                _ => skin.text,
            }),
            border_width: 2.0,
            border_color: skin.surface,
        },
    }
}

/// The scrubber under the reading area. Reads as a progress bar until you
/// reach for it, then reveals a handle.
pub fn scrubber(skin: Skin) -> impl Fn(&iced::Theme, slider::Status) -> slider::Style {
    move |_, status| {
        let idle = matches!(status, slider::Status::Active);
        slider::Style {
            rail: slider::Rail {
                backgrounds: (
                    Background::Color(skin.accent),
                    Background::Color(alpha(skin.faint, 0.45)),
                ),
                width: if idle { 3.0 } else { 5.0 },
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle {
                    radius: if idle { 0.0 } else { 7.0 },
                },
                background: Background::Color(skin.accent),
                border_width: if idle { 0.0 } else { 2.0 },
                border_color: skin.bg,
            },
        }
    }
}

pub fn scroll(skin: Skin) -> impl Fn(&iced::Theme, scrollable::Status) -> scrollable::Style {
    move |theme, status| {
        let idle = matches!(status, scrollable::Status::Active { .. });
        let rail = scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: Background::Color(if idle {
                    alpha(skin.faint, 0.45)
                } else {
                    alpha(skin.dim, 0.7)
                }),
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
            },
        };

        // Start from the built-in style so any field we do not care about
        // (the autoscroll overlay, for one) still looks right.
        scrollable::Style {
            container: container::Style::default(),
            vertical_rail: rail,
            horizontal_rail: rail,
            gap: None,
            ..scrollable::default(theme, status)
        }
    }
}

/// A soft drop shadow for panels that float above the reading area.
pub fn float_shadow(skin: Skin) -> Shadow {
    Shadow {
        color: alpha(Color::BLACK, if skin.is_dark { 0.45 } else { 0.12 }),
        offset: Vector::new(0.0, 6.0),
        blur_radius: 18.0,
    }
}
