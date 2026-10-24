// Comments for me to understand code (im a noob to Rust), but also for you to understand what is going on
// imports, imports are used to bring code from other files into your file
// so, this ones help you write less code, and make it more readable
use eframe::egui;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use std::path::Path;

fn main() -> eframe::Result {
    // ^ this tells main to return a Result type, which is a type that can be either Ok or Err
    eframe::run_native(
        // ^ this opens a window
        "WaveR",                          // window title
        eframe::NativeOptions::default(), // "default" tells eframe to use default options
        Box::new(|_cc| Ok(Box::new(WaveR::new()))), // Box::new(|_cc|...) is a closure, a function that gets called one single time at start,
                                                    // Ok(Box::new(WaveR::new())) creates the actual app
    )
}

struct WaveR {
    // ^ this is a struct, a struct is a collection of data, like a class in other languages
    _stream: OutputStream, // this is the connection to the audio device. _ is used to tell the compiler that this variable is not used
    sink: Sink,            // this is the song queue, it has the play and pause commands
    current_track: Option<String>, // this says "or there is a string, or there is nothing", functions like null in other languages
}
