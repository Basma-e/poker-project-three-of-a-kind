//! Desktop GUI client for Poker Game.

mod gui;
mod network;

fn main() {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([1100.0, 760.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Three-of-a-Kind Casino",
        options,
        Box::new(|cc| Ok(Box::new(gui::PokerUiApp::new(cc)))),
    );
}
