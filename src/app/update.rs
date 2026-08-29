//! The update loop.

use std::path::PathBuf;

use iced::keyboard::key::Named;
use iced::keyboard::{Event as KeyEvent, Key};
use iced::{Task, window};

use super::{App, Message, Panel};
use crate::config::{CHUNK_RANGE, FONT_RANGE, WPM_RANGE};
use crate::document::{Document, Source, extract};
use crate::ui::context;
use crate::ui::skin::FocusMark;

/// How much one keypress moves a slider-backed setting.
const WPM_STEP: u32 = 25;
const FONT_STEP: u32 = 4;

/// Runs `message`, then keeps the context rail pointed at the reading position.
///
/// Wrapping the loop rather than touching every arm that can move the reader
/// means a new kind of navigation cannot forget to bring the panel with it.
pub fn update(app: &mut App, message: Message) -> Task<Message> {
    let was = app.reader.index;
    let had_context = app.config.show_context;

    let task = handle(app, message);

    let now = app.reader.index;
    let opened = !had_context && app.config.show_context;
    let turned_page = context::page_of(was) != context::page_of(now);

    // Follow while reading, and whenever the rail's content changes underneath
    // the reader. Deliberately *not* on every paused step: when the reader is
    // stopped the panel is something to browse and click, and yanking it back
    // would undo the scrolling that got them there.
    let follow =
        app.config.show_context && (opened || turned_page || (was != now && app.reader.playing));

    if follow && app.document.is_some() {
        return Task::batch([task, context::follow(now, app.len())]);
    }

    task
}

