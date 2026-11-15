// Comments for me to understand code (im average to Rust), but also for you to understand what is going on

// This syntax down below is an attribute and it talks to the entire file
// it basically means: Only on releases versions, tell windows to not open a terminal window
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// `mod` declares a module and attaches a file to the crate's module tree
// (files in src/ are NOT compiled just by existing): this is what makes
// src/settings.rs part of the build
mod settings;

// headless UI test for the Theme menu (compiled only by `cargo test`)
#[cfg(test)]
mod ui;

// imports bring code from other crates, so we can write less code
// so, this ones below help you write less code, and make it more readable
use eframe::{NativeOptions, egui};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

// translations embedded in the exe at compile time
const IT: &str = include_str!("../assets/lang/it.txt");

#[derive(Clone, Copy, Debug, PartialEq)]
enum Language {
    English,
    Italian,
}

fn load_language(source: &'static str) -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new(); // starts empty, we fill it line by line

    for line in source.lines() {
        // skip empty lines and comments (lines starting with #)
        if line.trim().is_empty() || line.starts_with('#') {
            continue; // go straight to the next line
        }

        // split at the FIRST " = ": left side is the English key, right side the translation
        // split_once returns None if the line has no " = ", so bad lines are just ignored
        if let Some((key, value)) = line.split_once(" = ") {
            map.insert(key.trim(), value.trim());
        }
    }

    map // last expression without `;` = the returned value
}
fn strings_for(lang: Language) -> HashMap<&'static str, &'static str> {
    match lang {
        Language::English => HashMap::new(),
        Language::Italian => load_language(IT),
    }
}
// translation helper: free function, same level as skip_button()
fn tr(map: &HashMap<&'static str, &'static str>, key: &'static str) -> &'static str {
    // if the translation is missing, show the English key itself
    map.get(key).copied().unwrap_or(key)
}

fn main() -> eframe::Result {
    // ^ main returns a Result, which can be Ok or Err

    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_icon(load_icon()),
        ..Default::default() // all the other fields keep their default value
    };

    eframe::run_native(
        "WaveR",
        options,
        Box::new(|cc| {
            cc.egui_ctx
                .global_style_mut(|style| style.interaction.selectable_labels = false);
            Ok(Box::new(WaveR::new()))
        }),
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum RepeatMode {
    Off,
    One,
    All,
}

// Which theme the user wants. System = follow the OS (egui switches by
// itself when the OS changes theme). Custom = a palette derived from
// the user's own background color (see custom_visuals below).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Theme {
    System,
    Dark,
    Light,
    Custom,
}

struct WaveR {
    // ^ a struct is a collection of data, like a class in other languages
    _stream: OutputStream, // connection to the audio device, must stay alive or the sound stops
    sink: Sink,            // the song queue, it has the play and pause commands
    current_track: Option<String>, // "either there is a string, or there is nothing" (like null)
    total: Option<Duration>, // total duration of the song, None if unknown
    drag_pos: Option<f32>, // position chosen while dragging the bar (None = not dragging)
    queue: Vec<PathBuf>,   // position to where the song is saved
    current_path: Option<PathBuf>, // None when nothing has been played yet
    history: Vec<PathBuf>, // the last element is the most recent song
    show_about: bool,
    show_queue: bool,
    show_history: bool,
    shuffle: bool,
    repeat: RepeatMode,
    show_shuffle_button: bool,
    show_loop_button: bool,
    theme: Theme,
    custom_bg: egui::Color32, // the user's own background (used by Theme::Custom)
    language: Language,
    strings: Rc<HashMap<&'static str, &'static str>>, // the active translations
    last_saved: settings::Settings,                   // the settings as last written to disk
    last_look: Option<(Theme, egui::Color32)>,        // the look as last applied to the context
}

impl WaveR {
    // ^ an impl block defines the functions (methods) of the struct
    fn new() -> Self {
        // ^ Self is a shortcut for WaveR
        let stream =
            OutputStreamBuilder::open_default_stream().expect("no audio output device found"); // open the default audio device
        // ^ .expect() crashes with this message if there is no audio device
        let sink = Sink::connect_new(stream.mixer()); // connect the queue to the audio mixer

        // persisted settings are read ONCE, here at construction time:
        // the app then starts in the language the user chose last time
        let settings = settings::Settings::load();

        Self {
            // ^ create the struct with its starting values
            _stream: stream,
            sink,
            current_track: None,
            total: None,
            drag_pos: None,
            queue: Vec::new(),
            current_path: None,
            history: Vec::new(),
            show_about: false,
            // the toggles come from the settings file: initializing them
            // to fixed defaults would make the first frame compare
            // "defaults" against the loaded file and overwrite it
            show_queue: settings.show_queue,
            show_history: settings.show_history,
            shuffle: settings.shuffle,
            repeat: settings.repeat,
            show_shuffle_button: settings.show_shuffle_button,
            show_loop_button: settings.show_loop_button,
            theme: settings.theme,
            custom_bg: settings.custom_bg,
            language: settings.language,
            strings: Rc::new(strings_for(settings.language)),
            last_saved: settings,
            // None = "nothing applied yet": the first frame applies the look
            last_look: None,
        }
    }

    fn play_file(&mut self, path: &Path) {
        // ^ &mut self lets us change the struct, &Path borrows the path (no copy)
        let file = match std::fs::File::open(path) {
            // if opening the file fails, stop here, else use f
            Ok(f) => f,
            Err(_) => return,
        };
        let source = match Decoder::try_from(file) {
            // ^ Decoder reads the file header, understands the format and makes a source
            Ok(s) => s,
            Err(_) => return,
        };

        if let Some(old) = self.current_path.take() {
            self.history.push(old);
        }
        self.current_path = Some(path.to_path_buf());

        self.sink.clear(); // empty the queue so the new song can play
        self.total = source.total_duration(); // save the duration (must be before append, which moves source)
        self.sink.append(source); // add the song to the queue
        self.sink.play(); // clear() pauses the sink, so we start it again

        // save the file name to show it in the UI
        self.current_track = path.file_name().map(|n| n.to_string_lossy().to_string());
    }

    fn open_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Audio files", &["mp3", "flac", "wav", "ogg"])
            .pick_file()
        {
            self.play_file(&path);
        }
    }
    fn add_folder(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.queue.extend(collect_songs(&folder)); // add all the songs found
        }
    }
    fn add_to_queue(&mut self) {
        if let Some(files) = rfd::FileDialog::new()
            .add_filter("Audio files", &["mp3", "flac", "wav", "ogg"]) // only show these formats
            .pick_files()
        // opens the file window, gives Some(path) or None if cancelled
        {
            self.queue.extend(files);
        }
    }
    fn toggle_playback(&mut self) {
        if self.sink.is_paused() {
            self.sink.play();
        } else {
            self.sink.pause();
        }
    }
    fn remove_all_queue(&mut self) {
        self.queue.clear(); // clears the queue
    }
    fn next_track(&mut self) {
        self.sink.clear(); // stops the current song so the next can play
    }
    fn previous_track(&mut self) {
        if self.sink.get_pos() > Duration::from_secs(3) {
            let _ = self.sink.try_seek(Duration::ZERO);
            return;
        }

        if let Some(prev) = self.history.pop() {
            if let Some(cur) = self.current_path.take() {
                self.queue.insert(0, cur);
            }
            self.play_file(&prev);
        }
    }
    fn next_index(&self) -> usize {
        if self.shuffle {
            // the nanoseconds of the clock change all the time: good enough for a shuffle
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0); // if the clock is broken, use 0
            nanos as usize % self.queue.len() // % gives a number from 0 to len - 1
        } else {
            0 // no shuffle: always the first one
        }
    }
    fn play_next(&mut self) {
        // queue finished and repeat all is on: put everything back in the queue, in the original order
        if self.queue.is_empty() && self.repeat == RepeatMode::All {
            let mut all = std::mem::take(&mut self.history); // takes the whole list and leaves it empty
            all.extend(self.current_path.take()); // then adds the current song at the end
            self.queue = all;
        }
        if !self.queue.is_empty() {
            let i = self.next_index(); // your shuffle method
            let next = self.queue.remove(i);
            self.play_file(&next);
        } else {
            self.sink.clear(); // nothing left: stop
        }
    }

    /// Apply the theme preference and (for Custom) the derived palette
    /// to the context. Called only when the appearance changed (see
    /// last_look), not every frame: set_theme/set_visuals_of write into
    /// shared context state. With System, egui itself keeps following
    /// the OS theme after we set the preference.
    fn apply_appearance(&self, ctx: &egui::Context) {
        // ThemePreference is what egui switches on: Dark, Light or
        // System (follow the OS). Custom reuses System, because the
        // palette below overwrites BOTH built-in themes: whichever is
        // active, the look is the user's
        let preference = match self.theme {
            Theme::Dark => egui::ThemePreference::Dark,
            Theme::Light => egui::ThemePreference::Light,
            Theme::System | Theme::Custom => egui::ThemePreference::System,
        };
        ctx.set_theme(preference);

        match self.theme {
            Theme::Custom => {
                let custom = custom_visuals(self.custom_bg);
                ctx.set_visuals_of(egui::Theme::Dark, custom.clone());
                ctx.set_visuals_of(egui::Theme::Light, custom);
            }
            // the built-in look for both themes: a previous custom
            // theme may have overwritten them
            _ => {
                ctx.set_visuals_of(egui::Theme::Dark, egui::Visuals::dark());
                ctx.set_visuals_of(egui::Theme::Light, egui::Visuals::light());
            }
        }
    }

    /// The current settings, derived from the app state: this is what
    /// gets compared against `last_saved` and written to disk. Keeping
    /// the conversion in ONE place means a new field is wired here and
    /// in WaveR::new() only.
    fn settings_snapshot(&self) -> settings::Settings {
        settings::Settings {
            language: self.language,
            shuffle: self.shuffle,
            repeat: self.repeat,
            show_shuffle_button: self.show_shuffle_button,
            show_loop_button: self.show_loop_button,
            show_queue: self.show_queue,
            show_history: self.show_history,
            theme: self.theme,
            custom_bg: self.custom_bg,
        }
    }
}
// This is the UI part. eframe calls ui() again and again, many times per second.
// egui is "immediate mode": every frame we describe the whole UI from zero
impl eframe::App for WaveR {
    // ^ eframe::App is a trait, a contract: "to be an eframe app you must have a ui() function"
    // 'for WaveR' means "WaveR follows this contract"

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // ^ &mut self = our app data, ui = the "brush" we use to draw widgets

