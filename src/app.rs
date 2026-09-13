use iced::futures::SinkExt;
use iced::widget::{checkbox, column, container, radio, row, scrollable, text};

use crate::config::ThemeChoice;
use iced::window;
use iced::{color, Color, Element, Length, Subscription, Task, Theme};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::state_machine::GrabEvent;

static GRAB_RX: OnceLock<Arc<Mutex<Option<UnboundedReceiver<GrabEvent>>>>> = OnceLock::new();

/// Requests from native UI (the status-bar menu) into the iced app.
#[derive(Debug, Clone, Copy)]
pub enum UiEvent {
    OpenSettings,
}

struct UiChannel {
    tx: UnboundedSender<UiEvent>,
    rx: Mutex<Option<UnboundedReceiver<UiEvent>>>,
}

fn ui_channel() -> &'static UiChannel {
    static CHANNEL: OnceLock<UiChannel> = OnceLock::new();
    CHANNEL.get_or_init(|| {
        let (tx, rx) = unbounded_channel();
        UiChannel { tx, rx: Mutex::new(Some(rx)) }
    })
}

/// Hand a native UI event to the app. Safe from any thread, before or after
/// the app is running.
pub fn request(event: UiEvent) {
    let _ = ui_channel().tx.send(event);
}

const SETTINGS_COLUMNS: usize = 3;

const TEXT_SIZE: f32 = 28.0;
const CELL_PADDING: f32 = 14.0;
const CELL_SPACING: f32 = 4.0;
// Outer padding (20px) plus a little spare room for rounding.
const PADDING: f32 = 28.0;
const WINDOW_HEIGHT: f32 = 70.0;
const PAGE_COUNTER_WIDTH: f32 = 80.0;

fn window_width_for(variants: &[String], items_per_page: usize) -> f32 {
    if items_per_page == 0 {
        return row_width_for(variants);
    }
    // Reserve the widest page once, so cycling does not move or resize the picker.
    let width = variants
        .chunks(items_per_page)
        .map(row_width_for)
        .fold(PADDING, f32::max);
    width
        + if variants.len() > items_per_page {
            PAGE_COUNTER_WIDTH
        } else {
            0.0
        }
}

fn page_range(count: usize, selected_index: usize, items_per_page: usize) -> std::ops::Range<usize> {
    if items_per_page == 0 {
        return 0..count;
    }
    let start = selected_index / items_per_page * items_per_page;
    start..start + (count - start).min(items_per_page)
}

fn row_width_for(variants: &[String]) -> f32 {
    use iced::advanced::{
        graphics::text::Paragraph,
        text::{Paragraph as _, Text},
    };

    PADDING
        + variants
            .iter()
            .map(|variant| {
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
            })
            .sum::<f32>()
}

fn variant_label(ch: &str) -> String {
    // Display-only bases make combining marks visible. LRM prevents iced's
    // shrink-width labels from clipping RTL glyphs at the far edge.
    let base = if matches!(ch.chars().next(),
        Some('\u{0300}'..='\u{036f}' | '\u{05b0}'..='\u{05bd}' | '\u{05bf}' | '\u{05c1}' | '\u{05c2}' | '\u{05c7}')) { "◌" } else { "" };
    format!("\u{200e}{base}{ch}")
}

