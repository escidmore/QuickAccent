use iced::futures::SinkExt;
use iced::widget::{container, row, text};
use iced::window;
use iced::{color, Element, Length, Subscription, Task, Theme};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::state_machine::GrabEvent;

static GRAB_RX: std::sync::OnceLock<Arc<Mutex<Option<UnboundedReceiver<GrabEvent>>>>> =
    std::sync::OnceLock::new();

const TEXT_SIZE: f32 = 28.0;
const CELL_PADDING: f32 = 14.0;
const CELL_SPACING: f32 = 4.0;
// Outer padding (20px) plus a little spare room for rounding.
const PADDING: f32 = 28.0;
const WINDOW_HEIGHT: f32 = 70.0;

fn window_width_for(variants: &[String]) -> f32 {
    use iced::advanced::{graphics::text::Paragraph, text::{Paragraph as _, Text}};

    PADDING + variants.iter().map(|variant| {
        let content = variant_label(variant);
        let paragraph = Paragraph::with_text(Text {
            content: &content,
            bounds: iced::Size::INFINITY,
            size: TEXT_SIZE.into(),
            line_height: Default::default(),
            font: iced::Font::DEFAULT,
            horizontal_alignment: iced::alignment::Horizontal::Left,
            vertical_alignment: iced::alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        });
        paragraph.min_bounds().width.ceil() + 2.0 * CELL_PADDING + CELL_SPACING
    }).sum::<f32>()
}

fn variant_label(ch: &str) -> String {
    // Display-only bases make combining marks visible. LRM prevents iced's
    // shrink-width labels from clipping RTL glyphs at the far edge.
    let base = if matches!(ch.chars().next(),
        Some('\u{0300}'..='\u{036f}' | '\u{05b0}'..='\u{05bd}' | '\u{05bf}' | '\u{05c1}' | '\u{05c2}' | '\u{05c7}')) { "◌" } else { "" };
    format!("\u{200e}{base}{ch}")
}

/// Frame rect (x, y, w, h) of the window being typed in, set by the grab
/// thread right before ShowOverlay — the overlay opens centered on it (i.e.
/// on the monitor in use). None = center on the primary monitor.
static OVERLAY_ANCHOR: Mutex<Option<(f32, f32, f32, f32)>> = Mutex::new(None);

pub fn set_overlay_anchor(anchor: Option<(f32, f32, f32, f32)>) {
    *OVERLAY_ANCHOR.lock().unwrap() = anchor;
}

fn overlay_position(width: f32) -> window::Position {
    match *OVERLAY_ANCHOR.lock().unwrap() {
        Some((x, y, w, h)) => window::Position::Specific(iced::Point::new(
            x + (w - width) / 2.0,
            y + (h - WINDOW_HEIGHT) / 2.0,
        )),
        None => window::Position::Centered,
    }
}