        let strings = self.strings.clone(); // copies only the pointer, not the whole map

        // apply the look only when it actually changed: last_look starts
        // as None, so the very first frame applies the saved theme
        let look = (self.theme, self.custom_bg);
        if self.last_look != Some(look) {
            self.apply_appearance(ui.ctx());
            self.last_look = Some(look);
        }

        if self.sink.empty() {
            if self.repeat == RepeatMode::One {
                // take() empties current_path first, so play_file doesn't push the same song into the history
                if let Some(path) = self.current_path.take() {
                    self.play_file(&path);
                }
            } else if !self.queue.is_empty()
                || (self.repeat == RepeatMode::All && self.current_path.is_some())
            {
                self.play_next();
            }
        }
        if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space)) {
            self.toggle_playback();
        }

        egui::Panel::top("menu_bar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button(tr(&strings, "Open file")).clicked() {
                        self.open_file();
                        ui.close(); // closes the menu after the click
                    }
                    if ui.button(tr(&strings, "Add to queue")).clicked() {
                        self.add_to_queue();
                        ui.close();
                    }
                    if ui.button(tr(&strings, "Add folder")).clicked() {
                        self.add_folder();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(tr(&strings, "Quit")).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button(tr(&strings, "Controls"), |ui| {
                    if ui
                        .add(egui::Button::new(tr(&strings, "Play/pause")).shortcut_text("Space"))
                        .clicked()
                    {
                        self.toggle_playback();
                        ui.close();
                    }
                    if ui.button(tr(&strings, "Previous")).clicked() {
                        self.previous_track();
                        ui.close();
                    }
                    if ui.button(tr(&strings, "Next")).clicked() {
                        self.next_track();
                        ui.close();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.shuffle, tr(&strings, "Shuffle"));
                    ui.menu_button(tr(&strings, "Loop"), |ui| {
                        ui.radio_value(&mut self.repeat, RepeatMode::Off, "Off");
                        ui.radio_value(
                            &mut self.repeat,
                            RepeatMode::One,
                            tr(&strings, "Repeat one"),
                        );
                        ui.radio_value(
                            &mut self.repeat,
                            RepeatMode::All,
                            tr(&strings, "Repeat all"),
                        );
                    });
                });
                ui.menu_button(tr(&strings, "Queue"), |ui| {
                    if ui.button(tr(&strings, "Add to queue")).clicked() {
                        self.add_to_queue();
                        ui.close();
                    }
                    if ui.button(tr(&strings, "Remove all")).clicked() {
                        self.remove_all_queue();
                        ui.close();
                    }
                });
                ui.menu_button(tr(&strings, "View"), |ui| {
                    ui.checkbox(
                        &mut self.show_shuffle_button,
                        tr(&strings, "Shuffle button"),
                    );
                    ui.checkbox(&mut self.show_loop_button, tr(&strings, "Loop button"));
                    ui.menu_button(tr(&strings, "Panels"), |ui| {
                        ui.checkbox(&mut self.show_queue, tr(&strings, "Queue"));
                        ui.checkbox(&mut self.show_history, tr(&strings, "History"));
                    });
                });
                // The Theme menu must NOT close on a click inside it:
                // egui's default close behavior is CloseOnClick (ANY click,
                // inside or outside, closes the popup), so selecting Custom
                // used to close the menu and the picker was never reachable.
                // CloseOnClickOutside = close only when clicking outside.
                // The content lives in theme_menu_content() so the headless
                // UI test can exercise the exact same widgets.
                egui::menu::MenuButton::new(tr(&strings, "Theme"))
                    .config(
                        egui::menu::MenuConfig::new()
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                    )
                    .ui(ui, |ui| {
                        theme_menu_content(ui, &mut self.theme, &mut self.custom_bg, &strings)
                    });
                ui.menu_button(tr(&strings, "Language"), |ui| {
                    if ui
                        .radio_value(&mut self.language, Language::English, "English")
                        .changed()
                    {
                        self.strings = Rc::new(strings_for(self.language));
                    }
                    if ui
                        .radio_value(&mut self.language, Language::Italian, "Italiano")
                        .changed()
                    {
                        self.strings = Rc::new(strings_for(self.language));
                    }
                });
                ui.menu_button(tr(&strings, "Help"), |ui| {
                    if ui.button(tr(&strings, "About")).clicked() {
                        self.show_about = true;
                        ui.close();
                    }
                    if ui.button(tr(&strings, "Open an issue")).clicked() {
                        let _ = webbrowser::open("https://github.com/FRANORDE/WaveR/issues/new");
                    }
                });
            });
        });

        egui::Window::new(tr(&strings, "About WaveR"))
            .open(&mut self.show_about)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.heading("WaveR");
                ui.label(tr(&strings, "Version {}").replace("{}", env!("CARGO_PKG_VERSION")));
                ui.label(tr(&strings, "A simple, cozy local music player."));
                if ui.link("https://github.com/FRANORDE/WaveR").clicked() {
                    let _ = webbrowser::open("https://github.com/FRANORDE/WaveR");
                }
            });

        if self.show_queue || self.show_history {
            egui::Panel::right("side_panels").show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                if self.show_history {
                    let mut to_play_history: Option<PathBuf> = None;
                    ui.heading(tr(&strings, "History"));

                    let h = if self.show_queue {
                        ui.available_height() / 2.0
                    } else {
                        ui.available_height()
                    };

                    egui::ScrollArea::vertical()
                        .id_salt("history_scroll")
                        .max_height(h)
                        .show(ui, |ui| {
                            for path in self.history.iter().rev() {
                                if let Some(name) = path.file_name() {
                                    let r = ui.add(
                                        egui::Label::new(name.to_string_lossy().to_string())
                                            .sense(egui::Sense::click()),
                                    );
                                    ui.separator();
                                    if r.clicked() {
                                        to_play_history = Some(path.clone());
                                    }
                                }
                            }
                        });
                    if let Some(p) = to_play_history {
                        self.play_file(&p);
                    }
                    ui.separator(); // a thin line between the two sections
                }
                if self.show_queue {
                    ui.heading(tr(&strings, "Queue"));
                    let mut to_remove: Option<usize> = None;
                    let mut to_play: Option<usize> = None;
                    egui::ScrollArea::vertical()
                        .id_salt("queue_scroll")
                        .show(ui, |ui| {
                            for (i, path) in self.queue.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    if ui.button("x").clicked() {
                                        to_remove = Some(i);
                                    }
                                    if let Some(name) = path.file_name() {
                                        let r = ui.add(
                                            egui::Label::new(name.to_string_lossy().to_string())
                                                .sense(egui::Sense::click()),
                                        );
                                        if r.clicked() {
                                            to_play = Some(i);
                                        }
                                        r.on_hover_cursor(egui::CursorIcon::PointingHand); // hand cursor on hover
                                    }
                                });
                                ui.separator();
                            }
                        });
                    if let Some(i) = to_remove {
                        self.queue.remove(i);
                    }
                    if let Some(i) = to_play {
                        let path = self.queue.remove(i); // remove() gives back the removed element
                        self.play_file(&path);
                    }
                }
            });
        }

        egui::Panel::bottom("controls").show(ui, |ui| {
            // "playing" is true only if a song is loaded and it is not paused
            let playing = !self.sink.is_paused() && !self.sink.empty();
            ui.vertical_centered(|ui| {
                ui.add(
                    egui::Label::new(
                        self.current_track
                            .as_deref()
                            .unwrap_or(tr(&strings, "Nothing playing")),
                    )
                    .truncate(),
                );
            });

            ui.horizontal(|ui| {
                ui.set_min_height(40.0);

                // --- 1) measure the row (before drawing anything) ---
                let spacing = ui.spacing().item_spacing.x;
                let mut buttons_width = 28.0 + 40.0 + 28.0; // `mut` because we add to it below
                let mut count = 3;

                if self.show_shuffle_button {
                    buttons_width += 28.0;
                    count += 1;
                }
                if self.show_loop_button {
                    buttons_width += 28.0;
                    count += 1;
                }

                // total = all the buttons + the gaps between them (one gap less than buttons)
                let total = buttons_width + (count - 1) as f32 * spacing;
                // --- 2) center the row ---
                ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));

                if self.show_shuffle_button && shuffle_button(ui, self.shuffle).clicked() {
                    self.shuffle = !self.shuffle; // flip true/false
                }
                if skip_button(ui, false).clicked() {
                    self.previous_track();
                }
                if round_button(ui, playing).clicked() {
                    self.toggle_playback();
                }
                if skip_button(ui, true).clicked() {
                    self.next_track();
                }
                if self.show_loop_button && loop_button(ui, self.repeat).clicked() {
                    // cycle Off -> All -> One -> Off
                    self.repeat = match self.repeat {
                        RepeatMode::Off => RepeatMode::All,
                        RepeatMode::All => RepeatMode::One,
                        RepeatMode::One => RepeatMode::Off,
                    };
                }
            });

            if let Some(total) = self.total {
                // ^ only show the bar if the song duration is known
                let total_secs = total.as_secs_f32(); // Duration -> seconds as a decimal number

                // while dragging show the dragged value, else the real song position
                let mut position = self.drag_pos.unwrap_or(self.sink.get_pos().as_secs_f32());

                ui.horizontal(|ui| {
                    // ^ the three widgets (current time, bar, total time) go side by side
                    ui.spacing_mut().slider_width = (ui.available_width() - 100.0).max(50.0); // width of the duration line

                    ui.label(format_time(position));

                    let response = ui
                        .add(egui::Slider::new(&mut position, 0.0..=total_secs).show_value(false));

                    if response.dragged() {
                        // the user is holding the bar: only remember the value, don't jump yet
                        self.drag_pos = Some(position);
                    } else if response.drag_stopped() {
                        // the user just released the bar: now jump to that point
                        let _ = self.sink.try_seek(Duration::from_secs_f32(position));
                        self.drag_pos = None; // back to following the real position
                    } else if response.changed() {
                        // a simple click on the bar (no drag): jump right away
                        let _ = self.sink.try_seek(Duration::from_secs_f32(position));
                    }

                    ui.label(format_time(total_secs)); // show the total time on the right of the bar
                });
            }
            // with_layout lets you choose the direction widgets are placed in
            // right_to_left starts from the right edge, so the first widget ends up at the far right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut volume = self.sink.volume(); // read the current volume (0.0 to 1.0)
                if ui
                    .add(
                        egui::Slider::new(&mut volume, 0.0..=1.0)
                            .show_value(false)
                            .text("Volume"),
                    )
                    .changed()
                {
                    // only when the user moves the slider, apply the new volume
                    self.sink.set_volume(volume);
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            // ^ a panel that fills the whole window. The closure |ui| is the content inside it

            // egui only redraws when something happens, so we ask for a redraw every 200 ms
            // this keeps the progress bar moving by itself
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        });

        // ONE save point for the whole frame: compare the current
        // settings against the last ones written to disk, and persist
        // only when something actually changed. No per-widget save code:
        // a forgotten widget would be a bug, this cannot be forgotten
        let snapshot = self.settings_snapshot();
        if snapshot != self.last_saved {
            snapshot.save();
            self.last_saved = snapshot;
        }
    }
}

