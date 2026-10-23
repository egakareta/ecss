use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn intrinsic_widths_respect_white_space_and_emergency_breaks() {
    let context = Context::default();
    Stylesheet::parse(
        ".min { width:min-content; } .max { width:max-content; } \
         .nowrap { white-space:nowrap; } .pre { white-space:pre; } \
         .emergency { overflow-wrap:break-word; } .anywhere { overflow-wrap:anywhere; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.set_clip_rect(Rect::EVERYTHING);
        rects.clear();
        for selector in [
            ".max",
            ".min.nowrap",
            ".min.pre",
            ".min.emergency",
            ".min.anywhere",
        ] {
            rects.push(
                ui.add(egui::Label::new("longestword short").selector(selector))
                    .rect,
            );
        }
        rects.push(
            ui.add(egui::Label::new("longestword").selector(".max"))
                .rect,
        );
    });
    for index in [1, 2] {
        assert!(
            (rects[index].width() - rects[0].width()).abs() < 0.5,
            "{rects:?}"
        );
    }
    assert!(
        (rects[3].width() - rects[5].width()).abs() < 0.5,
        "{rects:?}"
    );
    assert!(rects[4].width() < rects[3].width() / 2.0, "{rects:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn flex_automatic_minimum_distinguishes_scrollable_and_clipped_overflow() {
    for (overflow, minimum, shrinks) in [
        ("visible", "auto", false),
        ("clip", "auto", false),
        ("hidden", "auto", true),
        ("auto", "auto", true),
        ("scroll", "auto", true),
        ("visible", "0", true),
    ] {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".row {{ display:flex; width:40px; }} \
             .item {{ flex:1 1 auto; min-width:{minimum}; white-space:nowrap; overflow:{overflow}; }} \
             .reference {{ width:max-content; }}",
        ))
        .unwrap()
        .install(&context);
        let mut item = Rect::NOTHING;
        let mut reference = Rect::NOTHING;
        frame(&context, 300.0, 0.0, Vec::new(), |ui| {
            reference = ui
                .add(egui::Label::new("longestword short").selector(".reference"))
                .rect;
            ui.add(
                (|ui: &mut Ui| {
                    item = ui
                        .add(egui::Label::new("longestword short").selector(".item"))
                        .rect;
                    ui.response()
                })
                .selector(".row"),
            );
        });
        let expected = if shrinks { 40.0 } else { reference.width() };
        assert!(
            (item.width() - expected).abs() < 0.5,
            "{overflow}: {item:?}, {reference:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn inline_emergency_breaks_do_not_reduce_intrinsic_minimums() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { width:min-content; overflow-wrap:break-word; } \
         .text { display:inline; } .reference { width:max-content; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = [Rect::NOTHING; 2];
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        rects[0] = ui
            .add(egui::Label::new("longestword").selector(".reference"))
            .rect;
        rects[1] = ui
            .add(
                (|ui: &mut Ui| {
                    ui.add(egui::Label::new("longestword short").selector(".text"));
                    ui.response()
                })
                .selector(".box"),
            )
            .rect;
    });
    assert!(
        (rects[1].width() - rects[0].width()).abs() < 0.5,
        "{rects:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn emergency_wrapping_still_breaks_words_in_a_definite_narrow_box() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { width:30px; overflow-wrap:break-word; } .reference { width:max-content; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = [Rect::NOTHING; 2];
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        rects[0] = ui
            .add(egui::Label::new("longestword").selector(".reference"))
            .rect;
        rects[1] = ui
            .add(egui::Label::new("longestword").selector(".box"))
            .rect;
    });
    assert!((rects[1].width() - 30.0).abs() < 0.5, "{rects:?}");
    assert!(rects[1].height() > rects[0].height() * 2.0, "{rects:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn inline_percentage_widths_do_not_resolve_against_intrinsic_available_space() {
    for (width, expected) in [
        ("100px", 50.0),
        ("min-content", 20.0),
        ("max-content", 20.0),
    ] {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".box {{ width:{width}; }} .item {{ display:inline-block; width:50%; min-width:20px; height:10px; }}",
        ))
        .unwrap()
        .install(&context);
        let mut item = Rect::NOTHING;
        frame(&context, 300.0, 0.0, Vec::new(), |ui| {
            ui.add(
                (|ui: &mut Ui| {
                    item = ui.add(empty_response.selector(".item")).rect;
                    ui.response()
                })
                .selector(".box"),
            );
        });
        assert!((item.width() - expected).abs() < 0.5, "{width}: {item:?}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn flex_and_grid_measure_wrapped_text_inside_the_content_box() {
    for (display, box_sizing) in [
        ("flex", "content-box"),
        ("flex", "border-box"),
        ("grid", "content-box"),
        ("grid", "border-box"),
    ] {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".container {{ display:{display}; width:100px; grid-template-columns:100px; align-items:start; }} \
             .item {{ box-sizing:{box_sizing}; flex:1; min-width:0; padding:10px; border:2px solid; }} \
             .reference {{ width:76px; }}",
        ))
        .unwrap()
        .install(&context);
        let mut rects = [Rect::NOTHING; 2];
        frame(&context, 300.0, 0.0, Vec::new(), |ui| {
            rects[0] = ui
                .add(egui::Label::new("one two three four five six").selector(".reference"))
                .rect;
            ui.add(
                (|ui: &mut Ui| {
                    rects[1] = ui
                        .add(egui::Label::new("one two three four five six").selector(".item"))
                        .rect;
                    ui.response()
                })
                .selector(".container"),
            );
        });
        assert!(
            (rects[1].width() - 100.0).abs() < 0.5,
            "{display}: {rects:?}"
        );
        assert!(
            (rects[1].height() - rects[0].height() - 24.0).abs() < 0.5,
            "{display}: {rects:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn inline_percentage_heights_require_a_definite_containing_height() {
    for (height, limits, expected) in [
        ("auto", "", 10.0),
        ("60px", "", 30.0),
        ("50%", "", 30.0),
        ("60px", "max-height:40px;", 20.0),
        ("auto", "min-height:60px;", 10.0),
    ] {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".outer {{ width:100px; height:120px; }} .inner {{ height:{height}; {limits} }} \
             .item {{ display:inline-block; width:20px; height:50%; min-height:10px; }}",
        ))
        .unwrap()
        .install(&context);
        let mut item = Rect::NOTHING;
        frame(&context, 300.0, 0.0, Vec::new(), |ui| {
            ui.add(
                (|ui: &mut Ui| {
                    ui.add(
                        (|ui: &mut Ui| {
                            item = ui.add(empty_response.selector(".item")).rect;
                            ui.response()
                        })
                        .selector(".inner"),
                    );
                    ui.response()
                })
                .selector(".outer"),
            );
        });
        assert!((item.height() - expected).abs() < 0.5, "{height}: {item:?}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn grid_intrinsic_tracks_use_text_contributions_instead_of_emergency_breaks() {
    let context = Context::default();
    Stylesheet::parse(
        ".grid { display:grid; grid-template-columns:min-content max-content; gap:10px; } \
         .item { overflow-wrap:break-word; } .reference { width:max-content; }",
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
                for _ in 0..2 {
                    rects.push(
                        ui.add(egui::Label::new("longestword short").selector(".item"))
                            .rect,
                    );
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
