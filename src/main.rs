//! Sxarty - a focused RSVP speed reader.
//!
//! One word at a time, held at the point your eye already wants to look.

// A double-clicked app should not open a console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod document;
mod fonts;
mod reader;
mod ui;

use app::App;

pub fn main() -> iced::Result {
    iced::application(App::boot, App::update, ui::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        // Georgian lives here and nowhere else on macOS. See `fonts`.
        .font(fonts::GEORGIAN_REGULAR)
        .font(fonts::GEORGIAN_BOLD)
        .antialiasing(true)
        // We save the reading position on the way out.
        .exit_on_close_request(false)
        .window_size((1100.0, 720.0))
        .centered()
        .run()
}