// --- theme menu content (shared with the headless UI test) ---

/// Rects and flags of the Theme menu rows, captured so the headless
/// UI test (src/ui.rs) can verify what the user actually sees.
/// The app discards the returned value; the fields are read only by
/// the test, which is why dead_code is silenced here.
// (the allow covers the struct fields AND the EMPTY const below: both
// are read only by the headless UI test, not by the app itself)
#[allow(dead_code)]
struct ThemeMenuLayout {
    /// the Custom radio row (present in every theme)
    custom_radio: egui::Rect,
    /// true when the inline color picker was drawn (Custom theme)
    picker_shown: bool,
}

#[allow(dead_code)] // EMPTY is used by the test only
impl ThemeMenuLayout {
    /// "nothing captured yet": neutral values meaning "not laid out"
    const EMPTY: Self = Self {
        custom_radio: egui::Rect::NOTHING,
        picker_shown: false,
    };
}

/// Content of the Theme menu: the four radios and, for Custom, the
/// inline background color picker. Split out of `update` so the
/// headless UI test exercises THE SAME widget code as the app, not a
/// copy.
fn theme_menu_content(
    ui: &mut egui::Ui,
    theme: &mut Theme,
    custom_bg: &mut egui::Color32,
    strings: &HashMap<&'static str, &'static str>,
) -> ThemeMenuLayout {
    ui.radio_value(theme, Theme::System, tr(strings, "Follow system"));
    ui.radio_value(theme, Theme::Dark, tr(strings, "Dark"));
    ui.radio_value(theme, Theme::Light, tr(strings, "Light"));
    let custom_radio = ui.radio_value(theme, Theme::Custom, tr(strings, "Custom")).rect;

    let mut picker_shown = false;
    if *theme == Theme::Custom {
        ui.separator();
        ui.label(tr(strings, "Background"));
        // INLINE color picker, NOT the swatch+popup version. egui keeps
        // ONE active popup per viewport (Memory.popups is a single
        // slot): opening the picker popup evicts the menu popup, and
        // since the picker is built INSIDE the menu closure, both die
        // within a frame (reproduced headless in src/ui.rs). The
        // inline version has no second popup: the menu stays open and
        // the app preview updates live while dragging.
        ui.spacing_mut().slider_width = 200.0; // room for the preview bar
        egui::color_picker::color_picker_color32(
            ui,
            custom_bg,
            egui::color_picker::Alpha::Opaque, // a background is not transparent
        );
        picker_shown = true;
    }

    ThemeMenuLayout { custom_radio, picker_shown }
}