fn handle(app: &mut App, message: Message) -> Task<Message> {
    match message {
        Message::OpenDialog => return pick_file(),

        Message::FilePicked(Some(path)) | Message::OpenPath(path) => return load(app, path),
        Message::FilePicked(None) => {}

        Message::Loaded(result) => {
            app.loading = false;
            match *result {
                Ok(document) => adopt(app, document),
                Err(error) => app.error = Some(error),
            }
        }

        Message::PasteRequested => {
            return iced::clipboard::read().map(Message::Pasted);
        }

        Message::Pasted(text) => {
            let text = text.unwrap_or_default();
            let document = Document::from_text("Pasted text", Source::Clipboard, &text);
            if document.is_empty() {
                app.error = Some("The clipboard has no text in it".to_string());
            } else {
                adopt(app, document);
            }
        }

        Message::DismissError => app.error = None,

        Message::ForgetRecents => {
            app.config.recents.clear();
            app.config.save();
        }

        Message::TogglePlay => {
            if app.document.is_some() {
                // Pressing play on the last word should replay, not sit still.
                if !app.reader.playing && app.reader.index + 1 >= app.len() {
                    app.reader.seek(0);
                }
                app.reader.toggle();
                if !app.reader.playing {
                    remember(app);
                }
            }
        }

        Message::Step(delta) => {
            app.reader.pause();
            app.reader.step(delta as isize, app.len());
        }

        Message::ParagraphStep(delta) => {
            if let Some(document) = &app.document {
                let target = document.paragraph_jump(app.reader.index, delta as isize);
                app.reader.pause();
                app.reader.seek(target);
            }
        }

        Message::Restart => {
            app.reader.pause();
            app.reader.seek(0);
        }

        Message::Seek(index) => {
            let last = app.len().saturating_sub(1);
            app.reader.seek((index as usize).min(last));
        }

        Message::JumpTo(index) => {
            let last = app.len().saturating_sub(1);
            app.reader.seek(index.min(last));
            // Jumping is a deliberate act of reading, not scrubbing, so the
            // position is worth keeping if the app closes right after it.
            remember(app);
        }

        Message::Tick(now) => {
            let pacing = app.pacing();
            if let Some(document) = &app.document {
                let was_playing = app.reader.playing;
                app.reader.tick(now, document, pacing);
                // Reaching the end counts as a stopping point worth saving.
                if was_playing && !app.reader.playing {
                    remember(app);
                }
            }
        }

        Message::WpmChanged(wpm) => {
            app.config.wpm = wpm.clamp(*WPM_RANGE.start(), *WPM_RANGE.end());
            app.wpm_input = app.config.wpm.to_string();
            app.config.save();
        }

        Message::WpmInput(raw) => {
            app.wpm_input = digits(&raw);
            if let Ok(wpm) = app.wpm_input.parse::<u32>() {
                app.config.wpm = wpm.clamp(*WPM_RANGE.start(), *WPM_RANGE.end());
            }
        }

        Message::WpmSubmit => {
            app.wpm_input = app.config.wpm.to_string();
            app.config.save();
        }

        Message::FontChanged(size) => {
            app.config.font_size = size.clamp(*FONT_RANGE.start(), *FONT_RANGE.end());
            app.font_input = app.config.font_size.to_string();
            app.config.save();
        }

        Message::FontInput(raw) => {
            app.font_input = digits(&raw);
            if let Ok(size) = app.font_input.parse::<u32>() {
                app.config.font_size = size.clamp(*FONT_RANGE.start(), *FONT_RANGE.end());
            }
        }

        Message::FontSubmit => {
            app.font_input = app.config.font_size.to_string();
            app.config.save();
        }

        Message::ChunkChanged(chunk) => {
            app.config.chunk = (chunk as usize).clamp(*CHUNK_RANGE.start(), *CHUNK_RANGE.end());
            app.config.save();
        }

        Message::SmartPacingToggled(on) => {
            app.config.smart_pacing = on;
            app.config.save();
        }

        Message::SkinChanged(skin) => {
            app.config.skin = skin;
            app.config.save();
        }

        Message::AccentChanged(accent) => {
            app.config.accent = accent;
            app.config.save();
        }

        Message::FocusMarkChanged(mark) => {
            app.config.focus_mark = mark;
            app.config.save();
        }

        Message::TogglePanel(panel) => {
            app.panel = if app.panel == Some(panel) {
                None
            } else {
                Some(panel)
            };
        }

        Message::ToggleContext => {
            app.config.show_context = !app.config.show_context;
            app.config.save();
        }

        Message::ToggleZen => {
            app.zen = !app.zen;
            if app.zen {
                app.panel = None;
            }
        }

        Message::LeaveZen => app.zen = false,

        Message::Key(event) => return key(app, event),

        Message::Window(event) => return self::window_event(app, event),
    }

    Task::none()
}

/// Installs a freshly loaded document and restores its saved position.
fn adopt(app: &mut App, document: Document) {
    app.error = None;
    app.reader.pause();

    let resume = document
        .path()
        .and_then(|path| app.config.resume_index(path))
        .filter(|index| *index + 1 < document.len())
        .unwrap_or(0);

    app.reader.seek(resume);
    app.document = Some(document);
    remember(app);
}

/// Records the current position in the recents list and writes it to disk.
fn remember(app: &mut App) {
    let Some(document) = &app.document else {
        return;
    };
    let Some(path) = document.path() else { return };

    let path = path.to_path_buf();
    let title = document.title.clone();
    let total = document.len();

    app.config.remember(&path, &title, app.reader.index, total);
    app.config.save();
}

fn pick_file() -> Task<Message> {
    Task::perform(
        async {
            rfd::AsyncFileDialog::new()
                .set_title("Open a document")
                .add_filter("Documents", extract::supported_extensions())
                .pick_file()
                .await
                .map(|handle| handle.path().to_path_buf())
        },
        Message::FilePicked,
    )
}

fn load(app: &mut App, path: PathBuf) -> Task<Message> {
    app.loading = true;
    app.error = None;

    // Parsing a large PDF takes seconds; keep it off the UI thread.
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                Document::load(&path).map_err(|error| error.to_string())
            })
            .await
            .unwrap_or_else(|_| Err("The document reader crashed".to_string()))
        },
        |result| Message::Loaded(Box::new(result)),
    )
}

