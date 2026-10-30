use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn template_flow_places_a_widget_in_its_named_slot() {
    let context = Context::default();
    Stylesheet::parse(
        ".grid { width:100px; height:40px; grid:20px * \"main side\" 40px; } .item { flow:side; }",
    )
    .unwrap()
    .install(&context);
    let mut rect = Rect::NOTHING;
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rect = ui.add(empty_response.selector(".item")).rect;
                ui.response()
            })
            .selector(".grid"),
        );
    });
    assert_eq!(rect.left(), 20.0, "{rect:?}");
    assert_eq!(rect.size(), vec2(80.0, 40.0), "{rect:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn chains_continue_each_flow_in_the_next_slot_when_the_current_slot_is_full() {
    let context = Context::default();
    Stylesheet::parse(
        ".grid { display:grid; width:200px; height:50px; grid-template-columns:50px 50px 50px 50px; grid-template-rows:50px; grid-template-areas:\"* a b c\"; chains:missing * a, missing-third b c; } .first-item, .second-item { width:50px; height:30px; } .third-item, .fourth-item { width:50px; height:30px; flow:b; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = vec![
                    ui.add(empty_response.selector(".first-item")).rect,
                    ui.add(empty_response.selector(".second-item")).rect,
                    ui.add(empty_response.selector(".third-item")).rect,
                    ui.add(empty_response.selector(".fourth-item")).rect,
                ];
                ui.response()
            })
            .selector(".grid"),
        );
    });
    assert_eq!(rects[0].left(), 0.0, "{rects:?}");
    assert_eq!(rects[1].left(), 50.0, "{rects:?}");
    assert_eq!(rects[0].top(), rects[1].top(), "{rects:?}");
    assert_eq!(rects[2].left(), 100.0, "{rects:?}");
    assert_eq!(rects[3].left(), 150.0, "{rects:?}");
    assert_eq!(rects[2].top(), rects[3].top(), "{rects:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn a_chain_is_ignored_when_a_nonfinal_slot_depends_on_content_size() {
    let context = Context::default();
    Stylesheet::parse(
        ".grid { display:grid; width:100px; height:50px; grid-template-columns:50px 50px; grid-template-rows:auto; grid-template-areas:\"first second\"; chains:first second; } .item { width:50px; height:20px; flow:first; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = vec![
                    ui.add(empty_response.selector(".item")).rect,
                    ui.add(empty_response.selector(".item")).rect,
                ];
                ui.response()
            })
            .selector(".grid"),
        );
    });
    assert_eq!(rects[0].left(), rects[1].left(), "{rects:?}");
    assert!(rects[1].top() >= rects[0].bottom(), "{rects:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn a_chain_is_ignored_when_a_nonfinal_slot_uses_percentage_tracks() {
    let context = Context::default();
    Stylesheet::parse(
        ".grid { display:grid; width:100px; height:50px; grid-template-columns:50% 50%; grid-template-rows:50px; grid-template-areas:\"first second\"; chains:first second; } .item { width:40px; height:20px; flow:first; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = vec![
                    ui.add(empty_response.selector(".item")).rect,
                    ui.add(empty_response.selector(".item")).rect,
                ];
                ui.response()
            })
            .selector(".grid"),
        );
    });
    assert_eq!(rects[0].left(), rects[1].left(), "{rects:?}");
    assert!(rects[1].top() >= rects[0].bottom(), "{rects:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn forced_breaks_advance_to_the_next_chained_slot() {
    let context = Context::default();
    Stylesheet::parse(
        ".grid { display:grid; width:90px; height:50px; grid-template-columns:30px 30px 30px; grid-template-rows:50px; grid-template-areas:\"first second third\"; chains:first second third; } .item { width:30px; height:10px; flow:first; } .before { width:30px; height:10px; flow:first; break-before:column; } .after { width:30px; height:10px; flow:first; break-after:column; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = vec![
                    ui.add(empty_response.selector(".item")).rect,
                    ui.add(empty_response.selector(".before")).rect,
                    ui.add(empty_response.selector(".after")).rect,
                    ui.add(empty_response.selector(".item")).rect,
                ];
                ui.response()
            })
            .selector(".grid"),
        );
    });
    assert_eq!(rects[0].left(), 0.0, "{rects:?}");
    assert_eq!(rects[1].left(), 30.0, "{rects:?}");
    assert_eq!(rects[1].top(), 0.0, "{rects:?}");
    assert_eq!(rects[2].left(), 30.0, "{rects:?}");
    assert_eq!(rects[2].top(), 10.0, "{rects:?}");
    assert_eq!(rects[3].left(), 60.0, "{rects:?}");
    assert_eq!(rects[3].top(), 0.0, "{rects:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn grid_template_shorthand_builds_tracks_and_named_areas() {
    let context = Context::default();
    Stylesheet::parse(
        ".grid { display:grid; width:100px; height:40px; grid-template:20px 20px / 30px 70px; } .a { grid-row:1; grid-column:1; } .b { grid-row:2; grid-column:2; } .areas { display:grid; width:100px; height:40px; grid-template:\"head head\" 10px \"left right\" 30px / 40px 60px; } .header { grid-area:head; } .left { grid-area:left; } .right { grid-area:right; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                rects.push(ui.add(empty_response.selector(".a")).rect);
                rects.push(ui.add(empty_response.selector(".b")).rect);
                ui.response()
            })
            .selector(".grid"),
        );
        ui.add(
            (|ui: &mut Ui| {
                rects.push(ui.add(empty_response.selector(".header")).rect);
                rects.push(ui.add(empty_response.selector(".left")).rect);
                rects.push(ui.add(empty_response.selector(".right")).rect);
                ui.response()
            })
            .selector(".areas"),
        );
    });
    assert_eq!(rects[0].size(), vec2(30.0, 20.0), "{rects:?}");
    assert_eq!(rects[1].size(), vec2(70.0, 20.0), "{rects:?}");
    assert_eq!(rects[1].left() - rects[0].left(), 30.0);
    assert_eq!(rects[1].top() - rects[0].top(), 20.0);
    assert_eq!(rects[2].size(), vec2(100.0, 10.0), "{rects:?}");
    assert_eq!(rects[3].size(), vec2(40.0, 30.0), "{rects:?}");
    assert_eq!(rects[4].size(), vec2(60.0, 30.0), "{rects:?}");
    assert_eq!(rects[4].left() - rects[3].left(), 40.0);
    for value in ["10px", "20px /", "/ 20px", "10px / 20px / 30px"] {
        assert!(
            Stylesheet::parse(&format!(".grid {{ grid-template:{value}; }}")).is_err(),
            "accepted invalid grid-template: {value}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn grid_shorthand_sets_auto_flow_and_implicit_tracks() {
    let context = Context::default();
    Stylesheet::parse(
        ".grid { display:grid; width:80px; height:30px; grid:auto-flow 10px / 40px 40px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                for _ in 0..3 {
                    rects.push(ui.add(empty_response.selector(".item")).rect);
                }
                ui.response()
            })
            .selector(".grid"),
        );
    });
    assert_eq!(rects[0].size(), vec2(40.0, 10.0), "{rects:?}");
    assert_eq!(rects[1].size(), vec2(40.0, 10.0), "{rects:?}");
    assert_eq!(rects[2].size(), vec2(40.0, 10.0), "{rects:?}");
    assert_eq!(rects[1].left() - rects[0].left(), 40.0);
    assert_eq!(rects[2].top() - rects[0].top(), 10.0);
    for value in ["10px", "10px /", "/ auto-flow", "auto-flow / auto-flow"] {
        assert!(
            Stylesheet::parse(&format!(".grid {{ grid:{value}; }}")).is_err(),
            "accepted invalid grid shorthand: {value}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn subgrid_uses_parent_track_sizes_and_intrinsic_child_contributions() {
    let context = Context::default();
    Stylesheet::parse(".grid { display:grid; width:200px; grid-template-columns:auto auto; grid-template-rows:20px 30px; column-gap:10px; } .sub { display:grid; grid-column:1 / -1; grid-row:2; grid-template-columns:subgrid; } .large { min-width:80px; } .small { min-width:20px; } .first { grid-row:1; grid-column:1; } .second { grid-row:1; grid-column:2; }").unwrap().install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                let first = ui.add(empty_response.selector(".first")).rect;
                let second = ui.add(empty_response.selector(".second")).rect;
                let mut nested = Vec::new();
                ui.add(
                    (|ui: &mut Ui| {
                        nested = vec![
                            ui.add(empty_response.selector(".large")).rect,
                            ui.add(empty_response.selector(".small")).rect,
                        ];
                        ui.response()
                    })
                    .selector(".sub"),
                );
                rects = vec![first, second, nested[0], nested[1]];
                ui.response()
            })
            .selector(".grid"),
        );
    });
    assert!((rects[0].width() - 125.0).abs() < 0.05, "{rects:?}");
    assert!((rects[1].width() - 65.0).abs() < 0.05, "{rects:?}");
    assert_eq!(rects[2].left(), rects[0].left());
    assert_eq!(rects[3].left(), rects[1].left());
    assert_eq!(rects[2].width(), rects[0].width());
    assert_eq!(rects[3].width(), rects[1].width());
    assert_eq!(rects[2].top(), 20.0);
    assert_eq!(rects[2].height(), 30.0);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn nested_flex_grid_items_contribute_intrinsic_track_sizes() {
    let context = Context::default();
    Stylesheet::parse(
        ".grid { display:grid; grid-template-columns:min-content max-content; gap:10px; } \
         .item { display:flex; } .reference { width:max-content; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        rects.push(
            ui.add(egui::Label::new("longestword").selector(".reference"))
                .rect,
        );
        rects.push(
            ui.add(egui::Label::new("longestword short").selector(".reference"))
                .rect,
        );
        ui.add(
            (|ui: &mut Ui| {
                for text in ["longestword", "longestword short"] {
                    let item = ui.add(
                        (|ui: &mut Ui| {
                            ui.add(egui::Label::new(text));
                            ui.response()
                        })
                        .selector(".item"),
                    );
                    rects.push(item.rect);
                }
                ui.response()
            })
            .selector(".grid"),
        );
    });
    assert!(
        (rects[2].width() - rects[0].width()).abs() < 0.5,
        "{rects:?}"
    );
    assert!(
        (rects[3].width() - rects[1].width()).abs() < 0.5,
        "{rects:?}"
    );
    assert!(
        (rects[3].left() - rects[2].right() - 10.0).abs() < 0.5,
        "{rects:?}"
    );
}
