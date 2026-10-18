use super::*;
use ecss::InlineStyle;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn example_theme_uses_supported_css() {
    let (_, theme) = include_str!("../../examples/theme.css")
        .split_once('\n')
        .unwrap();
    Stylesheet::parse(theme).unwrap();
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn css_wide_values_copy_parent_properties_and_restore_initial_values() {
    let context = Context::default();
    Stylesheet::parse(".parent { box-sizing:border-box; width:180px; padding:10px; } .inherited { box-sizing:inherit; width:80px; padding:inherit; height:40px; } .initial { box-sizing:inherit; box-sizing:initial; padding:10px; width:80px; height:20px; }").unwrap().install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = vec![
                    ui.add(empty_response.selector(".inherited")).rect,
                    ui.add(empty_response.selector(".initial")).rect,
                ];
                ui.response()
            })
            .selector(".parent"),
        );
    });
    assert_eq!(rects[0].size(), vec2(80.0, 40.0));
    assert_eq!(rects[1].size(), vec2(100.0, 40.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn unset_and_inherit_follow_writing_mode_when_mapping_logical_sizes() {
    let context = Context::default();
    Stylesheet::parse(".flow {writing-mode:vertical-rl; inline-size:40px; block-size:70px; white-space:pre;} .unset {writing-mode:unset; inline-size:inherit; block-size:inherit; white-space:unset;} .horizontal {writing-mode:horizontal-tb; inline-size:inherit; block-size:inherit; white-space:initial;}")
        .unwrap()
        .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = vec![
                    ui.add(empty_response.selector(".unset")).rect,
                    ui.add(empty_response.selector(".horizontal")).rect,
                ];
                ui.response()
            })
            .selector(".flow"),
        );
    });
    assert_eq!(rects[0].size(), vec2(70.0, 40.0), "{rects:?}");
    assert_eq!(rects[1].size(), vec2(40.0, 70.0), "{rects:?}");
    assert!(Stylesheet::parse(".box {width:revert-layer;}").is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn custom_properties_inherit_resolve_fallbacks_and_reject_cycles() {
    let context = Context::default();
    Stylesheet::parse(
        ".parent { --base:100px; --base:70px !important; --base:80px; --height:32px; }\
         .child { --derived:var(--base); width:var(--derived); height:var(--height); }\
         .fallback { width:var(--missing, var(--base)); height:var(--height); }\
         .cycle { --first:var(--second); --second:var(--first); width:var(--first, 48px); height:10px; }\
         .case-sensitive { --Size:55px; width:var(--size, 45px); height:10px; }\
         .escaped { --b\\61se:60px; width:v\\61r( --base ); height:10px; }\
         .joined-token { --count:4; width:var(--count)px; height:10px; }\
         .invalid { width:20px; width:var(--missing); height:10px; } .default { height:10px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    let mut inline_rect = Rect::NOTHING;
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = [
                    ".child",
                    ".fallback",
                    ".cycle",
                    ".case-sensitive",
                    ".escaped",
                    ".joined-token",
                    ".invalid",
                    ".default",
                ]
                .into_iter()
                .map(|selector| ui.add(empty_response.selector(selector)).rect)
                .collect();
                ui.response()
            })
            .selector(".parent"),
        );
        inline_rect = ui
            .add(empty_response.inline_style(
                InlineStyle::parse(
                    "--inline-width:38px !important; --inline-width:44px; width:var(--inline-width); height:10px;",
                )
                .unwrap(),
            ))
            .rect;
    });
    assert_eq!(rects[0].size(), vec2(70.0, 32.0));
    assert_eq!(rects[1].size(), vec2(70.0, 32.0));
    assert_eq!(rects[2].size(), vec2(48.0, 10.0));
    assert_eq!(rects[3].size(), vec2(45.0, 10.0));
    assert_eq!(rects[4].size(), vec2(60.0, 10.0));
    assert_eq!(rects[5].size(), rects[6].size());
    assert_eq!(rects[6].size(), rects[7].size());
    assert_eq!(inline_rect.size(), vec2(38.0, 10.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn css_math_defers_percentages_and_validates_dimensions() {
    let context = Context::default();
    Stylesheet::parse(".box { width:clamp(40px, calc(50% - 10px), 140px); height:calc(10px * sin(pi / 2)); margin-left:calc(2px - 5px); }").unwrap().install(&context);
    for (i, width) in [100.0, 200.0, 400.0].into_iter().enumerate() {
        let mut rect = Rect::NOTHING;
        frame(&context, width, i as f64, Vec::new(), |ui| {
            rect = ui.add(empty_response.selector(".box")).rect;
        });
        assert_eq!(rect.width(), [40.0, 90.0, 140.0][i]);
        assert!((rect.height() - 10.0).abs() < 0.01);
        assert_eq!(rect.left(), -3.0);
    }
    for (index, (expression, expected)) in [
        ("round(up, 10px, 6px)", 12.0),
        ("mod(-3px, 5px)", 2.0),
        ("calc(pow(2, 3) * 1px)", 8.0),
        ("clamp(none, 50%, none)", 100.0),
    ]
    .into_iter()
    .enumerate()
    {
        Stylesheet::parse(&format!(".box {{width:{expression};height:10px;}}"))
            .unwrap()
            .install(&context);
        let mut rect = Rect::NOTHING;
        frame(&context, 200.0, index as f64 + 3.0, Vec::new(), |ui| {
            rect = ui.add(empty_response.selector(".box")).rect;
        });
        assert!(
            (rect.width() - expected).abs() < 0.01,
            "{expression}: {rect:?}"
        );
    }
    Stylesheet::parse(".row {display:flex;width:150px;} .double {flex:calc(sqrt(4)) calc(1) calc(0px);height:10px;} .single {flex:calc(1);height:10px;}").unwrap().install(&context);
    let mut widths = Vec::new();
    frame(&context, 200.0, 8.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                widths = vec![
                    ui.add(empty_response.selector(".double")).rect.width(),
                    ui.add(empty_response.selector(".single")).rect.width(),
                ];
                ui.response()
            })
            .selector(".row"),
        );
    });
    assert_eq!(widths, vec![100.0, 50.0]);
    assert!(Stylesheet::parse(".box { width:calc(10px + 2); }").is_err());
    assert!(Stylesheet::parse(".box { width:calc(2px * 2px); }").is_err());
}
