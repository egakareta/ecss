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
        "rgba(0, 0, 0, none)",
        "hsl(none, 100%, 50%)",
        "hsla(0, 100%, 50%, none)",
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn lab_and_lch_match_css_color4_reference_values() {
    assert_background_colors(&[
        ("lab(51.2345% -13.6271 16.2401)", [106, 128, 93, 255]),
        (
            "lab(51.2345% /* L */ -13.6271 /* a */ 16.2401)",
            [106, 128, 93, 255],
        ),
        ("lab(51.2345 -13.6271 16.2401)", [106, 128, 93, 255]),
        ("lab(51.2345% -10.90168% 12.99208%)", [106, 128, 93, 255]),
        ("lch(51.2345% 21.2 130)", [106, 128, 93, 255]),
        ("lch(51.2345% 14.133333% 130deg)", [106, 128, 93, 255]),
        ("lch(51.2345% 21.2 144.44444grad)", [106, 128, 93, 255]),
        ("lch(51.2345% 21.2 2.268928rad)", [106, 128, 93, 255]),
        ("lch(51.2345% 21.2 .36111111turn)", [106, 128, 93, 255]),
    ])
    .await;
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn oklab_and_oklch_match_css_color4_reference_values() {
    assert_background_colors(&[
        ("oklch(65% 0.15 270)", [108, 136, 234, 255]),
        ("oklab(65% 0 -0.15)", [108, 136, 234, 255]),
        ("oklab(65% 0% -37.5%)", [108, 136, 234, 255]),
        ("oklch(65% 37.5% 270deg)", [108, 136, 234, 255]),
        ("lab(57.9% 11.4 -53.7)", [108, 136, 234, 255]),
        ("color(display-p3 0.445 0.529 0.891)", [108, 136, 234, 255]),
        ("color(rec2020 0.522 0.557 0.892)", [108, 136, 234, 255]),
    ])
    .await;
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn predefined_color_spaces_convert_to_bounded_srgb() {
    assert_background_colors(&[
        ("color(srgb 0.41587 0.503670 0.36664)", [106, 128, 93, 255]),
        (
            "color(display-p3 0.43313 0.50108 0.37950)",
            [106, 128, 93, 255],
        ),
        (
            "color(a98-rgb 0.44091 0.49971 0.37408)",
            [106, 128, 93, 255],
        ),
        (
            "color(prophoto-rgb 0.36589 0.41717 0.31333)",
            [106, 128, 93, 255],
        ),
        (
            "color(rec2020 0.47428 0.52316 0.41425)",
            [106, 128, 93, 255],
        ),
        ("color(srgb 0.691 0.139 0.259)", [176, 35, 66, 255]),
        ("color(srgb-linear 0.435 0.017 0.055)", [176, 35, 66, 255]),
        ("color(display-p3 0.591 0.123 0.264)", [165, 5, 66, 255]),
        (
            "color(display-p3-linear 0.3081 0.014 0.0567)",
            [165, 5, 66, 255],
        ),
        ("color(xyz-d50 0.2005 0.14089 0.4472)", [118, 84, 205, 255]),
        (
            "color(xyz-d65 0.21661 0.14602 0.59452)",
            [118, 84, 205, 255],
        ),
        ("color(xyz 0.21661 0.14602 0.59452)", [118, 84, 205, 255]),
    ])
    .await;
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn color_functions_apply_missing_components_and_host_clipping() {
    assert_background_colors(&[
        ("rgb(none 255 none)", [0, 255, 0, 255]),
        ("hsl(none 100% 50%)", [255, 0, 0, 255]),
        ("hwb(none 0% 0%)", [255, 0, 0, 255]),
        ("lab(none none none)", [0, 0, 0, 255]),
        ("lch(none none none)", [0, 0, 0, 255]),
        ("oklab(none none none)", [0, 0, 0, 255]),
        ("oklch(none none none)", [0, 0, 0, 255]),
        ("color(xyz none none none)", [0, 0, 0, 255]),
        ("color(srgb 4 -2 0.5)", [255, 0, 128, 255]),
        ("lab(-10 0 0)", [0, 0, 0, 255]),
        ("oklab(2 0 0)", [255, 255, 255, 255]),
        ("color(rec2020 0.42053 0.979780 0.00579)", [0, 255, 0, 255]),
        ("lab(0 0 0 / 50%)", [127, 127, 127, 255]),
        ("color(srgb 0 0 0 / 50%)", [127, 127, 127, 255]),
        ("color(srgb 1 0 0 / none)", [255, 255, 255, 255]),
        ("rgb(255 0 0 / none)", [255, 255, 255, 255]),
    ])
    .await;
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn color4_grammars_reject_unknown_spaces_malformed_components_and_non_finite_values() {
    for value in [
        "lab(1, 2, 3)",
        "lch(50% 20 30%)",
        "oklab(1px 0 0)",
        "oklch(50% 20 30%, .5)",
        "color(srgb, 1 0 0)",
        "color(profoto-rgb 0.4 0.5 0.6)",
        "color(srgb 1 0)",
        "color(srgb 1 0 0 /)",
        "color(srgb 1px 0 0)",
        "color(srgb 1 0 0 / 2deg)",
        "lab(1 2 3 / 4deg)",
        "lab(1e999 0 0)",
        "oklab(1e999 0 0)",
        "color(srgb 1e999 0 0)",
        "lch(50% 20 1e999turn)",
        "lab(1+2+3)",
        "oklab(.5+.1+.1)",
        "color(srgb 1+0+0)",
        "rgb(10%20%30%)",
        "lab(1/**/+2/**/+3)",
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn color4_conversions_preserve_neutral_endpoints_and_dark_transfer_curves() {
    assert_background_colors(&[
        ("lab(0 0 0)", [0, 0, 0, 255]),
        ("lab(1 0 0)", [4, 4, 4, 255]),
        ("lab(8 0 0)", [24, 24, 24, 255]),
        ("lab(50 0 0)", [119, 119, 119, 255]),
        ("lab(100 0 0)", [255, 255, 255, 255]),
        ("lch(50% -20 130)", [119, 119, 119, 255]),
        ("oklch(.5 -1 130)", [99, 99, 99, 255]),
        ("oklch(65% .15 -90deg)", [108, 136, 234, 255]),
        ("color(srgb-linear .001 .001 .001)", [3, 3, 3, 255]),
        (
            "color(srgb-linear .0031308 .0031308 .0031308)",
            [10, 10, 10, 255],
        ),
        ("color(prophoto-rgb .01 .01 .01)", [2, 2, 2, 255]),
        ("color(prophoto-rgb .03125 .03125 .03125)", [6, 6, 6, 255]),
        ("color(prophoto-rgb 1 1 1)", [255, 255, 255, 255]),
        ("color(a98-rgb 1 1 1)", [255, 255, 255, 255]),
        ("color(rec2020 1 1 1)", [255, 255, 255, 255]),
        ("color(display-p3 1 1 1)", [255, 255, 255, 255]),
        ("color(XYZ-D50 .9642957 1 .8251046)", [255, 255, 255, 255]),
        ("color(xyz-d65 .9504559 1 1.0890578)", [255, 255, 255, 255]),
        ("color(srgb 100% 0% 0% / 150%)", [255, 0, 0, 255]),
    ])
    .await;
}
