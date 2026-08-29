//! Everything that can happen to the app.

use std::path::PathBuf;
use std::time::Instant;

use crate::document::Document;
use crate::ui::skin::{Accent, FocusMark, SkinId};

/// Which panel, if any, is expanded in the right rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Settings,
    Recents,
}

#[derive(Debug, Clone)]
pub enum Message {
    // Loading
    OpenDialog,
    FilePicked(Option<PathBuf>),
    OpenPath(PathBuf),
    /// Boxed: a tokenized document is far larger than any other variant, and
    /// an un-boxed one would inflate every message the runtime moves around.
    Loaded(Box<Result<Document, String>>),
    PasteRequested,
    Pasted(Option<String>),
    DismissError,
    ForgetRecents,

    // Transport
    TogglePlay,
    Step(i32),
    ParagraphStep(i32),
    Restart,
    Seek(u32),
    /// Jump straight to a token, from a click in the context panel.
    JumpTo(usize),
    Tick(Instant),

    // Settings
    WpmChanged(u32),
    WpmInput(String),
    WpmSubmit,
    FontChanged(u32),
    FontInput(String),
    FontSubmit,
    ChunkChanged(u32),
    SmartPacingToggled(bool),
    SkinChanged(SkinId),
    AccentChanged(Accent),
    FocusMarkChanged(FocusMark),

    // Chrome
    TogglePanel(Panel),
    ToggleContext,
    ToggleZen,
    LeaveZen,

    // Lifecycle
    Key(iced::keyboard::Event),
    Window(iced::window::Event),
}
