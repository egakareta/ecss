mod layout {
    use ecss::{SelectorExt as _, Stylesheet};
    use egui::{Context, Event, Modifiers, PointerButton, RawInput, Rect, Ui, Vec2, pos2, vec2};

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn frame(
        context: &Context,
        width: f32,
        time: f64,
        events: Vec<Event>,
        build: impl FnMut(&mut Ui),
    ) -> egui::FullOutput {
        let mut output = context.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 200.0))),
                time: Some(time),
                events,
                ..Default::default()
            },
            build,
        );
        output.textures_delta.clear();
        output
    }

    fn empty(ui: &mut Ui) -> egui::Response {
        ui.allocate_exact_size(Vec2::ZERO, egui::Sense::hover()).1
    }

    automod::dir!("tests/layout");
}
