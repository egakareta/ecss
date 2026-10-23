use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn media_styles_update_and_revert_when_the_viewport_resizes() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { width:30px; height:10px; }
         @media (min-width:200px) { .box { width:60px; } }
         @media (min-width:300px) { .box { width:90px; } }
         @media (orientation:portrait) { .box { height:20px; } }
         @media (orientation:landscape) { .box { height:30px; } }",
    )
    .unwrap()
    .install(&context);
    for (index, (viewport_width, expected)) in [
        (300.0, vec2(90.0, 30.0)),
        (200.0, vec2(60.0, 20.0)),
        (199.0, vec2(30.0, 20.0)),
        (500.0, vec2(90.0, 30.0)),
        (100.0, vec2(30.0, 20.0)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut rect = Rect::NOTHING;
        frame(&context, viewport_width, index as f64, Vec::new(), |ui| {
            rect = ui.add(empty_response.selector(".box")).rect;
        });
        assert_eq!(rect.size(), expected, "viewport width {viewport_width}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn universal_media_colors_update_unwrapped_widgets_without_leaking_between_frames() {
    let context = Context::default();
    Stylesheet::parse(
        "* { color:red; }
         @media (min-width:200px) { * { color:lime; } }",
    )
    .unwrap()
    .install(&context);
    for (index, (width, expected)) in [
        (300.0, egui::Color32::GREEN),
        (100.0, egui::Color32::RED),
        (300.0, egui::Color32::GREEN),
    ]
    .into_iter()
    .enumerate()
    {
        let mut color = egui::Color32::TRANSPARENT;
        frame(&context, width, index as f64, Vec::new(), |ui| {
            color = ui.visuals().widgets.inactive.fg_stroke.color;
            let _ = ui.button("Unwrapped");
        });
        assert_eq!(color, expected);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn malformed_media_reports_source_lines_and_does_not_hide_declaration_errors() {
    for query in [
        "(min-width:)",
        "(width:10)",
        "(orientation:diagonal)",
        "(width > 10px) and",
        "(10px < width > 20px)",
        "screen or (width:10px)",
    ] {
        let error = Stylesheet::parse(&format!("\n@media {query} {{ .box {{ width:10px; }} }}"))
            .unwrap_err();
        assert_eq!(error.line(), 2, "{query}");
    }
    let error = Stylesheet::parse(
        "@media print {\n @media (min-width:200px) {\n .box { width:invalid; }\n }\n }",
    )
    .unwrap_err();
    assert_eq!(error.line(), 3);
}
