use super::*;

fn hover_frames(last: usize, changes: &[(usize, bool)]) -> Vec<Vec<egui::Event>> {
    (0..=last)
        .map(|frame| {
            changes
                .iter()
                .filter(|(at, _)| *at == frame)
                .map(|(_, hover)| {
                    egui::Event::PointerMoved(if *hover {
                        egui::pos2(20.0, 20.0)
                    } else {
                        egui::pos2(240.0, 120.0)
                    })
                })
                .collect()
        })
        .collect()
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn hover_transitions_reverse_and_reverse_again_without_jumping() {
    let stylesheet = Stylesheet::parse(
        ".box { width:48px; height:24px; background:black; transition:background-color 1s linear; }
         .box:hover { background:white; }",
    )
    .unwrap();
    for (last, changes, expected) in [
        (1, vec![(1, true)], 0),
        (6, vec![(1, true)], 128),
        (4, vec![(1, true), (4, false)], 77),
        (5, vec![(1, true), (4, false)], 51),
        (7, vec![(1, true), (4, false)], 0),
        (7, vec![(1, true), (4, false), (5, true)], 102),
    ] {
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_with_events(
            stylesheet.clone(),
            hover_frames(last, &changes),
            move |ui| {
                *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
            },
        )
        .await;
        let rect = bounds.lock().unwrap().unwrap();
        let actual = pixel(&pixels, rect.center().x, rect.center().y);
        assert!(
            actual[0].abs_diff(expected) <= 1 && actual[1] == actual[0] && actual[2] == actual[0],
            "frame {last}, changes {changes:?}: {actual:?}, expected {expected}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn transitions_override_important_values_and_animations_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes colors { from { background-color:red; } to { background-color:blue; } }\
         .box { width:48px; height:24px; background-color:black; transition:background-color 1s linear; animation:colors 1s linear both; }\
         .box:hover { background-color:white !important; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels =
        render_ui_pixels_with_events(stylesheet, hover_frames(6, &[(1, true)]), move |ui| {
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
        })
        .await;

    let rect = bounds.lock().unwrap().unwrap();
    let actual = pixel(&pixels, rect.center().x, rect.center().y);
    assert!(
        actual[0].abs_diff(255) <= 1
            && actual[1].abs_diff(128) <= 1
            && actual[2].abs_diff(128) <= 1,
        "transition sample was {actual:?}, expected a halfway red-to-white value"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_position_axis_transition_interpolates_start_and_end_edges() {
    let stylesheet = Stylesheet::parse(
        ".box { width:60px; height:20px; background-color:black; background-image:linear-gradient(to right, lime 50%, transparent 50%); background-size:20px 10px; background-repeat:no-repeat; background-position-x:left 2px; transition:background-position-x 1s linear; }\
         .box:hover { background-position-x:right 10%; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels =
        render_ui_pixels_with_events(stylesheet, hover_frames(6, &[(1, true)]), move |ui| {
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
        })
        .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 25.0, rect.top() + 5.0),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 35.0, rect.top() + 5.0),
        [0, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_position_axis_transition_cycles_layer_lists_before_interpolation() {
    let stylesheet = Stylesheet::parse(
        ".box { width:60px; height:20px; background-color:black; background-image:linear-gradient(to right, lime 50%, transparent 50%), linear-gradient(blue, blue); background-size:20px 10px, 20px 10px; background-repeat:no-repeat; background-position-x:left, right; background-position-y:top; transition:background-position-x 1s linear; }\
         .box:hover { background-position-x:right; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels =
        render_ui_pixels_with_events(stylesheet, hover_frames(6, &[(1, true)]), move |ui| {
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
        })
        .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 25.0, rect.top() + 5.0),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 45.0, rect.top() + 5.0),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_background_tbd_variable_masks_position_animation_and_preserves_color() {
    let stylesheet = Stylesheet::parse(
        "@keyframes shift { from { background-position-x:left; } to { background-position-x:right; } }\
         .box { width:60px; height:20px; --layer:linear-gradient(to right, lime 50%, transparent 50%) / 20px 10px no-repeat; background-color:red; background-tbd:var(--layer) !important; animation:shift 1s linear both; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_with_events(stylesheet, hover_frames(6, &[]), move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 5.0, rect.top() + 5.0),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 25.0, rect.top() + 5.0),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_logical_background_axes_mask_physical_axis_animations() {
    let stylesheet = Stylesheet::parse(
        "@keyframes shift { from { background-position-y:center; background-repeat-y:no-repeat; } to { background-position-y:bottom; background-repeat-y:repeat; } }\
         .box { width:40px; height:40px; writing-mode:vertical-rl; background-color:black; background-image:linear-gradient(to bottom, lime 50%, transparent 50%); background-size:10px 20px; background-position-inline:center !important; background-repeat-inline:no-repeat !important; animation:shift 1s linear both; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_with_events(stylesheet, hover_frames(6, &[]), move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 5.0, rect.top() + 12.0),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 5.0, rect.top() + 25.0),
        [0, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn changing_the_destination_retargets_from_the_current_color() {
    let stylesheet = Stylesheet::parse(
        ".box { width:48px; height:24px; transition:background-color 1s linear; }",
    )
    .unwrap();
    let styles = ["background:black", "background:white", "background:red"]
        .map(|style| InlineStyle::parse(style).unwrap());
    for (last, expected) in [
        (5, [102, 102, 102, 255]),
        (10, [179, 51, 51, 255]),
        (15, [255, 0, 0, 255]),
    ] {
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let styles = styles.clone();
        let pixels =
            render_ui_pixels_with_events(stylesheet.clone(), hover_frames(last, &[]), move |ui| {
                let time = ui.input(|input| input.time);
                let style = &styles[if time < 0.1 {
                    0
                } else if time < 0.5 {
                    1
                } else {
                    2
                }];
                *capture.lock().unwrap() = Some(
                    ui.add(empty_response.selector(".box").inline_style(style.clone()))
                        .rect,
                );
            })
            .await;
        let rect = bounds.lock().unwrap().unwrap();
        let actual = pixel(&pixels, rect.center().x, rect.center().y);
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(actual, expected)| actual.abs_diff(expected) <= 1),
            "frame {last}: {actual:?}, expected {expected:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn transitions_interpolate_computed_numeric_values_border_colors_and_radii() {
    let stylesheet = Stylesheet::parse(
        ".box { font-size:16px; width:1em; height:24px; padding:0; background:red; border:2px solid black; border-radius:0;
            transition:width 1s linear, padding 1s linear, border-radius 1s linear, border-color 1s linear; }
         .box:hover { width:4em; padding:4px; border-radius:12px; border-color:blue; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels =
        render_ui_pixels_with_events(stylesheet, hover_frames(6, &[(1, true)]), move |ui| {
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
        })
        .await;
    let rect = bounds.lock().unwrap().unwrap();
    near(rect.width(), 48.0);
    near(rect.height(), 32.0);
    assert_eq!(
        pixel(&pixels, rect.left() + 0.5, rect.top() + 0.5),
        [255, 255, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 0.5),
        [0, 0, 128, 255]
    );
    let corner = pixel(&pixels, rect.left() + 2.5, rect.top() + 2.5);
    assert!(
        corner[0] < 128 && corner[1] == 0 && corner[2] >= 64,
        "unexpected interpolated corner {corner:?}"
    );
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inherited_and_custom_property_values_supply_transition_endpoints() {
    let stylesheet = Stylesheet::parse(
        ".parent { font-size:20px; --motion:width 1s linear, color 1s linear, background-color 1s linear; }
         .parent.start { color:red; --surface:red; --size:20px; }
         .parent.end { color:blue; --surface:blue; --size:calc(2em + 20px); }
         .child { width:var(--size); height:24px; color:inherit; background-color:var(--surface); transition:var(--motion); }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_with_events(stylesheet, hover_frames(6, &[]), move |ui| {
        let selector = if ui.input(|input| input.time < 0.1) {
            ".parent.start"
        } else {
            ".parent.end"
        };
        ui.add(
            (|ui: &mut egui::Ui| {
                let response = ui.add(
                    (|ui: &mut egui::Ui| {
                        let response =
                            ui.allocate_response(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(response.rect, 0.0, ui.visuals().text_color());
                        response
                    })
                    .selector(".child"),
                );
                *capture.lock().unwrap() = Some(response.rect);
                ui.response()
            })
            .selector(selector),
        );
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    near(rect.width(), 40.0);
    for point in [rect.min + egui::vec2(4.0, 4.0), rect.center()] {
        assert_eq!(pixel(&pixels, point.x, point.y), [128, 0, 128, 255]);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn numeric_transitions_follow_computed_values_even_when_the_declaration_is_unchanged() {
    let stylesheet = Stylesheet::parse(
        ".box { font-size:20px; width:2em; height:24px; background:blue; transition:width 1s linear; }
         .box.large { font-size:40px; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_with_events(stylesheet, hover_frames(6, &[]), move |ui| {
        let selector = if ui.input(|input| input.time < 0.1) {
            ".box"
        } else {
            ".box.large"
        };
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(selector)).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    near(rect.width(), 60.0);
    assert_eq!(
        pixel(&pixels, rect.right() - 1.0, rect.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() + 1.0, rect.center().y),
        [255, 255, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn numeric_destinations_can_change_during_a_transition() {
    let stylesheet = Stylesheet::parse(
        ".box { font-size:20px; width:2em; height:24px; background:blue; transition:width 1s linear; }
         .box.large { font-size:40px; }
         .box.small { font-size:10px; }",
    ).unwrap();
    for (last, width) in [(5, 56.0), (10, 38.0), (15, 20.0)] {
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels =
            render_ui_pixels_with_events(stylesheet.clone(), hover_frames(last, &[]), move |ui| {
                let time = ui.input(|input| input.time);
                let selector = if time < 0.1 {
                    ".box"
                } else if time < 0.5 {
                    ".box.large"
                } else {
                    ".box.small"
                };
                *capture.lock().unwrap() = Some(ui.add(empty_response.selector(selector)).rect);
            })
            .await;
        let rect = bounds.lock().unwrap().unwrap();
        near(rect.width(), width);
        assert_eq!(
            pixel(&pixels, rect.right() - 1.0, rect.center().y),
            [0, 0, 255, 255]
        );
        assert_eq!(
            pixel(&pixels, rect.right() + 1.0, rect.center().y),
            [255, 255, 255, 255]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn numeric_transitions_mix_lengths_and_percentages() {
    let stylesheet = Stylesheet::parse(
        ".box { width:40px; height:24px; background:blue; transition:width 1s linear; }
         .box:hover { width:50%; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels =
        render_ui_pixels_with_events(stylesheet, hover_frames(6, &[(1, true)]), move |ui| {
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
        })
        .await;
    let rect = bounds.lock().unwrap().unwrap();
    near(rect.width(), 84.0);
    assert_eq!(
        pixel(&pixels, rect.right() - 1.0, rect.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() + 1.0, rect.center().y),
        [255, 255, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn retargeting_uses_the_displayed_value_after_numeric_clamping() {
    let stylesheet = Stylesheet::parse(
        ".box { width:20px; height:24px; background:blue; transition:width 1s cubic-bezier(0, -2, 1, -2); }
         .box.changed { width:40px; }
         .box.final { width:60px; transition-timing-function:linear; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_with_events(stylesheet, hover_frames(11, &[]), move |ui| {
        let time = ui.input(|input| input.time);
        let selector = if time < 0.1 {
            ".box"
        } else if time < 0.6 {
            ".box.changed"
        } else {
            ".box.final"
        };
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(selector)).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    near(rect.width(), 30.0);
    assert_eq!(
        pixel(&pixels, rect.right() - 1.0, rect.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() + 1.0, rect.center().y),
        [255, 255, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn transitions_interpolate_percentage_and_calculated_radius_values_across_frames() {
    let stylesheet = Stylesheet::parse(
        ".box { width:64px; height:32px; background:red; border-top-left-radius:0; transition:border-top-left-radius 1s linear; }
         .box.round { border-top-left-radius:calc(25% + 4px); }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_with_events(stylesheet, hover_frames(6, &[]), move |ui| {
        let selector = if ui.input(|input| input.time < 0.1) {
            ".box"
        } else {
            ".box.round"
        };
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(selector)).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 0.5, rect.top() + 0.5),
        [255, 255, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 5.5, rect.top() + 3.5),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() - 0.5, rect.top() + 0.5),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn transition_lists_delays_and_easing_control_each_property_independently() {
    let stylesheet = Stylesheet::parse(
        ".box { width:40px; height:24px; background:black;
            transition-property:width, background-color;
            transition-duration:1s, 2s;
            transition-delay:0.2s, -0.5s;
            transition-timing-function:linear, steps(4, end); }
         .box.changed { width:80px; background:white; }",
    )
    .unwrap();
    for (last, width, color) in [(1, 40.0, 64), (3, 40.0, 64), (8, 60.0, 128)] {
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels =
            render_ui_pixels_with_events(stylesheet.clone(), hover_frames(last, &[]), move |ui| {
                let selector = if ui.input(|input| input.time < 0.1) {
                    ".box"
                } else {
                    ".box.changed"
                };
                *capture.lock().unwrap() = Some(ui.add(empty_response.selector(selector)).rect);
            })
            .await;
        let rect = bounds.lock().unwrap().unwrap();
        near(rect.width(), width);
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.center().y),
            [color, color, color, 255]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn transition_values_validate_their_grammar() {
    for declaration in [
        "transition:background-color 1s linear",
        "transition:all 100ms ease-in 20ms, width 2s cubic-bezier(0.1, 0, 0.9, 1)",
        "transition:1s",
        "transition:none",
        "transition-property:future-property, --custom",
        "transition-duration:0s, 2s",
        "transition-delay:-1s",
        "transition-timing-function:linear(0, 0.25 75%, 1)",
        "transition:inherit",
    ] {
        assert!(
            Stylesheet::parse(&format!(".box {{ {declaration}; }}")).is_ok(),
            "{declaration}"
        );
    }
    for declaration in [
        "transition:width -1s",
        "transition:width 1s 2s 3s",
        "transition:width height 1s",
        "transition:width 1s linear ease",
        "transition:width 1s,",
        "transition-property:none, width",
        "transition-property:width height",
        "transition-duration:-1s",
        "transition-duration:2",
        "transition-delay:banana",
        "transition-timing-function:steps(0)",
        "transition:revert-layer",
    ] {
        assert!(
            Stylesheet::parse(&format!(".box {{ {declaration}; }}")).is_err(),
            "{declaration}"
        );
    }
}
