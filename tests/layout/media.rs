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

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn supports_conditions_select_nested_media_and_fallback_branches() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { width:10px; height:10px; }
         @supports (width:60px) {
             .box { width:20px; }
             @media (min-width:200px) { .box { width:40px; } }
         }
         @supports (unknown-property: value) { .box { width:80px; } }
         @supports not (unknown-property: value) { .box { height:20px; } }
         @supports ((display:flex) and (not (unknown-property:value))) {
             .box { height:60px; }
         }
         @supports (unknown-property: value) or (height:10px) {
             .box { height:30px; }
         }
         @supports (width:var(--length)) { .box { height:40px; } }
         @supports (height:10px !important) { .box { height:50px; } }
         @supports (display:unknown()) { .box { width:90px; } }
         @supports (unknown-property:var(--value)) { .box { width:100px; } }",
    )
    .unwrap()
    .install(&context);

    for (index, (width, expected)) in [(100.0, vec2(20.0, 50.0)), (300.0, vec2(40.0, 50.0))]
        .into_iter()
        .enumerate()
    {
        let mut rect = Rect::NOTHING;
        frame(&context, width, index as f64, Vec::new(), |ui| {
            rect = ui.add(empty_response.selector(".box")).rect;
        });
        assert_eq!(rect.size(), expected, "viewport width {width}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn supports_selector_queries_and_unknown_conditions_do_not_match() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { width:10px; height:10px; }
         @supports selector(.box) { .box { width:20px; } }
         @supports (selector(.box) and (display:flex)) { .box { width:25px; } }
         @supports (width:var(--value) unknown()) { .box { width:60px; } }
         @supports selector(div) { .box { width:30px; } }
         @supports future-feature(foo) { .box { width:40px; } }
         @supports (future-feature(foo)) { .box { width:45px; } }
         @supports (future-feature) { .box { width:47px; } }
         @supports not() { .box { width:50px; } }
         @supports not(display:flex) { .box { width:52px; } }
         @supports and() { .box { width:55px; } }
         @supports or() { .box { width:65px; } }
         @supports (--: value) { .box { width:70px; } }
         @supports (width:var(--)) { .box { height:20px; } }
         @supports (--custom: value !bogus) { .box { width:50px; } }
         @supports (width:var(foo)) { .box { height:20px; } }",
    )
    .unwrap()
    .install(&context);

    let mut rect = Rect::NOTHING;
    frame(&context, 100.0, 0.0, Vec::new(), |ui| {
        rect = ui.add(empty_response.selector(".box")).rect;
    });
    assert_eq!(rect.size(), vec2(60.0, 10.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn supports_ignores_unknown_nested_at_rules_and_continues_applying_the_group() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { width:10px; height:10px; }
         @supports (display:flex) {
             @future-rule feature { .box { width:90px; } }
             .box { height:30px; }
         }",
    )
    .unwrap()
    .install(&context);

    let mut rect = Rect::NOTHING;
    frame(&context, 100.0, 0.0, Vec::new(), |ui| {
        rect = ui.add(empty_response.selector(".box")).rect;
    });
    assert_eq!(rect.size(), vec2(10.0, 30.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn malformed_supports_conditions_report_source_lines() {
    for query in [
        "display: flex",
        "(display: flex) and",
        "(display: flex) xor (color:red)",
        "(display: flex; color:red)",
        "not",
        "(display:flex) or (color:red) and (width:1px)",
    ] {
        let error = Stylesheet::parse(&format!("\n@supports {query} {{ .box {{ width:10px; }} }}"))
            .unwrap_err();
        assert_eq!(error.line(), 2, "{query}");
    }

    let error = Stylesheet::parse(
        "@supports (display:flex)\n and\n or (color:red) { .box { width:10px; } }",
    )
    .unwrap_err();
    assert_eq!(error.line(), 3);

    let error =
        Stylesheet::parse("@supports (display:flex) {\n .box { width:invalid; }\n}").unwrap_err();
    assert_eq!(error.line(), 2);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn reserved_custom_property_name_is_a_parse_error() {
    let error = Stylesheet::parse(".box { --: value; }").unwrap_err();
    assert_eq!(error.line(), 1);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn supports_conditions_control_nested_keyframes() {
    let context = Context::default();
    Stylesheet::parse(
        "@supports (animation-name: conditional) {
             @keyframes conditional {
                 from { width:20px; }
                 to { width:40px; }
             }
         }
         @supports (unknown-property: value) {
             @keyframes conditional {
                 from { width:80px; }
                 to { width:80px; }
             }
         }
         .box { width:10px; height:10px; animation:conditional 1s linear infinite; }",
    )
    .unwrap()
    .install(&context);

    for (time, expected) in [(0.0, 20.0), (0.5, 30.0)] {
        let mut rect = Rect::NOTHING;
        frame(&context, 100.0, time, Vec::new(), |ui| {
            rect = ui.add(empty_response.selector(".box")).rect;
        });
        assert_eq!(rect.width(), expected, "animation time {time}");
    }
}
