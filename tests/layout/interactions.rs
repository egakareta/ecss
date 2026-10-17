use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn padding_and_css_size_are_part_of_the_widget_click_target() {
    let context = Context::default();
    Stylesheet::parse(".button { width:80px; height:30px; padding:10px; }")
        .unwrap()
        .install(&context);
    let mut clicked = false;
    let mut build = |ui: &mut Ui| {
        clicked |= ui.add(egui::Button::new("X").selector(".button")).clicked();
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let pos = pos2(90.0, 40.0);
    frame(
        &context,
        200.0,
        0.1,
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &mut build,
    );
    frame(
        &context,
        200.0,
        0.2,
        vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &mut build,
    );
    assert!(clicked, "the CSS padding box should receive clicks");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn paint_only_button_states_do_not_request_extra_passes() {
    let context = Context::default();
    Stylesheet::parse(
        ".button:hover { background-color:#2458b5; } .button:active { background-color:#19448f; }",
    )
    .unwrap()
    .install(&context);
    let rect = std::cell::Cell::new(Rect::NOTHING);
    let mut clicks = 0;
    let mut build = |ui: &mut Ui| {
        let response = ui.add(egui::Button::new("Save changes").selector(".button"));
        rect.set(response.rect);
        clicks += usize::from(response.clicked());
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let pos = rect.get().center();
    let hovered = frame(
        &context,
        200.0,
        0.1,
        vec![Event::PointerMoved(pos)],
        &mut build,
    );
    assert_eq!(hovered.platform_output.num_completed_passes, 1);

    for click in 0..5 {
        let time = 0.2 + click as f64 * 0.2;
        let pressed = frame(
            &context,
            200.0,
            time,
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            }],
            &mut build,
        );
        let released = frame(
            &context,
            200.0,
            time + 0.1,
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
            &mut build,
        );
        assert_eq!(pressed.platform_output.num_completed_passes, 1);
        assert_eq!(released.platform_output.num_completed_passes, 1);
    }

    assert_eq!(clicks, 5);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn z_index_controls_which_overlapping_selector_receives_a_click() {
    let context = Context::default();
    Stylesheet::parse(".root { position:relative; width:100px; height:50px; } .front { position:absolute; inset:0; z-index:2; } .back { position:absolute; inset:0; z-index:1; }").unwrap().install(&context);
    let mut clicks = (false, false);
    let click = |ui: &mut Ui| {
        ui.allocate_exact_size(vec2(1.0, 1.0), egui::Sense::click())
            .1
    };
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                clicks.0 |= ui.add(click.selector(".front")).clicked();
                clicks.1 |= ui.add(click.selector(".back")).clicked();
                ui.response()
            })
            .selector(".root"),
        );
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let pos = pos2(20.0, 20.0);
    frame(
        &context,
        200.0,
        0.1,
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &mut build,
    );
    frame(
        &context,
        200.0,
        0.2,
        vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &mut build,
    );
    assert_eq!(clicks, (true, false));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn hover_specificity_overrides_later_base_layout_declarations() {
    let context = Context::default();
    Stylesheet::parse(".box:hover { width:80px; } .box { width:40px; height:20px; }")
        .unwrap()
        .install(&context);
    let mut width = 0.0;
    let mut build = |ui: &mut Ui| {
        width = ui.add(empty.selector(".box")).rect.width();
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    frame(
        &context,
        200.0,
        0.1,
        vec![Event::PointerMoved(pos2(10.0, 10.0))],
        &mut build,
    );
    assert_eq!(width, 80.0);
}