fn window_event(app: &mut App, event: window::Event) -> Task<Message> {
    match event {
        // Drag and drop.
        window::Event::FileDropped(path) => load(app, path),
        window::Event::CloseRequested => {
            remember(app);
            app.config.save();
            iced::exit()
        }
        window::Event::Unfocused => {
            // Losing focus mid-sentence and coming back to a moved position is
            // disorienting, so hold the reader where the eye left it.
            if app.reader.playing {
                app.reader.pause();
                remember(app);
            }
            Task::none()
        }
        _ => Task::none(),
    }
}

fn key(app: &mut App, event: KeyEvent) -> Task<Message> {
    let KeyEvent::KeyPressed { key, modifiers, .. } = event else {
        return Task::none();
    };

    let command = modifiers.command();
    let shift = modifiers.shift();

    match key {
        Key::Named(Named::Space) => return handle(app, Message::TogglePlay),
        Key::Named(Named::ArrowLeft) if shift => {
            return handle(app, Message::ParagraphStep(-1));
        }
        Key::Named(Named::ArrowRight) if shift => {
            return handle(app, Message::ParagraphStep(1));
        }
        Key::Named(Named::ArrowLeft) => return handle(app, Message::Step(-1)),
        Key::Named(Named::ArrowRight) => return handle(app, Message::Step(1)),
        Key::Named(Named::ArrowUp) => {
            return handle(app, Message::WpmChanged(app.config.wpm + WPM_STEP));
        }
        Key::Named(Named::ArrowDown) => {
            return update(
                app,
                Message::WpmChanged(app.config.wpm.saturating_sub(WPM_STEP)),
            );
        }
        Key::Named(Named::Home) => return handle(app, Message::Restart),
        Key::Named(Named::End) => {
            let last = app.len().saturating_sub(1) as u32;
            return handle(app, Message::Seek(last));
        }
        Key::Named(Named::Escape) => {
            if app.zen {
                return handle(app, Message::LeaveZen);
            }
            if app.error.is_some() {
                return handle(app, Message::DismissError);
            }
            if app.panel.is_some() {
                app.panel = None;
            }
        }
        Key::Character(ref c) => return character(app, c.as_str(), command),
        _ => {}
    }

    Task::none()
}

fn character(app: &mut App, c: &str, command: bool) -> Task<Message> {
    match (c, command) {
        ("o", _) => handle(app, Message::OpenDialog),
        ("v", true) => handle(app, Message::PasteRequested),
        ("r", _) => handle(app, Message::Restart),
        ("c", false) => handle(app, Message::ToggleContext),
        ("f", false) => {
            let marks = FocusMark::ALL;
            let at = marks.iter().position(|m| *m == app.config.focus_mark);
            let next = marks[(at.unwrap_or(0) + 1) % marks.len()];
            handle(app, Message::FocusMarkChanged(next))
        }
        (",", true) | ("s", false) => handle(app, Message::TogglePanel(Panel::Settings)),
        ("z", false) => handle(app, Message::ToggleZen),
        ("+", _) | ("=", _) => handle(app, Message::FontChanged(app.config.font_size + FONT_STEP)),
        ("-", _) => update(
            app,
            Message::FontChanged(app.config.font_size.saturating_sub(FONT_STEP)),
        ),
        ("1", false) => handle(app, Message::ChunkChanged(1)),
        ("2", false) => handle(app, Message::ChunkChanged(2)),
        ("3", false) => handle(app, Message::ChunkChanged(3)),
        _ => Task::none(),
    }
}

/// Keeps a text field numeric while still allowing it to be emptied.
fn digits(raw: &str) -> String {
    raw.chars().filter(char::is_ascii_digit).collect()
}
