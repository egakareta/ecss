use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn wheel_scroll_moves_descendants_and_clamps_at_both_extents() {
    let context = Context::default();
    Stylesheet::parse(
        ".scroll { width:80px; height:40px; overflow:auto; scrollbar-width:none; } .child { width:20px; height:100px; }",
    )
    .unwrap()
    .install(&context);
    let top = std::cell::Cell::new(0.0);
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                top.set(ui.add(empty_response.selector(".child")).rect.top());
                ui.response()
            })
            .selector(".scroll"),
        );
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let output = frame(
        &context,
        200.0,
        0.1,
        vec![
            Event::PointerMoved(pos2(10.0, 10.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: vec2(0.0, -1000.0),
                source: egui::MouseWheelSource::Unknown,
                modifiers: Modifiers::NONE,
            },
        ],
        &mut build,
    );
    assert_eq!(output.platform_output.num_completed_passes, 1);
    for i in 2..8 {
        let output = frame(&context, 200.0, i as f64 * 0.1, Vec::new(), &mut build);
        assert_eq!(output.platform_output.num_completed_passes, 1);
    }
    assert!(
        (-60.05..=-59.95).contains(&top.get()),
        "scrolled content top: {}",
        top.get()
    );
    let output = frame(
        &context,
        200.0,
        0.8,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            phase: egui::TouchPhase::Move,
            delta: vec2(0.0, 1000.0),
            source: egui::MouseWheelSource::Unknown,
            modifiers: Modifiers::NONE,
        }],
        &mut build,
    );
    assert_eq!(output.platform_output.num_completed_passes, 1);
    for i in 9..16 {
        let output = frame(&context, 200.0, i as f64 * 0.1, Vec::new(), &mut build);
        assert_eq!(output.platform_output.num_completed_passes, 1);
    }
    assert!(
        top.get().abs() < 0.05,
        "scrolled content top: {}",
        top.get()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn sticky_boxes_remain_at_the_scrollport_edge_while_siblings_scroll() {
    let context = Context::default();
    Stylesheet::parse(".scroll { width:80px; height:40px; overflow:auto; } .sticky { position:sticky; top:0; height:10px; } .child { height:100px; }").unwrap().install(&context);
    let mut positions = (0.0, 0.0);
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                positions = (
                    ui.add(empty_response.selector(".sticky")).rect.top(),
                    ui.add(empty_response.selector(".child")).rect.top(),
                );
                ui.response()
            })
            .selector(".scroll"),
        );
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    frame(
        &context,
        200.0,
        0.1,
        vec![
            Event::PointerMoved(pos2(20.0, 20.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -1000.0),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
                source: egui::MouseWheelSource::Unknown,
            },
        ],
        &mut build,
    );
    for i in 2..8 {
        frame(&context, 200.0, i as f64 * 0.1, Vec::new(), &mut build);
    }
    assert!(positions.0.abs() < 0.05, "sticky top: {}", positions.0);
    assert!(
        (positions.1 + 60.0).abs() < 0.05,
        "scrolling sibling top: {}",
        positions.1
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn sticky_percentage_insets_resolve_against_the_nearest_scrollport() {
    let context = Context::default();
    Stylesheet::parse(
        ".scroll { width:100px; height:40px; overflow:auto; scrollbar-width:none; } .flow { height:100px; } .sticky { position:sticky; top:50%; height:10px; } .content { height:100px; }",
    )
    .unwrap()
    .install(&context);
    let sticky_top = std::cell::Cell::new(0.0);
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        sticky_top.set(ui.add(empty_response.selector(".sticky")).rect.top());
                        ui.add(empty_response.selector(".content"));
                        ui.response()
                    })
                    .selector(".flow"),
                );
                ui.response()
            })
            .selector(".scroll"),
        );
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    frame(
        &context,
        200.0,
        0.1,
        vec![
            Event::PointerMoved(pos2(20.0, 20.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: vec2(0.0, -20.0),
                source: egui::MouseWheelSource::Unknown,
                modifiers: Modifiers::NONE,
            },
        ],
        &mut build,
    );
    for i in 2..8 {
        frame(&context, 200.0, i as f64 * 0.1, Vec::new(), &mut build);
    }

    assert!(
        (sticky_top.get() - 20.0).abs() < 0.05,
        "sticky top: {}",
        sticky_top.get()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn sticky_box_remains_within_its_containing_block() {
    let context = Context::default();
    Stylesheet::parse(
        ".scroll { width:100px; height:40px; overflow:auto; scrollbar-width:none; } .flow { height:100px; } .sticky { position:sticky; top:0; height:10px; } .content { height:100px; }",
    )
    .unwrap()
    .install(&context);
    let sticky_top = std::cell::Cell::new(0.0);
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        sticky_top.set(ui.add(empty_response.selector(".sticky")).rect.top());
                        ui.add(empty_response.selector(".content"));
                        ui.response()
                    })
                    .selector(".flow"),
                );
                ui.add(empty_response.selector(".content"));
                ui.response()
            })
            .selector(".scroll"),
        );
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    frame(
        &context,
        200.0,
        0.1,
        vec![
            Event::PointerMoved(pos2(20.0, 20.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: vec2(0.0, -95.0),
                source: egui::MouseWheelSource::Unknown,
                modifiers: Modifiers::NONE,
            },
        ],
        &mut build,
    );
    for i in 2..8 {
        frame(&context, 200.0, i as f64 * 0.1, Vec::new(), &mut build);
    }

    assert!(
        (sticky_top.get() + 5.0).abs() < 0.05,
        "sticky top: {}",
        sticky_top.get()
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn dragging_a_scrollbar_moves_content_on_the_repaint_frame() {
    let context = Context::default();
    Stylesheet::parse(
        ".scroll { width:80px; height:40px; overflow:auto; } .child { width:80px; height:100px; }",
    )
    .unwrap()
    .install(&context);
    let mut top = 0.0;
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                top = ui.add(empty_response.selector(".child")).rect.top();
                ui.response()
            })
            .selector(".scroll"),
        );
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let pos = pos2(77.0, 5.0);
    frame(
        &context,
        200.0,
        0.1,
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &mut build,
    );
    let dragged = frame(
        &context,
        200.0,
        0.2,
        vec![Event::PointerMoved(pos2(77.0, 15.0))],
        &mut build,
    );
    assert_eq!(dragged.platform_output.num_completed_passes, 1);
    let settled = frame(&context, 200.0, 0.3, Vec::new(), &mut build);
    assert_eq!(settled.platform_output.num_completed_passes, 1);
    assert!((top + 25.0).abs() < 0.05, "scrolled content top: {top}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn logical_overflow_maps_to_block_axis_in_each_writing_mode() {
    for (i, mode) in ["horizontal-tb", "vertical-rl"].into_iter().enumerate() {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".flow {{ writing-mode:{mode}; width:40px; height:30px; overflow-block:hidden; }} .child {{ width:{}px; height:{}px; }}",
            if i == 0 { 20 } else { 60 },
            if i == 0 { 60 } else { 20 },
        ))
        .unwrap()
        .install(&context);
        let mut visible = Rect::NOTHING;
        frame(&context, 100.0, 0.0, Vec::new(), |ui| {
            ui.add(
                (|ui: &mut Ui| {
                    visible = ui.add(empty_response.selector(".child")).interact_rect;
                    ui.response()
                })
                .selector(".flow"),
            );
        });
        let clipped = if i == 0 {
            visible.height()
        } else {
            visible.width()
        };
        let expected = if i == 0 { 30.0 } else { 40.0 };
        assert!((clipped - expected).abs() < 0.05, "{mode}: {visible:?}");
    }

    let context = Context::default();
    Stylesheet::parse(
        ".parent { width:80px; height:30px; overflow-block:hidden; } .child { writing-mode:vertical-rl; width:60px; height:20px; overflow-block:inherit; } .grandchild { width:100px; height:10px; }",
    )
    .unwrap()
    .install(&context);
    let mut visible = Rect::NOTHING;
    frame(&context, 100.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        visible = ui.add(empty_response.selector(".grandchild")).interact_rect;
                        ui.response()
                    })
                    .selector(".child"),
                );
                ui.response()
            })
            .selector(".parent"),
        );
    });
    assert!((visible.width() - 60.0).abs() < 0.05, "{visible:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn overflow_clip_margin_expands_only_clip_overflow_edges() {
    let context = Context::default();
    Stylesheet::parse(
        ".clip { width:40px; height:30px; overflow:clip; overflow-clip-margin:10px; } .hidden { width:40px; height:30px; overflow:hidden; overflow-clip-margin:10px; } .content { width:40px; height:30px; padding:5px; border:2px solid; overflow:clip; overflow-clip-margin:content-box 3px; } .child { width:100px; height:60px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 150.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        for class in [".clip", ".hidden", ".content"] {
            ui.add(
                (|ui: &mut Ui| {
                    rects.push(ui.add(empty_response.selector(".child")).interact_rect);
                    ui.response()
                })
                .selector(class),
            );
        }
    });
    assert_eq!(rects[0].size(), vec2(50.0, 40.0), "{rects:?}");
    assert_eq!(rects[1].size(), vec2(40.0, 30.0), "{rects:?}");
    assert_eq!(rects[2].size(), vec2(43.0, 33.0), "{rects:?}");
    assert!(Stylesheet::parse(".clip { overflow-clip-margin:-1px; }").is_err());
    assert!(Stylesheet::parse(".clip { overflow-clip-margin:10%; }").is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn scrollbar_gutter_stable_reserves_scrollport_space() {
    let context = Context::default();
    Stylesheet::parse(
        ".auto { width:100px; height:40px; overflow:auto; } .stable { width:100px; height:40px; overflow:auto; scrollbar-gutter:stable; } .both { width:100px; height:40px; overflow:auto; scrollbar-gutter:stable both-edges; } .hidden { width:100px; height:40px; overflow:hidden; scrollbar-gutter:stable; } .clip { width:100px; height:40px; overflow:clip; scrollbar-gutter:stable; } .thin { width:100px; height:40px; overflow:auto; scrollbar-gutter:stable; scrollbar-width:thin; } .none { width:100px; height:40px; overflow:auto; scrollbar-gutter:stable; scrollbar-width:none; } .child { width:100%; height:10px; }",
    )
    .unwrap()
    .install(&context);
    let mut parents = Vec::new();
    let mut children = Vec::new();
    frame(&context, 600.0, 0.0, Vec::new(), |ui| {
        parents.clear();
        children.clear();
        for class in [
            ".auto", ".stable", ".both", ".hidden", ".clip", ".thin", ".none",
        ] {
            let response = ui.add(
                (|ui: &mut Ui| {
                    children.push(ui.add(empty_response.selector(".child")).rect);
                    ui.response()
                })
                .selector(class),
            );
            parents.push(response.rect);
        }
    });
    assert!(
        parents
            .iter()
            .all(|rect| (rect.width() - 100.0).abs() < 0.05),
        "{parents:?}"
    );
    assert!((children[0].width() - 100.0).abs() < 0.05, "{children:?}");
    assert!((children[1].width() - 94.0).abs() < 0.05, "{children:?}");
    assert!((children[2].width() - 88.0).abs() < 0.05, "{children:?}");
    assert!((children[2].left() - children[1].left() - 6.0).abs() < 0.05);
    assert!((children[3].width() - 94.0).abs() < 0.05, "{children:?}");
    assert!((children[4].width() - 100.0).abs() < 0.05, "{children:?}");
    assert!((children[5].width() - 96.0).abs() < 0.05, "{children:?}");
    assert!((children[6].width() - 100.0).abs() < 0.05, "{children:?}");
    for value in [
        "both-edges",
        "auto both-edges",
        "stable auto",
        "stable stable",
    ] {
        assert!(
            Stylesheet::parse(&format!(".scroll {{ scrollbar-gutter:{value}; }}")).is_err(),
            "accepted invalid value: {value}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn scrollbar_properties_validate_their_value_grammars() {
    for value in [
        "red",
        "auto blue",
        "red blue green",
        "red currentcolor green",
    ] {
        assert!(
            Stylesheet::parse(&format!(".scroll {{ scrollbar-color:{value}; }}")).is_err(),
            "accepted invalid scrollbar-color: {value}"
        );
    }
    for value in ["normal", "medium", "4px"] {
        assert!(
            Stylesheet::parse(&format!(".scroll {{ scrollbar-width:{value}; }}")).is_err(),
            "accepted invalid scrollbar-width: {value}"
        );
    }
    Stylesheet::parse(
        ".scroll { scrollbar-color:currentColor transparent; scrollbar-width:THIN; }",
    )
    .unwrap();
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn overscroll_behavior_properties_validate_their_value_grammars() {
    Stylesheet::parse(
        ".scroll { overscroll-behavior: AUTO contain; overscroll-behavior-x:none; \
         overscroll-behavior-y:CONTAIN; overscroll-behavior-block:auto; \
         overscroll-behavior-inline:none; }",
    )
    .unwrap();

    for (property, value) in [
        ("overscroll-behavior", "scroll"),
        ("overscroll-behavior", "auto contain none"),
        ("overscroll-behavior-x", "auto none"),
        ("overscroll-behavior-y", "initial auto"),
        ("overscroll-behavior-block", "overscroll"),
        ("overscroll-behavior-inline", "contain none"),
    ] {
        assert!(
            Stylesheet::parse(&format!(".scroll {{ {property}:{value}; }}")).is_err(),
            "accepted invalid {property}: {value}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn overscroll_behavior_controls_nested_scroll_chaining() {
    fn scroll(parent_properties: &str, inner_properties: &str) -> (f32, f32) {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".outer {{ width:80px; height:40px; overflow:auto; scrollbar-width:none; {parent_properties} }}\
             .inner {{ width:80px; height:30px; overflow:auto; scrollbar-width:none; {inner_properties} }}\
             .inner-content {{ width:80px; height:60px; }} .tail {{ width:80px; height:100px; }}"
        ))
        .unwrap()
        .install(&context);

        let inner_top = std::cell::Cell::new(0.0);
        let inner_content_top = std::cell::Cell::new(0.0);
        let tail_top = std::cell::Cell::new(0.0);
        let mut build = |ui: &mut Ui| {
            ui.add(
                (|ui: &mut Ui| {
                    let inner = ui.add(
                        (|ui: &mut Ui| {
                            inner_content_top
                                .set(ui.add(empty_response.selector(".inner-content")).rect.top());
                            ui.response()
                        })
                        .selector(".inner"),
                    );
                    inner_top.set(inner.rect.top());
                    tail_top.set(ui.add(empty_response.selector(".tail")).rect.top());
                    ui.response()
                })
                .selector(".outer"),
            );
        };

        frame(&context, 200.0, 0.0, Vec::new(), &mut build);
        for (time, delta, start) in [(0.1, -1000.0, 2), (0.9, -1000.0, 10)] {
            frame(
                &context,
                200.0,
                time,
                vec![
                    Event::PointerMoved(pos2(10.0, 10.0)),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: vec2(0.0, delta),
                        phase: egui::TouchPhase::Move,
                        modifiers: Modifiers::NONE,
                        source: egui::MouseWheelSource::Unknown,
                    },
                ],
                &mut build,
            );
            for i in start..start + 7 {
                frame(&context, 200.0, i as f64 * 0.1, Vec::new(), &mut build);
            }
        }

        (inner_content_top.get() - inner_top.get(), tail_top.get())
    }

    let (_, auto_tail) = scroll("", "overscroll-behavior-y:auto");
    assert!(
        auto_tail < 0.0,
        "auto should chain to the outer scroller: {auto_tail}"
    );

    for (parent, inner) in [
        ("", "overscroll-behavior:contain"),
        ("", "overscroll-behavior-y:none"),
        ("", "overscroll-behavior-block:contain"),
        (
            "",
            "writing-mode:vertical-rl; overscroll-behavior-inline:contain",
        ),
        (
            "overscroll-behavior-y:contain",
            "overscroll-behavior-y:inherit",
        ),
        (
            "overscroll-behavior-y:contain",
            "all:inherit; width:80px; height:30px; overflow:auto; scrollbar-width:none",
        ),
    ] {
        let (inner_offset, chained_tail) = scroll(parent, inner);
        assert!(
            (inner_offset + 30.0).abs() < 0.05,
            "inner scroll should reach its boundary: {inner_offset} ({inner})"
        );
        assert!(
            (chained_tail - 30.0).abs() < 0.05,
            "nested scroll should not chain: {chained_tail} ({inner})"
        );
    }

    let (inner_offset, chained_tail) =
        scroll("", "overflow-y:hidden; overscroll-behavior-y:contain");
    assert!(inner_offset.abs() < 0.05, "hidden overflow must not scroll");
    assert!(
        (chained_tail - 30.0).abs() < 0.05,
        "a contained hidden scroll axis must not chain: {chained_tail}"
    );
}
