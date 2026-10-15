use std::error::Error;

use ecss::{SelectorExt as _, Stylesheet};

fn main() -> Result<(), Box<dyn Error>> {
    let stylesheet = Stylesheet::parse(include_str!("theme.css"))?;

    eframe::run_native(
        "ecss",
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
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("CSS-styled egui widgets").selector(".title"));
                ui.add(
                    egui::Label::new("Hover or press the buttons to see their CSS states.")
                        .selector(".description"),
                );
                ui.add(
                    (|ui: &mut egui::Ui| {
                        if ui
                            .add(egui::Button::new("Save changes").selector(".primary"))
                            .clicked()
                        {
                            self.saved = true;
                        }
                        ui.add(egui::Button::new("Cancel").selector(".secondary"));
                        ui.response()
                    })
                    .selector(".actions"),
                );
                if self.saved {
                    ui.add(egui::Label::new("Changes saved.").selector(".status"));
                }
                ui.response()
            })
            .selector(".page"),
        );
    }
}
