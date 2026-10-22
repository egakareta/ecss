use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn compound_classes_match_independently_of_order_and_use_css_specificity() {
    let stylesheet = Stylesheet::parse(
        "* { box-sizing:border-box; }
         .button { width:24px; height:20px; margin:4px; }
         .button.primary { background:lime; width:32px; }
         .primary { background:blue; }
         .button { background:red; }
         #save.button { background:yellow; }
         .button.primary.extra { background:blue; }
         .primary.button { height:24px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = [
            ".button.primary",
            ".primary.button.button",
            ".button",
            "#save.primary.extra.button",
        ]
        .map(|selector| ui.add(empty_response.selector(selector)).rect)
        .to_vec();
    })
    .await;
    let bounds = bounds.lock().unwrap();
    for index in [0, 1] {
        assert_eq!(bounds[index].size(), egui::vec2(32.0, 24.0));
        assert_eq!(
            pixel(&pixels, bounds[index].center().x, bounds[index].center().y),
            [0, 255, 0, 255]
        );
    }
    assert_eq!(bounds[2].size(), egui::vec2(24.0, 20.0));
    assert_eq!(
        pixel(&pixels, bounds[2].center().x, bounds[2].center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[3].center().x, bounds[3].center().y),
        [255, 255, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn combined_hover_enabled_states_resolve_variables_layout_and_inline_styles() {
    let stylesheet = Stylesheet::parse(
        ".button.primary:hover:enabled { --surface:lime; width:72px; }
         .button:hover { --surface:red; width:60px; }
         .button.primary { --surface:blue; width:48px; }
         .button { height:24px; background:var(--surface); }
         .button:disabled:enabled { background:red; width:100px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let inline = InlineStyle::parse("width:32px; background:yellow").unwrap();
    let pixels = render_ui_pixels_with_events(
        stylesheet,
        vec![
            vec![],
            vec![egui::Event::PointerMoved(egui::pos2(20.0, 20.0))],
        ],
        move |ui| {
            *capture.lock().unwrap() = vec![
                ui.add(empty_response.selector(".primary.button")).rect,
                ui.add(
                    empty_response
                        .selector(".button.primary")
                        .inline_style(inline.clone()),
                )
                .rect,
            ];
        },
    )
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(bounds[0].width(), 72.0);
    assert_eq!(
        pixel(&pixels, bounds[0].center().x, bounds[0].center().y),
        [0, 255, 0, 255]
    );
    assert_eq!(bounds[1].width(), 32.0);
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
        [255, 255, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn enabled_and_disabled_follow_the_widget_and_update_when_reenabled() {
    let stylesheet = Stylesheet::parse(
        ".button { width:32px; height:24px; margin:4px; }
         .button.primary:enabled { width:48px; background:lime; }
         .button.primary:disabled { background:blue; }
         .button:enabled:disabled { background:red; width:100px; }",
    )
    .unwrap();
    for reenable in [false, true] {
        let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_with_events(
            stylesheet.clone(),
            vec![vec![], vec![], vec![]],
            move |ui| {
                let enabled = reenable && ui.input(|input| input.time >= 0.2);
                let first = ui.add_enabled(enabled, empty_response.selector(".button.primary"));
                let disabled = ui.add(
                    (|ui: &mut egui::Ui| ui.add_enabled(false, egui::Button::new("")))
                        .selector(".button.primary"),
                );
                assert!(!disabled.enabled());
                *capture.lock().unwrap() = vec![first.rect, disabled.rect];
            },
        )
        .await;
        let bounds = bounds.lock().unwrap();
        assert_eq!(bounds[0].width(), if reenable { 48.0 } else { 32.0 });
        assert_eq!(
            pixel(&pixels, bounds[0].center().x, bounds[0].center().y),
            if reenable {
                [0, 255, 0, 255]
            } else {
                [127, 127, 255, 255]
            }
        );
        assert_eq!(bounds[1].width(), 32.0);
        assert_eq!(
            pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
            [0, 0, 255, 255]
        );
    }
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn hiding_an_enabled_widget_does_not_make_it_match_disabled() {
    let stylesheet = Stylesheet::parse(
        ".box { width:32px; height:24px; background:red; }
         .box:enabled { display:none; }
         .box:disabled { display:block; background:blue; }
         .hidden { visibility:hidden; }
         .visible { visibility:visible; height:24px; }
         .visible:enabled { width:48px; background:lime; }
         .visible:disabled { width:80px; background:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut bounds = vec![
            ui.add(empty_response.selector(".box")).rect,
            ui.add_enabled(false, empty_response.selector(".box")).rect,
        ];
        ui.add(
            (|ui: &mut egui::Ui| {
                bounds.push(ui.add(empty_response.selector(".visible")).rect);
                ui.response()
            })
            .selector(".hidden"),
        );
        *capture.lock().unwrap() = bounds;
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(bounds[0].size(), egui::Vec2::ZERO);
    assert_eq!(bounds[1].size(), egui::vec2(32.0, 24.0));
    assert_eq!(bounds[2].size(), egui::vec2(48.0, 24.0));
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
        [127, 127, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[2].center().x, bounds[2].center().y),
        [0, 255, 0, 255]
    );
}

fn pointer_focus_events() -> Vec<Vec<egui::Event>> {
    let pos = egui::pos2(20.0, 20.0);
    vec![
        vec![],
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        vec![],
    ]
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn focus_visible_distinguishes_keyboard_focus_from_pointer_focus() {
    let stylesheet = Stylesheet::parse(
        ".button { width:48px; height:24px; margin:4px; background:white; }
         .button:focus { background:lime; width:64px; }
         .button:active { background:red; }
         .button:focus:focus-visible:enabled { outline:2px solid blue; outline-offset:2px; }",
    )
    .unwrap();
    for keyboard in [false, true] {
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let events = if keyboard {
            vec![
                vec![],
                vec![key(egui::Key::Tab)],
                vec![egui::Event::PointerMoved(egui::pos2(20.0, 20.0))],
            ]
        } else {
            pointer_focus_events()
        };
        let pixels = render_ui_pixels_with_events(stylesheet.clone(), events, move |ui| {
            let response = ui.add(
                (|ui: &mut egui::Ui| {
                    let response = ui.button("");
                    if response.clicked() {
                        response.request_focus();
                    }
                    response
                })
                .selector(".button"),
            );
            *capture.lock().unwrap() = Some((response.rect, response.has_focus()));
        })
        .await;
        let (rect, focused) = bounds.lock().unwrap().unwrap();
        assert!(focused);
        assert_eq!(rect.width(), 64.0);
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.center().y),
            [0, 255, 0, 255]
        );
        assert_eq!(
            pixel(&pixels, rect.right() + 3.0, rect.center().y),
            if keyboard {
                [0, 0, 255, 255]
            } else {
                [255, 255, 255, 255]
            }
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn keyboard_interaction_enables_focus_visible_after_a_pointer_click() {
    let stylesheet = Stylesheet::parse(
        ".button { width:48px; height:24px; margin:4px; background:white; }
         .button:focus-visible { outline:2px solid blue; }",
    )
    .unwrap();
    let mut events = pointer_focus_events();
    events.extend([vec![key(egui::Key::Space)], vec![]]);
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_with_events(stylesheet, events, move |ui| {
        let response = ui.add(
            (|ui: &mut egui::Ui| {
                let response = ui.button("");
                if response.clicked() {
                    response.request_focus();
                }
                response
            })
            .selector(".button"),
        );
        *capture.lock().unwrap() = Some(response.rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.right() + 1.0, rect.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn keyboard_focus_transfer_removes_the_previous_widgets_focus_styles() {
    let stylesheet = Stylesheet::parse(
        ".button { width:48px; height:24px; margin:4px; background:white; }
         :focus { background:lime; }
         .button:focus-visible { outline:2px solid blue; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let mut release_tab = key(egui::Key::Tab);
    if let egui::Event::Key { pressed, .. } = &mut release_tab {
        *pressed = false;
    }
    let pixels = render_ui_pixels_with_events(
        stylesheet,
        vec![
            vec![],
            vec![key(egui::Key::Tab)],
            vec![release_tab],
            vec![key(egui::Key::Tab)],
            vec![],
        ],
        move |ui| {
            *capture.lock().unwrap() = ["First", "Second"]
                .map(|text| {
                    let response = ui.add(egui::Button::new(text).selector(".button"));
                    (response.rect, response.has_focus())
                })
                .to_vec();
        },
    )
    .await;
    let bounds = bounds.lock().unwrap();
    assert!(!bounds[0].1);
    assert!(bounds[1].1);
    for (index, (rect, _)) in bounds.iter().enumerate() {
        let ring = if index == 0 {
            [255, 255, 255, 255]
        } else {
            [0, 0, 255, 255]
        };
        assert_eq!(pixel(&pixels, rect.right() + 1.0, rect.center().y), ring);
        let fill = if index == 0 {
            [255, 255, 255, 255]
        } else {
            [0, 255, 0, 255]
        };
        assert_eq!(pixel(&pixels, rect.left() + 3.0, rect.center().y), fill);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_input_keeps_focus_visible_when_focused_by_pointer() {
    let stylesheet = Stylesheet::parse(
        ".field { width:64px; height:24px; margin:4px; background:white; }
         .field:focus { outline:2px solid red; }
         .field:focus-visible { outline-color:blue; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let mut text = String::new();
    let pixels = render_ui_pixels_with_events(stylesheet, pointer_focus_events(), move |ui| {
        let response = ui.add(egui::TextEdit::singleline(&mut text).selector(".field"));
        *capture.lock().unwrap() = Some((response.rect, response.has_focus()));
    })
    .await;
    let (rect, focused) = bounds.lock().unwrap().unwrap();
    assert!(focused);
    assert_eq!(
        pixel(&pixels, rect.right() + 1.0, rect.center().y),
        [0, 0, 255, 255]
    );
}
