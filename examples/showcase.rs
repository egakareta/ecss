use std::error::Error;

use trcss::{SelectorExt as _, Stylesheet};

fn main() -> Result<(), Box<dyn Error>> {
    let stylesheet = Stylesheet::parse(include_str!("theme.css"))?;

    eframe::run_native(
        "trcss",
        eframe::NativeOptions::default(),
        Box::new(move |creation_context| {
            stylesheet.install(&creation_context.egui_ctx);
            Ok(Box::new(Showcase::default()))
        }),
    )?;

    Ok(())
}

#[derive(Default)]
struct Showcase {
    saved: bool,
}

impl eframe::App for Showcase {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("CSS-styled egui widgets");
        ui.label("Hover or press the buttons to see their CSS states.");

        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new("Save changes").selector(".primary"))
                .clicked()
            {
                self.saved = true;
            }

            ui.add(egui::Button::new("Cancel").selector(".secondary"));
        });

        if self.saved {
            ui.label("Changes saved.");
        }
    }
}
