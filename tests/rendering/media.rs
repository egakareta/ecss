use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn media_queries_paint_using_logical_viewport_dimensions() {
    let queries = [
        ("all", [true, true]),
        ("only screen", [true, true]),
        ("print", [false, false]),
        ("not print", [true, true]),
        ("(width)", [true, true]),
        ("(min-width:256px)", [true, false]),
        ("(max-width:128px)", [false, true]),
        ("(width:256px)", [true, false]),
        ("(width > 128px)", [true, false]),
        ("(width >= 128px)", [true, true]),
        ("(width < 256px)", [false, true]),
        ("(128px < width <= 256px)", [true, false]),
        ("(256px >= width > 128px)", [true, false]),
        ("(256px = width)", [true, false]),
        ("(height:128px)", [true, false]),
        ("(max-height:64px)", [false, true]),
        ("(orientation:landscape)", [true, true]),
        ("(orientation:portrait)", [false, false]),
        ("(aspect-ratio:2/1)", [true, true]),
        ("(min-aspect-ratio:3/1)", [false, false]),
        ("(1/1 < aspect-ratio <= 2/1)", [true, true]),
        ("(min-width:16em)", [true, false]),
        ("(max-width:8rem)", [false, true]),
        ("(width:2in)", [false, false]),
        (
            "screen and (min-width:200px) and (min-height:100px)",
            [true, false],
        ),
        ("not screen and (min-width:200px)", [false, true]),
        ("(max-width:128px), (min-height:128px)", [true, true]),
        ("(max-width:128px) or (min-height:128px)", [true, true]),
        ("not (width > 128px)", [false, true]),
        ("((width > 200px) and (height < 200px))", [true, false]),
        (
            "((width > 200px) or (height < 100px)) and (orientation:landscape)",
            [true, true],
        ),
        ("(unknown-feature: yes)", [false, false]),
        ("unknown-medium", [false, false]),
        (
            "SCREEN and (MIN-WIDTH: /* breakpoint */ 16EM)",
            [true, false],
        ),
    ];
    let mut source = String::from(
        ".grid { display:grid; width:100px; grid-template-columns:repeat(10, 10px); font-size:32px; }
         .probe { width:8px; height:8px; margin:1px; background:red; }",
    );
    for (index, (query, _)) in queries.iter().enumerate() {
        source.push_str(&format!(
            "@media {query} {{ #q{index} {{ background:lime; }} }}"
        ));
    }
    let stylesheet = Stylesheet::parse(&source).unwrap();
    for (scale_index, scale) in [1.0, 2.0].into_iter().enumerate() {
        let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_at_scale(stylesheet.clone(), scale, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    *capture.lock().unwrap() = (0..queries.len())
                        .map(|index| {
                            ui.add(empty_response.selector(format!(".probe#q{index}")))
                                .rect
                        })
                        .collect();
                    ui.response()
                })
                .selector(".grid"),
            );
        })
        .await;
        let bounds = bounds.lock().unwrap();
        for (index, (query, expected)) in queries.iter().enumerate() {
            let center = bounds[index].center() * scale;
            let color = if expected[scale_index] {
                [0, 255, 0, 255]
            } else {
                [255, 0, 0, 255]
            };
            assert_eq!(
                pixel(&pixels, center.x, center.y),
                color,
                "{query} at scale {scale}"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn nested_media_preserves_cascade_variables_states_and_inline_styles() {
    let stylesheet = Stylesheet::parse(
        ".parent { width:100px; --surface:red; }
         .box { width:20px; height:12px; background:var(--surface); }
         .box.specific { background:blue; }
         @media (min-width:200px) {
             .parent { --surface:lime; }
             .box { width:30px; }
             @media (min-height:100px) {
                 .box:enabled { width:40px; }
                 .box { background:var(--surface); }
             }
             @media (orientation:portrait) { .box { background:red; width:80px; } }
         }
         .box { height:16px; }
         @media print { .box { background:red; width:90px; } }",
    )
    .unwrap();
    for (scale, width, color) in [(1.0, 40.0, [0, 255, 0, 255]), (2.0, 20.0, [255, 0, 0, 255])] {
        let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
        let capture = bounds.clone();
        let inline = InlineStyle::parse("width:24px; background:yellow;").unwrap();
        let pixels = render_ui_pixels_at_scale(stylesheet.clone(), scale, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    *capture.lock().unwrap() = vec![
                        ui.add(empty_response.selector(".box")).rect,
                        ui.add(empty_response.selector(".box.specific")).rect,
                        ui.add(empty_response.selector(".box").inline_style(inline.clone()))
                            .rect,
                    ];
                    ui.response()
                })
                .selector(".parent"),
            );
        })
        .await;
        let bounds = bounds.lock().unwrap();
        assert_eq!(bounds[0].size(), egui::vec2(width, 16.0));
        assert_eq!(bounds[2].size(), egui::vec2(24.0, 16.0));
        for (rect, expected) in bounds
            .iter()
            .zip([color, [0, 0, 255, 255], [255, 255, 0, 255]])
        {
            let center = rect.center() * scale;
            assert_eq!(pixel(&pixels, center.x, center.y), expected);
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn media_keyframes_only_override_animations_when_matching() {
    let stylesheet = Stylesheet::parse(
        "@keyframes surface { from { background-color:red; } to { background-color:red; } }
         @media (min-width:200px) {
             @keyframes surface { from { background-color:lime; } to { background-color:lime; } }
         }
         @media print {
             @keyframes surface { from { background-color:blue; } to { background-color:blue; } }
         }
         .box { width:20px; height:20px; animation:surface 1s linear infinite; }",
    )
    .unwrap();
    for (scale, expected) in [(1.0, [0, 255, 0, 255]), (2.0, [255, 0, 0, 255])] {
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_at_scale(stylesheet.clone(), scale, move |ui| {
            *capture.lock().unwrap() = Some(ui.add(egui::Button::new("").selector(".box")).rect);
        })
        .await;
        let center = bounds.lock().unwrap().unwrap().center() * scale;
        assert_eq!(pixel(&pixels, center.x, center.y), expected);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn media_queries_control_pseudo_element_decorations() {
    let stylesheet = Stylesheet::parse(
        ".text { width:40px; font-size:16px; }
         .text::first-letter { border:2px solid red; }
         @media (min-width:200px) { .text::first-letter { border-color:lime; } }
         @media print { .text::first-letter { border-color:blue; } }",
    )
    .unwrap();
    for (scale, expected, absent) in [
        (1.0, [0, 255, 0, 255], [255, 0, 0, 255]),
        (2.0, [255, 0, 0, 255], [0, 255, 0, 255]),
    ] {
        let pixels = render_ui_pixels_at_scale(stylesheet.clone(), scale, |ui| {
            ui.add(egui::Label::new("A").selector(".text"));
        })
        .await;
        assert!(pixels.contains(&expected));
        assert!(!pixels.contains(&absent));
        assert!(!pixels.contains(&[0, 0, 255, 255]));
    }
}
