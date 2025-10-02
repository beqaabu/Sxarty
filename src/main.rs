use iced::Alignment;
use iced::alignment;
use iced::{
    Application, Color, Command, Element, Event, Length, Settings, Subscription, Theme, executor,
    time,
    widget::{button, column, container, row, slider, text},
};
use std::path::{Path, PathBuf};
use std::process::Command as PCommand;
use std::time::Duration;

fn split_words(content: &str) -> Vec<String> {
    // Dashes often appear surrounded by spaces; attach them to the previous token.
    const DASHES: &[char] = &['—', '–', '-'];
    // Opening punctuation sticks to the next token.
    const OPENERS: &[char] = &['(', '[', '{', '“', '‘', '«'];

    let mut out: Vec<String> = Vec::new();
    let mut pending_prefix = String::new();

    for raw in content.split_whitespace() {
        // If the token is ONLY dashes (e.g., "—", "–", or "-"), attach to the previous token
        if raw.chars().all(|c| DASHES.contains(&c)) {
            if let Some(last) = out.last_mut() {
                last.push_str(raw); // attach to previous word (e.g., "star—")
            } else {
                // At start: remember as a prefix for the next real token
                pending_prefix.push_str(raw);
            }
            continue;
        }

        // If the token is a single opener like "(" with spaces around, prefix it to the next token
        if raw.chars().count() == 1 {
            let ch = raw.chars().next().unwrap();
            if OPENERS.contains(&ch) {
                pending_prefix.push(ch);
                continue;
            }
        }

        // Normal token: apply any pending prefix and push
        if !pending_prefix.is_empty() {
            let mut s = pending_prefix.clone();
            s.push_str(raw);
            out.push(s);
            pending_prefix.clear();
        } else {
            out.push(raw.to_string());
        }
    }

    // Drop any dangling prefix to avoid showing lone punctuation.
    out
}

