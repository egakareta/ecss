use super::*;

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
                rects.push(ui.add(empty.selector(".a")).rect);
                rects.push(ui.add(empty.selector(".b")).rect);
                ui.response()
            })
            .selector(".grid"),
        );
        ui.add(
            (|ui: &mut Ui| {
                rects.push(ui.add(empty.selector(".header")).rect);
                rects.push(ui.add(empty.selector(".left")).rect);
                rects.push(ui.add(empty.selector(".right")).rect);
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
                    rects.push(ui.add(empty.selector(".item")).rect);
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
                let first = ui.add(empty.selector(".first")).rect;
                let second = ui.add(empty.selector(".second")).rect;
                let mut nested = Vec::new();
                ui.add(
                    (|ui: &mut Ui| {
                        nested = vec![
                            ui.add(empty.selector(".large")).rect,
                            ui.add(empty.selector(".small")).rect,
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
