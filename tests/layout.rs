use ecss::{SelectorExt as _, Stylesheet};
use egui::{Context, Event, Modifiers, PointerButton, RawInput, Rect, Ui, Vec2, pos2, vec2};

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn frame(
    context: &Context,
    width: f32,
    time: f64,
    events: Vec<Event>,
    build: impl FnMut(&mut Ui),
) -> egui::FullOutput {
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 200.0))),
            time: Some(time),
            events,
            ..Default::default()
        },
        build,
    );
    output.textures_delta.clear();
    output
}

fn empty(ui: &mut Ui) -> egui::Response {
    ui.allocate_exact_size(Vec2::ZERO, egui::Sense::hover()).1
}

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
                        ui.add(empty.selector(".item")).rect,
                        ui.add(empty.selector(".item")).rect,
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
fn wheel_scroll_moves_descendants_and_clamps_at_the_scroll_extent() {
    let context = Context::default();
    Stylesheet::parse(
        ".scroll { width:80px; height:40px; overflow:auto; } .child { width:20px; height:100px; }",
    )
    .unwrap()
    .install(&context);
    let mut top = 0.0;
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                top = ui.add(empty.selector(".child")).rect.top();
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
            Event::PointerMoved(pos2(10.0, 10.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                phase: egui::TouchPhase::Move,
                delta: vec2(0.0, -1000.0),
                modifiers: Modifiers::NONE,
            },
        ],
        &mut build,
    );
    for i in 2..8 {
        frame(&context, 200.0, i as f64 * 0.1, Vec::new(), &mut build);
    }
    assert!(
        (-60.05..=-59.95).contains(&top),
        "scrolled content top: {top}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn padding_and_css_size_are_part_of_the_widget_click_target() {
    let context = Context::default();
    Stylesheet::parse(".button { width:80px; height:30px; padding:10px; }")
        .unwrap()
        .install(&context);
    let mut clicked = false;
    let mut build = |ui: &mut Ui| {
        clicked |= ui.add(egui::Button::new("X").selector(".button")).clicked();
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let pos = pos2(90.0, 40.0);
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
    frame(
        &context,
        200.0,
        0.2,
        vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &mut build,
    );
    assert!(clicked, "the CSS padding box should receive clicks");
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
                    ui.add(empty.selector(".first")).rect,
                    ui.add(empty.selector(".second")).rect,
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
fn intrinsic_text_widths_measure_unbroken_words_and_unwrapped_text() {
    let context = Context::default();
    Stylesheet::parse(
        ".min { width:min-content; } .max { width:max-content; } .fit { width:fit-content(90px); }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        rects = vec![
            ui.add(egui::Label::new("short longestword short").selector(".min"))
                .rect,
            ui.add(egui::Label::new("longestword").selector(".max"))
                .rect,
            ui.add(egui::Label::new("short longestword short").selector(".max"))
                .rect,
            ui.add(egui::Label::new("short longestword short").selector(".fit"))
                .rect,
        ];
    });
    assert!(
        (rects[0].width() - rects[1].width()).abs() < 0.5,
        "{rects:?}"
    );
    assert!(rects[2].width() > rects[0].width());
    assert!((rects[3].width() - 90.0).abs() < 0.1);
    assert!(rects[0].height() > rects[2].height());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn min_width_min_content_uses_the_child_intrinsic_contribution() {
    let context = Context::default();
    Stylesheet::parse(
        ".intrinsic { width:10px; min-width:min-content; padding:5px; border:2px solid; } .fixed { width:10px; min-width:0; } .maximum { max-width:MIN-CONTENT; } .max-content { max-width:max-content; } .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        for class in [".intrinsic", ".fixed", ".maximum", ".max-content"] {
            rects.push(
                ui.add(
                    (|ui: &mut Ui| {
                        ui.add(egui::Label::new("short longestword short").selector(".text"));
                        ui.response()
                    })
                    .selector(class),
                )
                .rect,
            );
        }
    });
    assert!(rects[0].width() > rects[1].width() + 20.0, "{rects:?}");
    assert!(
        (rects[0].width() - rects[2].width() - 14.0).abs() < 0.5,
        "{rects:?}"
    );
    assert!(rects[3].width() > rects[2].width(), "{rects:?}");
    assert!(rects[3].width() < 300.0, "{rects:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn selectable_text_inside_selector_remains_highlighted_and_copyable() {
    let context = Context::default();
    Stylesheet::parse(
        ".page { display:flex; flex-direction:column; gap:12px; padding:16px; } .text {}",
    )
    .unwrap()
    .install(&context);
    let text = "select this text";
    let rect = std::cell::Cell::new(Rect::NOTHING);
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                rect.set(ui.add(egui::Label::new(text).selector(".text")).rect);
                ui.response()
            })
            .selector(".page"),
        );
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);

    let rect = rect.get();
    let start = pos2(rect.left() + 1.0, rect.center().y);
    let end = pos2(rect.right() - 1.0, rect.center().y);
    let output = frame(
        &context,
        200.0,
        0.1,
        vec![
            Event::PointerMoved(start),
            Event::PointerButton {
                pos: start,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
        &mut build,
    );
    assert_eq!(output.platform_output.num_completed_passes, 1);
    let output = frame(
        &context,
        200.0,
        0.2,
        vec![Event::PointerMoved(end)],
        &mut build,
    );
    assert_eq!(output.platform_output.num_completed_passes, 1);
    let output = frame(
        &context,
        200.0,
        0.3,
        vec![Event::PointerButton {
            pos: end,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &mut build,
    );
    assert_eq!(output.platform_output.num_completed_passes, 1);
    let selection_color = context.global_style().visuals.selection.bg_fill;
    let highlighted = context
        .tessellate(output.shapes.clone(), output.pixels_per_point)
        .iter()
        .any(|clipped| match &clipped.primitive {
            egui::epaint::Primitive::Mesh(mesh) => mesh
                .vertices
                .iter()
                .any(|vertex| vertex.color == selection_color),
            egui::epaint::Primitive::Callback(_) => false,
        });
    assert!(
        highlighted,
        "selected text should have visible highlight colors"
    );
    let mut output = context.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 200.0))),
            time: Some(0.4),
            events: vec![Event::Copy],
            ..Default::default()
        },
        &mut build,
    );
    let copied = output
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text.as_str()),
            _ => None,
        });
    output.textures_delta.clear();

    assert_eq!(copied, Some(text));
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
fn css_wide_values_copy_parent_properties_and_restore_initial_values() {
    let context = Context::default();
    Stylesheet::parse(".parent { box-sizing:border-box; width:180px; padding:10px; } .inherited { box-sizing:inherit; width:80px; padding:inherit; height:40px; } .initial { box-sizing:inherit; box-sizing:initial; padding:10px; width:80px; height:20px; }").unwrap().install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                rects = vec![
                    ui.add(empty.selector(".inherited")).rect,
                    ui.add(empty.selector(".initial")).rect,
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
                    ui.add(empty.selector(".unset")).rect,
                    ui.add(empty.selector(".horizontal")).rect,
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
fn sticky_boxes_remain_at_the_scrollport_edge_while_siblings_scroll() {
    let context = Context::default();
    Stylesheet::parse(".scroll { width:80px; height:40px; overflow:auto; } .sticky { position:sticky; top:0; height:10px; } .child { height:100px; }").unwrap().install(&context);
    let mut positions = (0.0, 0.0);
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                positions = (
                    ui.add(empty.selector(".sticky")).rect.top(),
                    ui.add(empty.selector(".child")).rect.top(),
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
fn z_index_controls_which_overlapping_selector_receives_a_click() {
    let context = Context::default();
    Stylesheet::parse(".root { position:relative; width:100px; height:50px; } .front { position:absolute; inset:0; z-index:2; } .back { position:absolute; inset:0; z-index:1; }").unwrap().install(&context);
    let mut clicks = (false, false);
    let click = |ui: &mut Ui| {
        ui.allocate_exact_size(vec2(1.0, 1.0), egui::Sense::click())
            .1
    };
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                clicks.0 |= ui.add(click.selector(".front")).clicked();
                clicks.1 |= ui.add(click.selector(".back")).clicked();
                ui.response()
            })
            .selector(".root"),
        );
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let pos = pos2(20.0, 20.0);
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
    frame(
        &context,
        200.0,
        0.2,
        vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &mut build,
    );
    assert_eq!(clicks, (true, false));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn hover_specificity_overrides_later_base_layout_declarations() {
    let context = Context::default();
    Stylesheet::parse(".box:hover { width:80px; } .box { width:40px; height:20px; }")
        .unwrap()
        .install(&context);
    let mut width = 0.0;
    let mut build = |ui: &mut Ui| {
        width = ui.add(empty.selector(".box")).rect.width();
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    frame(
        &context,
        200.0,
        0.1,
        vec![Event::PointerMoved(pos2(10.0, 10.0))],
        &mut build,
    );
    assert_eq!(width, 80.0);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn dragging_a_scrollbar_moves_content_once_per_input_frame() {
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
                top = ui.add(empty.selector(".child")).rect.top();
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
    frame(
        &context,
        200.0,
        0.2,
        vec![Event::PointerMoved(pos2(77.0, 15.0))],
        &mut build,
    );
    assert!((top + 25.0).abs() < 0.05, "scrolled content top: {top}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn css_math_defers_percentages_and_validates_dimensions() {
    let context = Context::default();
    Stylesheet::parse(".box { width:clamp(40px, calc(50% - 10px), 140px); height:calc(10px * sin(pi / 2)); margin-left:calc(2px - 5px); }").unwrap().install(&context);
    for (i, width) in [100.0, 200.0, 400.0].into_iter().enumerate() {
        let mut rect = Rect::NOTHING;
        frame(&context, width, i as f64, Vec::new(), |ui| {
            rect = ui.add(empty.selector(".box")).rect;
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
            rect = ui.add(empty.selector(".box")).rect;
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
                    ui.add(empty.selector(".double")).rect.width(),
                    ui.add(empty.selector(".single")).rect.width(),
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
                    ui.add(empty.selector(".inline")).rect,
                    ui.add(empty.selector(".inline")).rect,
                    ui.add(empty.selector(".block")).rect,
                    ui.add(empty.selector(".inline")).rect,
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
fn table_rows_share_columns_and_border_spacing() {
    let context = Context::default();
    Stylesheet::parse(".table { display:table; table-layout:fixed; box-sizing:border-box; width:200px; border-spacing:5px 3px; } .row { display:table-row; } .cell { display:table-cell; height:20px; }").unwrap().install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                let mut cells = Vec::new();
                for _ in 0..2 {
                    ui.add(
                        (|ui: &mut Ui| {
                            for _ in 0..2 {
                                cells.push(ui.add(empty.selector(".cell")).rect);
                            }
                            ui.response()
                        })
                        .selector(".row"),
                    );
                }
                rects = cells;
                ui.response()
            })
            .selector(".table"),
        );
    });
    assert_eq!(rects[0].min, pos2(5.0, 3.0));
    assert_eq!(rects[0].width(), 92.5);
    assert_eq!(rects[1].left(), 102.5);
    assert_eq!(rects[2].top(), 26.0);
    assert_eq!(rects[2].left(), rects[0].left());
    assert_eq!(rects[3].right(), 195.0);
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
                    ui.add(empty.selector(".item")).rect,
                    ui.add(empty.selector(".break")).rect,
                    ui.add(empty.selector(".item")).rect,
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
                        ui.add(empty.selector(".item")).rect,
                        ui.add(empty.selector(".item")).rect,
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
                    visible = ui.add(empty.selector(".child")).interact_rect;
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
                        visible = ui.add(empty.selector(".grandchild")).interact_rect;
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
fn collapsed_table_row_does_not_consume_a_row_track() {
    let context = Context::default();
    Stylesheet::parse(
        ".table { display:table; width:100px; border-collapse:collapse; } .row { display:table-row; } .collapsed { display:table-row; visibility:collapse; } .cell { display:table-cell; height:20px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                for class in [".row", ".collapsed", ".row"] {
                    ui.add(
                        (|ui: &mut Ui| {
                            rects.push(ui.add(empty.selector(".cell")).rect);
                            ui.response()
                        })
                        .selector(class),
                    );
                }
                ui.response()
            })
            .selector(".table"),
        );
    });
    assert!((rects[0].height() - 20.0).abs() < 0.05, "{rects:?}");
    assert!(
        (rects[2].top() - rects[0].bottom()).abs() < 0.05,
        "{rects:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn collapsed_table_column_does_not_consume_a_column_track() {
    let context = Context::default();
    Stylesheet::parse(
        ".table { display:table; width:100px; table-layout:fixed; border-collapse:collapse; } .column { display:table-column; width:50px; } .collapsed-column { display:table-column; width:50px; visibility:collapse; } .row { display:table-row; } .cell { display:table-cell; height:20px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                ui.add(empty.selector(".collapsed-column"));
                ui.add(empty.selector(".column"));
                ui.add(
                    (|ui: &mut Ui| {
                        rects.push(ui.add(empty.selector(".cell")).rect);
                        rects.push(ui.add(empty.selector(".cell")).rect);
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
    });
    assert_eq!(rects[0].width(), 0.0, "{rects:?}");
    assert!(
        (rects[1].left() - rects[0].left()).abs() < 0.05,
        "{rects:?}"
    );
    assert!((rects[1].width() - 50.0).abs() < 0.05, "{rects:?}");
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
                    items.push(ui.add(empty.selector(".item")).rect);
                    items.push(ui.add(empty.selector(".collapsed")).rect);
                    items.push(ui.add(empty.selector(".item")).rect);
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
                    items.push(ui.add(empty.selector(".item")).rect);
                    items.push(
                        ui.add(
                            (|ui: &mut Ui| {
                                ui.add(empty.selector(".inner"));
                                ui.response()
                            })
                            .selector(".collapsed"),
                        )
                        .rect,
                    );
                    items.push(ui.add(empty.selector(".item")).rect);
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
fn collapsed_table_row_group_removes_its_tracks() {
    let context = Context::default();
    Stylesheet::parse(
        ".table { display:table; width:100px; border-collapse:collapse; } .row { display:table-row; } .group { display:table-row-group; visibility:collapse; } .cell { display:table-cell; height:20px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        rects.push(ui.add(empty.selector(".cell")).rect);
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.add(
                    (|ui: &mut Ui| {
                        ui.add(
                            (|ui: &mut Ui| {
                                rects.push(ui.add(empty.selector(".cell")).rect);
                                ui.response()
                            })
                            .selector(".row"),
                        );
                        ui.response()
                    })
                    .selector(".group"),
                );
                ui.add(
                    (|ui: &mut Ui| {
                        rects.push(ui.add(empty.selector(".cell")).rect);
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
    });
    assert!((rects[0].height() - 20.0).abs() < 0.05, "{rects:?}");
    assert_eq!(rects[1].height(), 0.0, "{rects:?}");
    assert!((rects[2].height() - 20.0).abs() < 0.05, "{rects:?}");
    assert!(
        (rects[2].top() - rects[0].bottom()).abs() < 0.05,
        "{rects:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn text_align_match_parent_resolves_using_the_parents_direction() {
    let context = Context::default();
    Stylesheet::parse(
        ".outer { width:100px; height:40px; direction:rtl; text-align:start; } .inner { width:100px; height:20px; direction:ltr; text-align:match-parent; } .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut text = Rect::NOTHING;
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        text = ui.add(egui::Label::new("X").selector(".text")).rect;
                        ui.response()
                    })
                    .selector(".inner"),
                );
                ui.response()
            })
            .selector(".outer"),
        );
    });
    assert!(
        text.left() > 80.0 && (text.right() - 100.0).abs() < 0.05,
        "{text:?}"
    );
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
                    rects.push(ui.add(empty.selector(".child")).interact_rect);
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
        ".auto { width:100px; height:40px; overflow:auto; } .stable { width:100px; height:40px; overflow:auto; scrollbar-gutter:stable; } .both { width:100px; height:40px; overflow:auto; scrollbar-gutter:stable both-edges; } .hidden { width:100px; height:40px; overflow:hidden; scrollbar-gutter:stable; } .clip { width:100px; height:40px; overflow:clip; scrollbar-gutter:stable; } .child { width:100%; height:10px; }",
    )
    .unwrap()
    .install(&context);
    let mut parents = Vec::new();
    let mut children = Vec::new();
    frame(&context, 600.0, 0.0, Vec::new(), |ui| {
        parents.clear();
        children.clear();
        for class in [".auto", ".stable", ".both", ".hidden", ".clip"] {
            let response = ui.add(
                (|ui: &mut Ui| {
                    children.push(ui.add(empty.selector(".child")).rect);
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
fn margin_trim_removes_selected_first_and_last_child_margins() {
    let context = Context::default();
    Stylesheet::parse(
        ".container { display:flow-root; width:80px; margin-trim:block; } .first { height:10px; margin-block-start:8px; } .last { height:10px; margin-block-end:12px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                rects.push(ui.add(empty.selector(".first")).rect);
                rects.push(ui.add(empty.selector(".last")).rect);
                ui.response()
            })
            .selector(".container"),
        );
    });
    assert_eq!(rects[0].top(), 0.0, "{rects:?}");
    assert_eq!(rects[1].top(), rects[0].bottom(), "{rects:?}");
    assert!(Stylesheet::parse(".container { margin-trim: block invalid; }").is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn margin_trim_removes_inline_edge_margins_from_each_block_child() {
    let context = Context::default();
    Stylesheet::parse(
        ".container { display:flow-root; width:80px; margin-trim:inline-start; } .child { width:20px; height:10px; margin-inline-start:8px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                for _ in 0..3 {
                    rects.push(ui.add(empty.selector(".child")).rect);
                }
                ui.response()
            })
            .selector(".container"),
        );
    });
    assert!(
        rects.iter().all(|rect| rect.left().abs() < 0.05),
        "{rects:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn margin_trim_applies_to_ordered_flex_items() {
    let context = Context::default();
    Stylesheet::parse(
        ".container { display:flex; width:100px; height:20px; margin-trim:inline; } .first { order:2; width:20px; height:10px; margin-inline-end:8px; } .middle { order:1; width:20px; height:10px; margin-inline-start:4px; } .last { order:0; width:20px; height:10px; margin-inline-start:10px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                for class in [".first", ".middle", ".last"] {
                    rects.push(ui.add(empty.selector(class)).rect);
                }
                ui.response()
            })
            .selector(".container"),
        );
    });
    assert!((rects[2].left()).abs() < 0.05, "{rects:?}");
    assert!((rects[0].right() - 64.0).abs() < 0.05, "{rects:?}");
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
                rects.push(ui.add(empty.selector(".float")).rect);
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
fn text_overflow_ellipsis_truncates_and_paints_a_marker_at_the_inline_end() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { width:60px; height:20px; overflow:hidden; white-space:nowrap; text-overflow:ellipsis; } .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut text_rect = Rect::NOTHING;
    let mut has_ellipsis = false;
    frame(&context, 100.0, 0.0, Vec::new(), |ui| {
        text_rect = ui
            .add(
                (|ui: &mut Ui| {
                    text_rect = ui
                        .add(egui::Label::new("The overflow text is clipped").selector(".text"))
                        .rect;
                    ui.response()
                })
                .selector(".box"),
            )
            .rect;
        let layer = ui.layer_id();
        has_ellipsis = ui.ctx().graphics(|layers| {
            let Some(list) = layers.get(layer) else {
                return false;
            };
            fn has_ellipsis(shape: &egui::Shape) -> bool {
                match shape {
                    egui::Shape::Text(text) => text.galley.rows.iter().any(|row| {
                        let text = row.row.text();
                        text.ends_with('…') && text.chars().count() > 1
                    }),
                    egui::Shape::Vec(shapes) => shapes.iter().any(has_ellipsis),
                    _ => false,
                }
            }
            list.all_entries().any(|entry| has_ellipsis(&entry.shape))
        });
    });
    assert!((text_rect.width() - 60.0).abs() < 0.05, "{text_rect:?}");
    assert!(has_ellipsis, "the clipped label should paint an ellipsis");
    for value in ["clip ellipsis hidden", "ellipsis ellipsis ellipsis"] {
        assert!(
            Stylesheet::parse(&format!(".box {{ text-overflow:{value}; }}")).is_err(),
            "accepted invalid text-overflow: {value}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn text_overflow_uses_a_custom_string_marker() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { width:60px; height:20px; overflow:hidden; white-space:nowrap; text-overflow:clip \" [more]\"; } .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut has_marker = false;
    frame(&context, 100.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                ui.add(egui::Label::new("The overflow text is clipped").selector(".text"));
                ui.response()
            })
            .selector(".box"),
        );
        let layer = ui.layer_id();
        has_marker = ui.ctx().graphics(|layers| {
            let Some(list) = layers.get(layer) else {
                return false;
            };
            fn has_marker(shape: &egui::Shape) -> bool {
                match shape {
                    egui::Shape::Text(text) => text.galley.rows.iter().any(|row| {
                        let text = row.row.text();
                        text.ends_with(" [more]")
                            && text.chars().count() > " [more]".chars().count()
                    }),
                    egui::Shape::Vec(shapes) => shapes.iter().any(has_marker),
                    _ => false,
                }
            }
            list.all_entries().any(|entry| has_marker(&entry.shape))
        });
    });
    assert!(
        has_marker,
        "the clipped label should paint its custom marker"
    );
    assert!(Stylesheet::parse(".box { text-overflow: ellipsis ellipsis ellipsis; }").is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn text_overflow_uses_the_logical_inline_axis_in_vertical_writing() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { writing-mode:vertical-rl; width:20px; height:40px; overflow-inline:hidden; white-space:nowrap; text-overflow:ellipsis; } .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut has_ellipsis = false;
    frame(&context, 100.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                ui.add(egui::Label::new("The overflow text is clipped").selector(".text"));
                ui.response()
            })
            .selector(".box"),
        );
        let layer = ui.layer_id();
        has_ellipsis = ui.ctx().graphics(|layers| {
            let Some(list) = layers.get(layer) else {
                return false;
            };
            fn has_ellipsis(shape: &egui::Shape) -> bool {
                match shape {
                    egui::Shape::Text(text) => text.galley.rows.iter().any(|row| {
                        let text = row.row.text();
                        text.ends_with('…') && text.chars().count() > 1
                    }),
                    egui::Shape::Vec(shapes) => shapes.iter().any(has_ellipsis),
                    _ => false,
                }
            }
            list.all_entries().any(|entry| has_ellipsis(&entry.shape))
        });
    });
    assert!(
        has_ellipsis,
        "vertical inline overflow should paint an ellipsis"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn rtl_inline_overflow_is_clipped_at_the_logical_end() {
    let context = Context::default();
    Stylesheet::parse(
        ".box { direction:rtl; width:60px; height:20px; overflow:hidden; white-space:nowrap; text-overflow:ellipsis; } .text { display:inline; }",
    )
    .unwrap()
    .install(&context);
    let mut bounds = (Rect::NOTHING, Rect::NOTHING);
    frame(&context, 100.0, 0.0, Vec::new(), |ui| {
        bounds.0 = ui
            .add(
                (|ui: &mut Ui| {
                    bounds.1 = ui
                        .add(egui::Label::new("The overflow text is clipped").selector(".text"))
                        .rect;
                    ui.response()
                })
                .selector(".box"),
            )
            .rect;
    });
    assert!(bounds.1.left() >= bounds.0.left() - 0.05, "{bounds:?}");
    assert!(bounds.1.right() <= bounds.0.right() + 0.05, "{bounds:?}");
}