/// Frame rect (x, y, w, h) of the window being typed in, refreshed before
/// opening the overlay — the overlay opens centered on it (i.e.
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
        use iced::advanced::{
            graphics::text::Paragraph,
            text::{Paragraph as _, Text},
        };
        for variant in [
            "﷼",
            "؋",
            "°C",
            "°F",
            "V\u{0307}",
            "…",
            "\u{0301}",
            "SS",
            "א",
            "אַ",
            "ײַ",
            "דזש",
            "\u{05b7}",
            "Ꭰ",
            "ꭰ",
            "𐓘",
            "𐒰",
            "ᐁ",
            "ᢰ",
            "𑪰",
        ] {
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
            let glyphs: Vec<_> = paragraph
                .buffer()
                .layout_runs()
                .flat_map(|run| run.glyphs)
                .collect();
            assert!(!glyphs.is_empty());
            assert!(
                glyphs
                    .iter()
                    .all(|g| g.x >= 0.0 && g.x + g.w <= visible_width + 0.1),
                "{content:?}: glyphs outside visible width {visible_width}"
            );
            let row = vec![variant.to_string(); 4];
            let required_width = 20.0 + 4.0 * (visible_width + 28.0) + 3.0 * 4.0;
            assert!(
                window_width_for(&row, 12) >= required_width,
                "{variant:?}: shaped row is wider than the overlay window"
            );
        }
    }

    #[test]
    fn window_width_grows_with_variant_count() {
        assert!(window_width_for(&["é".into()], 12) < window_width_for(&["é".into(), "é".into()], 12));
        assert!(window_width_for(&["C".into()], 12) < window_width_for(&["°C".into()], 12));
        assert_eq!(window_width_for(&[], 12), PADDING);
    }

    #[test]
    fn zero_shows_the_complete_list_without_counter_space() {
        assert_eq!(page_range(0, 0, 0), 0..0);
        assert_eq!(window_width_for(&[], 0), PADDING);
        let variants = vec!["ᑌ".to_string(); 113];
        for index in 0..variants.len() {
            assert_eq!(page_range(variants.len(), index, 0), 0..variants.len());
        }
        assert_eq!(window_width_for(&variants, 0), row_width_for(&variants));
    }

    #[test]
    fn long_lists_keep_every_selection_on_a_bounded_page() {
        for size in [1, 8, 12, 24, 1000] {
            assert_eq!(page_range(0, 0, size), 0..0);
            for count in [1, 8, 9, 12, 13, 24, 25, 113, 726] {
                for index in 0..count {
                    let page = page_range(count, index, size);
                    assert!(page.contains(&index));
                    assert!(page.len() <= size);
                    assert!(page.end <= count);
                }
            }
            let variants = vec!["ᑌ".to_string(); 113];
            let counter = if variants.len() > size { PAGE_COUNTER_WIDTH } else { 0.0 };
            assert_eq!(window_width_for(&variants, size), row_width_for(&variants[..size.min(variants.len())]) + counter);
        }
        assert_eq!(page_range(25, 11, 12), 0..12);
        assert_eq!(page_range(25, 12, 12), 12..24);
        assert_eq!(page_range(25, 24, 12), 24..25);
        let mut variants = vec!["ᑌ".to_string(); 113];
        assert_eq!(
            window_width_for(&variants, 12),
            row_width_for(&variants[..12]) + PAGE_COUNTER_WIDTH
        );
        // A wider choice on a later page must also fit; the first page is not sufficient.
        variants[90] = "דזש".to_string();
        let width = window_width_for(&variants, 12);
        for page in variants.chunks(12) {
            assert!(width >= row_width_for(page) + PAGE_COUNTER_WIDTH);
        }
        assert!(width < 800.0, "paged picker unexpectedly wide: {width}");
    }

    #[test]
    fn overlay_follows_window_in_global_logical_coordinates() {
        // Screens left of or above the primary screen have negative origins.
        for (anchor, expected) in [
            ((2000.0, 100.0, 1000.0, 800.0), (2400.0, 465.0)),
            ((-1600.0, -900.0, 1200.0, 800.0), (-1100.0, -535.0)),
        ] {
            set_overlay_anchor(Some(anchor));
            let window::Position::Specific(point) = overlay_position(200.0) else {
                panic!("expected focused-window position");
            };
            assert_eq!((point.x, point.y), expected);
        }
        // An unavailable focused window must clear the previous anchor.
        set_overlay_anchor(None);
        assert!(matches!(overlay_position(200.0), window::Position::Centered));
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    ShowOverlay(Vec<String>, usize),
    UpdateSelection(usize),
    HideOverlay,
    InjectChar,
    WindowOpened(window::Id),
    WindowClosed(window::Id),
    OpenSettings,
    ToggleLanguage(String, bool),
    SetTheme(ThemeChoice),
    Noop,
}

