use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn installed_white_background_renders_every_pixel_white_test() {
    let pixels = render_ui_pixels(Stylesheet::default(), |_ui| {}).await;
    assert_eq!(pixels.len(), (WIDTH * HEIGHT) as usize);
    let first_non_white = pixels
        .iter()
        .enumerate()
        .find(|(_, pixel)| **pixel != [255; 4]);
    if let Some((index, pixel)) = first_non_white {
        panic!(
            "pixel ({}, {}) was {pixel:?}, expected opaque white",
            index % WIDTH as usize,
            index / WIDTH as usize,
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn selector_background_color_is_visible_in_rendered_widget_pixels_test() {
    let stylesheet = Stylesheet::parse(".primary { background-color: #f00; }").unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(egui::Button::new("X").selector(".primary"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    let x = (rect.min.x + 4.0).floor() as usize;
    let y = (rect.min.y + 3.0).floor() as usize;
    assert_eq!(pixels[y * WIDTH as usize + x], [255, 0, 0, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_background_shorthands_reset_components_and_keep_longhand_cascade_test() {
    let stylesheet = Stylesheet::parse(
        ".reset { width:60px; height:20px; background-image:linear-gradient(#f00, #f00); }\
         .reset { background:#0f0 !important; }\
         .reset.specific { background-color:#00f; }\
         .longhand { width:60px; height:20px; background:#f00 !important; background-color:#00f !important; }\
         .initial { width:60px; height:20px; background-color:red; background-image:linear-gradient(#f00, #f00); background:initial !important; }\
         .unset { width:60px; height:20px; background-color:red; background:unset; }\
         .revert { width:60px; height:20px; background-color:red; background:revert; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for selector in [
            ".reset.specific",
            ".longhand",
            ".initial",
            ".unset",
            ".revert",
        ] {
            capture
                .lock()
                .unwrap()
                .push(ui.add(empty_response.selector(selector)).rect);
        }
    })
    .await;

    let bounds = bounds.lock().unwrap();
    for (rect, expected) in bounds.iter().zip([
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255; 4],
        [255; 4],
        [255; 4],
    ]) {
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.center().y),
            expected,
            "pixel at {:?}",
            rect.center()
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_background_axes_and_layer_shorthand_preserve_other_animated_components() {
    let stylesheet = Stylesheet::parse(
        "@keyframes move { from { background-color:black; background-position-x:0%; background-position-y:0%; } \
                           to { background-color:white; background-position-x:100%; background-position-y:100%; } } \
         .box { width:40px; height:40px; background:red linear-gradient(lime,lime) left top / 4px 4px no-repeat; animation:move 1s linear both; } \
         .layers { background-tbd:linear-gradient(lime,lime) left top / 4px 4px no-repeat !important; } \
         .x { background-position-x:10px !important; } \
         .inline { writing-mode:vertical-rl; background-position-inline:10px !important; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        let mut capture = capture.lock().unwrap();
        capture.clear();
        for selector in [".box.layers", ".box.x", ".box.inline"] {
            capture.push(ui.add(empty_response.selector(selector)).rect);
        }
    })
    .await;
    let bounds = bounds.lock().unwrap();
    for (rect, x, y) in [
        (bounds[0], 1.0, 1.0),
        (bounds[1], 11.0, 19.0),
        (bounds[2], 19.0, 11.0),
    ] {
        assert_eq!(
            pixel(&pixels, rect.left() + x, rect.top() + y),
            [0, 255, 0, 255],
            "{rect:?}"
        );
        let color = pixel(&pixels, rect.left() + 30.0, rect.top() + 30.0);
        assert!(
            (127..=128).contains(&color[0]) && color[0] == color[1] && color[1] == color[2],
            "{color:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inherited_background_current_color_uses_the_parents_computed_color_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { width:120px; height:70px; color:#f00; background-color:currentColor; }\
         .child { display:block; width:60px; height:30px; color:#00f; }\
         .longhand { background-color:inherit; }\
         .shorthand { background-color:blue; background:inherit; }\
         .image-parent { width:60px; height:30px; background:linear-gradient(transparent, transparent) #f00; }\
         .image-child { display:block; width:60px; height:30px; background-color:#00f; background-image:inherit; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                for selector in [".longhand", ".shorthand"] {
                    capture
                        .lock()
                        .unwrap()
                        .push(ui.add(empty_response.selector(selector)).rect);
                }
                ui.response()
            })
            .selector(".parent"),
        );
        ui.add(
            (|ui: &mut egui::Ui| {
                capture
                    .lock()
                    .unwrap()
                    .push(ui.add(empty_response.selector(".image-child")).rect);
                ui.response()
            })
            .selector(".image-parent"),
        );
    })
    .await;

    for (rect, expected) in
        bounds
            .lock()
            .unwrap()
            .iter()
            .zip([[255, 0, 0, 255], [255, 0, 0, 255], [0, 0, 255, 255]])
    {
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.center().y),
            expected,
            "pixel at {:?}",
            rect.center()
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn all_inherit_keeps_the_parents_computed_background_current_color() {
    let stylesheet = Stylesheet::parse(
        ".parent { width:60px; height:40px; color:red; background-color:currentColor; } \
         .child { all:inherit; color:blue; width:40px; height:20px; } \
         .grandchild { background-color:inherit; width:20px; height:10px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([egui::Rect::NOTHING; 2]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                let child = ui
                    .add(
                        (|ui: &mut egui::Ui| {
                            capture.lock().unwrap()[1] =
                                ui.add(empty_response.selector(".grandchild")).rect;
                            ui.response()
                        })
                        .selector(".child"),
                    )
                    .rect;
                capture.lock().unwrap()[0] = child;
                ui.response()
            })
            .selector(".parent"),
        );
    })
    .await;
    for rect in *bounds.lock().unwrap() {
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.center().y),
            [255, 0, 0, 255]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn opacity_composites_overlapping_descendants_as_a_group_test() {
    let stylesheet = Stylesheet::parse(
        ".group { position:relative; width:60px; height:60px; opacity:50%; } \
         .red { position:absolute; left:0; top:0; width:40px; height:40px; background:#f00; } \
         .blue { position:absolute; left:20px; top:20px; width:40px; height:40px; background:#00f; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".red"));
                ui.add(empty_response.selector(".blue"));
                ui.response()
            })
            .selector(".group"),
        );
        *capture.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 10.5, rect.top() + 10.5),
        [255, 127, 127, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 30.5, rect.top() + 30.5),
        [127, 127, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn opacity_keyframes_composite_the_animated_group_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes fade { from { opacity:0; } to { opacity:1; } } \
         .group { position:relative; width:60px; height:60px; opacity:25%; animation:fade 1s linear both; } \
         .red { position:absolute; left:0; top:0; width:40px; height:40px; background:#f00; } \
         .blue { position:absolute; left:20px; top:20px; width:40px; height:40px; background:#00f; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        let response = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".red"));
                ui.add(empty_response.selector(".blue"));
                ui.response()
            })
            .selector(".group"),
        );
        *capture.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 30.5, rect.top() + 30.5),
        [127, 127, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inherited_custom_properties_resolve_widget_background_colors_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { --surface:#f00; } .primary { background-color:var(--surface); }",
    )
    .unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                let response = ui.add(egui::Button::new("X").selector(".primary"));
                *rect_from_ui.lock().unwrap() = Some(response.rect);
                ui.response()
            })
            .selector(".parent"),
        );
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    let x = (rect.min.x + 4.0).floor() as usize;
    let y = (rect.min.y + 3.0).floor() as usize;
    assert_eq!(pixels[y * WIDTH as usize + x], [255, 0, 0, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn custom_property_backgrounds_paint_on_selector_boxes_test() {
    let stylesheet = Stylesheet::parse(
        ".surface { width:60px; height:30px; --paint:#f00; background-color:var(--paint); }",
    )
    .unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(egui::Label::new("X").selector(".surface"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    let x = (rect.min.x + 50.0).floor() as usize;
    let y = (rect.min.y + 15.0).floor() as usize;
    assert_eq!(pixels[y * WIDTH as usize + x], [255, 0, 0, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn keyframes_animate_background_color_at_the_timeline_midpoint_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes fade { from { background-color: #000; width: 40px; padding: 0; }\
         to { background-color: #fff; width: 2in; padding: 20px 40px; } }\
         .animated { width: 40px; height: 24px; animation: fade 1s linear both; }",
    )
    .unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *rect_from_ui.lock().unwrap() =
            Some(ui.add(egui::Button::new("X").selector(".animated")).rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    assert!(
        (rect.width() - 156.0).abs() < 1.0,
        "animated width was {}",
        rect.width()
    );
    let pixel = pixel(&pixels, rect.min.x + 25.5, rect.min.y + 15.5);
    assert_eq!(pixel, [128, 128, 128, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_declarations_override_keyframes_while_normal_values_animate_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes pulse { from { width:20px; color:#000; background-color:#000; }\
         to { width:80px; color:#fff; background-color:#fff; } }\
         .box { width:20px; height:24px; color:#000; background-color:#000; animation:pulse 1s linear both; }\
         .box.important-width { width:36px !important; }\
         .box.important-color { color:#f00 !important; }\
         .box.important-background { background-color:#0f0 !important; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let rects = bounds.clone();
    let inline = InlineStyle::parse(
        "width:44px !important; color:#0ff !important; background-color:#f0f !important;",
    )
    .unwrap();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        let mut current_bounds = Vec::new();
        for selector in [
            ".box",
            ".box.important-width",
            ".box.important-color",
            ".box.important-background",
        ] {
            current_bounds.push(ui.add(egui::Button::new("X").selector(selector)).rect);
        }
        current_bounds.push(
            ui.add(
                egui::Button::new("X")
                    .selector(".box")
                    .inline_style(inline.clone()),
            )
            .rect,
        );
        *rects.lock().unwrap() = current_bounds;
    })
    .await;

    let bounds = bounds.lock().unwrap();
    for (rect, expected) in bounds.iter().zip([50.0, 36.0, 50.0, 50.0, 44.0]) {
        near(rect.width(), expected);
    }
    let has_tint = |rect: egui::Rect, red: bool| {
        (rect.min.y.floor() as usize..rect.max.y.ceil() as usize).any(|y| {
            (rect.min.x.floor() as usize..rect.max.x.ceil() as usize).any(|x| {
                let color = pixels[y * WIDTH as usize + x];
                if red {
                    color[0] > 220 && color[1] < 180 && color[2] < 180
                } else {
                    color[0] < 180 && color[1] > 220 && color[2] > 220
                }
            })
        })
    };
    assert_eq!(
        pixel(&pixels, bounds[0].min.x + 4.0, bounds[0].min.y + 4.0),
        [128, 128, 128, 255]
    );
    assert!(
        has_tint(bounds[2], true),
        "important text color was not red"
    );
    assert_eq!(
        pixel(&pixels, bounds[3].min.x + 4.0, bounds[3].min.y + 4.0),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[4].min.x + 4.0, bounds[4].min.y + 4.0),
        [255, 0, 255, 255]
    );
    assert!(
        has_tint(bounds[4], false),
        "inline important text color was not cyan"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn custom_properties_resolve_in_keyframe_declarations_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes fade { from { background-color:var(--from); } to { background-color:var(--to); } }\
         .animated { --from:#000; --to:#fff; width:40px; height:24px; animation:fade 1s linear both; }",
    )
    .unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *rect_from_ui.lock().unwrap() =
            Some(ui.add(egui::Button::new("X").selector(".animated")).rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.min.x + 5.5, rect.min.y + 5.5),
        [128, 128, 128, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn keyframes_interpolate_length_and_percentage_widths_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes grow { from { width: 40px; padding: 0; } to { width: 100%; padding: 0; } }\
         .animated { animation: grow 1s linear both; }",
    )
    .unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *rect_from_ui.lock().unwrap() =
            Some(ui.add(egui::Button::new("X").selector(".animated")).rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    assert!(
        (rect.width() - 148.0).abs() < 1.0,
        "animated width was {}",
        rect.width()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn keyframes_interpolate_background_position_axis_edges_at_the_midpoint_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes slide { from { background-position-x:left 2px; } to { background-position-x:right 10%; } }\
         .animated { width:60px; height:20px; background-color:black; background-image:linear-gradient(to right, lime 50%, transparent 50%); background-size:20px 10px; background-repeat:no-repeat; animation:slide 1s linear both; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".animated")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 24.5, rect.top() + 5.5),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 30.5, rect.top() + 5.5),
        [0, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn keyframe_position_axis_lists_cycle_before_interpolation_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes slide { from { background-position-x:left, right; } to { background-position-x:right; } }\
         .animated { width:60px; height:20px; background-color:black; background-image:linear-gradient(to right, lime 50%, transparent 50%), linear-gradient(blue, blue); background-size:20px 10px, 20px 10px; background-repeat:no-repeat; background-position-x:left, right; background-position-y:top; animation:slide 1s linear both; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".animated")).rect);
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
async fn background_repeat_axis_keyframes_switch_discretely_at_midpoint_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes tile { from { background-repeat-x:no-repeat; } to { background-repeat-x:repeat; } }\
         .animated { width:40px; height:20px; background-color:black; background-image:linear-gradient(to right, lime 50%, transparent 50%); background-size:10px 10px; background-repeat-y:no-repeat; background-repeat-x:no-repeat; animation:tile 1s linear both; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".animated")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 12.0, rect.top() + 5.0),
        [0, 255, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn longhand_animation_properties_keep_a_paused_animation_at_its_start_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes paused-fade { from { background-color: #000; } to { background-color: #fff; } }\
         .paused { animation-name: paused-fade; animation-duration: 1s; animation-delay: 0s;\
         animation-timing-function: linear; animation-iteration-count: 1; animation-direction: normal;\
         animation-fill-mode: both; animation-play-state: paused; }",
    )
    .unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *rect_from_ui.lock().unwrap() =
            Some(ui.add(egui::Button::new("X").selector(".paused")).rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    let pixel = pixel(&pixels, rect.min.x + 5.5, rect.min.y + 5.5);
    assert_eq!(pixel, [0, 0, 0, 255]);
}

#[test]
fn easing_function_values_follow_css_easing_level_2_grammar_test() {
    for easing in [
        "linear",
        "linear(0, 1)",
        "linear(0, 0.25, 1)",
        "linear(0, 0.25 75%, 1)",
        "linear(0, 0.25 25% 75%, 1)",
        "linear(0 20%, 0.5 10%, 1)",
        "linear(-0.5 -10%, 1 110%)",
        "linear(/*comment*/ 0, 1)",
        "linear(0 calc(50%), 0 100%)",
        "linear(0 0% 50%, 1 50% 100%)",
        "linear(0 calc(0%), 1)",
        "linear(0 calc(50% - 50%), 1)",
        "linear(0 calc(min(50%, 60%)), 1)",
        "linear(calc(0/0), 1)",
        "ease",
        "ease-in",
        "ease-out",
        "ease-in-out",
        "cubic-bezier(0, 0, 1, 1)",
        "cubic-bezier(/*comment*/ 0, 0, 1, 1)",
        "steps(1)",
        "steps(/*comment*/ 2)",
        "steps(1, start)",
        "steps(1, end)",
        "steps(2, jump-none)",
        "steps(4, jump-both)",
        "steps(4, jump-start)",
        "steps(4, jump-end)",
        "step-start",
        "step-end",
        "StEpS(3, JUMP-START)",
    ] {
        assert!(
            Stylesheet::parse(&format!(
                ".primary {{ animation-timing-function: {easing}; }}"
            ))
            .is_ok(),
            "rejected valid easing function: {easing}"
        );
    }

    for easing in [
        "cubic-bezier(-0.01, 0, 1, 1)",
        "cubic-bezier(0, 0, 1.01, 1)",
        "cubic-bezier(0, 0, 1, 1) trailing",
        "steps(0)",
        "steps(1, jump-none)",
        "steps(1.0)",
        "steps(4, jump-middle)",
        "steps(4, jump-both, end)",
        "linear()",
        "linear(0)",
        "linear(0,,1)",
        "linear(0,1,)",
        "linear(0 10% 20% 30%, 1)",
        "linear(0, 1 10% 20% 30%)",
        "linear(0% 1 50%)",
        "linear(0% 1)",
        "linear(0 calc(50px - 50%), 0 calc(50em + 50em))",
    ] {
        assert!(
            Stylesheet::parse(&format!(
                ".primary {{ animation-timing-function: {easing}; }}"
            ))
            .is_err(),
            "accepted invalid easing function: {easing}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn easing_functions_render_step_boundaries_and_before_fill_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes color { from { background-color: #000; } to { background-color: #fff; } }\
         .jump-start { animation: color 1s steps(4, jump-start) both; }\
         .jump-end { animation: color 1s steps(4, jump-end) both; }\
         .jump-none { animation: color 1s steps(4, jump-none) both; }\
         .jump-both { animation: color 1s steps(4, jump-both) both; }\
         .before { animation: color 1s 1s steps(1, start) backwards; }",
    )
    .unwrap();
    let widget_rects = std::sync::Arc::new(Mutex::new(Vec::new()));
    let rects_from_ui = widget_rects.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        rects_from_ui.lock().unwrap().clear();
        for selector in [
            ".jump-start",
            ".jump-end",
            ".jump-none",
            ".jump-both",
            ".before",
        ] {
            rects_from_ui
                .lock()
                .unwrap()
                .push(ui.add(egui::Button::new("X").selector(selector)).rect);
        }
    })
    .await;

    let rects = widget_rects.lock().unwrap();
    for (index, expected) in [(0, 191), (1, 128), (2, 170), (3, 153), (4, 0)] {
        let rect = rects[index];
        let pixel = pixel(&pixels, rect.min.x + 5.5, rect.center().y);
        assert!(
            (i16::from(pixel[0]) - expected).abs() <= 1,
            "{index} easing pixel was {pixel:?}, expected grayscale {expected}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn linear_easing_functions_interpolate_and_cubic_bezier_overshoots_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes color { from { background-color: #000; } to { background-color: #fff; } }\
         @keyframes stretch { from { width: 40px; animation-timing-function: linear(0, 0.5 50%, 1); }\
         to { width: 100px; } }\
         .linear-even { animation: color 1s linear(0, 0.25, 1) both; }\
         .linear-explicit { animation: color 1s linear(0, 0.25 75%, 1) both; }\
         .linear-double { animation: color 1s linear(0, 0.5 25% 75%, 1) both; }\
         .linear-clamped { animation: color 1s linear(0 20%, 0.5 10%, 1) both; }\
         .overshoot { box-sizing: border-box; animation: stretch 1s cubic-bezier(0, 2, 0, 2) both; }",
    )
    .unwrap();
    let widget_rects = std::sync::Arc::new(Mutex::new(Vec::new()));
    let rects_from_ui = widget_rects.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        rects_from_ui.lock().unwrap().clear();
        for selector in [
            ".linear-even",
            ".linear-explicit",
            ".linear-double",
            ".linear-clamped",
        ] {
            rects_from_ui
                .lock()
                .unwrap()
                .push(ui.add(egui::Button::new("X").selector(selector)).rect);
        }
        rects_from_ui
            .lock()
            .unwrap()
            .push(ui.add(egui::Button::new("X").selector(".overshoot")).rect);
    })
    .await;

    let rects = widget_rects.lock().unwrap();
    for (index, expected) in [(0, 64), (1, 42), (2, 128), (3, 175)] {
        let rect = rects[index];
        let pixel = pixel(&pixels, rect.min.x + 5.5, rect.center().y);
        assert!(
            (i16::from(pixel[0]) - expected).abs() <= 1,
            "{index} linear easing pixel was {pixel:?}, expected grayscale {expected}"
        );
    }
    assert!(
        (rects[4].width() - 129.0).abs() < 1.0,
        "linearly extrapolated width was {}",
        rects[4].width()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn linear_easing_extrapolates_outside_its_input_range_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes color { from { background-color: #000; } to { background-color: #fff; } }\
         .extrapolated { animation: color 1s linear(0.3 60%, 0.7 80%) both; }",
    )
    .unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *rect_from_ui.lock().unwrap() = Some(
            ui.add(egui::Button::new("X").selector(".extrapolated"))
                .rect,
        );
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    let pixel = pixel(&pixels, rect.min.x + 5.5, rect.center().y);
    assert!(
        (i16::from(pixel[0]) - 26).abs() <= 1,
        "linearly extrapolated pixel was {pixel:?}, expected grayscale 26"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn linear_easing_uses_last_duplicate_and_before_flag_uses_first_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes color { from { background-color: #000; } to { background-color: #fff; } }\
         .duplicate { animation: color 1s linear(0 0%, 0.5 0%, 1) both; }\
         .before { animation: color 1s 1s linear(0 0%, 0.5 0%, 1) backwards; }",
    )
    .unwrap();
    let widget_rects = std::sync::Arc::new(Mutex::new(Vec::new()));
    let rects_from_ui = widget_rects.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.0, move |ui| {
        rects_from_ui.lock().unwrap().clear();
        for selector in [".duplicate", ".before"] {
            rects_from_ui
                .lock()
                .unwrap()
                .push(ui.add(egui::Button::new("X").selector(selector)).rect);
        }
    })
    .await;

    let rects = widget_rects.lock().unwrap();
    for (index, expected) in [(0, 128), (1, 0)] {
        let rect = rects[index];
        let pixel = pixel(&pixels, rect.min.x + 5.5, rect.center().y);
        assert!(
            (i16::from(pixel[0]) - expected).abs() <= 1,
            "linear easing pixel was {pixel:?}, expected grayscale {expected}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_shorthand_gradient_and_content_clip_render_correctly_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { width: 60px; height: 24px; padding: 4px; border: 4px solid black; background: linear-gradient(to right, red, blue); background-clip: content-box; background-attachment: scroll; }",
    )
    .unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(empty_response.selector(".gradient"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    let center_y = rect.center().y;
    assert_eq!(
        pixel(&pixels, rect.min.x + 5.5, center_y),
        [255, 255, 255, 255]
    );
    assert_eq!(pixel(&pixels, rect.min.x + 1.5, center_y), [0, 0, 0, 255]);

    let left = pixel(&pixels, rect.min.x + 9.5, center_y);
    let right = pixel(&pixels, rect.max.x - 9.5, center_y);
    assert!(left[0] > left[2], "left gradient pixel was {left:?}");
    assert!(right[2] > right[0], "right gradient pixel was {right:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn gradient_background_respects_border_radius_and_padding_clip_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { width: 60px; height: 24px; border: 1px solid black; border-radius: 12px; background: linear-gradient(to right, red, blue); background-clip: padding-box; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".gradient")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.min.x + 2.5, rect.min.y + 2.5),
        [255, 255, 255, 255]
    );
    let center = pixel(&pixels, rect.center().x, rect.center().y);
    assert!(
        center[0] > 0 && center[2] > 0,
        "center gradient pixel was {center:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_image_data_url_is_decoded_and_tiled_test() {
    let source = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 255, 255]));
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    let css = format!(
        ".image {{ width: 16px; height: 12px; background-image: url(\"data:image/png;base64,{data}\"); }}"
    );
    let stylesheet = Stylesheet::load(&css).await.unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(empty_response.selector(".image"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [255, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_position_size_repeat_origin_and_shorthand_render_test() {
    let stylesheet = Stylesheet::parse(
        ".position { width:40px; height:20px; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-position:right bottom; background-repeat:no-repeat; }\
         .repeat { width:40px; height:20px; background-image:linear-gradient(to right, red 50%, blue 50%); background-size:10px 10px; background-repeat:repeat-x; }\
         .origin { width:40px; height:20px; padding:5px; border:2px solid black; background-image:linear-gradient(to right, red, blue); background-size:100% 100%; background-origin:content-box; background-clip:padding-box; background-repeat:no-repeat; }\
         .shorthand { width:40px; height:20px; background:linear-gradient(to right, red, blue) right bottom / 20px 10px no-repeat; }\
         .space { width:40px; height:20px; background-image:linear-gradient(to right, red 10%, blue 10%); background-size:50px 10px; background-repeat:space; background-position:left top; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for selector in [".position", ".repeat", ".origin", ".shorthand", ".space"] {
            capture
                .lock()
                .unwrap()
                .push(ui.add(empty_response.selector(selector)).rect);
        }
    })
    .await;

    let bounds = bounds.lock().unwrap();
    let position = bounds[0];
    assert_eq!(
        pixel(&pixels, position.left() + 5.0, position.top() + 5.0),
        [255; 4],
        "position rect was {position:?}"
    );
    let positioned = pixel(&pixels, position.right() - 5.0, position.bottom() - 5.0);
    assert!(
        positioned[2] > positioned[0],
        "positioned gradient was {positioned:?}"
    );

    let repeated = bounds[1];
    let first_red = pixel(&pixels, repeated.left() + 2.0, repeated.top() + 5.0);
    let first_blue = pixel(&pixels, repeated.left() + 7.0, repeated.top() + 5.0);
    let second_red = pixel(&pixels, repeated.left() + 12.0, repeated.top() + 5.0);
    assert!(first_red[0] > first_red[2], "first tile was {first_red:?}");
    assert!(
        first_blue[2] > first_blue[0],
        "first tile end was {first_blue:?}"
    );
    assert!(
        second_red[0] > second_red[2],
        "second tile was {second_red:?}"
    );
    assert_eq!(
        pixel(&pixels, repeated.center().x, repeated.bottom() - 3.0),
        [255; 4]
    );

    let origin = bounds[2];
    let content_left = pixel(&pixels, origin.left() + 10.0, origin.center().y);
    let content_right = pixel(&pixels, origin.left() + 43.0, origin.center().y);
    assert!(
        content_left[0] > content_left[2],
        "content origin start was {content_left:?}"
    );
    assert!(
        content_right[2] > content_right[0],
        "content origin end was {content_right:?}"
    );
    assert_eq!(
        pixel(&pixels, origin.left() + 4.0, origin.center().y),
        [255; 4]
    );

    let shorthand = bounds[3];
    assert_eq!(
        pixel(&pixels, shorthand.left() + 5.0, shorthand.top() + 5.0),
        [255; 4]
    );
    let shorthand_pixel = pixel(&pixels, shorthand.right() - 5.0, shorthand.bottom() - 5.0);
    assert!(
        shorthand_pixel[2] > shorthand_pixel[0],
        "shorthand gradient was {shorthand_pixel:?}"
    );

    let spaced = bounds[4];
    let first_pixel = pixel(&pixels, spaced.left() + 1.5, spaced.top() + 5.0);
    assert!(
        first_pixel[0] > first_pixel[2],
        "single space-repeated tile ignored its background position: {first_pixel:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_tbd_resets_layer_values_preserves_color_and_obeys_wide_and_logical_values_test()
{
    let stylesheet = Stylesheet::parse(
        ".tbd { width:40px; height:20px; background-color:red; background-position-x:right; background-repeat:no-repeat; background-size:10px 10px; background-tbd:linear-gradient(to right, lime 50%, transparent 50%) / 10px 10px; }\
         .initial, .unset, .revert { width:40px; height:20px; background-color:red; background-position-x:right; background-repeat:no-repeat; }\
         .initial { background-tbd:initial; background-image:linear-gradient(to right, lime 50%, transparent 50%); background-size:10px 10px; }\
         .unset { background-tbd:unset; background-image:linear-gradient(to right, lime 50%, transparent 50%); background-size:10px 10px; }\
         .revert { background-tbd:revert; background-image:linear-gradient(to right, lime 50%, transparent 50%); background-size:10px 10px; }\
         .logical { width:40px; height:20px; direction:ltr; background-color:red; background-tbd:linear-gradient(to right, lime 50%, transparent 50%) / 10px 10px no-repeat; background-position-inline:end; }\
         .vertical { width:40px; height:20px; writing-mode:vertical-rl; background-color:red; background-tbd:linear-gradient(to right, lime 50%, transparent 50%) / 10px 10px no-repeat; background-position-inline:end; }",
    )
    .unwrap();
    assert!(Stylesheet::parse(".invalid { background-tbd:url(missing.png) red; }").is_err());

    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for selector in [
            ".tbd",
            ".initial",
            ".unset",
            ".revert",
            ".logical",
            ".vertical",
        ] {
            capture
                .lock()
                .unwrap()
                .push(ui.add(empty_response.selector(selector)).rect);
        }
    })
    .await;

    let bounds = bounds.lock().unwrap();
    for rect in &bounds[..4] {
        assert_eq!(
            pixel(&pixels, rect.left() + 12.0, rect.top() + 5.0),
            [0, 255, 0, 255],
            "default position/repeat did not reset for {rect:?}"
        );
        assert_eq!(
            pixel(&pixels, rect.left() + 28.0, rect.top() + 5.0),
            [255, 0, 0, 255],
            "background-tbd changed background-color for {rect:?}"
        );
    }
    let logical = bounds[4];
    assert_eq!(
        pixel(&pixels, logical.right() - 8.0, logical.top() + 3.0),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, logical.left() + 3.0, logical.top() + 3.0),
        [255, 0, 0, 255]
    );
    let vertical = bounds[5];
    assert_eq!(
        pixel(&pixels, vertical.left() + 3.0, vertical.bottom() - 8.0),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, vertical.left() + 3.0, vertical.top() + 3.0),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_tbd_inherit_copies_layer_values_without_inheriting_color_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { width:40px; height:20px; background-color:blue; background-image:linear-gradient(to right, lime 50%, transparent 50%); background-size:10px 10px; background-repeat:no-repeat; background-position-x:right; }\
         .child { width:40px; height:20px; background-color:red; background-tbd:inherit; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 5.0, rect.top() + 5.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() - 8.0, rect.top() + 5.0),
        [0, 255, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_axis_longhands_position_and_cycle_layer_lists_test() {
    let stylesheet = Stylesheet::parse(
        ".position { width:60px; height:40px; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-position:left top; background-position-x:right 5px; background-position-y:bottom 3px; background-repeat-x:no-repeat; background-repeat-y:no-repeat; }\
         .cycled { width:30px; height:20px; background-image:linear-gradient(red, red), linear-gradient(blue, blue); background-size:10px 10px; background-position:left top; background-repeat-x:no-repeat; background-repeat-y:no-repeat; }\
         .listed { width:30px; height:20px; background-image:linear-gradient(red, red), linear-gradient(blue, blue); background-size:10px 10px; background-position:left top; background-repeat-x:no-repeat, repeat; background-repeat-y:no-repeat; }\
         .position-list { width:40px; height:20px; background-image:none, linear-gradient(red, red), linear-gradient(blue, blue); background-size:10px 10px; background-position-x:right, left; background-position-y:top; background-repeat:no-repeat; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for selector in [".position", ".cycled", ".listed", ".position-list"] {
            capture
                .lock()
                .unwrap()
                .push(ui.add(empty_response.selector(selector)).rect);
        }
    })
    .await;

    let bounds = bounds.lock().unwrap();
    let position = bounds[0];
    assert_eq!(
        pixel(&pixels, position.left() + 4.0, position.top() + 4.0),
        [255; 4]
    );
    let left = pixel(&pixels, position.left() + 37.0, position.top() + 29.0);
    let right = pixel(&pixels, position.left() + 53.0, position.top() + 29.0);
    assert!(
        left[0] > left[2],
        "left edge of positioned image was {left:?}"
    );
    assert!(
        right[2] > right[0],
        "right edge of positioned image was {right:?}"
    );
    assert_eq!(
        pixel(&pixels, position.left() + 37.0, position.top() + 15.0),
        [255; 4]
    );

    let cycled = bounds[1];
    assert_eq!(
        pixel(&pixels, cycled.left() + 2.0, cycled.top() + 2.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, cycled.left() + 12.0, cycled.top() + 2.0),
        [255; 4],
        "a one-item axis list must apply to every background layer"
    );

    let listed = bounds[2];
    assert_eq!(
        pixel(&pixels, listed.left() + 2.0, listed.top() + 2.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, listed.left() + 12.0, listed.top() + 2.0),
        [0, 0, 255, 255],
        "the second repeat value must apply to the second background layer"
    );

    let position_list = bounds[3];
    assert_eq!(
        pixel(
            &pixels,
            position_list.left() + 2.0,
            position_list.top() + 2.0
        ),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(
            &pixels,
            position_list.left() + 32.0,
            position_list.top() + 2.0
        ),
        [0, 0, 255, 255],
        "a shorter position list must cycle to the third background layer"
    );
    assert_eq!(
        pixel(
            &pixels,
            position_list.left() + 18.0,
            position_list.top() + 2.0
        ),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn logical_background_axis_values_follow_direction_and_writing_mode_test() {
    let stylesheet = Stylesheet::parse(
        ".horizontal-ltr { width:60px; height:40px; direction:ltr; writing-mode:horizontal-tb; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-position-inline:start 5px; background-position-block:start 4px; }\
         .horizontal-rtl { width:60px; height:40px; direction:rtl; writing-mode:horizontal-tb; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-position-inline:start 5px; background-position-block:start 4px; }\
         .vertical-rl { width:60px; height:40px; direction:ltr; writing-mode:vertical-rl; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-position-inline:start 5px; background-position-block:start 4px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for selector in [".horizontal-ltr", ".horizontal-rtl", ".vertical-rl"] {
            capture
                .lock()
                .unwrap()
                .push(ui.add(empty_response.selector(selector)).rect);
        }
    })
    .await;

    let bounds = bounds.lock().unwrap();
    for (rect, image_origin, outside) in [
        (bounds[0], egui::vec2(5.0, 4.0), egui::vec2(35.0, 4.0)),
        (bounds[1], egui::vec2(35.0, 4.0), egui::vec2(5.0, 4.0)),
        (bounds[2], egui::vec2(36.0, 5.0), egui::vec2(5.0, 5.0)),
    ] {
        let image = pixel(
            &pixels,
            rect.left() + image_origin.x + 1.0,
            rect.top() + image_origin.y + 1.0,
        );
        assert!(image[0] > image[2], "logical image origin was {image:?}");
        assert_eq!(
            pixel(&pixels, rect.left() + outside.x, rect.top() + outside.y,),
            [255; 4]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn logical_background_position_offsets_and_wide_defaults_follow_flow_start_test() {
    let stylesheet = Stylesheet::parse(
        ".block-offset { width:60px; height:40px; writing-mode:vertical-rl; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-position-block:5px; }\
         .inline-initial { width:60px; height:40px; direction:rtl; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-position-inline:initial; }\
         .block-unset { width:60px; height:40px; writing-mode:vertical-rl; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-position-block:unset; }\
         .vertical-rtl-inline-initial { width:60px; height:40px; writing-mode:vertical-rl; direction:rtl; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-position-inline:initial; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for row in [
            [".block-offset", ".inline-initial"],
            [".block-unset", ".vertical-rtl-inline-initial"],
        ] {
            ui.horizontal(|ui| {
                for selector in row {
                    capture
                        .lock()
                        .unwrap()
                        .push(ui.add(empty_response.selector(selector)).rect);
                }
            });
        }
    })
    .await;

    let bounds = bounds.lock().unwrap();
    let block_offset = bounds[0];
    assert!(
        pixel(
            &pixels,
            block_offset.left() + 36.0,
            block_offset.top() + 2.0
        )[0] > pixel(
            &pixels,
            block_offset.left() + 36.0,
            block_offset.top() + 2.0
        )[2],
        "a bare block offset in vertical-rl must be measured inward from the right"
    );
    assert_eq!(
        pixel(&pixels, block_offset.left() + 4.0, block_offset.top() + 2.0),
        [255; 4]
    );

    for rect in [bounds[1], bounds[2]] {
        assert!(
            pixel(&pixels, rect.left() + 41.0, rect.top() + 2.0)[0]
                > pixel(&pixels, rect.left() + 41.0, rect.top() + 2.0)[2],
            "initial/unset logical start should resolve to the right edge"
        );
        assert_eq!(
            pixel(&pixels, rect.left() + 2.0, rect.top() + 2.0),
            [255; 4]
        );
    }

    let vertical_inline = bounds[3];
    let bottom = pixel(
        &pixels,
        vertical_inline.left() + 2.0,
        vertical_inline.top() + 31.0,
    );
    assert!(
        bottom[0] > bottom[2],
        "RTL inline initial in vertical writing should start at the bottom: {bottom:?}"
    );
    assert_eq!(
        pixel(
            &pixels,
            vertical_inline.left() + 2.0,
            vertical_inline.top() + 2.0
        ),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_position_edge_percentages_use_free_space_in_longhands_and_shorthand_test() {
    let stylesheet = Stylesheet::parse(
        ".x-longhand { width:60px; height:40px; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-position-x:right 50%; }\
         .x-shorthand { width:60px; height:40px; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-position:right 50% top; }\
         .y-longhand { width:60px; height:40px; background-image:linear-gradient(to bottom, red, blue); background-size:10px 20px; background-repeat:no-repeat; background-position-y:bottom 50%; }\
         .xy-shorthand { width:60px; height:40px; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-position:right 50% bottom 50%; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for row in [
            [".x-longhand", ".x-shorthand"],
            [".y-longhand", ".xy-shorthand"],
        ] {
            ui.horizontal(|ui| {
                for selector in row {
                    capture
                        .lock()
                        .unwrap()
                        .push(ui.add(empty_response.selector(selector)).rect);
                }
            });
        }
    })
    .await;

    let bounds = bounds.lock().unwrap();
    for rect in [bounds[0], bounds[1]] {
        assert!(
            pixel(&pixels, rect.left() + 38.0, rect.top() + 2.0)[2]
                > pixel(&pixels, rect.left() + 38.0, rect.top() + 2.0)[0],
            "right 50% should position the image at the midpoint of the free space"
        );
        assert_eq!(
            pixel(&pixels, rect.left() + 11.0, rect.top() + 2.0),
            [255; 4]
        );
    }

    let y_longhand = bounds[2];
    let lower = pixel(&pixels, y_longhand.left() + 2.0, y_longhand.top() + 28.0);
    assert!(lower[2] > lower[0], "bottom 50% position was {lower:?}");
    assert_eq!(
        pixel(&pixels, y_longhand.left() + 2.0, y_longhand.top() + 2.0),
        [255; 4]
    );

    let xy_shorthand = bounds[3];
    let lower_right = pixel(
        &pixels,
        xy_shorthand.left() + 38.0,
        xy_shorthand.top() + 24.0,
    );
    assert!(
        lower_right[2] > lower_right[0],
        "right/bottom percentage offsets in the shorthand were {lower_right:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_position_axis_start_end_keywords_follow_writing_direction_test() {
    let stylesheet = Stylesheet::parse(
        ".x-start-rtl { width:60px; height:40px; direction:rtl; background-image:linear-gradient(red, red); background-size:10px 10px; background-repeat:no-repeat; background-position-x:x-start; }\
         .x-end-rtl { width:60px; height:40px; direction:rtl; background-image:linear-gradient(red, red); background-size:10px 10px; background-repeat:no-repeat; background-position-x:x-end; }\
         .x-start-vertical { width:60px; height:40px; writing-mode:vertical-rl; background-image:linear-gradient(red, red); background-size:10px 10px; background-repeat:no-repeat; background-position-x:x-start; }\
         .x-end-vertical { width:60px; height:40px; writing-mode:vertical-rl; background-image:linear-gradient(red, red); background-size:10px 10px; background-repeat:no-repeat; background-position-x:x-end; }\
         .y-start-vertical-rtl { width:60px; height:40px; writing-mode:vertical-rl; direction:rtl; background-image:linear-gradient(red, red); background-size:10px 10px; background-repeat:no-repeat; background-position-y:y-start; }\
         .y-end-vertical-rtl { width:60px; height:40px; writing-mode:vertical-rl; direction:rtl; background-image:linear-gradient(red, red); background-size:10px 10px; background-repeat:no-repeat; background-position-y:y-end; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for row in [
            [".x-start-rtl", ".x-end-rtl", ".x-start-vertical"],
            [
                ".x-end-vertical",
                ".y-start-vertical-rtl",
                ".y-end-vertical-rtl",
            ],
        ] {
            ui.horizontal(|ui| {
                for selector in row {
                    capture
                        .lock()
                        .unwrap()
                        .push(ui.add(empty_response.selector(selector)).rect);
                }
            });
        }
    })
    .await;

    let bounds = bounds.lock().unwrap();
    for (rect, x, y, outside_x, outside_y) in [
        (bounds[0], 51.0, 1.0, 2.0, 1.0),
        (bounds[1], 1.0, 1.0, 51.0, 1.0),
        (bounds[2], 51.0, 1.0, 2.0, 1.0),
        (bounds[3], 1.0, 1.0, 51.0, 1.0),
        (bounds[4], 1.0, 31.0, 1.0, 2.0),
        (bounds[5], 1.0, 1.0, 1.0, 31.0),
    ] {
        assert_eq!(
            pixel(&pixels, rect.left() + x, rect.top() + y),
            [255, 0, 0, 255]
        );
        assert_eq!(
            pixel(&pixels, rect.left() + outside_x, rect.top() + outside_y),
            [255; 4]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn logical_background_repeat_axes_map_to_physical_tiles_test() {
    for animated in [false, true] {
        let animation = if animated {
            "@keyframes inline { from,to { background-repeat:repeat-inline; } } \
         @keyframes block { from,to { background:linear-gradient(red, red) left top / 10px 10px repeat-block; } } \
         .horizontal-inline, .vertical-inline { animation:inline 1s linear infinite; } \
         .vertical-block { animation:block 1s linear infinite; }"
        } else {
            ""
        };
        let stylesheet = Stylesheet::parse(&format!("{} {animation}",
        ".horizontal-inline { width:30px; height:20px; background-image:linear-gradient(red, red); background-size:10px 10px; background-repeat:repeat-inline; }\
         .vertical-inline { width:30px; height:20px; writing-mode:vertical-rl; background-image:linear-gradient(red, red); background-size:10px 10px; background-repeat:repeat-inline; }\
         .vertical-block { width:30px; height:20px; writing-mode:vertical-rl; background-image:linear-gradient(red, red); background-size:10px 10px; background-repeat:repeat-block; }\
         .vertical-longhands { width:30px; height:20px; writing-mode:vertical-rl; background-image:linear-gradient(red, red); background-size:10px 10px; background-repeat-inline:no-repeat; background-repeat-block:repeat; }",
    ))
    .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
            for selector in [
                ".horizontal-inline",
                ".vertical-inline",
                ".vertical-block",
                ".vertical-longhands",
            ] {
                capture
                    .lock()
                    .unwrap()
                    .push(ui.add(empty_response.selector(selector)).rect);
            }
        })
        .await;

        let bounds = bounds.lock().unwrap();
        for (rect, repeat_x, repeat_y) in [
            (bounds[0], true, false),
            (bounds[1], false, true),
            (bounds[2], true, false),
            (bounds[3], true, false),
        ] {
            assert_eq!(
                pixel(&pixels, rect.left() + 12.0, rect.top() + 2.0),
                if repeat_x { [255, 0, 0, 255] } else { [255; 4] },
                "horizontal tile at {rect:?}"
            );
            assert_eq!(
                pixel(&pixels, rect.left() + 2.0, rect.top() + 12.0),
                if repeat_y { [255, 0, 0, 255] } else { [255; 4] },
                "vertical tile at {rect:?}"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inherited_logical_background_positions_remap_for_child_writing_mode_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { display:block; width:100px; height:60px; direction:ltr; writing-mode:horizontal-tb; background-position-inline:start 5px; background-position-block:end 3px; background-repeat-inline:no-repeat; background-position-x:right 5px; background-repeat-x:repeat; }\
         .child { display:block; width:60px; height:40px; direction:ltr; writing-mode:vertical-rl; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-repeat:no-repeat; background-repeat-inline:inherit; background-position-inline:inherit; background-position-block:inherit; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    let image = pixel(&pixels, rect.left() + 4.0, rect.top() + 27.0);
    assert!(image[0] > image[2], "inherited logical image was {image:?}");
    assert_eq!(
        pixel(&pixels, rect.left() + 30.0, rect.top() + 6.0),
        [255; 4]
    );
    let repeated = pixel(&pixels, rect.left() + 4.0, rect.top() + 16.0);
    assert!(
        repeated[0] > repeated[2],
        "the parent's cascaded physical repeat must inherit on the child's inline axis: {repeated:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_inheritance_and_shorthand_resets_render_correctly_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { display:block; width:100px; height:60px; background:#0f0 linear-gradient(to right, red, blue) right bottom / 20px 10px no-repeat; }\
         .shorthand-child { display:block; width:40px; height:24px; background:inherit; }\
         .axis-child { display:block; width:40px; height:24px; background-image:linear-gradient(to right, red, blue); background-size:20px 10px; background-position-x:inherit; background-position-y:inherit; background-repeat-x:inherit; background-repeat-y:inherit; }\
         .reset { width:40px; height:20px; background-image:linear-gradient(to right, red 50%, blue 50%); background-size:10px 10px; background-position-x:right; background-repeat-x:no-repeat; background:linear-gradient(to right, red 50%, blue 50%) left top / 10px 10px; }\
         .wide { width:40px; height:20px; background-image:linear-gradient(to right, red 50%, blue 50%); background-size:10px 10px; background-position-x:right; background-repeat-x:no-repeat; background-position-x:initial; background-repeat-x:unset; background-repeat-y:no-repeat; }\
         .cleared { width:40px; height:20px; background-image:linear-gradient(red, red); background:unset; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                for selector in [".shorthand-child", ".axis-child"] {
                    capture
                        .lock()
                        .unwrap()
                        .push(ui.add(empty_response.selector(selector)).rect);
                }
                ui.response()
            })
            .selector(".parent"),
        );
        for selector in [".reset", ".wide", ".cleared"] {
            capture
                .lock()
                .unwrap()
                .push(ui.add(empty_response.selector(selector)).rect);
        }
    })
    .await;

    let bounds = bounds.lock().unwrap();
    for rect in [bounds[0], bounds[1]] {
        assert_eq!(
            pixel(&pixels, rect.left() + 2.0, rect.top() + 2.0),
            [0, 255, 0, 255],
            "inherited right/bottom positioning and no-repeat should leave the start clear"
        );
        let left = pixel(&pixels, rect.left() + 22.0, rect.top() + 15.0);
        let right = pixel(&pixels, rect.left() + 38.0, rect.top() + 15.0);
        assert!(left[0] > left[2], "inherited background image was {left:?}");
        assert!(
            right[2] > right[0],
            "inherited background image was {right:?}"
        );
    }

    let reset = bounds[2];
    assert_eq!(
        pixel(&pixels, reset.left() + 2.0, reset.top() + 2.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, reset.left() + 12.0, reset.top() + 2.0),
        [255, 0, 0, 255],
        "the background shorthand must reset prior position and repeat longhands"
    );
    assert_eq!(
        pixel(&pixels, reset.left() + 7.0, reset.top() + 2.0),
        [0, 0, 255, 255]
    );

    let wide = bounds[3];
    assert_eq!(
        pixel(&pixels, wide.left() + 2.0, wide.top() + 2.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, wide.left() + 12.0, wide.top() + 2.0),
        [255, 0, 0, 255],
        "initial and unset must restore the axis longhands' initial values"
    );
    assert_eq!(
        pixel(&pixels, bounds[4].center().x, bounds[4].center().y),
        [255; 4],
        "background: unset must clear the shorthand's image"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_axis_longhands_update_when_hover_style_changes_test() {
    let stylesheet = Stylesheet::parse(
        ".box { width:60px; height:20px; background-image:linear-gradient(to right, red, blue); background-size:10px 10px; background-position-x:left; background-repeat:no-repeat; }\
         .box:hover { background-position-x:right; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_with_events(
        stylesheet,
        vec![
            Vec::new(),
            vec![egui::Event::PointerMoved(egui::pos2(20.0, 20.0))],
        ],
        move |ui| {
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
        },
    )
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 2.0, rect.top() + 5.0),
        [255; 4]
    );
    let hovered = pixel(&pixels, rect.left() + 52.0, rect.top() + 5.0);
    assert!(
        hovered[0] > hovered[2],
        "hovered background position was {hovered:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_reset_on_hover_replaces_the_previous_color_in_the_same_frame() {
    for with_child in [false, true] {
        let stylesheet = Stylesheet::parse(
            ".box { width:60px; height:30px; background:red; } \
         .box:hover { background:initial; } \
         .child { width:30px; height:15px; background-color:inherit; }",
        )
        .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_with_events(
            stylesheet,
            vec![
                Vec::new(),
                vec![egui::Event::PointerMoved(egui::pos2(50.0, 25.0))],
            ],
            move |ui| {
                if with_child {
                    let child = std::cell::Cell::new(egui::Rect::NOTHING);
                    let parent = ui.add(
                        (|ui: &mut egui::Ui| {
                            child.set(ui.add(empty_response.selector(".child")).rect);
                            ui.interact(
                                egui::Rect::from_min_size(
                                    ui.min_rect().min,
                                    egui::vec2(60.0, 30.0),
                                ),
                                ui.scope_id().with("hover-surface"),
                                egui::Sense::hover(),
                            )
                        })
                        .selector(".box"),
                    );
                    *capture.lock().unwrap() = Some((parent.rect, child.get(), parent.hovered()));
                } else {
                    let parent = ui.add(empty_response.selector(".box"));
                    *capture.lock().unwrap() = Some((parent.rect, parent.rect, parent.hovered()));
                }
            },
        )
        .await;
        let (parent, rect, hovered) = bounds.lock().unwrap().unwrap();
        assert!(hovered, "the pointer must hover the parent, not its child");
        assert_eq!(
            pixel(&pixels, parent.right() - 2.0, parent.bottom() - 2.0),
            [255; 4]
        );
        assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
    }
}

#[test]
fn malformed_background_position_size_repeat_and_origin_values_are_rejected() {
    for declaration in [
        "background-position:left top right",
        "background-size:-1px",
        "background-size:cover auto",
        "background-repeat:repeat no-repeat round",
        "background-origin:text",
    ] {
        assert!(
            Stylesheet::parse(&format!(".box {{ {declaration}; }}")).is_err(),
            "accepted {declaration}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn functional_colors_render_rgb_rgba_hsl_and_hsla_values_test() {
    let stylesheet = Stylesheet::parse(
        ".rgb-comma { background-color: rgb(255, 0, 0); }\
         .rgb-space { background-color: rgb(100% 0% 0%); }\
         .rgba-comma { background-color: rgba(0, 0, 255, 50%); }\
         .hsl-comma { background-color: hsl(120, 100%, 50%); }\
         .hsl-angle { background-color: hsl(0.5turn 100% 50%); }\
         .hsla-space { background-color: hsla(120 100% 50% / 25%); }",
    )
    .unwrap();
    let widget_rects = std::sync::Arc::new(Mutex::new(Vec::new()));
    let rects_from_ui = widget_rects.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut rects = Vec::new();
        for class in [
            ".rgb-comma",
            ".rgb-space",
            ".rgba-comma",
            ".hsl-comma",
            ".hsl-angle",
            ".hsla-space",
        ] {
            let widget = |ui: &mut egui::Ui| {
                ui.allocate_exact_size(egui::vec2(30.0, 18.0), egui::Sense::hover())
                    .1
            };
            let response = ui.add(widget.selector(class));
            rects.push(response.rect);
        }
        *rects_from_ui.lock().unwrap() = rects;
    })
    .await;

    let expected = [
        [255, 0, 0, 255],
        [255, 0, 0, 255],
        [127, 127, 255, 255],
        [0, 255, 0, 255],
        [0, 255, 255, 255],
        [191, 255, 191, 255],
    ];
    let rects = widget_rects.lock().unwrap();
    assert_eq!(rects.len(), expected.len());
    for (rect, expected) in rects.iter().zip(expected) {
        let actual = pixel(&pixels, rect.min.x + 4.0, rect.min.y + 3.0);
        assert!(
            actual
                .into_iter()
                .zip(expected)
                .all(|(actual, expected)| actual.abs_diff(expected) <= 1),
            "pixel at {rect:?} was {actual:?}, expected {expected:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn button_frame_stroke_tracks_the_css_padding_box_test() {
    let stylesheet = Stylesheet::parse(
        ".primary { background-color: #f00; padding: 8px 12px; border-radius: 6px; }",
    )
    .unwrap();
    let button_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = button_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(egui::Button::new("X").selector(".primary"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = button_rect.lock().unwrap().unwrap();
    let pixel_at = |x: f32, y: f32| {
        let x = x.floor() as usize;
        let y = y.floor() as usize;
        pixels[y * WIDTH as usize + x]
    };

    assert_eq!(
        pixel_at(rect.min.x + 0.5, rect.center().y),
        [118, 118, 118, 255]
    );
    assert_eq!(
        pixel_at(rect.min.x + 12.5, rect.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn asymmetric_box_model_is_visible_in_rendered_pixels_test() {
    let stylesheet = Stylesheet::parse(
        ".box { padding: 2px 6px 4px 10px; margin: 4px 3px 5px 7px; background-color: #f00; }",
    )
    .unwrap();
    let content_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = content_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let rect_from_widget = rect_from_ui.clone();
        let content = move |ui: &mut egui::Ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(12.0, 8.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, egui::Color32::GREEN);
            *rect_from_widget.lock().unwrap() = Some(rect);
            response
        };
        ui.add(content.selector(".box"));
    })
    .await;

    let rect = content_rect.lock().unwrap().unwrap();
    let pixel_at = |x: f32, y: f32| {
        let x = x.floor() as usize;
        let y = y.floor() as usize;
        pixels[y * WIDTH as usize + x]
    };
    let red = [255, 0, 0, 255];
    let green = [0, 255, 0, 255];
    let white = [255, 255, 255, 255];

    assert_eq!(pixel_at(rect.center().x, rect.center().y), green);
    assert_eq!(pixel_at(rect.min.x - 5.5, rect.center().y), red);
    assert_eq!(pixel_at(rect.max.x + 2.5, rect.center().y), red);
    assert_eq!(pixel_at(rect.center().x, rect.min.y - 1.5), red);
    assert_eq!(pixel_at(rect.center().x, rect.max.y + 2.5), red);

    assert_eq!(pixel_at(rect.min.x - 10.0 - 3.5, rect.center().y), white);
    assert_eq!(pixel_at(rect.max.x + 6.0 + 1.5, rect.center().y), white);
    assert_eq!(pixel_at(rect.center().x, rect.min.y - 2.0 - 2.5), white);
    assert_eq!(pixel_at(rect.center().x, rect.max.y + 4.0 + 2.5), white);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn percentage_and_auto_sizes_affect_styled_widget_bounds_test() {
    let stylesheet = Stylesheet::parse(
        ".percentage { width: 50%; height: 50%; background-color: #f00; }\
         .automatic { width: auto; height: auto; }\
         .fixed { width: 24px; height: 16px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let bounds_from_ui = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let available_rect = ui.available_rect_before_wrap();
        let inner_rects = std::sync::Arc::new(Mutex::new(Vec::new()));
        let make_box = || {
            let inner_rects = inner_rects.clone();
            move |ui: &mut egui::Ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(12.0, 8.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, egui::Color32::GREEN);
                inner_rects.lock().unwrap().push(rect);
                response
            }
        };

        let percentage = ui.add(make_box().selector(".percentage"));
        let automatic = ui.add(make_box().selector(".automatic"));
        let fixed = ui.add(make_box().selector(".fixed"));
        *bounds_from_ui.lock().unwrap() = Some((
            available_rect,
            percentage.rect,
            automatic.rect,
            fixed.rect,
            inner_rects.lock().unwrap().clone(),
        ));
    })
    .await;

    let (available_rect, percentage, automatic, fixed, inner_rects) =
        bounds.lock().unwrap().take().unwrap();
    assert_eq!(percentage.width(), available_rect.width() * 0.5);
    assert_eq!(percentage.height(), available_rect.height() * 0.5);
    assert_eq!(automatic.size(), egui::vec2(12.0, 8.0));
    assert_eq!(fixed.size(), egui::vec2(24.0, 16.0));

    let pixel_at =
        |x: f32, y: f32| pixels[y.floor() as usize * WIDTH as usize + x.floor() as usize];
    assert_eq!(
        pixel_at(inner_rects[0].center().x, inner_rects[0].center().y),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel_at(percentage.max.x - 4.0, percentage.max.y - 4.0),
        [255, 0, 0, 255]
    );
}
