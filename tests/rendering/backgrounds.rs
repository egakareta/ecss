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
