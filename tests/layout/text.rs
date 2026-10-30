use super::*;

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

fn text_rows(context: &Context, layer: egui::LayerId) -> Vec<Rect> {
    fn collect(shape: &egui::Shape, rows: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Text(text) => rows.extend(
                text.galley
                    .rows
                    .iter()
                    .filter(|row| !row.row.glyphs.is_empty())
                    .map(|row| {
                        row.rect_without_leading_space()
                            .translate(text.pos.to_vec2())
                    }),
            ),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, rows);
                }
            }
            _ => {}
        }
    }

    context.graphics(|layers| {
        let mut rows = Vec::new();
        if let Some(list) = layers.get(layer) {
            for entry in list.all_entries() {
                collect(&entry.shape, &mut rows);
            }
        }
        rows.sort_by(|a, b| {
            a.top()
                .total_cmp(&b.top())
                .then(a.left().total_cmp(&b.left()))
        });
        rows
    })
}

fn aligned_text_rows(style: &str, text: &str) -> Vec<Rect> {
    let context = Context::default();
    Stylesheet::parse(&format!(
        ".box {{ width:100px; height:40px; color:red; {style} }} .text {{ display:inline; }}"
    ))
    .unwrap()
    .install(&context);
    let mut rows = Vec::new();
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        let layer = ui.layer_id();
        ui.add(
            (|ui: &mut Ui| {
                ui.add(egui::Label::new(text).selector(".text"));
                ui.response()
            })
            .selector(".box"),
        );
        rows = text_rows(ui.ctx(), layer);
    });
    rows
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn text_align_last_applies_to_forced_break_lines() {
    let rows = aligned_text_rows(
        "white-space:pre; text-align:left; text-align-last:center;",
        "a\nb",
    );
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(
        (rows[0].center().x - rows[1].center().x).abs() < 0.05,
        "{rows:?}"
    );
    assert!(rows[0].left() > 35.0, "{rows:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn text_align_shorthand_resets_text_align_last() {
    let rows = aligned_text_rows(
        "white-space:pre; text-align-last:center; text-align:left;",
        "a\nb",
    );
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(rows.iter().all(|row| row.left() < 5.0), "{rows:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn text_align_last_maps_supported_values_to_the_inline_axis() {
    let row = |value| {
        aligned_text_rows(
            &format!("white-space:pre; text-align:start; text-align-last:{value};"),
            "a",
        )
        .remove(0)
    };
    let start = row("auto");
    let end = row("end");
    let left = row("left");
    let right = row("right");
    let center = row("center");
    let justify = row("justify");
    assert!(start.left() < 5.0 && left.left() < 5.0 && justify.left() < 5.0);
    assert!((end.right() - 100.0).abs() < 1.0, "{end:?}");
    assert!((right.right() - 100.0).abs() < 1.0, "{right:?}");
    assert!((center.center().x - 50.0).abs() < 1.0, "{center:?}");
    for candidate in [end, left, right, center, justify] {
        assert!((candidate.width() - start.width()).abs() < 0.05);
        assert!((candidate.height() - start.height()).abs() < 0.05);
    }
    assert!(Stylesheet::parse(".text { text-align-all:justify-all; }").is_err());
    assert!(Stylesheet::parse(".text { text-align-last:justify-all; }").is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn text_align_last_inherits_and_honors_css_wide_keywords() {
    fn render(child_style: &str) -> Rect {
        let context = Context::default();
        Stylesheet::parse(&format!(
            ".parent {{ width:100px; height:40px; white-space:pre; text-align:start; text-align-last:center; }}\
             .child {{ display:block; width:100px; height:40px; {child_style} }}\
             .text {{ display:inline; }}"
        ))
        .unwrap()
        .install(&context);
        let mut rows = Vec::new();
        frame(&context, 120.0, 0.0, Vec::new(), |ui| {
            let layer = ui.layer_id();
            ui.add(
                (|ui: &mut Ui| {
                    ui.add(
                        (|ui: &mut Ui| {
                            ui.add(egui::Label::new("a").selector(".text"));
                            ui.response()
                        })
                        .selector(".child"),
                    );
                    ui.response()
                })
                .selector(".parent"),
            );
            rows = text_rows(ui.ctx(), layer);
        });
        rows.into_iter().next().expect("text row")
    }

    let inherited = render("text-align-last:inherit;");
    let unset = render("text-align-last:unset;");
    let initial = render("text-align-last:initial;");
    assert!((inherited.center().x - 50.0).abs() < 1.0, "{inherited:?}");
    assert!((unset.center().x - 50.0).abs() < 1.0, "{unset:?}");
    assert!(initial.left() < 5.0, "{initial:?}");

    let shorthand_initial = render("text-align-last:center; text-align:initial;");
    assert!(shorthand_initial.left() < 5.0, "{shorthand_initial:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn text_align_last_maps_rtl_and_vertical_writing_axes() {
    let rtl_start = aligned_text_rows(
        "white-space:pre; direction:rtl; text-align-last:start;",
        "a",
    )
    .remove(0);
    let rtl_end =
        aligned_text_rows("white-space:pre; direction:rtl; text-align-last:end;", "a").remove(0);
    assert!((rtl_start.right() - 100.0).abs() < 1.0, "{rtl_start:?}");
    assert!(rtl_end.left() < 5.0, "{rtl_end:?}");

    let vertical_top = aligned_text_rows(
        "white-space:pre; writing-mode:vertical-rl; text-align-last:left;",
        "a",
    )
    .remove(0);
    let vertical_bottom = aligned_text_rows(
        "white-space:pre; writing-mode:vertical-rl; text-align-last:right;",
        "a",
    )
    .remove(0);
    let vertical_center = aligned_text_rows(
        "white-space:pre; writing-mode:vertical-rl; text-align-last:center;",
        "a",
    )
    .remove(0);
    assert!(vertical_top.top() < 5.0, "{vertical_top:?}");
    assert!(
        vertical_bottom.bottom() > vertical_top.bottom(),
        "{vertical_bottom:?}"
    );
    assert!(
        (vertical_center.center().y - (vertical_top.center().y + vertical_bottom.center().y) * 0.5)
            .abs()
            < 0.05,
        "{vertical_center:?}"
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
