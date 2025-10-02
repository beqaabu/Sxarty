use iced::{
    executor, time, Application, Color, Command, Element, Event, Length, Settings, Subscription, Theme,
    widget::{button, column, container, row, slider, text},
};
use iced::Alignment;
use std::path::PathBuf;
use std::time::Duration;

fn split_words(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .map(|w| w.to_string())
        .collect()
}

#[derive(Debug)]
struct SpeedReader {
    words: Vec<String>,
    index: usize,
    playing: bool,
    wpm: f32,         // words per minute
    font_size: f32,   // point size
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
    WpmChanged(f32),
    FontSizeChanged(f32),
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
                            .add_filter("Text", &["txt"]).pick_file()
                    },
                    Message::FilePicked,
                )
            }
            Message::FilePicked(path_opt) => {
                if let Some(path) = path_opt {
                    return Command::perform(
                        async move {
                            let path_clone = path.clone();
                            match std::fs::read_to_string(&path) {
                                Ok(content) => {
                                    let words = split_words(&content);
                                    Ok((path_clone, words))
                                }
                                Err(e) => Err(format!("Failed to read file: {e}")),
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
                self.wpm = v.max(1.0);
            }
            Message::FontSizeChanged(v) => {
                self.font_size = v.max(8.0);
            }
            Message::ColorR255Changed(v) => self.color_r = (v.min(255) as f32) / 255.0,
            Message::ColorG255Changed(v) => self.color_g = (v.min(255) as f32) / 255.0,
            Message::ColorB255Changed(v) => self.color_b = (v.min(255) as f32) / 255.0,
            Message::EventOccurred(Event::Keyboard(key_event)) => {
                use iced::keyboard::{Event as KeyEvent, Key};
                use iced::keyboard::key::Named;

                match key_event {
                    KeyEvent::KeyPressed { key, .. } => match key {
                        Key::Named(Named::ArrowLeft) => {
                            if self.index > 0 { self.index -= 1; }
                        }
                        Key::Named(Named::ArrowRight) => {
                            if self.index + 1 < self.words.len() { self.index += 1; }
                        }
                        Key::Named(Named::Space) => {
                            if !self.words.is_empty() { self.playing = !self.playing; }
                        }
                        Key::Named(Named::Home) => {
                            self.index = 0; self.playing = false;
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
        let mut subs: Vec<Subscription<Message>> = vec![
            iced::event::listen().map(Message::EventOccurred),
        ];

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
                path.file_name().and_then(|s| s.to_str()).unwrap_or_default()
            )
        } else {
            "No file loaded".to_string()
        };

        let controls = row![
            button("Open .txt").on_press(Message::OpenFile),
            button(if self.playing { "Pause" } else { "Play" }).on_press(Message::TogglePlayPressed),
            button("⟲ Restart").on_press(Message::Restart),
            button("← Prev").on_press(Message::Prev),
            button("Next →").on_press(Message::Next),
            text(format!("Speed: {} WPM", self.wpm as u32)),
            slider(50.0..=1200.0, self.wpm, Message::WpmChanged).width(Length::Fixed(200.0)),
            text(format!("Font: {}", self.font_size as u32)),
            slider(16.0..=128.0, self.font_size, Message::FontSizeChanged).width(Length::Fixed(160.0)),
        ]
        .align_items(Alignment::Center)
        .spacing(10);

        let r255: u16 = (self.color_r * 255.0).round().clamp(0.0, 255.0) as u16;
        let g255: u16 = (self.color_g * 255.0).round().clamp(0.0, 255.0) as u16;
        let b255: u16 = (self.color_b * 255.0).round().clamp(0.0, 255.0) as u16;

        let color_controls = row![
            text("R"),
            slider(0u16..=255u16, r255, Message::ColorR255Changed).width(Length::Fixed(180.0)).step(1u16),
            text("G"),
            slider(0u16..=255u16, g255, Message::ColorG255Changed).width(Length::Fixed(180.0)).step(1u16),
            text("B"),
            slider(0u16..=255u16, b255, Message::ColorB255Changed).width(Length::Fixed(180.0)).step(1u16),
            text(format!(
                "RGB: {} {} {}",
                r255, g255, b255
            )),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let word = self
            .words
            .get(self.index)
            .map(String::as_str)
            .unwrap_or("Load a file to start");

        let color = Color::from_rgb(self.color_r, self.color_g, self.color_b);
        let display_text = text(word)
            .size(self.font_size as u16)
            .style(color);

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

    fn theme(&self) -> Theme { Theme::Dark }
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
