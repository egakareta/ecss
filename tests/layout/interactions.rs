use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn padding_and_css_size_are_part_of_the_widget_click_target() {
    let context = Context::default();
    Stylesheet::parse(".button { width:80px; height:30px; padding:10px; cursor:crosshair; }")
        .unwrap()
        .install(&context);
    let mut clicked = false;
    let mut build = |ui: &mut Ui| {
        clicked |= ui.add(egui::Button::new("X").selector(".button")).clicked();
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let pos = pos2(90.0, 40.0);
    let hovered = frame(
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
    assert_eq!(
        hovered.platform_output.cursor_icon,
        egui::CursorIcon::Crosshair
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
fn paint_only_button_states_do_not_request_extra_passes() {
    let context = Context::default();
    Stylesheet::parse(
        ".button:hover { background-color:#2458b5; } .button:active { background-color:#19448f; }",
    )
    .unwrap()
    .install(&context);
    let rect = std::cell::Cell::new(Rect::NOTHING);
    let mut clicks = 0;
    let mut build = |ui: &mut Ui| {
        let response = ui.add(egui::Button::new("Save changes").selector(".button"));
        rect.set(response.rect);
        clicks += usize::from(response.clicked());
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let pos = rect.get().center();
    let hovered = frame(
        &context,
        200.0,
        0.1,
        vec![Event::PointerMoved(pos)],
        &mut build,
    );
    assert_eq!(hovered.platform_output.num_completed_passes, 1);

    for click in 0..5 {
        let time = 0.2 + click as f64 * 0.2;
        let pressed = frame(
            &context,
            200.0,
            time,
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            }],
            &mut build,
        );
        let released = frame(
            &context,
            200.0,
            time + 0.1,
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            }],
            &mut build,
        );
        assert_eq!(pressed.platform_output.num_completed_passes, 1);
        assert_eq!(released.platform_output.num_completed_passes, 1);
    }

    assert_eq!(clicks, 5);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn z_index_controls_which_overlapping_selector_receives_a_click() {
    let context = Context::default();
    Stylesheet::parse(".root { position:relative; width:100px; height:50px; } .front { position:absolute; inset:0; z-index:2; cursor:crosshair; } .back { position:absolute; inset:0; z-index:1; cursor:copy; }").unwrap().install(&context);
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
    let hovered = frame(
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
    assert_eq!(
        hovered.platform_output.cursor_icon,
        egui::CursorIcon::Crosshair
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
fn cursor_z_order_prefers_front_sibling_over_a_deeper_back_descendant() {
    let context = Context::default();
    Stylesheet::parse(
        ".root { position:relative; width:100px; height:50px; }\
         .back { position:absolute; inset:0; z-index:1; }\
         .back-child { position:absolute; inset:0; cursor:copy; }\
         .front { position:absolute; inset:0; z-index:2; cursor:crosshair; }",
    )
    .unwrap()
    .install(&context);
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        ui.add(egui::Button::new("Back").selector(".back-child"));
                        ui.response()
                    })
                    .selector(".back"),
                );
                ui.add(egui::Button::new("Front").selector(".front"));
                ui.response()
            })
            .selector(".root"),
        );
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let hovered = frame(
        &context,
        200.0,
        0.1,
        vec![Event::PointerMoved(pos2(20.0, 20.0))],
        &mut build,
    );

    assert_eq!(
        hovered.platform_output.cursor_icon,
        egui::CursorIcon::Crosshair
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn cursor_keywords_map_to_egui_cursor_icons() {
    let values = [
        ("default", egui::CursorIcon::Default),
        ("none", egui::CursorIcon::None),
        ("context-menu", egui::CursorIcon::ContextMenu),
        ("help", egui::CursorIcon::Help),
        ("pointer", egui::CursorIcon::PointingHand),
        ("progress", egui::CursorIcon::Progress),
        ("wait", egui::CursorIcon::Wait),
        ("cell", egui::CursorIcon::Cell),
        ("crosshair", egui::CursorIcon::Crosshair),
        ("text", egui::CursorIcon::Text),
        ("vertical-text", egui::CursorIcon::VerticalText),
        ("alias", egui::CursorIcon::Alias),
        ("copy", egui::CursorIcon::Copy),
        ("move", egui::CursorIcon::Move),
        ("no-drop", egui::CursorIcon::NoDrop),
        ("not-allowed", egui::CursorIcon::NotAllowed),
        ("grab", egui::CursorIcon::Grab),
        ("grabbing", egui::CursorIcon::Grabbing),
        ("e-resize", egui::CursorIcon::ResizeEast),
        ("n-resize", egui::CursorIcon::ResizeNorth),
        ("ne-resize", egui::CursorIcon::ResizeNorthEast),
        ("nw-resize", egui::CursorIcon::ResizeNorthWest),
        ("s-resize", egui::CursorIcon::ResizeSouth),
        ("se-resize", egui::CursorIcon::ResizeSouthEast),
        ("sw-resize", egui::CursorIcon::ResizeSouthWest),
        ("w-resize", egui::CursorIcon::ResizeWest),
        ("ew-resize", egui::CursorIcon::ResizeHorizontal),
        ("ns-resize", egui::CursorIcon::ResizeVertical),
        ("nesw-resize", egui::CursorIcon::ResizeNeSw),
        ("nwse-resize", egui::CursorIcon::ResizeNwSe),
        ("col-resize", egui::CursorIcon::ResizeColumn),
        ("row-resize", egui::CursorIcon::ResizeRow),
        ("all-scroll", egui::CursorIcon::AllScroll),
        ("zoom-in", egui::CursorIcon::ZoomIn),
        ("zoom-out", egui::CursorIcon::ZoomOut),
    ];
    let rules = values
        .iter()
        .enumerate()
        .map(|(index, (value, _))| format!(".cursor-{index} {{ cursor:{value}; }}"))
        .collect::<Vec<_>>()
        .join(" ");
    let context = Context::default();
    Stylesheet::parse(&rules).unwrap().install(&context);
    let selector = std::cell::RefCell::new(".cursor-0".to_owned());
    let rect = std::cell::Cell::new(Rect::NOTHING);
    let mut build = |ui: &mut Ui| {
        rect.set(
            ui.add(egui::Button::new("Cursor").selector(selector.borrow().clone()))
                .rect,
        );
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let pointer = rect.get().center();

    for (index, (value, expected)) in values.iter().enumerate() {
        *selector.borrow_mut() = format!(".cursor-{index}");
        let output = frame(
            &context,
            200.0,
            0.1 + index as f64 * 0.1,
            vec![Event::PointerMoved(pointer)],
            &mut build,
        );
        assert_eq!(
            output.platform_output.cursor_icon, *expected,
            "cursor:{value}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn cursor_auto_preserves_egui_widget_cursor() {
    fn output(css: Option<&str>) -> egui::CursorIcon {
        let context = Context::default();
        if let Some(css) = css {
            Stylesheet::parse(css).unwrap().install(&context);
        }
        let rect = std::cell::Cell::new(Rect::NOTHING);
        let mut build = |ui: &mut Ui| {
            rect.set(ui.add(egui::Button::new("Cursor").selector(".button")).rect);
        };
        frame(&context, 200.0, 0.0, Vec::new(), &mut build);
        frame(
            &context,
            200.0,
            0.1,
            vec![Event::PointerMoved(rect.get().center())],
            &mut build,
        )
        .platform_output
        .cursor_icon
    }

    assert_eq!(output(Some(".button { cursor:auto; }")), output(None));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn cursor_inheritance_wide_keywords_and_nested_overrides_work() {
    let context = Context::default();
    Stylesheet::parse(
        ".parent { width:180px; height:100px; cursor:crosshair; }\
         .child-inherit { cursor:inherit; }\
         .child-unset { cursor:unset; }\
         .child-auto { cursor:auto; }\
         .child-initial { cursor:initial; }\
         .child-revert { cursor:revert; }\
         .child-copy { cursor:copy; }\
         .child-all-inherit { all:inherit; }\
         .child-all-unset { all:unset; }",
    )
    .unwrap()
    .install(&context);
    let selector = std::cell::RefCell::new(".child-auto".to_owned());
    let child_rect = std::cell::Cell::new(Rect::NOTHING);
    let mut build = |ui: &mut Ui| {
        ui.add(
            (|ui: &mut Ui| {
                child_rect.set(
                    ui.add(egui::Button::new("Child").selector(selector.borrow().clone()))
                        .rect,
                );
                ui.response()
            })
            .selector(".parent"),
        );
    };
    frame(&context, 240.0, 0.0, Vec::new(), &mut build);
    let pointer = child_rect.get().center();

    for (index, (class, expected)) in [
        ("child-inherited", Some(egui::CursorIcon::Crosshair)),
        ("child-inherit", Some(egui::CursorIcon::Crosshair)),
        ("child-unset", Some(egui::CursorIcon::Crosshair)),
        ("child-auto", Some(egui::CursorIcon::Default)),
        ("child-initial", Some(egui::CursorIcon::Default)),
        ("child-revert", Some(egui::CursorIcon::Default)),
        ("child-copy", Some(egui::CursorIcon::Copy)),
        ("child-all-inherit", Some(egui::CursorIcon::Crosshair)),
        ("child-all-unset", Some(egui::CursorIcon::Crosshair)),
    ]
    .into_iter()
    .enumerate()
    {
        *selector.borrow_mut() = format!(".{class}");
        let output = frame(
            &context,
            240.0,
            0.1 + index as f64 * 0.1,
            vec![Event::PointerMoved(pointer)],
            &mut build,
        )
        .platform_output
        .cursor_icon;
        assert_eq!(output, expected.unwrap(), ".{class}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn cursor_custom_properties_and_hover_active_updates_reach_platform_output() {
    let context = Context::default();
    Stylesheet::parse(
        ".button { --cursor:crosshair; cursor:var(--cursor); }\
         .button:hover { --cursor:copy; }\
         .button:active { --cursor:grabbing; }",
    )
    .unwrap()
    .install(&context);
    let rect = std::cell::Cell::new(Rect::NOTHING);
    let mut build = |ui: &mut Ui| {
        rect.set(ui.add(egui::Button::new("Cursor").selector(".button")).rect);
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    let pointer = rect.get().center();

    let hovered = frame(
        &context,
        200.0,
        0.1,
        vec![Event::PointerMoved(pointer)],
        &mut build,
    );
    assert_eq!(hovered.platform_output.cursor_icon, egui::CursorIcon::Copy);

    let active = frame(
        &context,
        200.0,
        0.2,
        vec![Event::PointerButton {
            pos: pointer,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }],
        &mut build,
    );
    assert_eq!(
        active.platform_output.cursor_icon,
        egui::CursorIcon::Grabbing
    );

    let released = frame(
        &context,
        200.0,
        0.3,
        vec![Event::PointerButton {
            pos: pointer,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
        &mut build,
    );
    assert_eq!(released.platform_output.cursor_icon, egui::CursorIcon::Copy);

    let outside = frame(
        &context,
        200.0,
        0.4,
        vec![Event::PointerMoved(pos2(190.0, 190.0))],
        &mut build,
    );
    assert_eq!(
        outside.platform_output.cursor_icon,
        egui::CursorIcon::Default
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn custom_cursor_images_report_the_egui_host_constraint() {
    let error = Stylesheet::parse(".button { cursor:url(cursor.png) 0 0, pointer; }")
        .unwrap_err()
        .to_string();
    assert!(error.contains("custom cursor images are unsupported by egui"));
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
        width = ui.add(empty_response.selector(".box")).rect.width();
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
fn hover_custom_properties_resolve_widget_visuals_for_the_hover_state() {
    let context = Context::default();
    Stylesheet::parse(
        ".button { --surface:#2458b5; background-color:var(--surface); } .button:hover { --surface:#19448f; }",
    )
    .unwrap()
    .install(&context);
    let mut hovered_background = egui::Color32::TRANSPARENT;
    let mut build = |ui: &mut Ui| {
        let inspect_style = |ui: &mut Ui| {
            hovered_background = ui.style().visuals.widgets.hovered.bg_fill;
            empty_response(ui)
        };
        ui.add(inspect_style.selector(".button"));
    };
    frame(&context, 200.0, 0.0, Vec::new(), &mut build);
    assert_eq!(
        hovered_background,
        egui::Color32::from_rgb(0x19, 0x44, 0x8f)
    );
}