pub struct App {
    items_per_page: usize,
    variants: Vec<String>,
    selected_index: usize,
    overlay_window: Option<window::Id>,
    settings_window: Option<window::Id>,
    /// Enabled languages / symbol sets, in config order.
    languages: Vec<String>,
    /// Appearance from config; `System` follows macOS when a window opens.
    theme_choice: ThemeChoice,
    /// Resolved appearance for the open windows (macOS glass adapts to what is
    /// behind it; our text has to follow).
    dark: bool,
}

/// Whether windows should render dark for `choice`, sampling the system
/// appearance for `System` (macOS only; other platforms keep light).
fn resolve_dark(choice: ThemeChoice) -> bool {
    match choice {
        ThemeChoice::Light => false,
        ThemeChoice::Dark => true,
        #[cfg(target_os = "macos")]
        ThemeChoice::System => crate::macos::is_dark_appearance(),
        #[cfg(not(target_os = "macos"))]
        ThemeChoice::System => false,
    }
}

impl App {
    pub fn new(grab_rx: Arc<Mutex<Option<UnboundedReceiver<GrabEvent>>>>, items_per_page: usize) -> (Self, Task<Message>) {
        GRAB_RX.set(grab_rx).ok();
        // Development aid: QUICKACCENT_DEMO=overlay|settings opens that window
        // at startup without needing the keyboard grab (or its permissions).
        let demo = std::env::var("QUICKACCENT_DEMO");
        if let Ok(which) = &demo {
            eprintln!("[QuickAccent] demo mode: {which}");
        }
        let boot = match demo.as_deref() {
            Ok("overlay") => Task::done(Message::ShowOverlay(
                ["é", "è", "ê", "ë", "ē", "ė"].map(String::from).to_vec(),
                1,
            )),
            Ok("settings") => Task::done(Message::OpenSettings),
            _ => Task::none(),
        };
        (
            App {
                items_per_page,
                variants: Vec::new(),
                selected_index: 0,
                overlay_window: None,
                settings_window: None,
                languages: Vec::new(),
                theme_choice: ThemeChoice::System,
                dark: false,
            },
            boot,
        )
    }

