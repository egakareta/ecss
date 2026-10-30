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
fn object_properties_validate_their_css_grammar() {
    for value in [
        "fill",
        "contain",
        "cover",
        "none",
        "scale-down",
        "contain scale-down",
        "scale-down cover",
        "CoVeR",
        "inherit",
        "initial",
        "unset",
        "revert",
    ] {
        Stylesheet::parse(&format!(".image {{object-fit:{value}}}")).unwrap();
    }
    for value in [
        "auto",
        "contain cover",
        "scale-down none",
        "cover cover",
        "fill scale-down",
        "10px",
        "none, fill",
    ] {
        assert!(
            Stylesheet::parse(&format!(".image {{object-fit:{value}}}")).is_err(),
            "{value}"
        );
    }
    for value in [
        "left",
        "top",
        "center",
        "25%",
        "0",
        "bottom right",
        "center left",
        "center center",
        "left 10px",
        "10px top",
        "right 10px bottom 20px",
        "top -5% left calc(10% + 1em)",
        "calc(100% - 3px) -10px",
        "inherit",
        "initial",
        "unset",
        "revert",
    ] {
        Stylesheet::parse(&format!(".image {{object-position:{value}}}")).unwrap();
    }
    for value in [
        "auto",
        "left right",
        "top bottom",
        "top 10px",
        "10px left",
        "left 10px top",
        "center 5px top 10px",
        "left 1px right 2px",
        "1px 2px 3px 4px",
        "left, top",
        "1fr 2px",
        "1deg 2px",
        "left top extra",
    ] {
        assert!(
            Stylesheet::parse(&format!(".image {{object-position:{value}}}")).is_err(),
            "{value}"
        );
    }
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
fn all_property_inherits_resets_and_unsets_supported_declarations() {
    let context = Context::default();
    Stylesheet::parse(
        ".parent { --size:56px; box-sizing:border-box; width:120px; height:50px; padding:8px; }\
         .inherited { all:inherit; }\
         .initial { width:80px; height:30px; padding:6px; all:initial; width:40px; height:20px; }\
         .unset { all:unset; width:50px; height:15px; }\
         .custom { all:initial; width:var(--size); height:10px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = [".inherited", ".initial", ".unset", ".custom"]
                    .into_iter()
                    .map(|selector| ui.add(empty_response.selector(selector)).rect)
                    .collect();
                ui.response()
            })
            .selector(".parent"),
        );
    });

    assert_eq!(rects[0].size(), vec2(120.0, 50.0));
    assert_eq!(rects[1].size(), vec2(40.0, 20.0));
    assert_eq!(rects[2].size(), vec2(50.0, 15.0));
    assert_eq!(rects[3].size(), vec2(56.0, 10.0));
    assert!(Stylesheet::parse(".invalid { all: red; }").is_err());
    assert!(Stylesheet::parse(".invalid { all: revert-layer; }").is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn all_revert_restores_inherited_values_and_discards_author_overrides() {
    let context = Context::default();
    Stylesheet::parse(
        ".parent { color:red; font-size:32px; width:260px; }\
         .reverted { color:blue; font-size:12px; width:90px; all:revert; }",
    )
    .unwrap()
    .install(&context);
    let mut observed = None;
    let mut width = None;
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                let reverted = ui.add(
                    (|ui: &mut Ui| {
                        observed = Some((
                            ui.visuals().widgets.inactive.fg_stroke.color,
                            ui.style().text_styles[&egui::TextStyle::Body].size,
                        ));
                        ui.label("reverted")
                    })
                    .selector(".reverted"),
                );
                width = Some(reverted.rect.width());
                ui.response()
            })
            .selector(".parent"),
        );
    });

    assert_eq!(observed, Some((egui::Color32::RED, 32.0)));
    assert_eq!(width, Some(260.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn important_declarations_beat_normal_specificity_and_inline_styles() {
    let context = Context::default();
    Stylesheet::parse(
        ".same { width:40px; height:10px; }\
         .same { width:80px !important; }\
         .specific { width:40px !important; height:10px; }\
         .specific.extra { width:80px; }\
         .inline { width:40px !important; height:10px; }\
         .inline-important { width:40px !important; height:10px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        rects.push(ui.add(empty_response.selector(".same")).rect);
        rects.push(ui.add(empty_response.selector(".specific.extra")).rect);
        rects.push(
            ui.add(
                empty_response
                    .selector(".inline")
                    .inline_style(InlineStyle::parse("width:90px; height:10px;").unwrap()),
            )
            .rect,
        );
        rects.push(
            ui.add(
                empty_response.selector(".inline-important").inline_style(
                    InlineStyle::parse("width:90px !important; height:10px;").unwrap(),
                ),
            )
            .rect,
        );
    });

    assert_eq!(rects[0].width(), 80.0);
    assert_eq!(rects[1].width(), 40.0);
    assert_eq!(rects[2].width(), 40.0);
    assert_eq!(rects[3].width(), 90.0);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn important_shorthands_reset_longhands_at_their_cascade_priority() {
    let context = Context::default();
    Stylesheet::parse(
        ".shorthand { width:100px; height:10px; padding:20px !important; padding-left:4px; }\
         .longhand { width:100px; height:10px; padding:20px; padding-left:4px !important; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        rects.push(ui.add(empty_response.selector(".shorthand")).rect);
        rects.push(ui.add(empty_response.selector(".longhand")).rect);
    });

    assert_eq!(rects[0].size(), vec2(140.0, 50.0));
    assert_eq!(rects[1].size(), vec2(124.0, 50.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn revert_on_inherited_properties_uses_the_parent_computed_value() {
    let context = Context::default();
    Stylesheet::parse(
        ".parent { color:red; font-size:32px; }\
         .child { color:blue; color:revert; font-size:12px; font-size:revert; width:40px; height:10px; }",
    )
    .unwrap()
    .install(&context);
    let mut observed = None;
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        observed = Some((
                            ui.visuals().widgets.inactive.fg_stroke.color,
                            ui.style().text_styles[&egui::TextStyle::Body].size,
                        ));
                        ui.response()
                    })
                    .selector(".child"),
                );
                ui.response()
            })
            .selector(".parent"),
        );
    });

    assert_eq!(observed, Some((egui::Color32::RED, 32.0)));
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