#[derive(Debug)]
struct SpeedReader {
    words: Vec<String>,
    index: usize,
    playing: bool,
    wpm: f32,       // words per minute
    font_size: f32, // point size
    // Text inputs for precise entry
    wpm_input: String,
    font_input: String,
    // Color components (0.0-1.0)
    color_r: f32,
    color_g: f32,
    color_b: f32,
    current_file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
enum Message {
    OpenFile,
    FilePicked(Option<PathBuf>),
    FileLoaded(Result<(PathBuf, Vec<String>), String>),
    TogglePlayPressed,
    Tick,
    Next,
    Prev,
    Restart,
    // Sliders
    WpmChanged(f32),
    FontSizeChanged(f32),
    // Text inputs
    WpmTextChanged(String),
    WpmTextSubmit,
    FontTextChanged(String),
    FontTextSubmit,
    // Color sliders (0-255)
    ColorR255Changed(u16),
    ColorG255Changed(u16),
    ColorB255Changed(u16),
    EventOccurred(Event),
}

impl Application for SpeedReader {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            SpeedReader {
                words: vec![],
                index: 0,
                playing: false,
                wpm: 300.0,
                font_size: 64.0,
                wpm_input: "300".to_string(),
                font_input: "64".to_string(),
                color_r: 1.0,
                color_g: 1.0,
                color_b: 1.0,
                current_file: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Speed Reader".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::OpenFile => {
                return Command::perform(
                    async {
                        rfd::FileDialog::new()
                            .add_filter("Documents", &["txt", "pdf", "docx", "epub"])
                            .pick_file()
                    },
                    Message::FilePicked,
                );
            }
            Message::FilePicked(path_opt) => {
                if let Some(path) = path_opt {
                    return Command::perform(
                        async move {
                            let path_clone = path.clone();
                            let words_res: Result<Vec<String>, String> = match path
                                .extension()
                                .and_then(|e| e.to_str())
                                .map(|s| s.to_lowercase())
                            {
                                Some(ext) if ext == "txt" => match std::fs::read_to_string(&path) {
                                    Ok(content) => Ok(split_words(&content)),
                                    Err(e) => Err(format!("Failed to read text file: {e}")),
                                },
                                Some(ext) if ext == "pdf" => match extract_pdf_text(&path) {
                                    Ok(content) => Ok(split_words(&content)),
                                    Err(e) => Err(e),
                                },
                                Some(ext) if ext == "docx" => {
                                    match extract_with_pandoc(&path, "docx") {
                                        Ok(content) => Ok(split_words(&content)),
                                        Err(e) => Err(e),
                                    }
                                }
                                Some(ext) if ext == "epub" => {
                                    match extract_with_pandoc(&path, "epub") {
                                        Ok(content) => Ok(split_words(&content)),
                                        Err(e) => Err(e),
                                    }
                                }
                                Some(other) => {
                                    Err(format!("Unsupported file extension: .{}", other))
                                }
                                None => Err("Could not determine file extension".to_string()),
                            };

                            match words_res {
                                Ok(words) => Ok((path_clone, words)),
                                Err(err) => Err(err),
                            }
                        },
                        Message::FileLoaded,
                    );
                }
            }
            Message::FileLoaded(res) => match res {
                Ok((path, words)) => {
                    self.current_file = Some(path);
                    self.words = words;
                    self.index = 0;
                    self.playing = false;
                }
                Err(err) => {
                    eprintln!("{err}");
                }
            },
            Message::TogglePlayPressed => {
                if !self.words.is_empty() {
                    self.playing = !self.playing;
                }
            }
            Message::Tick => {
                if self.playing && self.index + 1 < self.words.len() {
                    self.index += 1;
                } else {
                    self.playing = false;
                }
            }
            Message::Next => {
                if self.index + 1 < self.words.len() {
                    self.index += 1;
                }
            }
            Message::Prev => {
                if self.index > 0 {
                    self.index -= 1;
                }
            }
            Message::Restart => {
                self.index = 0;
                self.playing = false;
            }
            Message::WpmChanged(v) => {
                let v = v.clamp(50.0, 1200.0);
                self.wpm = v;
                self.wpm_input = (v.round() as u32).to_string();
            }
            Message::FontSizeChanged(v) => {
                let v = v.clamp(16.0, 128.0);
                self.font_size = v;
                self.font_input = (v.round() as u32).to_string();
            }
            Message::WpmTextChanged(s) => {
                // Keep only digits, allow empty while typing
                let filtered: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                self.wpm_input = filtered.clone();
                if let Ok(val) = filtered.parse::<u32>() {
                    let clamped = val.clamp(50, 1200) as f32;
                    self.wpm = clamped;
                }
            }
            Message::WpmTextSubmit => {
                if let Ok(val) = self.wpm_input.parse::<u32>() {
                    let clamped = val.clamp(50, 1200) as f32;
                    self.wpm = clamped;
                    self.wpm_input = (clamped.round() as u32).to_string();
                } else {
                    // Reset to current numeric value
                    self.wpm_input = (self.wpm.round() as u32).to_string();
                }
            }
            Message::FontTextChanged(s) => {
                let filtered: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
                self.font_input = filtered.clone();
                if let Ok(val) = filtered.parse::<u32>() {
                    let clamped = val.clamp(16, 128) as f32;
                    self.font_size = clamped;
                }
            }
            Message::FontTextSubmit => {
                if let Ok(val) = self.font_input.parse::<u32>() {
                    let clamped = val.clamp(16, 128) as f32;
                    self.font_size = clamped;
                    self.font_input = (clamped.round() as u32).to_string();
                } else {
                    self.font_input = (self.font_size.round() as u32).to_string();
                }
            }
            Message::ColorR255Changed(v) => self.color_r = (v.min(255) as f32) / 255.0,
            Message::ColorG255Changed(v) => self.color_g = (v.min(255) as f32) / 255.0,
            Message::ColorB255Changed(v) => self.color_b = (v.min(255) as f32) / 255.0,
            Message::EventOccurred(Event::Keyboard(key_event)) => {
                use iced::keyboard::key::Named;
                use iced::keyboard::{Event as KeyEvent, Key};

                match key_event {
                    KeyEvent::KeyPressed { key, .. } => match key {
                        Key::Named(Named::ArrowLeft) => {
                            if self.index > 0 {
                                self.index -= 1;
                            }
                        }
                        Key::Named(Named::ArrowRight) => {
                            if self.index + 1 < self.words.len() {
                                self.index += 1;
                            }
                        }
                        Key::Named(Named::Space) => {
                            if !self.words.is_empty() {
                                self.playing = !self.playing;
                            }
                        }
                        Key::Named(Named::Home) => {
                            self.index = 0;
                            self.playing = false;
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
            Message::EventOccurred(_) => {}
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let mut subs: Vec<Subscription<Message>> =
            vec![iced::event::listen().map(Message::EventOccurred)];

        if self.playing && !self.words.is_empty() {
            let interval_ms = (60_000.0 / self.wpm) as u64;
            subs.push(time::every(Duration::from_millis(interval_ms)).map(|_| Message::Tick));
        }

        Subscription::batch(subs)
    }

    fn view(&self) -> Element<Message> {
        let title = if let Some(path) = &self.current_file {
            format!(
                "File: {}",
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
            )
        } else {
            "No file loaded".to_string()
        };

        let controls = row![
            button("Open file").on_press(Message::OpenFile),
            button(
                text(if self.playing { "Pause" } else { "Play" })
                    .horizontal_alignment(alignment::Horizontal::Center) // old API
            )
            .width(Length::Fixed(60.0))
            .on_press(Message::TogglePlayPressed),
            button("↻ Restart").on_press(Message::Restart),
            button("← Prev").on_press(Message::Prev),
            button("Next →").on_press(Message::Next),
            // WPM controls
            text("Speed:"),
            slider(50.0..=1200.0, self.wpm, Message::WpmChanged).width(Length::Fixed(200.0)),
            iced::widget::TextInput::new("WPM", &self.wpm_input)
                .on_input(Message::WpmTextChanged)
                .on_submit(Message::WpmTextSubmit)
                .width(Length::Fixed(80.0)),
            // Font size controls
            text("Font:"),
            slider(16.0..=128.0, self.font_size, Message::FontSizeChanged)
                .width(Length::Fixed(160.0)),
            iced::widget::TextInput::new("px", &self.font_input)
                .on_input(Message::FontTextChanged)
                .on_submit(Message::FontTextSubmit)
                .width(Length::Fixed(80.0)),
        ]
        .align_items(Alignment::Center)
        .spacing(10);

        let r255: u16 = (self.color_r * 255.0).round().clamp(0.0, 255.0) as u16;
        let g255: u16 = (self.color_g * 255.0).round().clamp(0.0, 255.0) as u16;
        let b255: u16 = (self.color_b * 255.0).round().clamp(0.0, 255.0) as u16;

        let color_controls = row![
            text("R"),
            slider(0u16..=255u16, r255, Message::ColorR255Changed)
                .width(Length::Fixed(180.0))
                .step(1u16),
            text("G"),
            slider(0u16..=255u16, g255, Message::ColorG255Changed)
                .width(Length::Fixed(180.0))
                .step(1u16),
            text("B"),
            slider(0u16..=255u16, b255, Message::ColorB255Changed)
                .width(Length::Fixed(180.0))
                .step(1u16),
            text(format!("RGB: {} {} {}", r255, g255, b255)),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let word = self
            .words
            .get(self.index)
            .map(String::as_str)
            .unwrap_or("Load a file to start");

        let color = Color::from_rgb(self.color_r, self.color_g, self.color_b);
        let display_text = text(word).size(self.font_size as u16).style(color);

        let content = column![
            text(title),
            controls,
            color_controls,
            container(display_text)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y(),
        ]
        .spacing(15)
        .padding(15)
        .align_items(Alignment::Center);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

fn extract_pdf_text(path: &Path) -> Result<String, String> {
    // Use `pdftotext` from poppler-utils to extract text.
    // On Ubuntu/Debian: sudo apt-get install poppler-utils
    let output = PCommand::new("pdftotext")
        .arg("-layout")
        .arg("-nopgbrk")
        .arg(path)
        .arg("-") // write to stdout
        .output()
        .map_err(|e| {
            format!("Failed to run 'pdftotext'. Is poppler-utils installed? Error: {e}")
        })?;

    if !output.status.success() {
        return Err(format!(
            "pdftotext failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    Ok(text)
}

fn extract_with_pandoc(path: &Path, from: &str) -> Result<String, String> {
    // Use `pandoc` to convert documents to plain text.
    // On Ubuntu/Debian: sudo apt-get install pandoc
    let output = PCommand::new("pandoc")
        .arg("-f")
        .arg(from)
        .arg("-t")
        .arg("plain")
        .arg(path)
        .output()
        .map_err(|e| format!("Failed to run 'pandoc'. Is it installed? Error: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "pandoc failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn main() -> iced::Result {
    SpeedReader::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(900.0, 600.0),
            ..Default::default()
        },
        antialiasing: true,
        ..Default::default()
    })
}