// --- custom theme: a whole palette derived from ONE picked color ---

/// Build the visuals for the custom theme: the background stays as the
/// user picked it, buttons become lighter versions of it, and text
/// takes the opposite brightness (light text on dark backgrounds and
/// the reverse). Starts from the built-in theme closest to the chosen
/// color, then overrides the colors that define the look.
fn custom_visuals(bg: egui::Color32) -> egui::Visuals {
    let is_dark = luminance(bg) < 0.5;

    let mut v = if is_dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    // the three button states, progressively lighter versions of the bg
    let button = lighten(bg, 0.16);
    let button_hover = lighten(bg, 0.28);
    let button_active = lighten(bg, 0.40);
    let text = if is_dark {
        egui::Color32::from_rgb(235, 235, 240) // near-white
    } else {
        egui::Color32::from_rgb(28, 28, 32) // near-black
    };
    // recessed areas (slider tracks, text fields) go the other way
    let extreme = if is_dark {
        darken(bg, 0.25)
    } else {
        lighten(bg, 0.25)
    };

    v.dark_mode = is_dark;
    v.panel_fill = bg; // main panels
    v.window_fill = bg; // windows and popups (menus, the picker itself)
    v.faint_bg_color = bg; // faint stripes and alternating rows
    v.extreme_bg_color = extreme;
    v.override_text_color = Some(text);
    v.hyperlink_color = button_active;
    v.selection.bg_fill = button_active; // selected-text highlight
    v.selection.stroke.color = text;

    // every widget family: the fills become the derived button colors,
    // and the foreground (labels/icons ON the widgets) the text color
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.fg_stroke.color = text;
    }
    v.widgets.noninteractive.bg_fill = bg;
    v.widgets.noninteractive.weak_bg_fill = bg;
    v.widgets.inactive.bg_fill = button;
    v.widgets.inactive.weak_bg_fill = button;
    v.widgets.hovered.bg_fill = button_hover;
    v.widgets.hovered.weak_bg_fill = button_hover;
    v.widgets.active.bg_fill = button_active;
    v.widgets.active.weak_bg_fill = button_active;
    v.widgets.open.bg_fill = button_hover;
    v.widgets.open.weak_bg_fill = button_hover;

    v
}

