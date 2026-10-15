use ecss::{SelectorExt as _, Stylesheet};

#[derive(Default)]
struct App {
    saved: bool,
}

impl eframe::App for App {
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

fn create_app(
    cc: &eframe::CreationContext<'_>,
) -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
    let stylesheet = Stylesheet::parse(include_str!("theme.css"))?;
    stylesheet.install(&cc.egui_ctx);

    Ok(Box::new(App::default()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        eframe::run_native(
            "this is an app",
            eframe::NativeOptions::default(),
            Box::new(create_app),
        )?;
    }

    #[cfg(target_arch = "wasm32")]
    {
        use web_sys::wasm_bindgen::JsCast;
        wasm_bindgen_futures::spawn_local(async {
            eframe::WebLogger::init(log::LevelFilter::Debug).ok();

            let document = web_sys::window()
                .expect("browser window is unavailable")
                .document()
                .expect("browser document is unavailable");
            let canvas = document
                .get_element_by_id("the_canvas_id")
                .expect("the_canvas_id canvas is missing")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("the_canvas_id is not a canvas");

            eframe::WebRunner::new()
                .start(canvas, eframe::WebOptions::default(), Box::new(create_app))
                .await
                .expect("failed to start eframe");
        });
    }

    Ok(())
}
