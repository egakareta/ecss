use super::*;

async fn assert_background_colors(cases: &[(&str, [u8; 4])]) {
    let mut css = String::from(
        ".palette { display:flex; flex-wrap:wrap; width:280px; } \
         .sample { width:32px; height:24px; }",
    );
    for (index, (value, _)) in cases.iter().enumerate() {
        css.push_str(&format!(".c{index} {{ background-color:{value}; }}"));
    }
    let stylesheet = Stylesheet::parse(&css).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let count = cases.len();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = (0..count)
                    .map(|index| {
                        ui.add(empty_response.selector(format!(".sample.c{index}")))
                            .rect
                    })
                    .collect();
                ui.response()
            })
            .selector(".palette"),
        );
    })
    .await;
    for ((value, expected), rect) in cases.iter().zip(bounds.lock().unwrap().iter()) {
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.center().y),
            *expected,
            "{value}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn hwb_colors_paint_hues_normalized_gray_and_alpha() {
    assert_background_colors(&[
        ("hwb(0 0% 0%)", [255, 0, 0, 255]),
        ("hwb(150 20% 10%)", [51, 230, 140, 255]),
        ("hwb(150 20 10)", [51, 230, 140, 255]),
        ("hwb(45 40% 80%)", [85, 85, 85, 255]),
        ("hwb(45 200% 100%)", [170, 170, 170, 255]),
        ("HWB(-.5TURN 0 0 / 50%)", [127, 255, 255, 255]),
        ("hwb(400grad 0% 0% / 2)", [255, 0, 0, 255]),
        ("hwb(1e37turn 50% 50%)", [128, 128, 128, 255]),
        ("hwb(1000000.25turn 0% 0%)", [128, 255, 0, 255]),
    ])
    .await;
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn modern_rgb_and_hsl_accept_mixed_numeric_components() {
    assert_background_colors(&[
        ("rgb(100% 0 0% / 100%)", [255, 0, 0, 255]),
        ("rgb(0, 255, 0, 1)", [0, 255, 0, 255]),
        ("hsl(240 100 50%)", [0, 0, 255, 255]),
        ("hsl(120deg 100% 50)", [0, 255, 0, 255]),
        ("hsla(-240, 100%, 50%, 1)", [0, 255, 0, 255]),
        ("hsl(130 200 50)", [0, 255, 0, 255]),
        ("hsl(30 200% 75%)", [255, 191, 64, 255]),
    ])
    .await;
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn color_grammars_reject_mixed_legacy_channels_and_separators() {
    for value in [
        "rgb(100%, 0, 0)",
        "rgba(0, 50%, 0, 1)",
        "hsl(0, 100, 50%)",
        "hsl(0, 100%, 50)",
        "hwb(0, 0%, 0%)",
        "hwb(0 0% 0%, .5)",
        "hwb(0 0% 0% / .5 .5)",
        "hwb(0px 0% 0%)",
        "hwb(0 0% / .5)",
        "rgb(0 0 0 /)",
    ] {
        assert!(
            Stylesheet::parse(&format!(".sample {{ color:{value}; }}")).is_err(),
            "{value}"
        );
        assert!(
            InlineStyle::parse(&format!("background-color:{value}")).is_err(),
            "{value}"
        );
    }
}
