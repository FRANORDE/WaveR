// Headless UI test for the Theme menu: simulated clicks, no window,
// same widget code as the app (theme_menu_content in main.rs).
// How-to, click simulation and the popup root-cause history:
// agent/settings-design.md.

use std::collections::HashMap;
use std::sync::{Mutex, Once};

use egui::{Color32, Event, Modifiers, PointerButton, Rect};

use crate::{Theme, theme_menu_content};

// egui and eframe report problems through the `log` crate, and without
// a logger installed those messages silently vanish. This tiny logger
// captures them, so the test can assert that the whole simulated flow
// produces no warnings (the real app printed two of them in console).
static CAPTURED: Mutex<Vec<String>> = Mutex::new(Vec::new());
static INSTALL: Once = Once::new();

struct CapturingLogger;

impl log::Log for CapturingLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        // capture warnings and errors, ignore the noisy info/debug
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            CAPTURED
                .lock()
                .unwrap()
                .push(format!("{}: {}", record.level(), record.args()));
        }
    }

    fn flush(&self) {}
}

fn install_logger() {
    INSTALL.call_once(|| {
        let _ = log::set_boxed_logger(Box::new(CapturingLogger));
        log::set_max_level(log::LevelFilter::Warn);
    });
}

/// Take everything captured so far, leaving the buffer empty.
fn drain_warnings() -> Vec<String> {
    std::mem::take(&mut CAPTURED.lock().unwrap())
}

/// One headless frame: builds the menu bar exactly like the app does,
/// feeds the given events, and captures where the button and the menu
/// rows ended up on screen.
fn show(
    ctx: &egui::Context,
    events: Vec<Event>,
    theme: &mut Theme,
    custom_bg: &mut Color32,
    strings: &HashMap<&'static str, &'static str>,
    button_rect: &mut Option<Rect>,
    layout: &mut crate::ThemeMenuLayout,
) {
    let input = egui::RawInput {
        // a virtual 600x400 window: popup positions depend on this
        screen_rect: Some(Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(600.0, 400.0))),
        events,
        ..Default::default()
    };
    // the output must not be dropped raw: egui panics if the
    // per-frame texture deltas are never applied (the headless test
    // is not a renderer, so we tell egui to forget them on purpose,
    // same as egui's own integration tests do)
    let output = ctx.run_ui(input, |ui| {
        // reset first: if the menu is closed this frame its content is
        // not built at all, and last frame's rects must not leak into
        // the assertions (stale data would lie)
        *layout = crate::ThemeMenuLayout::EMPTY;
        egui::Panel::top("menu_bar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                let (response, content) = egui::menu::MenuButton::new("Theme")
                    .config(
                        egui::menu::MenuConfig::new()
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                    )
                    .ui(ui, |ui| theme_menu_content(ui, theme, custom_bg, strings));
                *button_rect = Some(response.rect);
                if let Some(inner) = content {
                    *layout = inner.inner;
                }
            });
        });
    });
    output.drop_without_applying_deltas();
}

/// A full click on a widget: move the pointer there, press in one
/// frame, release in the next (egui counts a click when press and
/// release land on the same widget).
fn click_frames(button: Rect) -> [Vec<Event>; 2] {
    let center = button.center();
    [
        vec![
            Event::PointerMoved(center),
            Event::PointerButton {
                pos: center,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        vec![Event::PointerButton {
            pos: center,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    ]
}

#[test]
fn clicking_custom_keeps_menu_open_and_shows_picker() {
    install_logger();
    let ctx = egui::Context::default();
    let mut theme = Theme::System;
    let mut custom_bg = Color32::from_rgb(0x20, 0x22, 0x2B);
    let strings: HashMap<&'static str, &'static str> = HashMap::new(); // English keys
    let mut button_rect = None;
    let mut layout = crate::ThemeMenuLayout::EMPTY;

    // 1. idle frame: the menu bar draws itself, we learn where the
    //    button is (nothing is hard-coded)
    show(
        &ctx,
        vec![],
        &mut theme,
        &mut custom_bg,
        &strings,
        &mut button_rect,
        &mut layout,
    );
    let theme_button = button_rect.expect("Theme button must be laid out");

    // 2. click the button: the menu popup opens
    let [press, release] = click_frames(theme_button);
    show(
        &ctx,
        press,
        &mut theme,
        &mut custom_bg,
        &strings,
        &mut button_rect,
        &mut layout,
    );
    show(
        &ctx,
        release,
        &mut theme,
        &mut custom_bg,
        &strings,
        &mut button_rect,
        &mut layout,
    );

    // menu is open now: the Custom radio is laid out, but the picker
    // row is not there yet (theme is still System)
    show(
        &ctx,
        vec![],
        &mut theme,
        &mut custom_bg,
        &strings,
        &mut button_rect,
        &mut layout,
    );
    assert_eq!(theme, Theme::System);
    assert_ne!(
        layout.custom_radio,
        Rect::NOTHING,
        "the Custom radio must be visible while the menu is open"
    );
    assert!(
        !layout.picker_shown,
        "the color picker must be hidden while the theme is not Custom"
    );

    // 3. click "Custom": THIS is the moment that used to close the
    //    menu before the user could reach the picker
    let custom_radio = layout.custom_radio;
    let [press, release] = click_frames(custom_radio);
    show(
        &ctx,
        press,
        &mut theme,
        &mut custom_bg,
        &strings,
        &mut button_rect,
        &mut layout,
    );
    show(
        &ctx,
        release,
        &mut theme,
        &mut custom_bg,
        &strings,
        &mut button_rect,
        &mut layout,
    );

    // 4. the menu must still be open and the picker visible
    show(
        &ctx,
        vec![],
        &mut theme,
        &mut custom_bg,
        &strings,
        &mut button_rect,
        &mut layout,
    );
    assert_eq!(theme, Theme::Custom, "the radio click must select Custom");
    assert_ne!(
        layout.custom_radio,
        Rect::NOTHING,
        "the menu must stay open after clicking Custom (CloseOnClickOutside)"
    );
    assert!(
        layout.picker_shown,
        "the inline color picker must appear once the theme is Custom"
    );

    // 5. an idle frame LATER (this is where the old swatch+popup picker
    //    used to die: opening its popup evicted the menu, and the
    //    picker, built inside the menu closure, disappeared the frame
    //    after). The inline picker must still be there, and the color
    //    must not have drifted just from being displayed
    show(
        &ctx,
        vec![],
        &mut theme,
        &mut custom_bg,
        &strings,
        &mut button_rect,
        &mut layout,
    );
    assert_ne!(
        layout.custom_radio,
        Rect::NOTHING,
        "the menu must stay open while the user looks at the picker"
    );
    assert!(
        layout.picker_shown,
        "the inline color picker must keep being drawn frame after frame"
    );
    assert_eq!(
        custom_bg,
        Color32::from_rgb(0x20, 0x22, 0x2B),
        "just displaying the picker must not alter the color"
    );

    // 6. egui must not complain about anything we did here: whatever it
    //    logs at warn level or above is a real problem for the user
    //    (the real app printed two warnings doing exactly this flow)
    let mut captured = drain_warnings();
    let total = captured.len();
    captured.sort();
    captured.dedup();
    assert!(
        captured.is_empty(),
        "egui logged {total} warning(s) while simulating the Theme menu: {captured:#?}"
    );
}
