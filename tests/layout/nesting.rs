use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn nested_descendants_apply_in_widget_tree_and_declarations_keep_source_order() {
    let context = Context::default();
    Stylesheet::parse(
        ".panel { --child-width:40px; width:120px; height:60px;
             .child { width:var(--child-width); height:12px; }
             .child { width:var(--child-width); }
             width:160px;
         }",
    )
    .unwrap()
    .install(&context);

    let mut panel = Rect::NOTHING;
    let mut child = Rect::NOTHING;
    frame(&context, 240.0, 0.0, Vec::new(), |ui| {
        panel = ui
            .add(
                (|ui: &mut Ui| {
                    child = ui.add(empty_response.selector(".child")).rect;
                    ui.response()
                })
                .selector(".panel"),
            )
            .rect;
    });

    assert_eq!(panel.size(), vec2(160.0, 60.0));
    assert_eq!(child.size(), vec2(40.0, 12.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn explicit_child_nesting_does_not_match_deeper_descendants() {
    let context = Context::default();
    Stylesheet::parse(
        ".panel { display:block; width:160px; height:80px;
             > .child { width:40px; height:12px; }
             .child { height:12px; }
         }
         .middle { display:block; width:100px; height:40px; }
         .child { width:20px; height:12px; }",
    )
    .unwrap()
    .install(&context);

    let mut child_sizes = Vec::new();
    frame(&context, 240.0, 0.0, Vec::new(), |ui| {
        child_sizes.clear();
        ui.add(
            (|ui: &mut Ui| {
                child_sizes.push(ui.add(empty_response.selector(".child")).rect.size());
                ui.add(
                    (|ui: &mut Ui| {
                        child_sizes.push(ui.add(empty_response.selector(".child")).rect.size());
                        ui.response()
                    })
                    .selector(".middle"),
                );
                ui.response()
            })
            .selector(".panel"),
        );
    });

    assert_eq!(child_sizes, [vec2(40.0, 12.0), vec2(20.0, 12.0)]);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn nested_media_and_supports_apply_to_descendants_and_parent_declarations() {
    let context = Context::default();
    Stylesheet::parse(
        ".panel { width:100px; height:30px;
             @media (min-width:200px) {
                 height:50px;
                 .child { width:64px; }
                 @supports (display:flex) { .child { height:16px; } }
             }
         }",
    )
    .unwrap()
    .install(&context);

    for (index, (viewport, expected_panel, expected_child)) in [
        (240.0, vec2(100.0, 50.0), vec2(64.0, 16.0)),
        (120.0, vec2(100.0, 30.0), vec2(100.0, 0.0)),
    ]
    .into_iter()
    .enumerate()
    {
        let mut panel = Rect::NOTHING;
        let mut child = Rect::NOTHING;
        frame(&context, viewport, index as f64, Vec::new(), |ui| {
            panel = ui
                .add(
                    (|ui: &mut Ui| {
                        child = ui.add(empty_response.selector(".child")).rect;
                        ui.response()
                    })
                    .selector(".panel"),
                )
                .rect;
        });
        assert_eq!(panel.size(), expected_panel, "viewport {viewport}");
        assert_eq!(child.size(), expected_child, "viewport {viewport}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn nested_selector_lists_use_the_maximum_parent_specificity() {
    let context = Context::default();
    Stylesheet::parse(
        ".panel, #alternate { .child { width:40px; height:12px; } }
         .child.extra { width:20px; }",
    )
    .unwrap()
    .install(&context);

    let mut child = Rect::NOTHING;
    frame(&context, 120.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                child = ui.add(empty_response.selector(".child.extra")).rect;
                ui.response()
            })
            .selector(".panel"),
        );
    });
    assert_eq!(child.size(), vec2(40.0, 12.0));
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn unsupported_nested_relations_and_malformed_rules_are_rejected() {
    for source in [
        ".panel { & + .sibling { width:20px; } }",
        ".panel { & .child:hover .grandchild { width:20px; } }",
        ".panel { .child { width:invalid; } }",
        ".panel { & > { width:20px; } }",
    ] {
        assert!(Stylesheet::parse(source).is_err(), "accepted {source}");
    }

    let error = Stylesheet::parse(".panel {\n .child { width:invalid; }\n}").unwrap_err();
    assert_eq!(error.line(), 2);

    let mut nested = String::from(".panel {");
    for _ in 0..80 {
        nested.push_str("& {");
    }
    nested.push_str("width:20px;");
    for _ in 0..81 {
        nested.push('}');
    }
    assert!(Stylesheet::parse(&nested).is_err());
}