/// Mix the color toward white by a fraction t: 0.0 = unchanged, 1.0 = white
fn lighten(c: egui::Color32, t: f32) -> egui::Color32 {
    let mix = |x: u8| (x as f32 + (255.0 - x as f32) * t).round() as u8;
    egui::Color32::from_rgb(mix(c.r()), mix(c.g()), mix(c.b()))
}

/// Mix the color toward black by a fraction t: 0.0 = unchanged, 1.0 = black
fn darken(c: egui::Color32, t: f32) -> egui::Color32 {
    let mix = |x: u8| (x as f32 * (1.0 - t)).round() as u8;
    egui::Color32::from_rgb(mix(c.r()), mix(c.g()), mix(c.b()))
}

/// Perceived brightness from 0.0 (black) to 1.0 (white): the classic
/// eye-weighted average, where green counts more than blue
fn luminance(c: egui::Color32) -> f32 {
    (0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32) / 255.0
}

/// The color this theme draws text and icons with: custom themes set it
/// directly, built-in themes don't override it so read the widget default
fn text_color(ui: &egui::Ui) -> egui::Color32 {
    ui.visuals()
        .override_text_color
        .unwrap_or(ui.visuals().widgets.noninteractive.fg_stroke.color)
}

fn round_button(ui: &mut egui::Ui, playing: bool) -> egui::Response {
    // ^ returns a Response, so the caller can use .clicked() just like with ui.button()
    let size = egui::vec2(40.0, 40.0); // width and height of the button

    // reserve a square area; Sense::click() tells egui this area reacts to clicks
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    // --- BUTTON COLOR: same as the panel background, but a bit darker ---
    let bg_color = ui.visuals().panel_fill; // background color of the panels

    // scale each color channel (red, green, blue) by 0.75 to make it darker
    // .r() gives a u8 (0-255): we convert to f32 to multiply, then back to u8
    let r = (bg_color.r() as f32 * 0.75) as u8;
    let g = (bg_color.g() as f32 * 0.75) as u8;
    let b = (bg_color.b() as f32 * 0.75) as u8;
    let color = egui::Color32::from_rgb(r, g, b); // a new, fully opaque color

    // draw the circle: center = middle of the rect, radius = half the width
    ui.painter()
        .circle_filled(rect.center(), rect.width() / 2.0, color);

    let c = rect.center(); // the middle of the button, used as origin for the icons

    // --- ANIMATION ---
    // t moves smoothly from 0.0 to 1.0 (or back) when `playing` changes
    // 0.0 = paused (show the play triangle), 1.0 = playing (show the pause bars)
    // the last argument is the duration in seconds
    let t = ui.ctx().animate_bool_with_time(response.id, playing, 0.15);

    // gamma_multiply(x) changes the opacity: 1.0 = fully visible, 0.0 = invisible
    // the icon follows the theme's text color: visible on any background
    let icon = text_color(ui);
    let play_color = icon.gamma_multiply(1.0 - t); // fades OUT when t grows
    let pause_color = icon.gamma_multiply(t); // fades IN when t grows

    // --- PAUSE ICON: two vertical bars, one left and one right of the center ---
    ui.painter().rect_filled(
        egui::Rect::from_center_size(c + egui::vec2(-5.0, 0.0), egui::vec2(4.0, 14.0)),
        1.0, // corner radius
        pause_color,
    );
    ui.painter().rect_filled(
        egui::Rect::from_center_size(c + egui::vec2(5.0, 0.0), egui::vec2(4.0, 14.0)),
        1.0, // corner radius
        pause_color,
    );

    // --- PLAY ICON: a triangle pointing right, defined by its 3 corners ---
    let points = vec![
        c + egui::vec2(-4.0, -8.0), // top left
        c + egui::vec2(-4.0, 8.0),  // bottom left
        c + egui::vec2(8.0, 0.0),   // right tip
    ];
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        play_color,
        egui::Stroke::NONE, // no outline
    ));

    response.on_hover_cursor(egui::CursorIcon::PointingHand) // hand cursor on hover
}

