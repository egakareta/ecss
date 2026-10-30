use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn resizing_reflows_nested_percentage_and_flex_boxes() {
    let context = Context::default();
    Stylesheet::parse(
        ".row { display:flex; width:100%; gap:10px; } .item { flex:1; min-width:0; height:20px; }",
    )
    .unwrap()
    .install(&context);
    for (i, width) in [200.0, 300.0, 120.0].into_iter().enumerate() {
        let mut rectangles = Vec::new();
        frame(&context, width, i as f64, Vec::new(), |ui| {
            ui.add(
                (|ui: &mut Ui| {
                    rectangles = vec![
                        ui.add(empty_response.selector(".item")).rect,
                        ui.add(empty_response.selector(".item")).rect,
                    ];
                    ui.response()
                })
                .selector(".row"),
            );
        });
        assert!((rectangles[0].width() - (width - 10.0) / 2.0).abs() < 0.05);
        assert!((rectangles[1].right() - width).abs() < 0.05);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn right_to_left_flex_and_logical_margins_resolve_from_computed_direction() {
    let context = Context::default();
    Stylesheet::parse(".row { display:flex; width:100px; direction:rtl; } .first { margin-inline-start:10px; width:20px; height:10px; } .second { width:30px; height:10px; }").unwrap().install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = vec![
                    ui.add(empty_response.selector(".first")).rect,
                    ui.add(empty_response.selector(".second")).rect,
                ];
                ui.response()
            })
            .selector(".row"),
        );
    });
    assert!((rects[0].right() - 90.0).abs() < 0.05);
    assert!((rects[1].right() - rects[0].left()).abs() < 0.05);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn inline_boxes_wrap_and_block_boxes_start_new_lines() {
    let context = Context::default();
    Stylesheet::parse(".flow { width:100px; } .inline { display:inline-block; width:60px; height:20px; } .block { display:block; height:10px; }").unwrap().install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = vec![
                    ui.add(empty_response.selector(".inline")).rect,
                    ui.add(empty_response.selector(".inline")).rect,
                    ui.add(empty_response.selector(".block")).rect,
                    ui.add(empty_response.selector(".inline")).rect,
                ];
                ui.response()
            })
            .selector(".flow"),
        );
    });
    assert_eq!(rects[0].min, pos2(0.0, 0.0));
    assert_eq!(rects[1].min, pos2(0.0, 20.0));
    assert_eq!(rects[2].min, pos2(0.0, 40.0));
    assert_eq!(rects[3].min, pos2(0.0, 50.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn columns_balance_blocks_and_respect_forced_column_breaks() {
    let context = Context::default();
    Stylesheet::parse(".columns { width:210px; column-count:2; column-gap:10px; } .item { height:20px; } .break { height:20px; break-before:column; }").unwrap().install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = vec![
                    ui.add(empty_response.selector(".item")).rect,
                    ui.add(empty_response.selector(".break")).rect,
                    ui.add(empty_response.selector(".item")).rect,
                ];
                ui.response()
            })
            .selector(".columns"),
        );
    });
    assert!((rects[0].left()).abs() < 0.05);
    assert!((rects[1].left() - 110.0).abs() < 0.05, "{rects:?}");
    assert!((rects[1].top()).abs() < 0.05);
    assert!((rects[2].top() - 20.0).abs() < 0.05);
    assert!((rects[2].width() - 100.0).abs() < 0.05);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn vertical_writing_uses_logical_sizes_and_block_progression() {
    for (i, mode) in ["vertical-rl", "vertical-lr"].into_iter().enumerate() {
        let context = Context::default();
        Stylesheet::parse(&format!(".flow {{ writing-mode:{mode}; box-sizing:border-box; width:120px; height:100px; padding-block-start:5px; }} .item {{ inline-size:30px; block-size:20px; }}")).unwrap().install(&context);
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
                .selector(".flow"),
            );
        });
        assert_eq!(rects[0].size(), vec2(20.0, 30.0), "{mode}: {rects:?}");
        assert_eq!(rects[0].left(), [95.0, 5.0][i], "{mode}: {rects:?}");
        assert_eq!(rects[1].left(), [75.0, 25.0][i]);
        assert_eq!(rects[0].top(), 0.0);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn logical_sizes_use_the_final_writing_mode_regardless_of_declaration_order() {
    let context = Context::default();
    Stylesheet::parse(".item { inline-size:30px; block-size:20px; writing-mode:vertical-rl; }")
        .unwrap()
        .install(&context);
    let mut rect = Rect::NOTHING;
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        rect = ui.add(empty_response.selector(".item")).rect;
    });
    assert_eq!(rect.size(), vec2(20.0, 30.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn collapsed_flex_item_keeps_its_line_strut_but_not_main_axis_space() {
    let context = Context::default();
    Stylesheet::parse(
        ".flex { display:flex; width:100px; align-items:flex-start; } .item { width:20px; height:10px; } .collapsed { width:30px; height:40px; visibility:collapse; }",
    )
    .unwrap()
    .install(&context);
    let mut container = Rect::NOTHING;
    let mut items = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        items.clear();
        container = ui
            .add(
                (|ui: &mut Ui| {
                    items.push(ui.add(empty_response.selector(".item")).rect);
                    items.push(ui.add(empty_response.selector(".collapsed")).rect);
                    items.push(ui.add(empty_response.selector(".item")).rect);
                    ui.response()
                })
                .selector(".flex"),
            )
            .rect;
    });
    assert!((container.height() - 40.0).abs() < 0.05, "{container:?}");
    assert_eq!(items[1].width(), 0.0, "{items:?}");
    assert!((items[1].height() - 40.0).abs() < 0.05, "{items:?}");
    assert!(
        (items[2].left() - items[0].right()).abs() < 0.05,
        "{items:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn collapsed_flex_item_uses_its_current_intrinsic_cross_size() {
    let context = Context::default();
    Stylesheet::parse(
        ".flex { display:flex; width:100px; align-items:flex-start; } .item { width:20px; height:10px; } .collapsed { width:30px; visibility:collapse; } .inner { height:40px; }",
    )
    .unwrap()
    .install(&context);
    let mut container = Rect::NOTHING;
    let mut items = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        items.clear();
        container = ui
            .add(
                (|ui: &mut Ui| {
                    items.push(ui.add(empty_response.selector(".item")).rect);
                    items.push(
                        ui.add(
                            (|ui: &mut Ui| {
                                ui.add(empty_response.selector(".inner"));
                                ui.response()
                            })
                            .selector(".collapsed"),
                        )
                        .rect,
                    );
                    items.push(ui.add(empty_response.selector(".item")).rect);
                    ui.response()
                })
                .selector(".flex"),
            )
            .rect;
    });
    assert!((container.height() - 40.0).abs() < 0.05, "{container:?}");
    assert_eq!(items[1].width(), 0.0, "{items:?}");
    assert!((items[1].height() - 40.0).abs() < 0.05, "{items:?}");
    assert!(
        (items[2].left() - items[0].right()).abs() < 0.05,
        "{items:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn margin_trim_removes_selected_first_and_last_child_margins() {
    let context = Context::default();
    Stylesheet::parse(
        ".container { display:flow-root; width:80px; margin-trim:block-end block-start; } .first { height:10px; margin-block-start:8px; } .last { height:10px; margin-block-end:12px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                rects.push(ui.add(empty_response.selector(".first")).rect);
                rects.push(ui.add(empty_response.selector(".last")).rect);
                ui.response()
            })
            .selector(".container"),
        );
    });
    assert_eq!(rects[0].top(), 0.0, "{rects:?}");
    assert_eq!(rects[1].top(), rects[0].bottom(), "{rects:?}");
    assert!(Stylesheet::parse(".container { margin-trim: block invalid; }").is_err());
    for value in [
        "all",
        "in-flow",
        "inline-start",
        "block block-start",
        "block-start block-start",
        "none block-end",
    ] {
        assert!(
            Stylesheet::parse(&format!(".container {{ margin-trim: {value}; }}")).is_err(),
            "accepted invalid margin-trim value: {value}"
        );
    }
    for value in [
        "none",
        "block",
        "block-start",
        "block-end",
        "block-start block-end",
        "BLOCK",
    ] {
        assert!(
            Stylesheet::parse(&format!(".container {{ margin-trim: {value}; }}")).is_ok(),
            "rejected valid margin-trim value: {value}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn margin_trim_does_not_apply_to_flex_containers() {
    let context = Context::default();
    Stylesheet::parse(
        ".container { display:flex; flex-direction:column; width:80px; margin-trim:block-start; } .child { height:10px; margin-block-start:8px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = None;
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        let mut child = None;
        let container = ui.add(
            (|ui: &mut Ui| {
                child = Some(ui.add(empty_response.selector(".child")).rect);
                ui.response()
            })
            .selector(".container"),
        );
        rects = Some((container.rect, child.unwrap()));
    });
    let (container, child) = rects.unwrap();
    assert!(
        (child.top() - container.top() - 8.0).abs() < 0.05,
        "{container:?} {child:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn margin_trim_removes_collapsed_descendant_margins() {
    let context = Context::default();
    Stylesheet::parse(
        ".container { display:flow-root; width:80px; margin-trim:block-start; } .descendant { height:10px; margin-block-start:8px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = None;
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        let mut descendant = None;
        let container = ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        descendant = Some(ui.add(empty_response.selector(".descendant")).rect);
                        ui.response()
                    })
                    .selector(".first"),
                );
                ui.response()
            })
            .selector(".container"),
        );
        rects = Some((container.rect, descendant.unwrap()));
    });
    let (container, descendant) = rects.unwrap();
    assert!(
        (descendant.top() - container.top()).abs() < 0.05,
        "{container:?} {descendant:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn margin_trim_preserves_descendant_margins_separated_by_padding() {
    let context = Context::default();
    Stylesheet::parse(
        ".container { display:flow-root; width:80px; margin-trim:block-start; } .first { padding-block-start:4px; } .descendant { height:10px; margin-block-start:8px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = None;
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        let mut first = None;
        let mut descendant = None;
        ui.add(
            (|ui: &mut Ui| {
                first = Some(
                    ui.add(
                        (|ui: &mut Ui| {
                            descendant = Some(ui.add(empty_response.selector(".descendant")).rect);
                            ui.response()
                        })
                        .selector(".first"),
                    )
                    .rect,
                );
                ui.response()
            })
            .selector(".container"),
        );
        rects = Some((first.unwrap(), descendant.unwrap()));
    });
    let (first, descendant) = rects.unwrap();
    assert!(
        (descendant.top() - first.top() - 12.0).abs() < 0.05,
        "{first:?} {descendant:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn margin_trim_supports_the_inherit_keyword() {
    let context = Context::default();
    Stylesheet::parse(
        ".outer { display:flow-root; width:80px; margin-trim:block-start; } .inner { display:flow-root; margin-trim:inherit; } .descendant { height:10px; margin-block-start:8px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = None;
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        let mut descendant = None;
        let inner = ui.add(
            (|ui: &mut Ui| {
                let inner = ui.add(
                    (|ui: &mut Ui| {
                        descendant = Some(ui.add(empty_response.selector(".descendant")).rect);
                        ui.response()
                    })
                    .selector(".inner"),
                );
                ui.response().union(inner)
            })
            .selector(".outer"),
        );
        rects = Some((inner.rect, descendant.unwrap()));
    });
    let (inner, descendant) = rects.unwrap();
    assert!(
        (descendant.top() - inner.top()).abs() < 0.05,
        "{inner:?} {descendant:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn inline_text_flows_around_a_float() {
    let context = Context::default();
    Stylesheet::parse(
        ".flow { width:100px; height:80px; } .float { float:left; width:40px; height:40px; } .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    let mut text = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        text.clear();
        ui.add(
            (|ui: &mut Ui| {
                rects.push(ui.add(empty_response.selector(".float")).rect);
                for _ in 0..16 {
                    text.push(ui.add(egui::Label::new("X ").selector(".text")).rect);
                }
                ui.response()
            })
            .selector(".flow"),
        );
    });
    assert_eq!(rects[0].left(), 0.0);
    assert!(text[0].left() >= rects[0].right(), "{text:?}");
    assert!(
        text.iter()
            .any(|rect| rect.top() >= rects[0].bottom() && rect.left() < 1.0),
        "{text:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn inline_display_float_is_removed_from_inline_flow() {
    let context = Context::default();
    Stylesheet::parse(
        ".flow { display:flow-root; width:100px; height:80px; } \
         .float { display:inline; float:left; width:40px; height:40px; } \
         .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut float = Rect::NOTHING;
    let mut text = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        text.clear();
        ui.add(
            (|ui: &mut Ui| {
                float = ui.add(empty_response.selector(".float")).rect;
                for _ in 0..12 {
                    text.push(ui.add(egui::Label::new("X ").selector(".text")).rect);
                }
                ui.response()
            })
            .selector(".flow"),
        );
    });
    assert_eq!(float.min, pos2(0.0, 0.0), "{float:?}");
    assert!(text[0].left() >= float.right(), "{float:?} {text:?}");
}