    /// Keep the settings window's native chrome (title bar) on the resolved
    /// theme; iced only paints the content area.
    fn sync_settings_appearance(&self) -> Task<Message> {
        #[cfg(target_os = "macos")]
        if let Some(id) = self.settings_window {
            let dark = self.dark;
            return window::run_with_handle(id, move |handle| {
                use iced::window::raw_window_handle::RawWindowHandle;
                if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
                    crate::macos::apply_window_appearance(appkit.ns_view.as_ptr(), dark);
                }
            })
            .map(|_| Message::Noop);
        }
        Task::none()
    }

    /// Re-read appearance from config (it may have been edited by hand) and
    /// resolve it for the windows about to open.
    fn refresh_appearance(&mut self) {
        self.theme_choice = crate::config::read_config()
            .map(|c| c.theme_parsed())
            .unwrap_or(ThemeChoice::System);
        self.dark = resolve_dark(self.theme_choice);
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ShowOverlay(variants, index) => {
                self.selected_index = index;
                let width = window_width_for(&variants, self.items_per_page);
                self.variants = variants;

                if let Some(id) = self.overlay_window {
                    // Window already open, just resize and update
                    return window::resize(id, iced::Size::new(width, WINDOW_HEIGHT));
                }

                #[cfg(target_os = "macos")]
                set_overlay_anchor(crate::macos::focused_window_rect());
                self.refresh_appearance();

                let settings = overlay_settings(width);
                log::debug!("opening overlay window at {:?}", settings.position);
                let (id, open_task) = window::open(settings);
                log::debug!("overlay window id {id:?}");
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
            Message::WindowOpened(id) => {
                log::debug!("window opened {id:?}");
                #[cfg(target_os = "macos")]
                if self.overlay_window == Some(id) {
                    // Runs on the event-loop thread before the first frame is
                    // shown, so the glass is there from the start.
                    let dark = self.dark;
                    return window::run_with_handle(id, move |handle| {
                        use iced::window::raw_window_handle::RawWindowHandle;
                        if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
                            crate::macos::attach_glass_backdrop(appkit.ns_view.as_ptr(), dark);
                        }
                    })
                    .map(|_| Message::Noop);
                }
                if self.settings_window == Some(id) {
                    #[cfg(target_os = "macos")]
                    {
                        crate::macos::activate_app();
                        return Task::batch([self.sync_settings_appearance(), window::gain_focus(id)]);
                    }
                    #[cfg(not(target_os = "macos"))]
                    return window::gain_focus(id);
                }
                Task::none()
            }
            Message::WindowClosed(id) => {
                if self.settings_window == Some(id) {
                    self.settings_window = None;
                }
                if self.overlay_window == Some(id) {
                    self.overlay_window = None;
                    self.variants.clear();
                }
                Task::none()
            }
            Message::OpenSettings => {
                if let Some(id) = self.settings_window {
                    #[cfg(target_os = "macos")]
                    crate::macos::activate_app();
                    return window::gain_focus(id);
                }
                self.languages = crate::config::read_config()
                    .map(|c| c.languages)
                    .unwrap_or_else(|| crate::config::Config::default().languages);
                self.refresh_appearance();
                let (id, open_task) = window::open(window::Settings {
                    size: iced::Size::new(560.0, 640.0),
                    min_size: Some(iced::Size::new(420.0, 360.0)),
                    position: window::Position::Centered,
                    level: window::Level::Normal,
                    exit_on_close_request: true,
                    ..Default::default()
                });
                self.settings_window = Some(id);
                open_task.map(Message::WindowOpened)
            }
            Message::ToggleLanguage(name, enabled) => {
                log::debug!("toggle {name} -> {enabled}");
                if enabled {
                    if !self.languages.contains(&name) {
                        self.languages.push(name);
                    }
                } else {
                    self.languages.retain(|l| *l != name);
                }
                // Apply now; the config watcher will reload once more when the
                // file lands, which is harmless.
                crate::mappings::reload(&self.languages);
                if let Err(e) = crate::config::set_languages(&self.languages) {
                    eprintln!("[QuickAccent] Failed to save languages: {e}");
                }
                Task::none()
            }
            Message::SetTheme(choice) => {
                self.theme_choice = choice;
                self.dark = resolve_dark(choice);
                if let Err(e) = crate::config::set_theme(choice) {
                    eprintln!("[QuickAccent] Failed to save theme: {e}");
                }
                self.sync_settings_appearance()
            }
            Message::Noop => Task::none(),
        }
    }

    pub fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        if self.settings_window == Some(window_id) {
            return self.settings_view();
        }
        if self.overlay_window != Some(window_id) || self.variants.is_empty() {
            return container(text(""))
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        // macOS: chips float on the glass backdrop, tinted for the current
        // appearance. Elsewhere: the opaque dark panel.
        let glass = cfg!(target_os = "macos");
        let (chip, chip_text, selected) = match (glass, self.dark) {
            (true, true) => (color!(0xFFFFFF, 0.16), color!(0xFFFFFF), color!(0x0A84FF)),
            (true, false) => (color!(0x000000, 0.08), color!(0x1C1C1E), color!(0x007AFF)),
            (false, _) => (color!(0x3C3C3C), color!(0xCCCCCC), color!(0x4A90D9)),
        };

        let page = page_range(self.variants.len(), self.selected_index, self.items_per_page);
        let mut cells: Vec<Element<Message>> = self
            .variants
            .iter()
            .enumerate()
            .skip(page.start)
            .take(page.len())
            .map(|(i, ch)| {
                let is_selected = i == self.selected_index;
                let label = text(variant_label(ch))
                    .size(TEXT_SIZE)
                    .font(iced::Font::DEFAULT)
                    .shaping(text::Shaping::Advanced)
                    .wrapping(text::Wrapping::None);

                let cell = container(label)
                    .padding([8.0, CELL_PADDING])
                    .style(move |_theme: &Theme| container::Style {
                        background: Some(iced::Background::Color(if is_selected {
                            selected
                        } else {
                            chip
                        })),
                        border: iced::Border {
                            radius: 8.0.into(),
                            ..Default::default()
                        },
                        text_color: Some(if is_selected { color!(0xFFFFFF) } else { chip_text }),
                        ..Default::default()
                    });

                cell.into()
            })
            .collect();

        if self.items_per_page > 0 && self.variants.len() > self.items_per_page {
            cells.push(
                container(
                    text(format!(
                        "{}/{}",
                        self.selected_index + 1,
                        self.variants.len()
                    ))
                    .size(14)
                    .color(color!(0xCCCCCC)),
                )
                .center_x(PAGE_COUNTER_WIDTH)
                .into(),
            );
        }

        container(row(cells).spacing(CELL_SPACING).align_y(iced::Alignment::Center))
            .padding(10)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(move |_theme: &Theme| container::Style {
                background: (!glass).then_some(iced::Background::Color(color!(0x2D2D2D, 0.95))),
                border: iced::Border {
                    radius: 12.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
    }

    fn settings_view(&self) -> Element<'_, Message> {
        let section = |title: &str, names: &'static [&'static str]| -> Element<'_, Message> {
            let rows = names.chunks(SETTINGS_COLUMNS).map(|chunk| {
                let mut cells: Vec<Element<Message>> = chunk
                    .iter()
                    .map(|name| {
                        let enabled = self.languages.iter().any(|l| l == name);
                        checkbox(*name, enabled)
                            .on_toggle(move |on| Message::ToggleLanguage(name.to_string(), on))
                            .width(Length::Fill)
                            .into()
                    })
                    .collect();
                // Keep the grid aligned on a short last row.
                while cells.len() < SETTINGS_COLUMNS {
                    cells.push(iced::widget::Space::with_width(Length::Fill).into());
                }
                row(cells).spacing(8).into()
            });
            column(
                std::iter::once(text(title.to_string()).size(15).into())
                    .chain(rows)
                    .collect::<Vec<Element<Message>>>(),
            )
            .spacing(8)
            .into()
        };

        let appearance: Element<'_, Message> = column(vec![
            text("Appearance").size(15).into(),
            row(ThemeChoice::ALL
                .iter()
                .map(|&choice| {
                    radio(choice.label(), choice, Some(self.theme_choice), Message::SetTheme)
                        .width(Length::Fill)
                        .into()
                })
                .collect::<Vec<Element<Message>>>())
            .spacing(8)
            .into(),
        ])
        .spacing(8)
        .into();

        let body = column(vec![
            text("QuickAccent").size(22).into(),
            text("Hold a letter, press Space, pick a variant, release the letter.")
                .size(13)
                .into(),
            appearance,
            section("Languages", crate::mappings::LANGUAGES),
            section("Symbol sets", crate::mappings::SYMBOL_SETS),
            text(format!(
                "Changes apply immediately and are saved to {}",
                crate::config::config_path().display()
            ))
            .size(11)
            .into(),
        ])
        .spacing(18)
        .padding(24);

        container(scrollable(body))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(iced::Background::Color(theme.palette().background)),
                ..Default::default()
            })
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            Subscription::run(grab_subscription),
            Subscription::run(ui_subscription),
            window::close_events().map(Message::WindowClosed),
        ])
    }

    pub fn theme(&self, window_id: window::Id) -> Theme {
        if self.settings_window == Some(window_id) {
            if self.dark { Theme::Dark } else { Theme::Light }
        } else {
            Theme::CatppuccinMocha
        }
    }

    /// Window backgrounds. On macOS the picker window is see-through so the
    /// glass backdrop shows; the settings window paints its own background.
    pub fn style(&self, theme: &Theme) -> iced::daemon::Appearance {
        iced::daemon::Appearance {
            background_color: if cfg!(target_os = "macos") {
                Color::TRANSPARENT
            } else {
                theme.palette().background
            },
            text_color: theme.palette().text,
        }
    }
}

fn ui_subscription() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(8, |mut output| async move {
        let mut rx = ui_channel().rx.lock().unwrap().take().expect("ui channel already taken");
        while let Some(event) = rx.recv().await {
            let msg = match event {
                UiEvent::OpenSettings => Message::OpenSettings,
            };
            output.send(msg).await.ok();
        }
        std::future::pending::<()>().await;
    })
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
