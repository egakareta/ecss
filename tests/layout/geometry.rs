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
    assert_eq!(float.size(), vec2(40.0, 40.0), "{float:?}");
    assert!(text[0].left() >= float.right(), "{float:?} {text:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn floated_inline_container_types_keep_their_used_box_size() {
    let context = Context::default();
    Stylesheet::parse(
        ".flow { display:flow-root; width:100px; height:50px; } \
         .inline-block { display:inline-block; float:left; width:40px; height:40px; } \
         .flex { display:inline-flex; float:left; width:40px; height:40px; } \
         .grid { display:inline-grid; float:left; width:40px; height:40px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        for selector in [".inline-block", ".flex", ".grid"] {
            let mut rect = Rect::NOTHING;
            ui.add(
                (|ui: &mut Ui| {
                    rect = ui.add(empty_response.selector(selector)).rect;
                    ui.response()
                })
                .selector(".flow"),
            );
            rects.push(rect);
        }
    });
    for rect in rects {
        assert_eq!(rect.size(), vec2(40.0, 40.0), "{rect:?}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn auto_width_inline_floats_shrink_to_fit_the_containing_block() {
    for display in ["inline", "inline-block"] {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".flow {{ display:flow-root; width:100px; }} \
             .float {{ display:{display}; float:left; }} \
             .reference {{ width:max-content; }}"
        ))
        .unwrap()
        .install(&context);
        let mut reference = Rect::NOTHING;
        let mut floated = Rect::NOTHING;
        frame(&context, 200.0, 0.0, vec![], |ui| {
            reference = ui
                .add(egui::Label::new("longestword short").selector(".reference"))
                .rect;
            ui.add(
                (|ui: &mut Ui| {
                    floated = ui
                        .add(egui::Label::new("longestword short").selector(".float"))
                        .rect;
                    ui.response()
                })
                .selector(".flow"),
            );
        });
        assert!(reference.width() > 100.0, "{reference:?}");
        assert!(
            (floated.width() - 100.0).abs() < 0.5,
            "{display}: {floated:?}"
        );
        assert!(
            floated.height() > reference.height(),
            "{display}: {floated:?} {reference:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn inline_floats_contribute_to_intrinsic_container_widths() {
    for side in ["left", "right"] {
        for sizing in ["min-content", "max-content"] {
            let context = Context::default();
            Stylesheet::parse(&format!(
                ".flow {{ display:flow-root; width:{sizing}; }} \
                 .float {{ display:inline; float:{side}; width:40px; height:40px; }} \
                 .text {{ display:inline; }} .reference {{ width:max-content; }}"
            ))
            .unwrap()
            .install(&context);
            let mut reference = Rect::NOTHING;
            let mut flow = Rect::NOTHING;
            frame(&context, 200.0, 0.0, vec![], |ui| {
                reference = ui.add(egui::Label::new("X").selector(".reference")).rect;
                flow = ui
                    .add(
                        (|ui: &mut Ui| {
                            ui.add(empty_response.selector(".float"));
                            ui.add(egui::Label::new("X").selector(".text"));
                            ui.response()
                        })
                        .selector(".flow"),
                    )
                    .rect;
            });
            let expected = if sizing == "max-content" {
                40.0 + reference.width()
            } else {
                40.0
            };
            assert!(
                (flow.width() - expected).abs() < 0.5,
                "{side} {sizing}: {flow:?}, expected {expected}"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn inline_content_moves_below_a_float_when_no_item_fits() {
    let context = Context::default();
    Stylesheet::parse(
        ".flow { display:flow-root; width:100px; } \
         .float { float:left; width:95px; height:40px; } \
         .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut floated = Rect::NOTHING;
    let mut text = Rect::NOTHING;
    frame(&context, 200.0, 0.0, vec![], |ui| {
        ui.add(
            (|ui: &mut Ui| {
                floated = ui.add(empty_response.selector(".float")).rect;
                text = ui.add(egui::Label::new("X").selector(".text")).rect;
                ui.response()
            })
            .selector(".flow"),
        );
    });
    assert!(text.top() >= floated.bottom(), "{text:?} {floated:?}");
    assert!(
        (text.left() - floated.left()).abs() < 0.5,
        "{text:?} {floated:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn vertically_separated_floats_do_not_sum_their_intrinsic_widths() {
    let context = Context::default();
    Stylesheet::parse(
        ".flow { display:flow-root; width:max-content; white-space:pre; } \
         .float { float:left; width:40px; height:1px; } \
         .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut flow = Rect::NOTHING;
    let mut floated = [Rect::NOTHING; 2];
    frame(&context, 200.0, 0.0, vec![], |ui| {
        flow = ui
            .add(
                (|ui: &mut Ui| {
                    floated[0] = ui.add(empty_response.selector(".float")).rect;
                    ui.add(egui::Label::new("\n").selector(".text"));
                    floated[1] = ui.add(empty_response.selector(".float")).rect;
                    ui.response()
                })
                .selector(".flow"),
            )
            .rect;
    });
    assert!(floated[1].top() >= floated[0].bottom(), "{floated:?}");
    assert!((flow.width() - 40.0).abs() < 0.5, "{flow:?} {floated:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn floating_table_parts_compute_to_block_display() {
    let render = |display| {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".table {{ display:table; }} \
             .float {{ display:{display}; float:left; width:30px; height:20px; }} \
             .row {{ display:table-row; }} .cell {{ display:table-cell; width:40px; height:10px; }}"
        ))
        .unwrap()
        .install(&context);
        let mut bounds = [Rect::NOTHING; 3];
        frame(&context, 200.0, 0.0, vec![], |ui| {
            bounds[0] = ui
                .add(
                    (|ui: &mut Ui| {
                        bounds[1] = ui.add(empty_response.selector(".float")).rect;
                        bounds[2] = ui
                            .add(
                                (|ui: &mut Ui| {
                                    ui.add(empty_response.selector(".cell"));
                                    ui.response()
                                })
                                .selector(".row"),
                            )
                            .rect;
                        ui.response()
                    })
                    .selector(".table"),
                )
                .rect;
        });
        bounds
    };
    let reference = render("block");
    assert_eq!(reference[1].size(), vec2(30.0, 20.0));
    for display in [
        "table-row",
        "table-cell",
        "table-row-group",
        "table-header-group",
        "table-footer-group",
        "table-column",
        "table-column-group",
        "table-caption",
    ] {
        assert_eq!(render(display), reference, "{display}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn floats_preserve_inline_text_flow_and_clear_left_and_both() {
    let context = Context::default();
    Stylesheet::parse(
        ".flow { display:flow-root; width:100px; height:100px; } \
         .float { display:inline; float:left; width:40px; height:40px; } \
         .text { display:inline; } \
         .left-float { float:left; width:25px; height:20px; } \
         .right-float { float:right; width:25px; height:30px; } \
         .clear-left { display:block; clear:left; width:10px; height:10px; } \
         .clear-both { display:block; clear:both; width:10px; height:10px; }",
    )
    .unwrap()
    .install(&context);
    let mut float = Rect::NOTHING;
    let mut before = Rect::NOTHING;
    let mut after = Vec::new();
    let mut clears = [Rect::NOTHING; 2];
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                before = ui.add(egui::Label::new("P ").selector(".text")).rect;
                float = ui.add(empty_response.selector(".float")).rect;
                after.clear();
                for _ in 0..16 {
                    after.push(ui.add(egui::Label::new("X ").selector(".text")).rect);
                }
                ui.response()
            })
            .selector(".flow"),
        );
        ui.add(
            (|ui: &mut Ui| {
                let left = ui.add(empty_response.selector(".left-float")).rect;
                clears[0] = ui.add(empty_response.selector(".clear-left")).rect;
                assert!(left.height() > 0.0);
                ui.response()
            })
            .selector(".flow"),
        );
        ui.add(
            (|ui: &mut Ui| {
                ui.add(empty_response.selector(".left-float"));
                let right = ui.add(empty_response.selector(".right-float")).rect;
                clears[1] = ui.add(empty_response.selector(".clear-both")).rect;
                assert!(right.height() > 0.0);
                ui.response()
            })
            .selector(".flow"),
        );
    });
    assert_eq!(float.size(), vec2(40.0, 40.0), "{float:?}");
    assert!(
        (before.top() - float.top()).abs() < 0.5,
        "{before:?} {float:?}"
    );
    assert!(before.left() >= float.right(), "{before:?} {float:?}");
    assert!(after[0].left() >= float.right(), "{float:?} {after:?}");
    assert!(
        after
            .iter()
            .any(|rect| rect.top() >= float.bottom() && rect.left() < 1.0),
        "{float:?} {after:?}"
    );
    assert!(clears[0].top() >= 20.0, "{clears:?}");
    assert!(clears[1].top() >= 30.0, "{clears:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn nested_floats_inside_inline_boxes_leave_inline_flow() {
    let context = Context::default();
    Stylesheet::parse(
        ".flow { display:flow-root; width:100px; height:80px; } \
         .parent { display:inline; } .float { display:inline; float:left; width:40px; height:40px; } \
         .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut float = Rect::NOTHING;
    let mut text = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        float = ui.add(empty_response.selector(".float")).rect;
                        text.clear();
                        for _ in 0..16 {
                            text.push(ui.add(egui::Label::new("X ").selector(".text")).rect);
                        }
                        ui.response()
                    })
                    .selector(".parent"),
                );
                ui.response()
            })
            .selector(".flow"),
        );
    });
    assert_eq!(float.size(), vec2(40.0, 40.0), "{float:?}");
    assert!(text[0].left() >= float.right(), "{float:?} {text:?}");
    assert!(
        text.iter()
            .any(|rect| rect.top() >= float.bottom() && rect.left() < 1.0),
        "{float:?} {text:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn nested_float_in_an_inline_box_affects_following_block_content() {
    let context = Context::default();
    Stylesheet::parse(
        ".flow { display:flow-root; width:100px; height:80px; } \
         .parent { display:inline; } .float { display:inline; float:left; width:40px; height:40px; } \
         .sibling { display:block; } .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut float = Rect::NOTHING;
    let mut sibling = Rect::NOTHING;
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        float = ui.add(empty_response.selector(".float")).rect;
                        ui.response()
                    })
                    .selector(".parent"),
                );
                sibling = ui
                    .add(
                        (|ui: &mut Ui| {
                            ui.add(egui::Label::new("X").selector(".text"));
                            ui.response()
                        })
                        .selector(".sibling"),
                    )
                    .rect;
                ui.response()
            })
            .selector(".flow"),
        );
    });
    assert_eq!(float.size(), vec2(40.0, 40.0), "{float:?}");
    assert!(sibling.top() < float.bottom(), "{float:?} {sibling:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn changing_a_nested_float_updates_wrapping_and_its_click_target() {
    let context = Context::default();
    Stylesheet::parse(
        ".flow { display:flow-root; width:120px; height:60px; } \
         .parent, .text { display:inline; } \
         .float { display:inline; float:left; width:40px; height:40px; } \
         .float.right { float:right; }",
    )
    .unwrap()
    .install(&context);
    let right = std::cell::Cell::new(false);
    let bounds = std::cell::Cell::new([Rect::NOTHING; 3]);
    let clicks = std::cell::Cell::new(0);
    let mut build = |ui: &mut Ui| {
        let mut floated = Rect::NOTHING;
        let mut text = Rect::NOTHING;
        let flow = ui
            .add(
                (|ui: &mut Ui| {
                    ui.add(
                        (|ui: &mut Ui| {
                            let response = ui.add(
                                (|ui: &mut Ui| {
                                    ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::click())
                                        .1
                                })
                                .selector(if right.get() {
                                    ".float.right"
                                } else {
                                    ".float"
                                }),
                            );
                            floated = response.rect;
                            clicks.set(clicks.get() + usize::from(response.clicked()));
                            text = ui.add(egui::Label::new("X").selector(".text")).rect;
                            ui.response()
                        })
                        .selector(".parent"),
                    );
                    ui.response()
                })
                .selector(".flow"),
            )
            .rect;
        bounds.set([flow, floated, text]);
    };
    frame(&context, 200.0, 0.0, vec![], &mut build);
    let [_, floated, text] = bounds.get();
    assert!(text.left() >= floated.right());
    right.set(true);
    frame(&context, 200.0, 1.0, vec![], &mut build);
    let [flow, floated, text] = bounds.get();
    assert!((floated.right() - flow.right()).abs() < 0.5);
    assert!(text.right() <= floated.left());
    let position = floated.center();
    frame(
        &context,
        200.0,
        1.1,
        vec![
            Event::PointerMoved(position),
            Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &mut build,
    );
    frame(
        &context,
        200.0,
        1.2,
        vec![Event::PointerButton {
            pos: position,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &mut build,
    );
    assert_eq!(clicks.get(), 1);
}