fn skip_button(ui: &mut egui::Ui, forward: bool) -> egui::Response {
    let size = egui::vec2(28.0, 28.0); // smaller than the play button (40x40)
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let c = rect.center();

    // idle/hover colors follow the theme's text color: readable on any
    // custom background (0.7 opacity looks like the old gray 170)
    let icon = text_color(ui);
    let color = if response.hovered() {
        icon
    } else {
        icon.gamma_multiply(0.7)
    };

    let dir = if forward { 1.0 } else { -1.0 };

    let points = vec![
        c + egui::vec2(-7.0 * dir, -7.0), // top
        c + egui::vec2(-7.0 * dir, 7.0),  // bottom
        c + egui::vec2(4.0 * dir, 0.0),   // tip
    ];
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));

    ui.painter().rect_filled(
        egui::Rect::from_center_size(c + egui::vec2(7.0 * dir, 0.0), egui::vec2(3.0, 14.0)),
        1.0, // corner radius
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// draws a small triangle pointing right, with its tip at `tip`
fn arrow_head(painter: &egui::Painter, tip: egui::Pos2, color: egui::Color32) {
    let points = vec![
        tip,                          // the tip
        tip + egui::vec2(-5.0, -3.5), // back, upper corner
        tip + egui::vec2(-5.0, 3.5),  // back, lower corner
    ];
    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
}

fn shuffle_button(ui: &mut egui::Ui, active: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
    let c = rect.center();

    // blue stays the "active" accent; idle/hover follow the theme's text
    // color so the icons stay readable on any custom background
    let icon = text_color(ui);
    let color = if active {
        egui::Color32::from_rgb(90, 160, 255)
    } else if response.hovered() {
        icon
    } else {
        icon.gamma_multiply(0.7)
    };

    let stroke = egui::Stroke::new(2.0, color);
    let painter = ui.painter();

    // two lines that cross each other
    painter.line_segment(
        [c + egui::vec2(-9.0, -6.0), c + egui::vec2(6.0, 6.0)],
        stroke,
    );
    painter.line_segment(
        [c + egui::vec2(-9.0, 6.0), c + egui::vec2(6.0, -6.0)],
        stroke,
    );

    // an arrowhead at the right end of each line
    arrow_head(painter, c + egui::vec2(11.0, 6.0), color);
    arrow_head(painter, c + egui::vec2(11.0, -6.0), color);

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn loop_button(ui: &mut egui::Ui, mode: RepeatMode) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
    let c = rect.center();

    // the loop is "on" for both All and One
    let active = mode != RepeatMode::Off;

    // same rule as the shuffle button: blue accent when on, theme text
    // color otherwise
    let icon = text_color(ui);
    let color = if active {
        egui::Color32::from_rgb(90, 160, 255)
    } else if response.hovered() {
        icon
    } else {
        icon.gamma_multiply(0.7)
    };

    let painter = ui.painter();

    // open path, clockwise, with a gap at the top right
    // the cut corners look rounded at this size
    let path = vec![
        c + egui::vec2(8.0, -1.0), // start on the right side, below the gap
        c + egui::vec2(8.0, 2.0),
        c + egui::vec2(5.0, 5.0),
        c + egui::vec2(-5.0, 5.0),
        c + egui::vec2(-8.0, 2.0),
        c + egui::vec2(-8.0, -2.0),
        c + egui::vec2(-5.0, -5.0),
        c + egui::vec2(2.0, -5.0), // end of the top line
    ];
    painter.add(egui::Shape::line(path, egui::Stroke::new(2.0, color)));

    // arrowhead at the end of the top line, pointing into the gap
    arrow_head(painter, c + egui::vec2(7.0, -5.0), color);

    // a small "1" in the middle only for "repeat one"
    if mode == RepeatMode::One {
        painter.text(
            c,
            egui::Align2::CENTER_CENTER,
            "1",
            egui::FontId::proportional(9.0),
            color,
        );
    }

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn load_icon() -> egui::IconData {
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/WaveR.png"))
        .expect("invalid icon"); // crashes with this message if the PNG is broken
    icon
}

fn format_time(secs: f32) -> String {
    // ^ turns seconds into a text like 3:05
    let total = secs as u32; // remove the decimals
    format!("{}:{:02}", total / 60, total % 60) // minutes : seconds (always 2 digits)
}

fn collect_songs(folder: &Path) -> Vec<PathBuf> {
    let mut songs = Vec::new(); // the list we will return

    // read_dir gives a Result: if the folder can't be read, return the empty list
    let entries = match std::fs::read_dir(folder) {
        Ok(e) => e,
        Err(_) => return songs,
    };

    for entry in entries {
        // each entry is a Result too (the file could be unreadable)
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // skip this one and go to the next
        };
        let path = entry.path(); // the full path of the file

        // check the file extension: only keep audio files
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext = ext.to_lowercase(); // so ".MP3" works too
            if ["mp3", "flac", "wav", "ogg"].contains(&ext.as_str()) {
                songs.push(path); // add the song to the list we return
            }
        }
    } // <- this closes the for loop

    songs.sort(); // read_dir gives no order, so we sort the songs
    songs // no ';' here: this is the value the function returns
}
