// Comments for me to understand code (im a noob to Rust), but also for you to understand what is going on
// imports bring code from other crates, so we can write less code
// so, this ones below help you write less code, and make it more readable
use eframe::egui;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn main() -> eframe::Result {
    // ^ main returns a Result, which can be Ok or Err
    eframe::run_native(
        // ^ this opens the window and keeps the program running until it closes
        "WaveR",                          // window title
        eframe::NativeOptions::default(), // "default" means standard window options
        Box::new(|_cc| Ok(Box::new(WaveR::new()))), // a closure called once at start
                                          // WaveR::new() creates the actual app
    )
}

struct WaveR {
    // ^ a struct is a collection of data, like a class in other languages
    _stream: OutputStream, // connection to the audio device, must stay alive or the sound stops
    sink: Sink,            // the song queue, it has the play and pause commands
    current_track: Option<String>, // "either there is a string, or there is nothing" (like null)
    total: Option<Duration>, // total duration of the song, None if unknown
    drag_pos: Option<f32>, // position chosen while dragging the bar (None = not dragging)
    queue: Vec<PathBuf>,   // position to where the song is saved
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

        self.sink.clear(); // empty the queue so the new song can play
        self.total = source.total_duration(); // save the duration (must be before append, which moves source)
        self.sink.append(source); // add the song to the queue
        self.sink.play(); // clear() pauses the sink, so we start it again

        // save the file name to show it in the UI
        self.current_track = path.file_name().map(|n| n.to_string_lossy().to_string());
    }
}

// This is the UI part. eframe calls ui() again and again, many times per second.
// egui is "immediate mode": every frame we describe the whole UI from zero
impl eframe::App for WaveR {
    // ^ eframe::App is a trait, a contract: "to be an eframe app you must have a ui() function"
    // 'for WaveR' means "WaveR follows this contract"
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // ^ &mut self = our app data, ui = the "brush" we use to draw widgets
        if self.sink.empty() && !self.queue.is_empty() {
            let next = self.queue.remove(0); // take the first song out of the queue
            self.play_file(&next); // play it
        }

        egui::Panel::right("queue_panel").show(ui, |ui| {
            ui.heading("Queue");
            let mut to_remove: Option<usize> = None;
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, path) in self.queue.iter().enumerate() {
                    ui.horizontal(|ui| {
                        if ui.button("x").clicked() {
                            to_remove = Some(i);
                        }
                        if let Some(name) = path.file_name() {
                            ui.label(name.to_string_lossy());
                        }
                    });
                }
            });
            if let Some(i) = to_remove {
                self.queue.remove(i);
            }
        });

        egui::CentralPanel::default().show(ui, |ui| {
            // ^ a panel that fills the whole window. The closure |ui| is the content inside it
            ui.heading("WaveR"); // big title text

            if ui.button("Open file").clicked() {
                // ^ draws the button, clicked() is true only in the frame where it is pressed
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Audio files", &["mp3", "flac", "wav", "ogg"]) // only show these formats
                    .pick_file()
                // opens the file window, gives Some(path) or None if cancelled
                {
                    self.play_file(&path);
                }
            }

            if ui.button("Add to queue").clicked() {
                // ^ draws the button, clicked() is true only in the frame where it is pressed
                if let Some(files) = rfd::FileDialog::new()
                    .add_filter("Audio files", &["mp3", "flac", "wav", "ogg"]) // only show these formats
                    .pick_files()
                // opens the file window, gives Some(path) or None if cancelled
                {
                    self.queue.extend(files);
                }
            }
            if ui.button("Add folder").clicked() {
                // pick_folder() gives Some(path) if the user chooses a folder, None if cancelled
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    self.queue.extend(collect_songs(&folder)); // add all the songs found
                }
            }

            ui.label(format!("Queue: {} songs", self.queue.len()));

            if let Some(total) = self.total {
                // ^ only show the bar if the song duration is known
                let total_secs = total.as_secs_f32(); // Duration -> seconds as a decimal number

                // while dragging show the dragged value, else the real song position
                let mut position = self.drag_pos.unwrap_or(self.sink.get_pos().as_secs_f32());

                let response =
                    ui.add(egui::Slider::new(&mut position, 0.0..=total_secs).show_value(false));

                if response.dragged() {
                    // ^ the user is holding the bar: only remember the value, don't jump yet
                    self.drag_pos = Some(position);
                } else if response.drag_stopped() {
                    // ^ the user just released the bar: now jump to that point
                    let _ = self.sink.try_seek(Duration::from_secs_f32(position));
                    self.drag_pos = None; // back to following the real position
                } else if response.changed() {
                    // ^ a simple click on the bar (no drag): jump right away
                    let _ = self.sink.try_seek(Duration::from_secs_f32(position));
                }

                // show the time like 1:23 / 3:45
                ui.label(format!(
                    "{} / {}",
                    format_time(position),
                    format_time(total_secs)
                ));
            }

            // show the song name, or "Nothing playing" if there is None
            ui.label(self.current_track.as_deref().unwrap_or("Nothing playing"));

            ui.horizontal(|ui| {
                // ^ horizontal puts the widgets side by side instead of one under the other
                if ui.button("Pause").clicked() {
                    self.sink.pause();
                }
                if ui.button("Play").clicked() {
                    self.sink.play();
                }
            });

            let mut volume = self.sink.volume(); // read the current volume (0.0 to 1.0)
            if ui
                .add(egui::Slider::new(&mut volume, 0.0..=1.0).text("Volume"))
                .changed()
            {
                // ^ only when the user moves the slider, apply the new volume
                self.sink.set_volume(volume);
            }

            // egui only redraws when something happens, so we ask for a redraw every 200 ms
            // this keeps the progress bar moving by itself
            ui.ctx().request_repaint_after(Duration::from_millis(200));
        });
    }
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
