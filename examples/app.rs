use ecss::{SelectorExt as _, Stylesheet};

#[derive(Default)]
struct App {
    saved: bool,
    name: String,
}

fn feature_card(ui: &mut egui::Ui, id: &str, kicker: &str, title: &str, copy: &str) {
    ui.push_id(id, |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new(kicker).selector(".feature-kicker"));
                ui.add(egui::Label::new(title).selector(".feature-title"));
                ui.add(egui::Label::new(copy).selector(".feature-copy"));
                ui.response()
            })
            .selector(".feature-card"),
        );
    });
}

fn typography_card(ui: &mut egui::Ui) {
    ui.push_id("typography", |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("TYPOGRAPHY").selector(".feature-kicker"));
                ui.add(egui::Label::new("Outfit + SUSE Mono").selector(".feature-title"));
                ui.add(egui::Label::new("Aa 0123456789").selector(".type-sample"));
                ui.add(
                    egui::Label::new("Font family, size, weight, and line-height rules.")
                        .selector(".feature-copy"),
                );
                ui.response()
            })
            .selector(".feature-card"),
        );
    });
}

fn sizing_card(ui: &mut egui::Ui) {
    ui.push_id("sizing", |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("SIZING").selector(".feature-kicker"));
                ui.add(egui::Label::new("Percentage widths").selector(".feature-title"));
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(
                            (|ui: &mut egui::Ui| {
                                ui.allocate_exact_size(egui::Vec2::ZERO, egui::Sense::hover())
                                    .1
                            })
                            .selector(".meter-fill"),
                        );
                        ui.response()
                    })
                    .selector(".meter-track"),
                );
                ui.add(
                    egui::Label::new("The fill is sized relative to its track.")
                        .selector(".feature-copy"),
                );
                ui.response()
            })
            .selector(".feature-card"),
        );
    });
}

fn hero(ui: &mut egui::Ui) {
    ui.add(
        (|ui: &mut egui::Ui| {
            ui.add(egui::Label::new("ECSS · FEATURE TOUR").selector(".eyebrow"));
            ui.add(egui::Label::new("CSS for egui").selector(".title"));
            ui.add(
                egui::Label::new(
                    "Layout, typography, sizing, and interactive states—all from a stylesheet.",
                )
                .selector(".hero-copy"),
            );
            ui.response()
        })
        .selector(".hero"),
    );
}

fn feature_grid(ui: &mut egui::Ui) {
    ui.add(
        (|ui: &mut egui::Ui| {
            feature_card(
                ui,
                "layout",
                "LAYOUT",
                "Grid + flexbox",
                "A grid of cards with wrapping flex actions.",
            );
            feature_card(
                ui,
                "interaction",
                "INTERACTION",
                "Hover + active",
                "Move over a button or press it to see state rules.",
            );
            typography_card(ui);
            sizing_card(ui);
            ui.response()
        })
        .selector(".feature-grid"),
    );
}

fn form_panel(ui: &mut egui::Ui, app: &mut App) {
    ui.add(
        (|ui: &mut egui::Ui| {
            ui.add(egui::Label::new("TRY THE FORM").selector(".section-kicker"));
            ui.add(egui::Label::new("A styled input and button states").selector(".section-title"));
            ui.add(
                egui::TextEdit::singleline(&mut app.name)
                    .hint_text("Your name")
                    .selector(".field"),
            );
            ui.add(
                (|ui: &mut egui::Ui| {
                    if ui
                        .add(egui::Button::new("Save changes").selector(".primary"))
                        .clicked()
                    {
                        app.saved = true;
                    }
                    if ui
                        .add(egui::Button::new("Cancel").selector(".secondary"))
                        .clicked()
                    {
                        app.saved = false;
                        app.name.clear();
                    }
                    ui.response()
                })
                .selector(".actions"),
            );
            if app.saved {
                let name = if app.name.is_empty() {
                    "your changes".to_owned()
                } else {
                    app.name.clone()
                };
                ui.add(egui::Label::new(format!("Saved for {name}.")).selector(".status"));
            }
            ui.response()
        })
        .selector(".panel"),
    );
}

fn overflow_panel(ui: &mut egui::Ui) {
    ui.add((|ui: &mut egui::Ui| {
        ui.add(egui::Label::new("OVERFLOW").selector(".section-kicker"));
        ui.add(egui::Label::new("Single-line ellipsis").selector(".section-title"));
        ui.add(egui::Label::new(
            "This deliberately long line demonstrates clipped overflow with an ellipsis at the inline end.",
        )
        .selector(".truncate"));
        ui.response()
    })
    .selector(".panel"));
}

