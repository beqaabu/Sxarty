//! Application state and its wiring into the iced runtime.

mod update;

pub mod message;

pub use message::{Message, Panel};

use std::time::Duration;

use iced::{Subscription, Task};

use crate::config::Config;
use crate::document::Document;
use crate::reader::{Pacing, Reader, TICK};
use crate::ui::skin::Skin;

pub struct App {
    pub document: Option<Document>,
    pub reader: Reader,
    pub config: Config,

    /// Free text next to the sliders, so a half-typed number is not clobbered.
    pub wpm_input: String,
    pub font_input: String,

    pub panel: Option<Panel>,
    pub zen: bool,
    pub error: Option<String>,
    pub loading: bool,
}

impl App {
    pub fn boot() -> (Self, Task<Message>) {
        let mut config = Config::load();
        config.clamp();

        // `sxarty book.epub`, and whatever the desktop hands us on "Open With".
        let argument = std::env::args_os()
            .nth(1)
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_file());

        let app = App {
            wpm_input: config.wpm.to_string(),
            font_input: config.font_size.to_string(),
            document: None,
            reader: Reader::default(),
            config,
            panel: None,
            zen: false,
            error: None,
            loading: false,
        };

        let task = match argument {
            Some(path) => Task::done(Message::OpenPath(path)),
            None => Task::none(),
        };

        (app, task)
    }

    pub fn title(&self) -> String {
        match &self.document {
            Some(document) => format!("{} - Sxarty", document.title),
            None => "Sxarty".to_string(),
        }
    }

    pub fn skin(&self) -> Skin {
        Skin::new(self.config.skin, self.config.accent)
    }

    pub fn theme(&self) -> iced::Theme {
        self.skin().theme(self.config.skin)
    }

    pub fn pacing(&self) -> Pacing {
        Pacing {
            wpm: self.config.wpm,
            chunk: self.config.chunk,
            smart: self.config.smart_pacing,
        }
    }

    pub fn len(&self) -> usize {
        self.document.as_ref().map_or(0, Document::len)
    }

    /// The tokens currently on screen, joined for display.
    pub fn current_chunk(&self) -> Option<String> {
        let document = self.document.as_ref()?;
        let start = self.reader.index;
        let end = (start + self.config.chunk.max(1)).min(document.len());
        if start >= end {
            return None;
        }

        Some(
            document.tokens[start..end]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    pub fn remaining(&self) -> Duration {
        match &self.document {
            Some(document) => self.reader.remaining(document, self.pacing()),
            None => Duration::ZERO,
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        update::update(self, message)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            // `keyboard::listen` only yields events no widget consumed, so
            // typing a number into the WPM field cannot also trigger shortcuts.
            iced::keyboard::listen().map(Message::Key),
            iced::window::events().map(|(_id, event)| Message::Window(event)),
        ];

        if self.reader.playing {
            subscriptions.push(iced::time::every(TICK).map(Message::Tick));
        }

        Subscription::batch(subscriptions)
    }
}