fn overlay_settings(width: f32) -> window::Settings {
    #[allow(unused_mut)]
    let mut settings = window::Settings {
        size: iced::Size::new(width, WINDOW_HEIGHT),
        decorations: false,
        transparent: true,
        level: window::Level::AlwaysOnTop,
        position: overlay_position(width),
        ..Default::default()
    };
    #[cfg(target_os = "linux")]
    {
        settings.platform_specific.override_redirect = true;
        settings.platform_specific.application_id = "quickaccent".into();
    }
    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_glyphs_stay_inside_the_visible_label() {
        use iced::advanced::{text::{Paragraph as _, Text}, graphics::text::Paragraph};
        for variant in ["﷼", "؋", "°C", "°F", "V\u{0307}", "…", "\u{0301}", "SS", "א", "אַ", "ײַ", "דזש", "\u{05b7}"] {
            let content = variant_label(variant);
            let paragraph = Paragraph::with_text(Text {
                content: &content,
                bounds: iced::Size::new(1000.0, 42.0),
                size: 28.0.into(),
                line_height: Default::default(),
                font: iced::Font::DEFAULT,
                horizontal_alignment: iced::alignment::Horizontal::Left,
                vertical_alignment: iced::alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: Default::default(),
            });
            let visible_width = paragraph.min_bounds().width;
            let glyphs: Vec<_> = paragraph.buffer().layout_runs().flat_map(|run| run.glyphs).collect();
            assert!(!glyphs.is_empty());
            assert!(glyphs.iter().all(|g| g.x >= 0.0 && g.x + g.w <= visible_width + 0.1),
                "{content:?}: glyphs outside visible width {visible_width}");
            let row = vec![variant.to_string(); 4];
            let required_width = 20.0 + 4.0 * (visible_width + 28.0) + 3.0 * 4.0;
            assert!(window_width_for(&row) >= required_width,
                "{variant:?}: shaped row is wider than the overlay window");
        }
    }

    #[test]
    fn window_width_grows_with_variant_count() {
        assert!(window_width_for(&["é".into()]) < window_width_for(&["é".into(), "é".into()]));
        assert!(window_width_for(&["C".into()]) < window_width_for(&["°C".into()]));
        assert_eq!(window_width_for(&[]), PADDING);
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ShowOverlay(Vec<String>, usize),
    UpdateSelection(usize),
    HideOverlay,
    InjectChar,
    WindowOpened(window::Id),
}

pub struct App {
    variants: Vec<String>,
    selected_index: usize,
    overlay_window: Option<window::Id>,
}

impl App {
    pub fn new(grab_rx: Arc<Mutex<Option<UnboundedReceiver<GrabEvent>>>>) -> (Self, Task<Message>) {
        GRAB_RX.set(grab_rx).ok();
        (
            App {
                variants: Vec::new(),
                selected_index: 0,
                overlay_window: None,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ShowOverlay(variants, index) => {
                self.selected_index = index;
                let width = window_width_for(&variants);
                self.variants = variants;

                if let Some(id) = self.overlay_window {
                    // Window already open, just resize and update
                    return window::resize(id, iced::Size::new(width, WINDOW_HEIGHT));
                }

                let settings = overlay_settings(width);
                let (id, open_task) = window::open(settings);
                self.overlay_window = Some(id);
                open_task.map(Message::WindowOpened)
            }
            Message::UpdateSelection(index) => {
                self.selected_index = index;
                Task::none()
            }
            // On InjectChar the injection already happened (Linux: grab
            // thread via uinput; macOS: grab dispatch) — just close.
            Message::HideOverlay | Message::InjectChar => {
                self.variants.clear();
                if let Some(id) = self.overlay_window.take() {
                    return window::close(id);
                }
                Task::none()
            }
            Message::WindowOpened(_id) => Task::none(),
        }
    }

    pub fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        if self.overlay_window != Some(window_id) || self.variants.is_empty() {
            return container(text(""))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        let cells: Vec<Element<Message>> = self
            .variants
            .iter()
            .enumerate()
            .map(|(i, ch)| {
                let is_selected = i == self.selected_index;
                let label = text(variant_label(ch))
                    .size(TEXT_SIZE)
                    .font(iced::Font::DEFAULT)
                    .shaping(text::Shaping::Advanced)
                    .wrapping(text::Wrapping::None);

                let cell = container(label)
                    .padding([8.0, CELL_PADDING])
                    .style(move |_theme: &Theme| {
                        if is_selected {
                            container::Style {
                                background: Some(iced::Background::Color(color!(0x4A90D9))),
                                border: iced::Border {
                                    radius: 6.0.into(),
                                    ..Default::default()
                                },
                                text_color: Some(color!(0xFFFFFF)),
                                ..Default::default()
                            }
                        } else {
                            container::Style {
                                background: Some(iced::Background::Color(color!(0x3C3C3C))),
                                border: iced::Border {
                                    radius: 6.0.into(),
                                    ..Default::default()
                                },
                                text_color: Some(color!(0xCCCCCC)),
                                ..Default::default()
                            }
                        }
                    });

                cell.into()
            })
            .collect();

        container(row(cells).spacing(CELL_SPACING).align_y(iced::Alignment::Center))
            .padding(10)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(color!(0x2D2D2D, 0.95))),
                border: iced::Border {
                    radius: 12.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::run(grab_subscription)
    }

    pub fn theme(&self, _window: window::Id) -> Theme {
        Theme::CatppuccinMocha
    }
}

fn grab_subscription() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(50, |mut output| async move {
        let rx_holder = GRAB_RX.get().expect("GRAB_RX not initialized");
        let mut rx = rx_holder
            .lock()
            .unwrap()
            .take()
            .expect("grab_rx already taken");

        // recv() returns None when the grab side is gone (grab disabled or
        // its thread died) — stop instead of busy-looping on a closed channel.
        while let Some(event) = rx.recv().await {
            let msg = match event {
                GrabEvent::ShowOverlay { variants, index } => {
                    Message::ShowOverlay(variants, index)
                }
                GrabEvent::UpdateSelection(index) => Message::UpdateSelection(index),
                GrabEvent::HideOverlay => Message::HideOverlay,
                GrabEvent::InjectChar(_) => Message::InjectChar,
                GrabEvent::FalseStart => Message::HideOverlay,
            };
            output.send(msg).await.ok();
        }
        std::future::pending::<()>().await;
    })
}
