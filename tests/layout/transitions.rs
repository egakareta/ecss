use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn transitions_schedule_frames_only_until_they_finish() {
    let context = Context::default();
    Stylesheet::parse(".box { width:32px; height:24px; transition:width 1s linear; } .box.changed { width:64px; }").unwrap().install(&context);
    let changed = std::cell::Cell::new(false);
    let bounds = std::cell::Cell::new(Rect::NOTHING);
    let mut build = |ui: &mut Ui| {
        bounds.set(
            ui.add(empty_response.selector(if changed.get() {
                ".box.changed"
            } else {
                ".box"
            }))
            .rect,
        );
    };
    for time in [0.0, 0.01, 0.02] {
        frame(&context, 200.0, time, vec![], &mut build);
    }
    changed.set(true);
    frame(&context, 200.0, 1.0, vec![], &mut build);
    let active = frame(&context, 200.0, 1.5, vec![], &mut build);
    assert!((bounds.get().width() - 48.0).abs() < 0.05);
    assert!(
        active.viewport_output[&egui::ViewportId::ROOT].repaint_delay
            <= std::time::Duration::from_millis(17)
    );
    let finished = frame(&context, 200.0, 2.0, vec![], &mut build);
    assert_eq!(bounds.get().width(), 64.0);
    assert_eq!(
        finished.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
        std::time::Duration::MAX
    );
    for time in [2.1, 2.2] {
        frame(&context, 200.0, time, vec![], &mut build);
    }
    let idle = frame(&context, 200.0, 2.3, vec![], &mut build);
    assert_eq!(bounds.get().width(), 64.0);
    assert_eq!(
        idle.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
        std::time::Duration::MAX
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn equivalent_computed_values_do_not_start_transitions() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { font-size:16px; width:32px; height:24px; background:#f00; transition:all 10s linear; }
         .box.changed { width:calc(1em + 16px); background:rgb(255, 0, 0); }",
    ).unwrap().install(&context);
    let changed = std::cell::Cell::new(false);
    let mut build = |ui: &mut Ui| {
        let response = ui.add(empty_response.selector(if changed.get() {
            ".box.changed"
        } else {
            ".box"
        }));
        assert_eq!(response.rect.width(), 32.0);
    };
    for time in [0.0, 0.1, 0.2] {
        frame(&context, 200.0, time, vec![], &mut build);
    }
    changed.set(true);
    for time in [1.0, 1.1, 1.2] {
        frame(&context, 200.0, time, vec![], &mut build);
    }
    let output = frame(&context, 200.0, 1.3, vec![], &mut build);
    assert_eq!(
        output.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
        std::time::Duration::MAX
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn removing_transition_properties_cancels_running_transitions_and_repaints() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { width:32px; height:24px; transition:width 10s linear; }
         .box.changed { width:64px; }
         .box.changed.off { transition-property:none; }",
    )
    .unwrap()
    .install(&context);
    let selector = std::cell::Cell::new(".box");
    let width = std::cell::Cell::new(0.0);
    let mut build = |ui: &mut Ui| {
        width.set(ui.add(empty_response.selector(selector.get())).rect.width());
    };
    frame(&context, 200.0, 0.0, vec![], &mut build);
    selector.set(".box.changed");
    frame(&context, 200.0, 1.0, vec![], &mut build);
    frame(&context, 200.0, 6.0, vec![], &mut build);
    assert!((width.get() - 48.0).abs() < 0.05);
    selector.set(".box.changed.off");
    for time in [6.1, 6.2, 6.3] {
        frame(&context, 200.0, time, vec![], &mut build);
    }
    let idle = frame(&context, 200.0, 6.4, vec![], &mut build);
    assert_eq!(width.get(), 64.0);
    assert_eq!(
        idle.viewport_output[&egui::ViewportId::ROOT].repaint_delay,
        std::time::Duration::MAX
    );
}
