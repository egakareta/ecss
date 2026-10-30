use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn fully_important_overridden_animations_do_not_schedule_repaints() {
    let context = Context::default();
    Stylesheet::parse(
        "@keyframes grow { from { width:20px; } to { width:200px; } }\
         .box { width:64px !important; height:24px; animation:grow 1s linear infinite; }",
    )
    .unwrap()
    .install(&context);

    let width = std::cell::Cell::new(0.0);
    let mut build = |ui: &mut Ui| {
        width.set(ui.add(empty_response.selector(".box")).rect.width());
    };
    let mut output = None;
    for time in [0.0, 0.1, 0.2] {
        output = Some(frame(&context, 200.0, time, Vec::new(), &mut build));
        assert_eq!(width.get(), 64.0);
    }
    let output = output.unwrap();
    assert_eq!(
        output.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
        std::time::Duration::MAX
    );
}
