// Comments for me to understand code (im a noob to Rust), but also for you to understand what is going on
// This syntax down below is an attribute and it talks to the entire file
// it basically means: Only on releases versions, tell windows to not open a terminal window
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// imports bring code from other crates, so we can write less code
// so, this ones below help you write less code, and make it more readable
use eframe::{NativeOptions, egui};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::path::{Path, PathBuf};
use std::time::Duration;

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

#[derive(Clone, Copy, PartialEq)]
enum RepeatMode {
    Off,
    One,
    All,
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
}

impl WaveR {
    // ^ an impl block defines the functions (methods) of the struct
    fn new() -> Self {
        // ^ Self is a shortcut for WaveR
        let stream =
            OutputStreamBuilder::open_default_stream().expect("no audio output device found"); // open the default audio device
        // ^ .expect() crashes with this message if there is no audio device
        let sink = Sink::connect_new(stream.mixer()); // connect the queue to the audio mixer

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
            show_queue: true,
            show_history: false,
            shuffle: false,
            repeat: RepeatMode::Off,
            show_shuffle_button: true,
            show_loop_button: true,
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
}

// This is the UI part. eframe calls ui() again and again, many times per second.
// egui is "immediate mode": every frame we describe the whole UI from zero
impl eframe::App for WaveR {
    // ^ eframe::App is a trait, a contract: "to be an eframe app you must have a ui() function"
    // 'for WaveR' means "WaveR follows this contract"

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // ^ &mut self = our app data, ui = the "brush" we use to draw widgets

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
                    if ui.button("Open file").clicked() {
                        self.open_file();
                        ui.close(); // closes the menu after the click
                    }
                    if ui.button("Add to queue").clicked() {
                        self.add_to_queue();
                        ui.close();
                    }
                    if ui.button("Add folder").clicked() {
                        self.add_folder();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Controls", |ui| {
                    if ui
                        .add(egui::Button::new("Play/pause").shortcut_text("Space"))
                        .clicked()
                    {
                        self.toggle_playback();
                        ui.close();
                    }
                    if ui.button("Previous").clicked() {
                        self.previous_track();
                        ui.close();
                    }
                    if ui.button("Next").clicked() {
                        self.next_track();
                        ui.close();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.shuffle, "Shuffle");
                    ui.menu_button("Repeat", |ui| {
                        ui.radio_value(&mut self.repeat, RepeatMode::Off, "Off");
                        ui.radio_value(&mut self.repeat, RepeatMode::One, "Repeat one");
                        ui.radio_value(&mut self.repeat, RepeatMode::All, "Repeat all");
                    });
                });
                ui.menu_button("Queue", |ui| {
                    if ui.button("Add to queue").clicked() {
                        self.add_to_queue();
                        ui.close();
                    }
                    if ui.button("Remove all").clicked() {
                        self.remove_all_queue();
                        ui.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_shuffle_button, "Shuffle button");
                    ui.checkbox(&mut self.show_loop_button, "Loop button");
                    ui.menu_button("Panels", |ui| {
                        ui.checkbox(&mut self.show_queue, "Queue");
                        ui.checkbox(&mut self.show_history, "History");
                    });
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        self.show_about = true;
                        ui.close();
                    }
                    if ui.button("Open an issue").clicked() {
                        let _ = webbrowser::open("https://github.com/FRANORDE/WaveR/issues/new");
                    }
                });
            });
        });

        egui::Window::new("About WaveR")
            .open(&mut self.show_about)
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.heading("WaveR");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.label("A simple, cozy local music player.");
                if ui.link("https://github.com/FRANORDE/WaveR").clicked() {
                    let _ = webbrowser::open("https://github.com/FRANORDE/WaveR");
                }
            });

        if self.show_queue || self.show_history {
            egui::Panel::right("side_panels").show(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                if self.show_history {
                    let mut to_play_history: Option<PathBuf> = None;
                    ui.heading("History");

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
                    ui.heading("Queue");
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
                    egui::Label::new(self.current_track.as_deref().unwrap_or("Nothing playing"))
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
    }
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
    let play_color = egui::Color32::WHITE.gamma_multiply(1.0 - t); // fades OUT when t grows
    let pause_color = egui::Color32::WHITE.gamma_multiply(t); // fades IN when t grows

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

    let color = if response.hovered() {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_gray(170)
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

    // blue when active, white on hover, gray otherwise
    let color = if active {
        egui::Color32::from_rgb(90, 160, 255)
    } else if response.hovered() {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_gray(170)
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

    let color = if active {
        egui::Color32::from_rgb(90, 160, 255)
    } else if response.hovered() {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_gray(170)
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