impl App {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.push_id("hero", hero);
                ui.push_id("features", feature_grid);
                ui.push_id("form", |ui| form_panel(ui, self));
                ui.push_id("overflow", overflow_panel);

                ui.response()
            })
            .selector(".page"),
        );
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn collect_text_bounds(
        shape: &egui::Shape,
        text_bounds: &mut Vec<(String, egui::Rect, usize)>,
    ) {
        match shape {
            egui::Shape::Text(shape) => text_bounds.push((
                shape.galley.job.text.to_string(),
                shape.visual_bounding_rect(),
                shape.galley.rows.len().max(1),
            )),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect_text_bounds(shape, text_bounds);
                }
            }
            _ => {}
        }
    }

    fn run_frame(
        context: &egui::Context,
        app: &mut App,
        time: f64,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        run_frame_with_size(context, app, time, egui::vec2(1000.0, 900.0), events)
    }

    fn run_frame_with_size(
        context: &egui::Context,
        app: &mut App,
        time: f64,
        size: egui::Vec2,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| app.show(ui),
        )
    }

    fn select_and_copy(
        context: &egui::Context,
        app: &mut App,
        bounds: egui::Rect,
        rows: usize,
        start_time: f64,
    ) -> (String, bool) {
        let row_height = bounds.height() / rows as f32;
        let start = egui::pos2(
            egui::emath::lerp(bounds.left()..=bounds.right(), 0.2),
            bounds.top() + row_height * 0.5,
        );
        let end = egui::pos2(
            egui::emath::lerp(bounds.left()..=bounds.right(), 0.8),
            bounds.bottom() - row_height * 0.5,
        );
        let mut output = run_frame(
            context,
            app,
            start_time,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        output.textures_delta.clear();
        let mut output = run_frame(
            context,
            app,
            start_time + 0.1,
            vec![egui::Event::PointerMoved(end)],
        );
        output.textures_delta.clear();
        let mut output = run_frame(
            context,
            app,
            start_time + 0.2,
            vec![egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        let selection_color = context.global_style().visuals.selection.bg_fill;
        let selection_visible = context
            .tessellate(output.shapes.clone(), output.pixels_per_point)
            .iter()
            .any(|clipped| match &clipped.primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh
                    .vertices
                    .iter()
                    .any(|vertex| vertex.color == selection_color),
                egui::epaint::Primitive::Callback(_) => false,
            });
        output.textures_delta.clear();
        let mut output = run_frame(context, app, start_time + 0.3, vec![egui::Event::Copy]);
        let copied = output
            .platform_output
            .commands
            .iter()
            .find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_default();
        output.textures_delta.clear();
        (copied, selection_visible)
    }

    fn collect_error_borders(
        shape: &egui::Shape,
        color: egui::Color32,
        borders: &mut Vec<egui::Rect>,
    ) {
        match shape {
            egui::Shape::Rect(shape)
                if shape.stroke.width == 1.0 && shape.stroke.color == color =>
            {
                borders.push(shape.rect);
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect_error_borders(shape, color, borders);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn demo_has_no_egui_widget_id_clash_diagnostics() {
        let context = egui::Context::default();
        context.options_mut(|options| options.warn_on_id_clash = true);
        let theme = include_str!("theme.css").split_once('\n').unwrap().1;
        Stylesheet::parse(theme).unwrap().install(&context);
        let mut app = App::default();
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 900.0),
                )),
                ..Default::default()
            },
            |ui| app.show(ui),
        );
        let error_color = context.global_style().visuals.error_fg_color;
        let mut borders = Vec::new();
        for clipped in &output.shapes {
            collect_error_borders(&clipped.shape, error_color, &mut borders);
        }
        output.textures_delta.clear();
        assert!(context.options(|options| options.warn_on_id_clash));
        assert!(
            borders.is_empty(),
            "egui drew ID-clash borders: {borders:?}"
        );
    }

    #[test]
    fn hero_and_feature_copy_labels_remain_selectable() {
        let context = egui::Context::default();
        let theme = include_str!("theme.css").split_once('\n').unwrap().1;
        Stylesheet::parse(theme).unwrap().install(&context);
        let mut app = App::default();
        let mut output = run_frame(&context, &mut app, 0.0, Vec::new());
        let mut text_bounds = Vec::new();
        for clipped in &output.shapes {
            collect_text_bounds(&clipped.shape, &mut text_bounds);
        }
        output.textures_delta.clear();

        let targets = [
            "CSS for egui",
            "Layout, typography, sizing, and interactive states—all from a stylesheet.",
            "A grid of cards with wrapping flex actions.",
            "Move over a button or press it to see state rules.",
            "Font family, size, weight, and line-height rules.",
            "The fill is sized relative to its track.",
        ];
        for (index, target) in targets.into_iter().enumerate() {
            let (_, bounds, rows) = text_bounds
                .iter()
                .find(|(text, _, _)| text == target)
                .unwrap_or_else(|| panic!("text shape for {target:?} was not rendered"));
            let (copied, selection_visible) =
                select_and_copy(&context, &mut app, *bounds, *rows, 0.1 + index as f64 * 0.4);
            let copied = copied.split_whitespace().collect::<Vec<_>>().join(" ");
            let expected = target.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(
                !copied.is_empty() && copied != expected && expected.contains(&copied),
                "could not partially select {target:?}: {copied:?}"
            );
            assert!(
                selection_visible,
                "selection highlight missing for {target:?}"
            );
        }
    }

    #[test]
    fn demo_page_scrolls_to_content_below_the_viewport() {
        let context = egui::Context::default();
        let theme = include_str!("theme.css").split_once('\n').unwrap().1;
        Stylesheet::parse(theme).unwrap().install(&context);
        let mut app = App::default();
        let viewport = egui::vec2(1000.0, 480.0);
        let target = "OVERFLOW";
        let mut output = run_frame_with_size(&context, &mut app, 0.0, viewport, Vec::new());
        let mut text_bounds = Vec::new();
        for clipped in &output.shapes {
            collect_text_bounds(&clipped.shape, &mut text_bounds);
        }
        output.textures_delta.clear();
        assert!(text_bounds.iter().all(|(text, _, _)| text != target));

        let mut output = run_frame_with_size(
            &context,
            &mut app,
            0.1,
            viewport,
            vec![
                egui::Event::PointerMoved(egui::pos2(500.0, 400.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    phase: egui::TouchPhase::Move,
                    delta: egui::vec2(0.0, -2000.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        let passes = output.platform_output.num_completed_passes;
        let reasons = output.platform_output.request_discard_reasons.clone();
        output.textures_delta.clear();
        assert_eq!(passes, 1, "scroll frame discarded passes: {reasons:?}");

        let mut final_bounds = None;
        for frame in 2..8 {
            let mut output =
                run_frame_with_size(&context, &mut app, frame as f64 * 0.1, viewport, Vec::new());
            let passes = output.platform_output.num_completed_passes;
            let reasons = output.platform_output.request_discard_reasons.clone();
            let mut text_bounds = Vec::new();
            for clipped in &output.shapes {
                collect_text_bounds(&clipped.shape, &mut text_bounds);
            }
            final_bounds = text_bounds
                .into_iter()
                .find(|(text, _, _)| text == target)
                .map(|(_, bounds, _)| bounds);
            output.textures_delta.clear();
            assert_eq!(passes, 1, "scroll frame discarded passes: {reasons:?}");
        }
        let final_bounds = final_bounds.expect("scrolling should reveal the lower page content");
        assert!(final_bounds.top() >= 0.0);
        assert!(final_bounds.bottom() <= viewport.y);
    }

    #[test]
    fn demo_input_fits_inside_its_panel() {
        let context = egui::Context::default();
        let theme = include_str!("theme.css").split_once('\n').unwrap().1;
        Stylesheet::parse(theme).unwrap().install(&context);
        let mut name = String::new();
        let mut field = egui::Rect::NOTHING;
        let mut panel = egui::Rect::NOTHING;
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000.0, 500.0),
                )),
                ..Default::default()
            },
            |ui| {
                panel = ui
                    .add(
                        (|ui: &mut egui::Ui| {
                            field = ui
                                .add(egui::TextEdit::singleline(&mut name).selector(".field"))
                                .rect;
                            ui.response()
                        })
                        .selector(".panel"),
                    )
                    .rect;
            },
        );
        output.textures_delta.clear();
        assert!(field.left() >= panel.left());
        assert!(field.right() <= panel.right());
    }
}
