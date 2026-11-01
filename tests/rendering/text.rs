use super::*;

fn shadow_color_count(pixels: &[[u8; 4]], rect: egui::Rect, channel: usize) -> usize {
    pixels
        .iter()
        .enumerate()
        .filter(|(index, pixel)| {
            let point = egui::pos2(
                (*index % WIDTH as usize) as f32,
                (*index / WIDTH as usize) as f32,
            );
            rect.contains(point)
                && (0..3)
                    .filter(|other| *other != channel)
                    .all(|other| pixel[channel] > pixel[other].saturating_add(8))
        })
        .count()
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_shadow_ink_does_not_change_layout_or_expand_hit_targets() {
    async fn render(
        shadow: &str,
        frames: Vec<Vec<egui::Event>>,
    ) -> (Vec<[u8; 4]>, egui::Rect, egui::Rect, bool) {
        let stylesheet = Stylesheet::parse(&format!(
            ".text {{ color:transparent; font-size:28px; width:max-content; text-shadow:{shadow}; }} .after {{ width:20px; height:12px; background:black; }}"
        )).unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_with_events(stylesheet, frames, move |ui| {
            let text = ui.add(
                egui::Label::new("MM")
                    .sense(egui::Sense::click())
                    .selector(".text"),
            );
            let after = ui.add(empty_response.selector(".after"));
            *capture.lock().unwrap() = Some((text.rect, after.rect, text.clicked()));
        })
        .await;
        let (text, after, clicked) = bounds.lock().unwrap().unwrap();
        (pixels, text, after, clicked)
    }
    let (_, control_text, control_after, _) = render("none", vec![Vec::new(), Vec::new()]).await;
    let shadow = "24px 0 4px red, -8px 6px 2px blue";
    let (pixels, text, after, _) = render(shadow, vec![Vec::new(), Vec::new()]).await;
    assert_eq!(
        text, control_text,
        "shadows changed the text's public bounds"
    );
    assert_eq!(
        after, control_after,
        "shadows changed following content placement"
    );
    let point = pixels
        .iter()
        .enumerate()
        .filter_map(|(index, pixel)| {
            let point = egui::pos2(
                (index % WIDTH as usize) as f32 + 0.5,
                (index / WIDTH as usize) as f32 + 0.5,
            );
            (point.x > text.right() + 2.0
                && pixel[0] > pixel[1].saturating_add(8)
                && pixel[0] > pixel[2].saturating_add(8))
            .then_some(point)
        })
        .max_by(|first, second| first.x.total_cmp(&second.x))
        .expect("no visible shadow beyond the text's public bounds");
    assert!(!text.contains(point));
    let button = |pressed| egui::Event::PointerButton {
        pos: point,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    let frames = vec![
        Vec::new(),
        vec![egui::Event::PointerMoved(point), button(true)],
        vec![egui::Event::PointerMoved(point), button(false)],
    ];
    let (_, _, _, control_clicked) = render("none", frames.clone()).await;
    assert!(
        !control_clicked,
        "the probe was within the control's hit region"
    );
    let (_, hit_text, hit_after, clicked) = render(shadow, frames).await;
    assert_eq!(hit_text, control_text);
    assert_eq!(hit_after, control_after);
    assert!(!clicked, "shadow-only ink expanded the text's click target");
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn pseudo_color_changes_resolve_inherited_currentcolor_shadows() {
    for (pseudo, reference) in [
        (
            ".text::first-letter { color:red; }",
            ".text::first-letter { text-shadow:0 70px red; }",
        ),
        (
            ".text::first-line { color:red; }",
            ".text::first-line { text-shadow:0 70px red; }",
        ),
        (
            ".text::first-line { color:red; } .text::first-letter { color:lime; }",
            ".text::first-line { text-shadow:0 70px red; } .text::first-letter { text-shadow:0 70px lime; }",
        ),
        (
            ".text::first-line { color:red; text-shadow:10px 70px currentColor; } .text::first-letter { color:lime; }",
            ".text::first-line { text-shadow:10px 70px red; } .text::first-letter { text-shadow:10px 70px lime; }",
        ),
        (
            ".text::first-line { color:red; } .text::first-letter { color:inherit; }",
            ".text::first-line { text-shadow:0 70px red; }",
        ),
        (
            ".text { text-shadow:0 70px rgb(0 255 0 / 25%), 0 70px currentColor; } .text::first-letter { color:red; }",
            ".text { text-shadow:0 70px rgb(0 255 0 / 25%), 0 70px blue; } .text::first-letter { text-shadow:0 70px rgb(0 255 0 / 25%), 0 70px red; }",
        ),
    ] {
        let base =
            ".text { color:blue; font-size:28px; width:65px; text-shadow:0 70px currentColor; }";
        let actual = render_ui_pixels(
            Stylesheet::parse(&format!("{base} {pseudo}")).unwrap(),
            |ui| {
                ui.add(egui::Label::new("MM MM").selector(".text"));
            },
        )
        .await;
        let expected = render_ui_pixels(
            Stylesheet::parse(&format!(
                "{base} .text {{ text-shadow:0 70px blue; }} {reference}"
            ))
            .unwrap(),
            |ui| {
                ui.add(egui::Label::new("MM MM").selector(".text"));
            },
        )
        .await;
        let shadow =
            egui::Rect::from_min_max(egui::pos2(0.0, 70.0), egui::pos2(WIDTH as f32, 100.0));
        assert!(shadow_color_count(&expected, shadow, 0) > 0);
        let differences = actual
            .iter()
            .zip(&expected)
            .enumerate()
            .filter(|(index, (actual, expected))| {
                (70..100).contains(&(index / WIDTH as usize)) && actual != expected
            })
            .count();
        assert_eq!(
            differences, 0,
            "pseudo shadow used the originating element's currentColor: {pseudo}"
        );
    }
    fn block_text(ui: &mut egui::Ui) {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("MM MM").selector(".text"));
                ui.response()
            })
            .selector(".block"),
        );
    }
    for (child, shadow) in [
        ("", "10px 70px red"),
        ("color:lime;", "10px 70px lime"),
        ("text-shadow:0 70px currentColor;", "0 70px red"),
        (
            "color:lime; text-shadow:0 70px currentColor;",
            "0 70px lime",
        ),
        ("animation:green 1s linear both;", "10px 70px lime"),
        ("animation:own-shadow 1s linear both;", "0 70px red"),
        ("color:initial;", "10px 70px black"),
        ("color:inherit;", "10px 70px red"),
        ("color:revert;", "10px 70px red"),
        ("text-shadow:none;", "none"),
        ("text-shadow:0 70px blue;", "0 70px blue"),
    ] {
        let base = format!(
            "@keyframes green {{ from,to {{ color:lime; }} }} @keyframes own-shadow {{ from,to {{ text-shadow:0 70px currentColor; }} }} .block {{ color:blue; font-size:28px; width:65px; text-shadow:0 70px currentColor; }} .text {{ display:block; width:65px; {child} }}"
        );
        let actual = render_ui_pixels(
            Stylesheet::parse(&format!(
                "{base} .block::first-line {{ color:red; text-shadow:10px 70px currentColor; }}"
            ))
            .unwrap(),
            block_text,
        )
        .await;
        let expected = render_ui_pixels(
            Stylesheet::parse(&format!(
                "{base} .text {{ text-shadow:{shadow} !important; animation:none !important; }}"
            ))
            .unwrap(),
            block_text,
        )
        .await;
        let region = 70 * WIDTH as usize..100 * WIDTH as usize;
        if shadow != "none" {
            assert!(
                expected[region.clone()]
                    .iter()
                    .any(|pixel| pixel[..3].iter().any(|channel| *channel < 128))
            );
        }
        assert_eq!(
            actual[region.clone()]
                .iter()
                .zip(&expected[region])
                .filter(|(actual, expected)| actual != expected)
                .count(),
            0,
            "descendant's own color/shadow precedence was lost: {child}"
        );
    }
    let base = ".block { color:blue; font-size:28px; width:65px; text-shadow:0 70px currentColor; } .block::first-line { color:red; } .text { width:65px; text-shadow:0 70px blue; }";
    let actual = render_ui_pixels(
        Stylesheet::parse(&format!(
            "{base} .text::first-letter {{ text-shadow:0 70px currentColor; }}"
        ))
        .unwrap(),
        block_text,
    )
    .await;
    let expected = render_ui_pixels(
        Stylesheet::parse(&format!(
            "{base} .text::first-letter {{ text-shadow:0 70px red; }}"
        ))
        .unwrap(),
        block_text,
    )
    .await;
    let region = 70 * WIDTH as usize..100 * WIDTH as usize;
    assert!(
        shadow_color_count(
            &expected,
            egui::Rect::from_min_max(egui::pos2(0.0, 70.0), egui::pos2(WIDTH as f32, 100.0)),
            0
        ) > 0
    );
    assert_eq!(
        actual[region.clone()]
            .iter()
            .zip(&expected[region])
            .filter(|(actual, expected)| actual != expected)
            .count(),
        0,
        "a descendant's pseudo lost inherited first-line color after specifying its own shadow list"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn max_content_wrapping_does_not_override_constrained_descendants() {
    fn nested_text(ui: &mut egui::Ui) {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("MMMM MM").selector(".child"));
                ui.response()
            })
            .selector(".parent"),
        );
    }
    fn contents_text(ui: &mut egui::Ui) {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("MMMM MM").selector(".contents"));
                ui.response()
            })
            .selector(".parent"),
        );
    }
    let base = ".parent { width:40px; } .contents { display:contents; font-size:20px; color:red; }";
    let reference = render_ui_pixels(Stylesheet::parse(base).unwrap(), contents_text).await;
    let actual = render_ui_pixels(
        Stylesheet::parse(&format!("{base} .contents {{ width:max-content; }}")).unwrap(),
        contents_text,
    )
    .await;
    assert!(shadow_color_count(&reference, egui::Rect::EVERYTHING, 0) > 0);
    assert_eq!(
        actual
            .iter()
            .zip(&reference)
            .filter(|(actual, expected)| actual != expected)
            .count(),
        0,
        "width on a contents element changed anonymous text wrapping"
    );
    for child in ["width:40px;", "width:max-content; max-width:40px;"] {
        let base = format!(".child {{ {child} color:red; font-size:28px; white-space:normal; }}");
        let reference = render_ui_pixels(Stylesheet::parse(&base).unwrap(), nested_text).await;
        let actual = render_ui_pixels(
            Stylesheet::parse(&format!("{base} .parent {{ width:max-content; }}")).unwrap(),
            nested_text,
        )
        .await;
        assert!(shadow_color_count(&reference, egui::Rect::EVERYTHING, 0) > 0);
        assert_eq!(
            actual
                .iter()
                .zip(&reference)
                .filter(|(actual, expected)| actual != expected)
                .count(),
            0,
            "max-content ancestor changed descendant wrapping: {child}"
        );
    }
    for parent in [
        "display:flex; width:60px;",
        "display:grid; grid-template-columns:minmax(0,1fr); width:60px;",
    ] {
        let stylesheet = Stylesheet::parse(&format!(".parent {{ {parent} }} .middle {{ display:contents; }} .child {{ color:red; font-size:20px; width:max-content; min-width:0; }}")).unwrap();
        let render = |stylesheet: Stylesheet, contents: bool| {
            render_ui_pixels(stylesheet, move |ui| {
                ui.add(
                    (move |ui: &mut egui::Ui| {
                        if contents {
                            ui.add(
                                (|ui: &mut egui::Ui| {
                                    ui.add(egui::Label::new("MMMM MM").selector(".child"));
                                    ui.response()
                                })
                                .selector(".middle"),
                            );
                        } else {
                            ui.add(egui::Label::new("MMMM MM").selector(".child"));
                        }
                        ui.response()
                    })
                    .selector(".parent"),
                );
            })
        };
        let reference = render(stylesheet.clone(), false).await;
        let actual = render(stylesheet, true).await;
        assert!(shadow_color_count(&reference, egui::Rect::EVERYTHING, 0) > 0);
        assert_eq!(
            actual
                .iter()
                .zip(&reference)
                .filter(|(actual, expected)| actual != expected)
                .count(),
            0,
            "display:contents changed the child's formatting parent: {parent}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn shadow_animations_do_not_start_transitions_or_leave_them_after_finishing() {
    for transition in ["text-shadow .4s linear", "all .4s linear"] {
        for frames in [3, 10] {
            let stylesheet = Stylesheet::parse(&format!(
                "@keyframes shadows {{ from {{ text-shadow:none; }} to {{ text-shadow:16px 0 blue; }} }}\
                 .text {{ color:transparent; font-size:28px; width:120px; text-shadow:none; animation:shadows .8s linear; transition:{transition}; }}"
            )).unwrap();
            let actual = render_ui_pixels_with_events(stylesheet, vec![Vec::new(); frames], |ui| {
                ui.add(egui::Label::new("MM").selector(".text"));
            })
            .await;
            let shadow = if frames == 3 {
                "4px 0 rgb(0 0 255 / 25%)"
            } else {
                "none"
            };
            let stylesheet = Stylesheet::parse(&format!(
                ".text {{ color:transparent; font-size:28px; width:120px; text-shadow:{shadow}; }}"
            ))
            .unwrap();
            let reference = render_ui_pixels(stylesheet, |ui| {
                ui.add(egui::Label::new("MM").selector(".text"));
            })
            .await;
            if frames == 3 {
                assert!(shadow_color_count(&reference, egui::Rect::EVERYTHING, 2) > 0);
            }
            assert_eq!(
                actual
                    .iter()
                    .zip(&reference)
                    .filter(|(actual, expected)| actual != expected)
                    .count(),
                0,
                "animation incorrectly triggered {transition}, frames={frames}"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn animated_shadow_currentcolor_uses_the_running_color_transition() {
    for (from, to, target_font, expected_font, expected_offset) in [
        ("0 0 currentColor", "16px 0 blue", 28, 28, 4.0),
        ("1em 0 currentColor", "2em 0 blue", 56, 35, 43.75),
    ] {
        let stylesheet = Stylesheet::parse(&format!(
        "@keyframes shadows {{ from {{ text-shadow:{from}; }} to {{ text-shadow:{to}; }} }}\
         .text {{ color:red; font-size:28px; width:120px; animation:shadows .8s linear both; transition:color .4s linear, font-size .4s linear; }}\
         .text:hover {{ color:blue; font-size:{target_font}px; }}"
    )).unwrap();
        let hovered = std::sync::Arc::new(Mutex::new(false));
        let capture = hovered.clone();
        let actual = render_ui_pixels_with_events(
            stylesheet,
            vec![
                Vec::new(),
                vec![egui::Event::PointerMoved(egui::pos2(10.0, 10.0))],
                vec![egui::Event::PointerMoved(egui::pos2(10.0, 10.0))],
            ],
            move |ui| {
                let response = ui.add(
                    (|ui: &mut egui::Ui| {
                        let text = ui.label("MM");
                        ui.interact(
                            text.rect,
                            egui::Id::unique("animated-shadow-color-transition"),
                            egui::Sense::hover(),
                        )
                    })
                    .selector(".text"),
                );
                *capture.lock().unwrap() = response.hovered();
            },
        )
        .await;
        assert!(
            *hovered.lock().unwrap(),
            "the text hover surface was not exercised"
        );
        let reference = render_ui_pixels(Stylesheet::parse(&format!(
        ".text {{ color:rgb(191 0 64); font-size:{expected_font}px; width:120px; text-shadow:{expected_offset}px 0 rgb(143 0 112); }}"
    )).unwrap(), |ui| { ui.add(egui::Label::new("MM").selector(".text")); }).await;
        assert!(
            reference
                .iter()
                .any(|pixel| pixel[2] > pixel[1].saturating_add(8))
        );
        assert_eq!(
            actual
                .iter()
                .zip(&reference)
                .filter(|(actual, expected)| actual != expected)
                .count(),
            0,
            "shadow did not use the running color/font transitions: {from} to {to}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_shadow_keyframes_interpolate_computed_lists_with_transparent_padding() {
    for (from, to, expected) in [
        (
            "",
            "12px 4px 4px rgb(0 0 255 / 80%)",
            "3px 1px 1px rgb(0 0 255 / 20%)",
        ),
        (
            "12px 4px 4px rgb(0 0 255 / 80%)",
            "",
            "9px 3px 3px rgb(0 0 255 / 60%)",
        ),
        (
            "none",
            "12px 4px 4px rgb(0 0 255 / 80%)",
            "3px 1px 1px rgb(0 0 255 / 20%)",
        ),
        (
            "4px 0 red",
            "12px 0 red, 40px 0 blue",
            "6px 0 red, 10px 0 rgb(0 0 255 / 25%)",
        ),
        (
            "0 0 red, 40px 0 blue",
            "8px 0 red",
            "2px 0 red, 30px 0 rgb(0 0 255 / 75%)",
        ),
        ("1em 0 red", "56px 0 red", "35px 0 red"),
        (
            "1em 0 red; font-size:28px",
            "2em 0 red; font-size:56px",
            "43.75px 0 red; font-size:35px",
        ),
        (
            "1em 0 red; font-size:28px; max-width:40px",
            "2em 0 red; font-size:56px; max-width:40px",
            "43.75px 0 red; font-size:35px; max-width:40px",
        ),
        (
            "0 0 currentColor; color:red",
            "12px 0 currentColor; color:blue",
            "3px 0 rgb(191 0 64); color:rgb(191 0 64)",
        ),
    ] {
        let base = ".text { color:transparent; font-size:28px; width:max-content; }";
        let from_rule = if from.is_empty() {
            String::new()
        } else {
            format!("from {{ text-shadow:{from}; }}")
        };
        let to_rule = if to.is_empty() {
            String::new()
        } else {
            format!("to {{ text-shadow:{to}; }}")
        };
        let stylesheet = Stylesheet::parse(&format!(
            "{base} @keyframes shadows {{ {from_rule} {to_rule} }}\
             .text {{ animation:shadows 2s linear both; }}"
        ))
        .unwrap();
        let actual = render_ui_pixels_at_time(stylesheet, 0.5, |ui| {
            ui.add(egui::Label::new("MM").selector(".text"));
        })
        .await;
        let stylesheet =
            Stylesheet::parse(&format!("{base} .text {{ text-shadow:{expected}; }}")).unwrap();
        let reference = render_ui_pixels_at_time(stylesheet, 0.5, |ui| {
            ui.add(egui::Label::new("MM").selector(".text"));
        })
        .await;
        assert!(
            reference
                .iter()
                .any(|pixel| pixel[0] != pixel[1] || pixel[1] != pixel[2])
        );
        let differences = actual
            .iter()
            .zip(&reference)
            .filter(|(actual, expected)| actual != expected)
            .count();
        assert_eq!(differences, 0, "interpolation from {from} to {to}");
    }

    fn nested_text(ui: &mut egui::Ui) {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("MM").selector(".text"));
                ui.response()
            })
            .selector(".parent"),
        );
    }
    for from in [
        "",
        "from { text-shadow:inherit; }",
        "from { text-shadow:revert; }",
    ] {
        let base = ".parent { font-size:16px; text-shadow:1em 0 red; } .text { color:transparent; font-size:28px; width:max-content; }";
        let own = if from.contains("revert") {
            ".text { text-shadow:30px 0 blue; }"
        } else {
            ""
        };
        let stylesheet = Stylesheet::parse(&format!("{base} {own} @keyframes shadows {{ {from} to {{ text-shadow:12px 0 blue; }} }} .text {{ animation:shadows 2s linear both; }}")).unwrap();
        let actual = render_ui_pixels_at_time(stylesheet, 0.5, nested_text).await;
        let stylesheet = Stylesheet::parse(&format!(
            "{base} .text {{ text-shadow:15px 0 rgb(191 0 64); }}"
        ))
        .unwrap();
        let reference = render_ui_pixels_at_time(stylesheet, 0.5, nested_text).await;
        assert!(shadow_color_count(&reference, egui::Rect::EVERYTHING, 0) > 0);
        assert_eq!(
            actual
                .iter()
                .zip(&reference)
                .filter(|(actual, expected)| actual != expected)
                .count(),
            0,
            "inherited computed endpoint was re-evaluated at the child's font size: {from}"
        );
    }
    for important in [
        "text-shadow:0 0 red !important;",
        "--shadow:0 0 red; text-shadow:var(--shadow) !important;",
    ] {
        let base = ".text { color:transparent; font-size:28px; width:max-content; }";
        let stylesheet = Stylesheet::parse(&format!("{base} .text {{ {important} animation:shadows 2s linear both; }} @keyframes shadows {{ from {{ text-shadow:none; }} to {{ text-shadow:12px 0 blue; }} }}")).unwrap();
        let actual = render_ui_pixels_at_time(stylesheet, 0.5, |ui| {
            ui.add(egui::Label::new("MM").selector(".text"));
        })
        .await;
        let stylesheet =
            Stylesheet::parse(&format!("{base} .text {{ text-shadow:0 0 red; }}")).unwrap();
        let reference = render_ui_pixels(stylesheet, |ui| {
            ui.add(egui::Label::new("MM").selector(".text"));
        })
        .await;
        assert!(shadow_color_count(&reference, egui::Rect::EVERYTHING, 0) > 0);
        assert_eq!(
            actual
                .iter()
                .zip(&reference)
                .filter(|(actual, expected)| actual != expected)
                .count(),
            0,
            "animation overrode {important}"
        );
    }
    for (first, second, expected) in [
        (
            "from,to { text-shadow:4px 0 red; }",
            "to { text-shadow:20px 0 blue; }",
            "8px 0 rgb(191 0 64)",
        ),
        (
            "from { text-shadow:none; } to { text-shadow:4px 0 red; }",
            "from,to { text-shadow:20px 0 blue; }",
            "20px 0 blue",
        ),
    ] {
        let base =
            ".text { color:transparent; font-size:28px; width:max-content; text-shadow:none; }";
        let stylesheet = Stylesheet::parse(&format!("{base} @keyframes first {{ {first} }} @keyframes second {{ {second} }} .text {{ animation:first 2s linear both, second 2s linear both; }}")).unwrap();
        let actual = render_ui_pixels_at_time(stylesheet, 0.5, |ui| {
            ui.add(egui::Label::new("MM").selector(".text"));
        })
        .await;
        let stylesheet =
            Stylesheet::parse(&format!("{base} .text {{ text-shadow:{expected}; }}")).unwrap();
        let reference = render_ui_pixels(stylesheet, |ui| {
            ui.add(egui::Label::new("MM").selector(".text"));
        })
        .await;
        assert!(reference.iter().any(|pixel| pixel[0] != pixel[2]));
        assert_eq!(
            actual
                .iter()
                .zip(&reference)
                .filter(|(actual, expected)| actual != expected)
                .count(),
            0,
            "animation list {first} then {second}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_shadow_transitions_interpolate_padded_lists_on_hover() {
    for (from, to, expected) in [
        (
            "0 0 currentColor; color:red",
            "12px 0 blue; color:red",
            "3px 0 rgb(191 0 64); color:red",
        ),
        (
            "none",
            "12px 4px 4px rgb(0 0 255 / 80%)",
            "3px 1px 1px rgb(0 0 255 / 20%)",
        ),
        (
            "4px 0 red",
            "12px 0 red, 40px 0 blue",
            "6px 0 red, 10px 0 rgb(0 0 255 / 25%)",
        ),
        (
            "0 0 red, 40px 0 blue",
            "8px 0 red",
            "2px 0 red, 30px 0 rgb(0 0 255 / 75%)",
        ),
    ] {
        for reverse in [false, true] {
            let stylesheet = Stylesheet::parse(&format!(
        ".text {{ color:transparent; font-size:28px; width:max-content; text-shadow:{from}; transition:text-shadow .4s linear; }}\
         .text:hover {{ text-shadow:{to}; }}"
    )).unwrap();
            let hovered = std::sync::Arc::new(Mutex::new(false));
            let capture = hovered.clone();
            let mut frames = vec![
                Vec::new(),
                vec![egui::Event::PointerMoved(egui::pos2(10.0, 10.0))],
                vec![egui::Event::PointerMoved(egui::pos2(10.0, 10.0))],
            ];
            if reverse {
                frames.extend([
                    vec![egui::Event::PointerMoved(egui::pos2(200.0, 100.0))],
                    vec![egui::Event::PointerMoved(egui::pos2(200.0, 100.0))],
                ]);
            }
            let actual = render_ui_pixels_with_events(stylesheet, frames, move |ui| {
                let response = ui.add(
                    (|ui: &mut egui::Ui| {
                        let text = ui.label("MM");
                        ui.interact(
                            text.rect,
                            egui::Id::unique("text-shadow-transition"),
                            egui::Sense::hover(),
                        )
                    })
                    .selector(".text"),
                );
                *capture.lock().unwrap() = response.hovered();
            })
            .await;
            assert_eq!(
                *hovered.lock().unwrap(),
                !reverse,
                "the text hover surface did not reach the expected state"
            );
            let stylesheet = Stylesheet::parse(&format!(
        ".text {{ color:transparent; font-size:28px; width:max-content; text-shadow:{expected}; }}"
    )).unwrap();
            let reference = render_ui_pixels(stylesheet, |ui| {
                ui.add(egui::Label::new("MM").selector(".text"));
            })
            .await;
            assert!(
                reference
                    .iter()
                    .any(|pixel| pixel[2] > pixel[1].saturating_add(8))
            );
            let differences = actual
                .iter()
                .zip(&reference)
                .filter(|(actual, expected)| actual != expected)
                .count();
            assert_eq!(
                differences, 0,
                "transition from {from} to {to}, reverse={reverse}"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn pseudo_shadows_replace_base_shadows_and_preserve_inherited_alpha() {
    async fn render(style: &str, text: &'static str) -> (Vec<[u8; 4]>, egui::Rect) {
        let stylesheet = Stylesheet::parse(style).unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            *capture.lock().unwrap() = Some(ui.add(egui::Label::new(text).selector(".text")).rect);
        })
        .await;
        (pixels, bounds.lock().unwrap().unwrap())
    }

    for keyword in ["none", "initial"] {
        for decoration in ["none", "underline", "overline", "line-through"] {
            for (writing, orientation) in [
                ("horizontal-tb", "mixed"),
                ("vertical-rl", "mixed"),
                ("vertical-lr", "mixed"),
                ("vertical-rl", "upright"),
                ("sideways-rl", "mixed"),
                ("sideways-lr", "mixed"),
            ] {
                let vertical = writing != "horizontal-tb";
                let offset = if vertical { "4px 0" } else { "0 4px" };
                let (pixels, rect) = render(
        &format!(
            ".text {{ color:transparent; font-size:32px; width:max-content; height:max-content; writing-mode:{writing}; text-orientation:{orientation}; text-shadow:{offset} red; text-decoration:{decoration} lime 2px; }}\
             .text::first-letter {{ text-shadow:{keyword}; }}"
        ),
        "MM",
    )
    .await;
                let (mut first, mut second) = if vertical {
                    (
                        egui::Rect::from_min_max(
                            rect.min,
                            egui::pos2(rect.right() + 8.0, rect.center().y),
                        ),
                        egui::Rect::from_min_max(
                            egui::pos2(rect.left(), rect.center().y),
                            rect.max + egui::vec2(8.0, 0.0),
                        ),
                    )
                } else {
                    (
                        egui::Rect::from_min_max(
                            rect.min,
                            egui::pos2(rect.center().x, rect.bottom() + 8.0),
                        ),
                        egui::Rect::from_min_max(
                            egui::pos2(rect.center().x, rect.top()),
                            rect.max + egui::vec2(0.0, 8.0),
                        ),
                    )
                };
                if writing == "sideways-lr" {
                    std::mem::swap(&mut first, &mut second);
                }
                assert_eq!(
                    shadow_color_count(&pixels, first, 0),
                    0,
                    "{keyword} retained {decoration} shadow in {writing}/{orientation}, rect={rect:?}"
                );
                assert!(shadow_color_count(&pixels, second, 0) > 0);
            }
        }
    }

    let (pixels, rect) = render(
        ".text { color:transparent; font-size:32px; width:max-content; text-shadow:0 0 red; }\
         .text::first-letter { text-shadow:0 0 lime, 0 0 blue; }",
        "MM",
    )
    .await;
    let first = egui::Rect::from_min_max(rect.min, egui::pos2(rect.center().x, rect.bottom()));
    let second = egui::Rect::from_min_max(egui::pos2(rect.center().x, rect.top()), rect.max);
    assert!(shadow_color_count(&pixels, first, 1) > 0);
    assert_eq!(shadow_color_count(&pixels, first, 0), 0);
    assert_eq!(shadow_color_count(&pixels, first, 2), 0);
    assert!(shadow_color_count(&pixels, second, 0) > 0);

    for (letter, expected_blue) in [
        ("", false),
        (".text::first-letter { text-shadow:-10px 0 blue; }", true),
    ] {
        let (pixels, rect) = render(
            &format!(
                ".text {{ color:transparent; font-size:24px; width:64px; text-shadow:0 0 red; }}\
                 .text::first-line {{ text-shadow:10px 0 lime; }} {letter}"
            ),
            "MMMMMM",
        )
        .await;
        assert!(rect.height() > 40.0);
        let first = egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.top() + 28.0));
        let following =
            egui::Rect::from_min_max(egui::pos2(rect.left(), rect.top() + 32.0), rect.max);
        assert!(shadow_color_count(&pixels, first, 1) > 0);
        assert_eq!(shadow_color_count(&pixels, first, 0), 0);
        assert_eq!(shadow_color_count(&pixels, first, 2) > 0, expected_blue);
        assert!(shadow_color_count(&pixels, following, 0) > 0);
    }

    let base = ".text { color:transparent; font-size:28px; width:max-content; text-shadow:0 0 rgb(255 0 0 / 50%); }";
    let (expected, _) = render(base, "MM").await;
    for declaration in [
        "text-shadow:inherit",
        "text-shadow:unset",
        "text-shadow:revert",
        "all:inherit",
        "all:unset",
        "all:revert",
    ] {
        let (actual, _) = render(
            &format!("{base} .text::first-letter {{ {declaration}; }}"),
            "MM",
        )
        .await;
        assert_eq!(actual, expected, "{declaration} changed inherited alpha");
    }

    let stylesheet = Stylesheet::parse(
        ".text { color:transparent; font-size:32px; width:max-content; white-space:nowrap; text-decoration:underline lime 2px; }\
         .text::first-letter { text-shadow:0 4px red; }",
    ).unwrap();
    let text = "M".repeat(2048);
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.set_clip_rect(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(100_000.0, 100.0),
        ));
        ui.add(egui::Label::new(text.clone()).selector(".text"));
    })
    .await;
    assert!(shadow_color_count(&pixels, egui::Rect::EVERYTHING, 1) > 0);
    assert!(
        shadow_color_count(&pixels, egui::Rect::EVERYTHING, 0) > 0,
        "a one-letter shadow was dropped for the whole decoration's mask size"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn ancestor_pseudo_shadows_respect_explicit_descendant_values() {
    for first_class in [".text", ".text.explicit", ".text.animated"] {
        let stylesheet = Stylesheet::parse(
            "@keyframes blue-shadow { from,to { text-shadow:0 0 blue; } }\
             .block { display:block; width:100px; color:transparent; font-size:32px; text-shadow:0 0 red; }\
             .text { display:block; }\
             .explicit { text-shadow:0 0 blue; }\
             .animated { animation:blue-shadow 1s linear both; }\
             .block::first-line { text-shadow:12px 0 lime; }",
        )
        .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    *capture.lock().unwrap() = [first_class, ".text"]
                        .map(|selector| ui.add(egui::Label::new("MM").selector(selector)).rect)
                        .to_vec();
                    ui.response()
                })
                .selector(".block"),
            );
        })
        .await;
        let bounds = bounds.lock().unwrap();
        let expected = if first_class == ".text" { 1 } else { 2 };
        assert!(shadow_color_count(&pixels, bounds[0], expected) > 0);
        assert_eq!(shadow_color_count(&pixels, bounds[0], 0), 0);
        assert_eq!(shadow_color_count(&pixels, bounds[0], 3 - expected), 0);
        assert!(shadow_color_count(&pixels, bounds[1], 0) > 0);
        assert_eq!(shadow_color_count(&pixels, bounds[1], 1), 0);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_shadows_respect_all_clip_boundaries() {
    for offset in ["-16px 0", "16px 0", "0 -16px", "0 16px"] {
        let stylesheet = Stylesheet::parse(&format!(
            ".clip {{ width:48px; height:48px; overflow:hidden; }}\
             .text {{ color:transparent; font-size:64px; white-space:nowrap; text-shadow:{offset} red; }}"
        ))
        .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            let clip = ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(egui::Label::new("M").selector(".text"));
                    ui.response()
                })
                .selector(".clip"),
            );
            *capture.lock().unwrap() = Some(clip.rect);
        })
        .await;
        let rect = bounds.lock().unwrap().unwrap();
        let inside = shadow_color_count(&pixels, rect, 0);
        assert!(inside > 0, "no shadow inside the clip for {offset}");
        assert_eq!(
            shadow_color_count(&pixels, egui::Rect::EVERYTHING, 0),
            inside,
            "shadow escaped the clip for {offset}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn pseudo_text_shadows_are_included_in_element_images() {
    let stylesheet = Stylesheet::parse(
        "#source { position:relative; z-index:0; width:60px; height:40px; color:transparent; font-size:32px; text-shadow:0 0 red; }\
         #source::first-letter { text-shadow:0 0 lime; }\
         .target { width:60px; height:40px; background:white element(#source) left top / 60px 40px no-repeat; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let source = ui.add(egui::Label::new("MM").selector("#source")).rect;
        let target = ui.add(empty_response.selector(".target")).rect;
        *capture.lock().unwrap() = Some((source, target));
    })
    .await;
    let (source, target) = bounds.lock().unwrap().unwrap();
    assert!(shadow_color_count(&pixels, source, 0) > 0);
    assert!(shadow_color_count(&pixels, source, 1) > 0);
    for y in 0..40 {
        for x in 0..60 {
            assert_eq!(
                pixel(&pixels, source.left() + x as f32, source.top() + y as f32),
                pixel(&pixels, target.left() + x as f32, target.top() + y as f32),
                "element image differs at {x},{y}"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_shadow_changes_update_element_images_in_the_same_frame() {
    for (change, target_first) in [
        (".container:hover::first-line", false),
        (".container:hover::first-line", true),
        (".container:hover", true),
        (".container", true),
        ("#source", true),
    ] {
        let declaration = match change {
            ".container" => "animation:shadow-update .4s linear both",
            "#source" => "animation:release-source .4s linear both",
            _ => "text-shadow:0 0 lime",
        };
        let stylesheet = Stylesheet::parse(&format!(
        "@keyframes shadow-update {{ from {{ text-shadow:0 0 red; }} 50%,to {{ text-shadow:0 0 lime; }} }}\
         @keyframes release-source {{ from {{ z-index:0; }} 50%,to {{ z-index:auto; }} }}\
         .container {{ width:80px; height:80px; color:transparent; font-size:32px; text-shadow:0 0 red; }}\
         {change} {{ {declaration}; }}\
         #source {{ position:relative; z-index:0; width:60px; height:40px; }}\
         .target {{ width:60px; height:40px; background:white element(#source) left top / 60px 40px no-repeat; }}"
    ))
    .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_with_events(
            stylesheet,
            vec![
                Vec::new(),
                vec![egui::Event::PointerMoved(egui::pos2(75.0, 10.0))],
                vec![egui::Event::PointerMoved(egui::pos2(75.0, 10.0))],
            ],
            move |ui| {
                let mut rects = None;
                let response = ui.add(
                    (|ui: &mut egui::Ui| {
                        let early_target =
                            target_first.then(|| ui.add(empty_response.selector(".target")).rect);
                        let source = ui.add(egui::Label::new("MM").selector("#source")).rect;
                        let target = early_target
                            .unwrap_or_else(|| ui.add(empty_response.selector(".target")).rect);
                        rects = Some((source, target));
                        ui.interact(
                            egui::Rect::from_min_size(ui.min_rect().min, egui::vec2(80.0, 80.0)),
                            egui::Id::unique("pseudo-shadow-parent-surface"),
                            egui::Sense::hover(),
                        )
                    })
                    .selector(".container"),
                );
                let (source, target) = rects.unwrap();
                *capture.lock().unwrap() = Some((response.hovered(), source, target));
            },
        )
        .await;
        let (hovered, source, target) = bounds.lock().unwrap().unwrap();
        assert!(hovered, "the ancestor hover surface was not exercised");
        if change == "#source" {
            assert!(shadow_color_count(&pixels, source, 0) > 0);
            assert_eq!(
                shadow_color_count(&pixels, target, 0),
                0,
                "removed source image remained visible"
            );
            continue;
        }
        assert!(shadow_color_count(&pixels, source, 1) > 0);
        assert_eq!(shadow_color_count(&pixels, source, 0), 0);
        assert!(
            shadow_color_count(&pixels, target, 1) > 0,
            "stale target for {change}, target_first={target_first}"
        );
        assert_eq!(shadow_color_count(&pixels, target, 0), 0);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn named_flow_pseudo_shadows_follow_the_logical_first_line() {
    for height in [24, 50] {
        let stylesheet = Stylesheet::parse(&format!(
        ".row {{ display:flex; width:150px; }}\
         .region {{ display:block; width:70px; height:{height}px; flow-from:article; }}\
         .source {{ flow-into:article content; font-size:20px; color:transparent; text-shadow:0 0 red; }}\
         .source::first-line {{ text-shadow:0 0 lime; }}"
    ))
    .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(
                        egui::Label::new("one two three four five six seven eight nine ten")
                            .selector(".source"),
                    );
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            *capture.lock().unwrap() = [".region", ".region"]
                                .map(|selector| ui.add(empty_response.selector(selector)).rect)
                                .to_vec();
                            ui.response()
                        })
                        .selector(".row"),
                    );
                    ui.response()
                })
                .selector(".container"),
            );
        })
        .await;
        let bounds = bounds.lock().unwrap();
        near(bounds[0].top(), bounds[1].top());
        assert!(bounds[1].left() >= bounds[0].right());
        let first = egui::Rect::from_min_max(
            bounds[0].min,
            egui::pos2(bounds[0].right(), bounds[0].top() + 24.0),
        );
        let following = egui::Rect::from_min_max(
            egui::pos2(bounds[0].left(), bounds[0].top() + 28.0),
            bounds[0].max,
        );
        assert!(shadow_color_count(&pixels, first, 1) > 0);
        assert_eq!(shadow_color_count(&pixels, first, 0), 0);
        if height == 50 {
            assert!(shadow_color_count(&pixels, following, 0) > 0);
        }
        assert!(
            shadow_color_count(&pixels, bounds[1], 0) > 0,
            "second region {:?} has {} red and {} green shadow pixels",
            bounds[1],
            shadow_color_count(&pixels, bounds[1], 0),
            shadow_color_count(&pixels, bounds[1], 1),
        );
        assert_eq!(shadow_color_count(&pixels, bounds[1], 1), 0);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn text_shadow_values_validate_lists_lengths_colors_and_css_wide_keywords() {
    for declaration in [
        "text-shadow:none",
        "text-shadow:-1em 2px 3px currentColor, rgb(0 0 255) calc(1px + 1px) -2px 0",
        "text-shadow:inherit",
        "text-shadow:initial",
        "text-shadow:unset",
        "text-shadow:revert",
    ] {
        assert!(Stylesheet::parse(&format!(".text {{ {declaration}; }}")).is_ok());
        assert!(InlineStyle::parse(declaration).is_ok());
    }
    for declaration in [
        "text-shadow:1px",
        "text-shadow:1px 2px -1px",
        "text-shadow:1px 2px 3px 4px",
        "text-shadow:red blue 1px 2px",
        "text-shadow:none, 1px 2px",
        "text-shadow:10% 0",
    ] {
        assert!(Stylesheet::parse(&format!(".text {{ {declaration}; }}")).is_err());
        assert!(InlineStyle::parse(declaration).is_err());
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_shadow_offsets_inherit_currentcolor_and_preserve_layer_order() {
    let stylesheet = Stylesheet::parse(
        ".parent { color:red; text-shadow:14px 0 currentColor; }\
         .default-parent { color:red; text-shadow:14px 0; }\
         .child { color:blue; font-size:24px; width:max-content; }\
         .default-child { color:blue; font-size:24px; width:max-content; }\
         .stacked { color:transparent; font-size:24px; width:max-content; text-shadow:0 0 red, 0 0 blue; }\
         .variable { --shadow:12px 0 lime; color:transparent; font-size:24px; width:max-content; text-shadow:var(--shadow); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add_space(20.0);
        let mut child = egui::Rect::NOTHING;
        let mut default_child = egui::Rect::NOTHING;
        ui.add(
            (|ui: &mut egui::Ui| {
                child = ui.add(egui::Label::new("W").selector(".child")).rect;
                ui.response()
            })
            .selector(".parent"),
        );
        ui.add(
            (|ui: &mut egui::Ui| {
                default_child = ui
                    .add(egui::Label::new("W").selector(".default-child"))
                    .rect;
                ui.response()
            })
            .selector(".default-parent"),
        );
        let stacked = ui.add(egui::Label::new("M").selector(".stacked")).rect;
        let variable = ui.add(egui::Label::new("V").selector(".variable")).rect;
        *capture.lock().unwrap() = Some((child, default_child, stacked, variable));
    })
    .await;
    let (child, default_child, stacked, variable) = bounds.lock().unwrap().unwrap();
    let inherited_blue_pixels = pixels
        .iter()
        .enumerate()
        .filter(|(index, pixel)| {
            let x = index % WIDTH as usize;
            let y = index / WIDTH as usize;
            x as f32 >= child.right() + 2.0
                && y as f32 >= child.top() - 2.0
                && y as f32 <= child.bottom() + 2.0
                && pixel[2] > pixel[0].saturating_add(8)
                && pixel[2] > pixel[1].saturating_add(8)
        })
        .count();
    assert!(inherited_blue_pixels > 0);
    assert!(pixels.iter().enumerate().all(|(index, pixel)| {
        let x = index % WIDTH as usize;
        let y = index / WIDTH as usize;
        (x as f32) < child.right() + 2.0
            || (y as f32) < child.top() - 2.0
            || (y as f32) > child.bottom() + 2.0
            || pixel[0] <= pixel[2].saturating_add(8)
    }));
    assert!(pixels.iter().enumerate().any(|(index, pixel)| {
        let x = index % WIDTH as usize;
        let y = index / WIDTH as usize;
        x as f32 >= default_child.right() + 2.0
            && y as f32 >= default_child.top() - 2.0
            && y as f32 <= default_child.bottom() + 2.0
            && pixel[2] > pixel[0].saturating_add(8)
            && pixel[2] > pixel[1].saturating_add(8)
    }));

    let (red_pixels, blue_pixels) = pixels
        .iter()
        .enumerate()
        .filter(|(index, pixel)| {
            let x = index % WIDTH as usize;
            let y = index / WIDTH as usize;
            x as f32 >= stacked.left() - 2.0
                && x as f32 <= stacked.right() + 2.0
                && y as f32 >= stacked.top() - 2.0
                && y as f32 <= stacked.bottom() + 2.0
                && pixel[3] > 0
        })
        .fold((0, 0), |(red, blue), (_, pixel)| {
            (
                red + usize::from(pixel[0] > pixel[2].saturating_add(8)),
                blue + usize::from(pixel[2] > pixel[0].saturating_add(8)),
            )
        });
    assert!(red_pixels > 0);
    assert_eq!(blue_pixels, 0);

    assert!(pixels.iter().enumerate().any(|(index, pixel)| {
        let x = index % WIDTH as usize;
        let y = index / WIDTH as usize;
        x as f32 >= variable.right() + 2.0
            && y as f32 >= variable.top() - 2.0
            && y as f32 <= variable.bottom() + 2.0
            && pixel[1] > pixel[0].saturating_add(8)
            && pixel[1] > pixel[2].saturating_add(8)
    }));
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_shadow_blur_reaches_pixels_beyond_the_unblurred_glyph_bounds() {
    let stylesheet = Stylesheet::parse(
        ".text { color:transparent; font-size:28px; width:max-content; text-shadow:0 0 8px blue; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add_space(24.0);
        *capture.lock().unwrap() = Some(ui.add(egui::Label::new("M").selector(".text")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let fringe = pixels
        .iter()
        .enumerate()
        .filter(|(index, pixel)| {
            let x = index % WIDTH as usize;
            let y = index / WIDTH as usize;
            ((x as f32) < rect.left() - 2.0
                || (x as f32) > rect.right() + 2.0
                || (y as f32) < rect.top() - 2.0
                || (y as f32) > rect.bottom() + 2.0)
                && pixel[2] > pixel[0].saturating_add(8)
        })
        .count();
    assert!(fringe > 0);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_shadow_preserves_partial_alpha_after_blurring() {
    async fn shadow_pixels(color: &str) -> Vec<[u8; 4]> {
        let stylesheet = Stylesheet::parse(&format!(
            ".text {{ color:transparent; font-size:28px; width:max-content; text-shadow:0 0 8px {color}; }}"
        ))
        .unwrap();
        render_ui_pixels(stylesheet, |ui| {
            ui.add_space(24.0);
            ui.add(egui::Label::new("M").selector(".text"));
        })
        .await
    }

    let half_alpha = shadow_pixels("rgb(0 0 255 / 50%)").await;
    let opaque = shadow_pixels("blue").await;
    assert!(half_alpha.iter().zip(&opaque).any(|(half_alpha, opaque)| {
        opaque[0] < 200
            && half_alpha[0] > opaque[0].saturating_add(10)
            && half_alpha[0] < 255
            && half_alpha[2] == 255
    }));
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_shadow_rasterizes_underline_overline_and_strikethrough_geometry() {
    async fn blue_shadow_pixels(decoration: &str) -> Vec<[u8; 4]> {
        let stylesheet = Stylesheet::parse(&format!(
            ".text {{ color:transparent; font-size:28px; width:max-content; text-shadow:0 4px blue; {decoration} }}"
        ))
        .unwrap();
        render_ui_pixels(stylesheet, |ui| {
            ui.add_space(24.0);
            ui.add(egui::Label::new("M M").selector(".text"));
        })
        .await
    }

    let plain = blue_shadow_pixels("").await;
    for decoration in [
        "text-decoration:underline lime 2px;",
        "text-decoration:overline lime 2px;",
        "text-decoration:line-through lime 2px;",
    ] {
        let decorated = blue_shadow_pixels(decoration).await;
        let blue_dominant = |pixel: &[u8; 4]| pixel[2] > pixel[0] && pixel[2] > pixel[1];
        let extra_blue_pixels = decorated
            .iter()
            .zip(&plain)
            .filter(|(decorated, plain)| blue_dominant(decorated) && !blue_dominant(plain))
            .count();
        assert!(
            extra_blue_pixels > 0,
            "no decoration shadow for {decoration}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_shadows_follow_wrapped_and_vertical_text_and_respect_clips() {
    let stylesheet = Stylesheet::parse(
        ".flow { width:72px; }\
         .wrapped { color:transparent; font-size:20px; text-shadow:0 1px 2px lime; }\
         .vertical { width:48px; height:120px; writing-mode:vertical-rl; color:transparent; font-size:20px; text-shadow:2px 0 2px lime; }\
         .clip { width:42px; height:46px; overflow:hidden; }\
         .clipped { color:transparent; font-size:24px; text-shadow:20px 0 red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            let mut wrapped = egui::Rect::NOTHING;
            ui.add(
                (|ui: &mut egui::Ui| {
                    wrapped = ui
                        .add(egui::Label::new("one two three four five").selector(".wrapped"))
                        .rect;
                    ui.response()
                })
                .selector(".flow"),
            );
            let vertical = ui.add(egui::Label::new("ABC").selector(".vertical")).rect;
            let clipped = ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(egui::Label::new("MMMM").selector(".clipped"));
                    ui.response()
                })
                .selector(".clip"),
            );
            *capture.lock().unwrap() = Some((wrapped, vertical, clipped.rect));
        });
    })
    .await;
    let (wrapped, vertical, clip) = bounds.lock().unwrap().unwrap();
    let green = |pixel: &[u8; 4]| {
        pixel[1] > pixel[0].saturating_add(8) && pixel[1] > pixel[2].saturating_add(8)
    };
    let wrapped_rows = (wrapped.top().floor().max(0.0) as usize
        ..wrapped.bottom().ceil().min(HEIGHT as f32) as usize)
        .filter(|y| {
            (wrapped.left().floor().max(0.0) as usize
                ..wrapped.right().ceil().min(WIDTH as f32) as usize)
                .any(|x| green(&pixels[*y * WIDTH as usize + x]))
        })
        .count();
    assert!(wrapped.height() > 40.0);
    assert!(wrapped_rows > 20);

    let vertical_pixels = pixels
        .iter()
        .enumerate()
        .filter(|(index, pixel)| {
            let x = index % WIDTH as usize;
            let y = index / WIDTH as usize;
            x as f32 >= vertical.left() - 8.0
                && x as f32 <= vertical.right() + 8.0
                && y as f32 >= vertical.top() - 8.0
                && y as f32 <= vertical.bottom() + 8.0
                && green(pixel)
        })
        .count();
    assert!(vertical_pixels > 0);
    assert!(pixels.iter().enumerate().any(|(index, pixel)| {
        let x = index % WIDTH as usize;
        let y = index / WIDTH as usize;
        x as f32 >= clip.left()
            && (x as f32) < clip.right()
            && y as f32 >= clip.top()
            && (y as f32) < clip.bottom()
            && pixel[0] > pixel[1].saturating_add(8)
            && pixel[0] > pixel[2].saturating_add(8)
    }));
    assert!(pixels.iter().enumerate().all(|(index, pixel)| {
        let x = index % WIDTH as usize;
        let y = index / WIDTH as usize;
        (x as f32) < clip.right()
            || (y as f32) < clip.top()
            || y as f32 >= clip.bottom()
            || pixel[0] <= pixel[1].saturating_add(8)
            || pixel[0] <= pixel[2].saturating_add(8)
    }));
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inline_text_continues_across_selectors_and_paints_line_fragments_test() {
    let stylesheet=Stylesheet::parse(".flow {width:80px;} .one {display:inline; background-color:red; color:white;} .two {display:inline; background-color:blue; color:white;} .probe {width:max-content;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut a = None;
        let mut b = None;
        ui.add(
            (|ui: &mut egui::Ui| {
                a = Some(
                    ui.add(egui::Label::new("aa bb ccccccc ").selector(".one"))
                        .rect,
                );
                b = Some(ui.add(egui::Label::new("dd").selector(".two")).rect);
                ui.response()
            })
            .selector(".flow"),
        );
        let reference = ui.add(egui::Label::new("aa bb").selector(".probe")).rect;
        *capture.lock().unwrap() = Some((a.unwrap(), b.unwrap(), reference));
    })
    .await;
    let (a, b, reference) = bounds.lock().unwrap().unwrap();
    assert!(a.height() > reference.height(), "{a:?} {b:?}");
    assert!(b.top() > a.top() && b.top() < a.bottom(), "{a:?} {b:?}");
    assert_eq!(
        pixel(&pixels, a.left() + 0.5, a.top() + 0.5),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, b.left() + 0.5, b.top() + 0.5),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, a.left() + reference.width() + 1.5, a.top() + 0.5),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn pseudo_element_borders_paint_first_letter_and_first_line_test() {
    let stylesheet = Stylesheet::parse(
        ".letter, .line { width:200px; }\
         ::first-letter { border:3px solid green; }\
         .letter::first-letter { border:2px solid red; }\
         .line::first-line { border:2px solid magenta; border-radius:8px; box-shadow:0 0 0 3px blue; }\
         .legacy:first-letter { border:2px solid purple; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let letter = ui.add(egui::Label::new("A long line of text").selector(".letter"));
        let line = ui.add(egui::Label::new("First line").selector(".line"));
        ui.add(egui::Label::new("Legacy spelling").selector(".legacy"));
        *capture.lock().unwrap() = Some((letter.rect, line.rect));
    })
    .await;

    assert!(pixels.contains(&[255, 0, 0, 255]));
    assert!(pixels.contains(&[0, 0, 255, 255]));
    assert!(pixels.contains(&[0, 128, 0, 255]));
    assert!(pixels.contains(&[128, 0, 128, 255]));
    assert!(!pixels.contains(&[255, 0, 255, 255]));
    let (letter, line) = bounds.lock().unwrap().unwrap();
    let horizontal_bounds = |color| {
        pixels
            .iter()
            .enumerate()
            .filter(|(_, pixel)| **pixel == color)
            .fold((usize::MAX, 0), |(min, max), (index, _)| {
                (min.min(index % 256), max.max(index % 256))
            })
    };
    let red_bounds = horizontal_bounds([255, 0, 0, 255]);
    let blue_bounds = horizontal_bounds([0, 0, 255, 255]);
    assert!(
        ((red_bounds.1 - red_bounds.0) as f32) < letter.width() * 0.5,
        "first-letter border spans the full text box: {red_bounds:?} {letter:?}"
    );
    assert!(
        ((blue_bounds.1 - blue_bounds.0) as f32) < line.width() * 0.8,
        "first-line border spans the full text box: {blue_bounds:?} {line:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn first_letter_css_wide_color_and_background_reset_pseudo_decoration_test() {
    let stylesheet = Stylesheet::parse(
        ".letter { color:green; background-color:yellow; font-size:32px; }\
         .letter::first-letter { color:red; background-color:blue; }\
         .letter::first-letter { color:initial; background-color:initial; border:2px solid currentcolor; }",
    )
    .unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(egui::Label::new("A").selector(".letter"));
    })
    .await;

    assert!(
        pixels.contains(&[0, 0, 0, 255]),
        "initial pseudo color should reset to black"
    );
    assert!(
        pixels.contains(&[255, 255, 0, 255]),
        "initial pseudo background should be transparent"
    );
    assert!(
        !pixels.contains(&[255, 0, 0, 255]),
        "pseudo border kept the earlier red color"
    );
    assert!(
        !pixels.contains(&[0, 0, 255, 255]),
        "pseudo background kept the earlier blue fill"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn pseudo_first_letter_and_first_line_text_shadows_paint_selected_text() {
    let stylesheet = Stylesheet::parse(
        ".letter, .line { color:transparent; font-size:32px; width:max-content; }\
         .letter::first-letter { text-shadow:14px 0 red; }\
         .line::first-line { text-shadow:14px 0 blue; }",
    )
    .unwrap();
    let pixels = render_ui_pixels(stylesheet, |ui| {
        ui.add_space(20.0);
        ui.add(egui::Label::new("AB").selector(".letter"));
        ui.add(egui::Label::new("M N").selector(".line"));
    })
    .await;

    assert!(pixels.contains(&[255, 0, 0, 255]));
    assert!(pixels.contains(&[0, 0, 255, 255]));
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn adjacent_sibling_text_does_not_hide_uncovered_shadow_pixels() {
    let stylesheet = Stylesheet::parse(
        ".first { color:transparent; font-size:28px; text-shadow:16px 0 blue; }\
         .second { color:red; font-size:28px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        ui.horizontal(|ui| {
            let first = ui.add(egui::Label::new("M").selector(".first")).rect;
            let second = ui.add(egui::Label::new("i").selector(".second")).rect;
            *capture.lock().unwrap() = Some((first, second));
        });
    })
    .await;
    let (first, second) = bounds.lock().unwrap().unwrap();
    assert!(second.left() < first.right() + 16.0);
    assert!(pixels.iter().enumerate().any(|(index, pixel)| {
        let x = index % WIDTH as usize;
        let y = index / WIDTH as usize;
        x as f32 >= second.left()
            && (x as f32) < second.right()
            && y as f32 >= second.top()
            && (y as f32) < second.bottom()
            && pixel[2] > pixel[0]
            && pixel[2] > pixel[1]
    }));
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn first_letter_decoration_selects_numbers_and_associated_punctuation_test() {
    let stylesheet = Stylesheet::parse(
        ".number, .punctuation, .symbol { font-size:32px; }\
         .number::first-letter { border:3px solid red; }\
         .punctuation::first-letter { border:3px solid blue; }\
         .symbol::first-letter { border:3px solid lime; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 3]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add_space(40.0);
        *capture.lock().unwrap() = [
            Some(ui.add(egui::Label::new("1A").selector(".number")).rect),
            Some(
                ui.add(egui::Label::new("« A").selector(".punctuation"))
                    .rect,
            ),
            Some(ui.add(egui::Label::new("$A").selector(".symbol")).rect),
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    for (index, color) in [[255, 0, 0, 255], [0, 0, 255, 255], [0, 255, 0, 255]]
        .into_iter()
        .enumerate()
    {
        let rect = bounds[index].unwrap();
        let left = pixels
            .iter()
            .enumerate()
            .filter(|(_, pixel)| **pixel == color)
            .map(|(index, _)| index % 256)
            .min()
            .expect("first-letter border should paint");
        assert!(
            (left as f32) < rect.left() + 8.0,
            "first-letter decoration omitted its leading text: x={left}, rect={rect:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn first_letter_decoration_excludes_following_dashes_test() {
    let stylesheet = Stylesheet::parse(
        ".plain, .dashed { font-size:32px; }\
         .plain::first-letter { border:3px solid red; }\
         .dashed::first-letter { border:3px solid blue; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 2]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add_space(40.0);
        *capture.lock().unwrap() = [
            Some(ui.add(egui::Label::new("A").selector(".plain")).rect),
            Some(ui.add(egui::Label::new("A—B").selector(".dashed")).rect),
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let horizontal_bounds = |color, rect: egui::Rect| {
        pixels
            .iter()
            .enumerate()
            .filter(|(index, pixel)| {
                **pixel == color
                    && (index / 256) as f32 >= rect.top() - 3.0
                    && (index / 256) as f32 <= rect.bottom() + 3.0
            })
            .fold(None, |bounds, (index, _)| {
                let x = index % 256;
                Some(bounds.map_or((x, x), |(min, max): (usize, usize)| {
                    (min.min(x), max.max(x))
                }))
            })
            .expect("first-letter border should paint")
    };
    let plain = horizontal_bounds([255, 0, 0, 255], bounds[0].unwrap());
    let dashed = horizontal_bounds([0, 0, 255, 255], bounds[1].unwrap());
    assert_eq!(dashed, plain);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn vertical_first_letter_border_tracks_rotated_glyph_geometry_test() {
    let stylesheet = Stylesheet::parse(
        ".vertical { writing-mode:vertical-rl; width:60px; height:60px; }\
         .vertical::first-letter { border:2px solid red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add_space(40.0);
        let text = ui.add(egui::Label::new("A").selector(".vertical"));
        *capture.lock().unwrap() = Some(text.rect);
    })
    .await;
    let text = bounds.lock().unwrap().unwrap();
    let left = (text.left() - 4.0).floor().max(0.0) as usize;
    let right = (text.right() + 4.0).ceil().min(256.0) as usize;
    let top = (text.top() - 4.0).floor().max(0.0) as usize;
    let bottom = (text.bottom() + 4.0).ceil().min(128.0) as usize;
    assert!(
        (top..bottom)
            .any(|y| (left..right).any(|x| pixel(&pixels, x as f32, y as f32) == [255, 0, 0, 255])),
        "vertical first-letter border was not painted around its glyph: {text:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn vertical_upright_first_letter_maps_inline_start_through_direction_test() {
    async fn decoration_bounds(direction: &str) -> ((usize, usize), (usize, usize)) {
        let stylesheet = Stylesheet::parse(&format!(
            ".vertical {{ writing-mode:vertical-rl; text-orientation:upright; direction:{direction}; color:blue; width:60px; height:80px; }}\
             .vertical::first-letter {{ border-inline-start:4px solid red; background:blue; }}"
        ))
        .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            ui.add_space(20.0);
            let text = ui.add(egui::Label::new("A").selector(".vertical"));
            *capture.lock().unwrap() = Some(text.rect);
        })
        .await;
        let rect = bounds.lock().unwrap().unwrap();
        let y_bounds = |color| {
            let left = rect.left().floor().max(0.0) as usize;
            let right = rect.right().ceil().min(256.0) as usize;
            let top = (rect.top() - 4.0).floor().max(0.0) as usize;
            let bottom = (rect.bottom() + 4.0).ceil().min(128.0) as usize;
            (top..bottom)
                .filter(|y| (left..right).any(|x| pixel(&pixels, x as f32, *y as f32) == color))
                .fold(None, |bounds, y| {
                    Some(bounds.map_or((y, y), |(min, max): (usize, usize)| {
                        (min.min(y), max.max(y))
                    }))
                })
                .expect("first-letter decoration should paint")
        };
        (y_bounds([255, 0, 0, 255]), y_bounds([0, 0, 255, 255]))
    }

    let (ltr_red, ltr_glyph) = decoration_bounds("ltr").await;
    let (rtl_red, rtl_glyph) = decoration_bounds("rtl").await;
    let midpoint = |bounds: (usize, usize)| (bounds.0 + bounds.1) as f32 / 2.0;
    assert!(
        midpoint(ltr_red) < midpoint(ltr_glyph),
        "LTR vertical inline-start border should precede the glyph: {ltr_red:?} {ltr_glyph:?}"
    );
    assert!(
        midpoint(rtl_red) < midpoint(rtl_glyph),
        "upright orientation should use LTR inline direction: {rtl_red:?} {rtl_glyph:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn first_letter_percentage_padding_uses_the_containing_block_basis_test() {
    let stylesheet = Stylesheet::parse(
        ".pixels, .percentage { width:100px; }\
         .pixels::first-letter { padding-left:10px; background:red; }\
         .percentage::first-letter { padding-left:10%; background:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let pixels = ui.add(egui::Label::new("AB").selector(".pixels"));
        let percentage = ui.add(egui::Label::new("AB").selector(".percentage"));
        *capture.lock().unwrap() = Some((pixels.rect, percentage.rect));
    })
    .await;
    let (pixels_rect, percentage_rect) = bounds.lock().unwrap().unwrap();
    let red_pixels = |rect: egui::Rect| {
        let left = rect.left().floor().max(0.0) as usize;
        let right = rect.right().ceil().min(256.0) as usize;
        let top = rect.top().floor().max(0.0) as usize;
        let bottom = rect.bottom().ceil().min(128.0) as usize;
        (top..bottom)
            .flat_map(|y| (left..right).map(move |x| (x, y)))
            .filter(|(x, y)| pixel(&pixels, *x as f32, *y as f32) == [255, 0, 0, 255])
            .count()
    };
    assert_eq!(
        red_pixels(pixels_rect),
        red_pixels(percentage_rect),
        "10% first-letter padding should match 10px in a 100px containing block"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn first_letter_inline_decoration_extents_contribute_to_text_width_test() {
    let stylesheet = Stylesheet::parse(
        ".decorated::first-letter { margin-left:4px; padding-left:2px; border-left:3px solid red; margin-right:5px; padding-right:6px; border-right:1px solid red; }\n         .wrapped-plain, .wrapped-decorated { width:32px; }\n         .wrapped-decorated::first-letter { padding-right:12px; }\n         .flow-plain, .flow-decorated { width:max-content; }\n         .flow-word { display:inline; }\n         .flow-decorated::first-letter { margin-left:4px; padding-left:2px; border-left:3px solid red; margin-right:5px; padding-right:6px; border-right:1px solid red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let plain = ui.add(egui::Label::new("AB").selector(".plain"));
        let decorated = ui.add(egui::Label::new("AB").selector(".decorated"));
        let wrapped_plain = ui.add(egui::Label::new("A B").selector(".wrapped-plain"));
        let wrapped_decorated = ui.add(egui::Label::new("A B").selector(".wrapped-decorated"));
        let flow_plain = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("AB").selector(".flow-word"));
                ui.response()
            })
            .selector(".flow-plain"),
        );
        let flow_decorated = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("AB").selector(".flow-word"));
                ui.response()
            })
            .selector(".flow-decorated"),
        );
        *capture.lock().unwrap() = Some((
            plain.rect,
            decorated.rect,
            wrapped_plain.rect,
            wrapped_decorated.rect,
            flow_plain.rect,
            flow_decorated.rect,
        ));
    })
    .await;
    let (plain, decorated, wrapped_plain, wrapped_decorated, flow_plain, flow_decorated) =
        bounds.lock().unwrap().unwrap();
    assert!(
        (decorated.width() - plain.width() - 21.0).abs() < 0.5,
        "plain {plain:?}, decorated {decorated:?}"
    );
    assert!((flow_decorated.width() - flow_plain.width() - 21.0).abs() < 0.5);
    assert!(wrapped_decorated.height() > wrapped_plain.height());
    assert!(pixels.contains(&[255, 0, 0, 255]));
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn pseudo_element_decorations_apply_to_descendant_text_shapes_test() {
    let stylesheet = Stylesheet::parse(
        ".flow { width:200px; }\
         .word { display:inline; }\
         .flow::first-line { box-shadow:0 0 0 2px orange; }\
         .flow::first-letter { border:2px solid purple; }",
    )
    .unwrap();
    let pixels = render_ui_pixels(stylesheet, |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("first ").selector(".word"));
                ui.add(egui::Label::new("line").selector(".word"));
                ui.response()
            })
            .selector(".flow"),
        );
    })
    .await;

    assert!(pixels.contains(&[255, 165, 0, 255]));
    assert!(pixels.contains(&[128, 0, 128, 255]));
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inline_box_shadows_follow_box_decoration_break_test() {
    async fn shadow_pixels(decoration_break: &str, shadow: &str) -> usize {
        let stylesheet = Stylesheet::parse(&format!(
            ".flow {{ width:80px; }} .prefix, .fragment {{ display:inline; }} .fragment {{ background:white; box-shadow:{shadow}; box-decoration-break:{decoration_break}; }}"
        ))
        .unwrap();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(egui::Label::new("prefix ").selector(".prefix"));
                    ui.add(egui::Label::new("alpha beta gamma delta").selector(".fragment"));
                    ui.response()
                })
                .selector(".flow"),
            );
        })
        .await;
        pixels
            .iter()
            .filter(|pixel| **pixel == [255, 0, 0, 255])
            .count()
    }

    for shadow in ["0 0 0 3px red", "inset 0 0 0 3px red"] {
        let sliced_shadow = shadow_pixels("slice", shadow).await;
        let cloned_shadow = shadow_pixels("clone", shadow).await;
        assert!(
            cloned_shadow > sliced_shadow,
            "clone should paint a shadow around each fragment for {shadow}: clone={cloned_shadow}, slice={sliced_shadow}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inline_slice_background_gradients_continue_across_fragments_test() {
    async fn background_pixels(
        decoration_break: &str,
        image: &str,
        direction: &str,
    ) -> Vec<[u8; 4]> {
        let stylesheet = Stylesheet::load(&format!(
            ".flow {{ width:80px; direction:{direction}; }} .fragment {{ display:inline; background:{image}; box-decoration-break:{decoration_break}; }}"
        ))
        .await
        .unwrap();
        render_ui_pixels(stylesheet, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(
                        egui::Label::new("one two three four five six seven").selector(".fragment"),
                    );
                    ui.response()
                })
                .selector(".flow"),
            );
        })
        .await
    }

    let source = image::RgbaImage::from_fn(3, 1, |x, _| {
        image::Rgba(match x {
            0 => [255, 0, 0, 255],
            1 => [0, 255, 0, 255],
            _ => [0, 0, 255, 255],
        })
    });
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    let mut images = vec![
        "linear-gradient(to right, red, blue)".to_owned(),
        "radial-gradient(circle, red, blue)".to_owned(),
        "conic-gradient(red, blue)".to_owned(),
    ];
    images.push(format!("url(\"data:image/png;base64,{data}\")"));
    for image in images {
        for direction in ["ltr", "rtl"] {
            let sliced = background_pixels("slice", &image, direction).await;
            let cloned = background_pixels("clone", &image, direction).await;
            assert_ne!(
                sliced, cloned,
                "slice should continue {image} across {direction} line fragments"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inline_slice_border_images_continue_across_fragments_test() {
    async fn border_image_pixels(decoration_break: &str, direction: &str) -> Vec<[u8; 4]> {
        let stylesheet = Stylesheet::parse(&format!(
            ".flow {{ width:80px; direction:{direction}; }} .fragment {{ display:inline; border:4px solid black; border-image-source:linear-gradient(to right, red, blue); border-image-slice:4; border-image-width:4; box-decoration-break:{decoration_break}; }}"
        ))
        .unwrap();
        render_ui_pixels(stylesheet, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(
                        egui::Label::new("one two three four five six seven").selector(".fragment"),
                    );
                    ui.response()
                })
                .selector(".flow"),
            );
        })
        .await
    }

    for direction in ["ltr", "rtl"] {
        let sliced = border_image_pixels("slice", direction).await;
        let cloned = border_image_pixels("clone", direction).await;
        assert!(
            cloned.iter().any(|pixel| pixel[0] > 200 && pixel[2] < 100)
                && cloned.iter().any(|pixel| pixel[2] > 200 && pixel[0] < 100),
            "clone should paint the border image on every {direction} fragment"
        );
        assert_ne!(
            sliced, cloned,
            "slice should draw border images across the composite {direction} inline box"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_overflow_ellipsis_paints_at_the_clip_edge_test() {
    async fn render(overflow: &str) -> (Vec<[u8; 4]>, egui::Rect) {
        let stylesheet = Stylesheet::parse(&format!(
            ".box {{ width:60px; height:20px; overflow:hidden; white-space:nowrap; text-overflow:{overflow}; }} .text {{ display:inline; color:red; }}"
        ))
        .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            let response = ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(egui::Label::new("The overflow text is clipped").selector(".text"));
                    ui.response()
                })
                .selector(".box"),
            );
            *capture.lock().unwrap() = Some(response.rect);
        })
        .await;
        (pixels, bounds.lock().unwrap().unwrap())
    }

    let (ellipsis, bounds) = render("ellipsis").await;
    let (clipped, _) = render("clip").await;
    let mut changed_edge_pixels = 0;
    for x in bounds.right().floor() as i32 - 12..bounds.right().floor() as i32 {
        for y in bounds.top().floor() as i32..bounds.bottom().ceil() as i32 {
            let index = y as usize * WIDTH as usize + x as usize;
            changed_edge_pixels += usize::from(ellipsis[index] != clipped[index]);
        }
    }
    assert!(
        changed_edge_pixels > 0,
        "ellipsis did not change pixels near the inline clip edge"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn font_size_resizes_text_and_inherits_through_nested_selectors_test() {
    let stylesheet = Stylesheet::parse(
        ".small { font-size: 8px; }\
         .large { font-size: 24px; }\
         .relative { font-size: 1.5em; }\
         .percentage { font-size: 150%; }\
         .parent { font-size: 12px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let small = ui.add(egui::Label::new("MMMM").selector(".small")).rect;
        let large = ui.add(egui::Label::new("MMMM").selector(".large")).rect;
        let relative = ui.add(egui::Label::new("MMMM").selector(".relative")).rect;
        let percentage = ui
            .add(egui::Label::new("MMMM").selector(".percentage"))
            .rect;
        let mut inherited = None;
        ui.add(
            (|ui: &mut egui::Ui| {
                inherited = Some(ui.add(egui::Label::new("MMMM").selector(".inherited")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = vec![small, large, relative, percentage, inherited.unwrap()];
    })
    .await;

    let bounds = bounds.lock().unwrap();
    let ink_bounds = |rect: egui::Rect| {
        let mut ink = egui::Rect::NOTHING;
        let top = (rect.top().floor() as i32).clamp(0, HEIGHT as i32) as usize;
        let bottom = (rect.bottom().ceil() as i32).clamp(0, HEIGHT as i32) as usize;
        let left = (rect.left().floor() as i32).clamp(0, WIDTH as i32) as usize;
        let right = (rect.right().ceil() as i32).clamp(0, WIDTH as i32) as usize;
        for y in top..bottom {
            for x in left..right {
                let color = pixel(&pixels, x as f32, y as f32);
                if color[..3].iter().all(|channel| *channel < 240) {
                    ink = ink.union(egui::Rect::from_min_size(
                        egui::pos2(x as f32, y as f32),
                        egui::Vec2::splat(1.0),
                    ));
                }
            }
        }
        ink
    };
    let small = ink_bounds(bounds[0]);
    let large = ink_bounds(bounds[1]);
    let relative = ink_bounds(bounds[2]);
    let percentage = ink_bounds(bounds[3]);
    let inherited = ink_bounds(bounds[4]);
    assert!(large.height() > small.height() * 2.5, "{small:?} {large:?}");
    for actual in [relative, percentage] {
        assert!(
            (actual.height() - large.height()).abs() < 1.0,
            "{actual:?} {large:?}"
        );
    }
    assert!(
        (inherited.height() - small.height() * 1.5).abs() < 2.0,
        "{small:?} {inherited:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn font_shorthand_changes_font_metrics_and_line_height_in_rendered_text_test() {
    let stylesheet = Stylesheet::parse(
        ".small { font:10px/20px sans-serif; color:red; }\
         .large { font:italic bold 28px/40px sans-serif; color:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let small = ui.add(egui::Label::new("MMMM").selector(".small")).rect;
        let large = ui.add(egui::Label::new("MMMM").selector(".large")).rect;
        *capture.lock().unwrap() = vec![small, large];
    })
    .await;

    let bounds = bounds.lock().unwrap();
    assert!((bounds[0].height() - 20.0).abs() < 0.5, "{bounds:?}");
    assert!((bounds[1].height() - 40.0).abs() < 0.5, "{bounds:?}");
    let ink_height = |rect: egui::Rect| {
        let top = rect.top().floor().max(0.0) as usize;
        let bottom = rect.bottom().ceil().min(HEIGHT as f32) as usize;
        let left = rect.left().floor().max(0.0) as usize;
        let right = rect.right().ceil().min(WIDTH as f32) as usize;
        let mut ink = None;
        for y in top..bottom {
            if (left..right).any(|x| pixel(&pixels, x as f32, y as f32)[..3] == [255, 0, 0]) {
                ink = Some(ink.map_or((y, y), |(top, _)| (top, y)));
            }
        }
        ink.map_or(0, |(top, bottom)| bottom - top + 1)
    };
    assert!(
        ink_height(bounds[1]) > ink_height(bounds[0]) * 2,
        "{bounds:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn vertical_text_positions_lines_and_honors_text_orientation_test() {
    let stylesheet = Stylesheet::parse(".row {display:flex; width:200px; gap:10px;} .rl {writing-mode:vertical-rl; white-space:pre; width:32px; height:60px; color:red;} .lr {writing-mode:vertical-lr; white-space:pre; width:32px; height:60px; color:red;} .upright {writing-mode:vertical-lr; text-orientation:upright; width:20px; height:20px; color:red;} .sideways {writing-mode:vertical-lr; text-orientation:sideways; width:20px; height:20px; color:red;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut job = egui::text::LayoutJob::default();
        for (text, color) in [("W\n", egui::Color32::RED), ("I", egui::Color32::BLUE)] {
            job.append(
                text,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(14.0),
                    color,
                    ..Default::default()
                },
            );
        }
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = vec![
                    ui.add(egui::Label::new(job.clone()).selector(".rl")).rect,
                    ui.add(egui::Label::new(job).selector(".lr")).rect,
                    ui.add(egui::Label::new("L").selector(".upright")).rect,
                    ui.add(egui::Label::new("L").selector(".sideways")).rect,
                ];
                ui.response()
            })
            .selector(".row"),
        );
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let ink = |rect: egui::Rect, red: bool| {
        let mut count = 0;
        let mut area = egui::Rect::NOTHING;
        for y in rect.top() as u32..rect.bottom() as u32 {
            for x in rect.left() as u32..rect.right() as u32 {
                let p = pixel(&pixels, x as f32, y as f32);
                if p[1] < 128
                    && if red {
                        p[0] > 128 && p[2] < 128
                    } else {
                        p[2] > 128 && p[0] < 128
                    }
                {
                    count += 1;
                    area = area.union(egui::Rect::from_min_size(
                        egui::pos2(x as f32, y as f32),
                        egui::Vec2::splat(1.0),
                    ));
                }
            }
        }
        (count, area)
    };
    for (index, rect) in bounds[..2].iter().enumerate() {
        let red = ink(*rect, true).1;
        let blue = ink(*rect, false).1;
        assert!(
            red.is_positive() && blue.is_positive(),
            "{index}: {red:?} {blue:?}"
        );
        assert!(
            if index == 0 {
                red.center().x > blue.center().x
            } else {
                red.center().x < blue.center().x
            },
            "{index}: {red:?} {blue:?}"
        );
    }
    let upright = ink(bounds[2], true).1;
    let sideways = ink(bounds[3], true).1;
    assert!(
        upright.is_positive() && upright.height() > upright.width(),
        "{upright:?}"
    );
    assert!(
        sideways.is_positive() && sideways.width() > sideways.height(),
        "{sideways:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inline_padding_and_borders_paint_around_text_without_expanding_line_height_test() {
    let stylesheet = Stylesheet::parse(".flow {width:100px; margin-top:16px;} .text {display:inline; padding:4px 8px; border:2px solid blue; margin-left:3px; background-color:red; color:white;} .next {display:inline-block; width:10px; height:10px; background-color:green;} .probe {width:max-content;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut boxes = Vec::new();
        let flow = ui
            .add(
                (|ui: &mut egui::Ui| {
                    boxes.push(ui.add(egui::Label::new("X").selector(".text")).rect);
                    boxes.push(ui.add(empty_response.selector(".next")).rect);
                    ui.response()
                })
                .selector(".flow"),
            )
            .rect;
        boxes.push(flow);
        boxes.push(ui.add(egui::Label::new("X").selector(".probe")).rect);
        *capture.lock().unwrap() = boxes;
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let text = bounds[0];
    assert!((text.width() - bounds[3].width() - 20.0).abs() < 0.5);
    assert!((text.height() - bounds[3].height() - 12.0).abs() < 0.5);
    near(text.left(), 3.0);
    near(bounds[1].left(), text.right());
    assert!((bounds[2].height() - bounds[3].height()).abs() < 0.5);
    assert_eq!(
        pixel(&pixels, text.left() + 0.5, text.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, text.left() + 3.0, text.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_transform_matches_the_transformed_source_and_spacing_expands_text_test() {
    async fn render(styles: &str, text: &str) -> (Vec<[u8; 4]>, egui::Rect) {
        let stylesheet = Stylesheet::parse(styles).unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let text = text.to_owned();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            let response = ui.add(egui::Label::new(text.clone()).selector(".text"));
            *capture.lock().unwrap() = Some(response.rect);
        })
        .await;
        (pixels, bounds.lock().unwrap().unwrap())
    }

    let transformed = render(
        ".text { color:red; width:120px; height:32px; text-transform:uppercase; }",
        "hello",
    )
    .await;
    let reference = render(".text { color:red; width:120px; height:32px; }", "HELLO").await;
    assert_eq!(transformed.0, reference.0);

    let spaced = render(
        ".text { width:max-content; letter-spacing:2px; word-spacing:3px; }",
        "A B C",
    )
    .await;
    let plain = render(".text { width:max-content; }", "A B C").await;
    assert!(spaced.1.width() > plain.1.width() + 8.0);
    assert_ne!(spaced.0, plain.0);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_transform_and_spacing_are_inherited_by_nested_widgets_test() {
    async fn render(styles: &str) -> (Vec<[u8; 4]>, egui::Rect) {
        let stylesheet = Stylesheet::parse(styles).unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    let child = ui.add(egui::Label::new("inherit me").selector(".child"));
                    *capture.lock().unwrap() = Some(child.rect);
                    ui.response()
                })
                .selector(".parent"),
            );
        })
        .await;
        (pixels, bounds.lock().unwrap().unwrap())
    }

    let inherited = render(
        ".parent { text-transform:uppercase; letter-spacing:2px; word-spacing:3px; } \
         .child { color:red; width:max-content; }",
    )
    .await;
    let specified = render(
        ".child { color:red; width:max-content; text-transform:uppercase; \
         letter-spacing:2px; word-spacing:3px; }",
    )
    .await;

    assert_eq!(inherited.0, specified.0);
    assert_eq!(inherited.1.size(), specified.1.size());
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn css_text_declarations_are_accepted_together_test() {
    let stylesheet = Stylesheet::parse(
        ".text { color:red; hanging-punctuation:first last allow-end; hyphens:auto; letter-spacing:1px; \
         line-break:strict; overflow-wrap:anywhere; tab-size:4; text-align:center; \
         text-align-all:start; text-align-last:justify; text-indent:2px each-line hanging; \
         text-justify:inter-character; text-transform:uppercase; white-space:pre-wrap; \
         word-break:break-all; word-spacing:2px; }",
    )
    .unwrap();
    let pixels = render_ui_pixels(stylesheet, |ui| {
        ui.add(egui::Label::new("css text\nproperties").selector(".text"));
    })
    .await;
    assert!(
        pixels
            .iter()
            .any(|pixel| pixel[0] == 255 && pixel[1] == 0 && pixel[2] == 0)
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_indent_offsets_the_first_line_and_tab_size_changes_preserved_tabs_test() {
    async fn render(styles: &str, text: &str) -> (Vec<[u8; 4]>, egui::Rect) {
        let stylesheet = Stylesheet::parse(styles).unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let text = text.to_owned();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            let response = ui.add(egui::Label::new(text.clone()).selector(".text"));
            *capture.lock().unwrap() = Some(response.rect);
        })
        .await;
        (pixels, bounds.lock().unwrap().unwrap())
    }

    let indented = render(
        ".text { color:red; width:160px; height:32px; text-indent:18px; }",
        "X",
    )
    .await;
    let plain = render(".text { color:red; width:160px; height:32px; }", "X").await;
    let first_red_x = |(pixels, rect): &(Vec<[u8; 4]>, egui::Rect)| {
        (rect.left().floor() as usize..rect.right().ceil() as usize)
            .find(|x| {
                (rect.top().floor() as usize..rect.bottom().ceil() as usize).any(|y| {
                    let color = pixel(pixels, *x as f32, y as f32);
                    color[0] > color[1] && color[0] > color[2]
                })
            })
            .unwrap()
    };
    assert!(first_red_x(&indented) > first_red_x(&plain) + 12);

    let four = render(
        ".text { width:max-content; white-space:pre; tab-size:4; }",
        "\tB",
    )
    .await;
    let two = render(
        ".text { width:max-content; white-space:pre; tab-size:2; }",
        "\tB",
    )
    .await;
    assert!(
        four.1.width() > two.1.width() + 4.0,
        "tab-size 4 width {} should exceed tab-size 2 width {}",
        four.1.width(),
        two.1.width()
    );
    assert_ne!(four.0, two.0);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn font_kerning_none_uses_unadjusted_glyph_spacing_and_inherits_test() {
    let stylesheet = Stylesheet::parse(
        ".normal { width: 250px; font-size: 48px; font-kerning: normal; white-space: nowrap; }\
         .auto { width: 250px; font-size: 48px; font-kerning: auto; white-space: nowrap; }\
         .none { width: 250px; font-size: 48px; font-kerning: none; white-space: nowrap; }\
         .parent { width: 250px; font-size: 48px; font-kerning: none; white-space: nowrap; }\
         .reset { font-kerning: normal; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let text = "AVAVAVAV";
        let mut bounds = Vec::new();
        ui.add(
            (|ui: &mut egui::Ui| {
                bounds.push(ui.add(egui::Label::new(text)).rect);
                ui.response()
            })
            .selector(".normal"),
        );
        ui.add(
            (|ui: &mut egui::Ui| {
                bounds.push(ui.add(egui::Label::new(text)).rect);
                ui.response()
            })
            .selector(".auto"),
        );
        ui.add(
            (|ui: &mut egui::Ui| {
                bounds.push(ui.add(egui::Label::new(text)).rect);
                ui.response()
            })
            .selector(".none"),
        );
        ui.add(
            (|ui: &mut egui::Ui| {
                bounds.push(ui.add(egui::Label::new(text)).rect);
                ui.add(
                    (|ui: &mut egui::Ui| {
                        bounds.push(ui.add(egui::Label::new(text)).rect);
                        ui.response()
                    })
                    .selector(".reset"),
                );
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = bounds;
    })
    .await;

    let bounds = bounds.lock().unwrap();
    let ink_width = |rect: egui::Rect| {
        let mut left = f32::INFINITY;
        let mut right = f32::NEG_INFINITY;
        let top = (rect.top().floor() as i32).clamp(0, HEIGHT as i32);
        let bottom = (rect.bottom().ceil() as i32).clamp(0, HEIGHT as i32);
        let left_edge = (rect.left().floor() as i32).clamp(0, WIDTH as i32);
        let right_edge = (rect.right().ceil() as i32).clamp(0, WIDTH as i32);
        for y in top..bottom {
            for x in left_edge..right_edge {
                let color = pixel(&pixels, x as f32, y as f32);
                if color[..3].iter().any(|channel| *channel < 240) {
                    left = left.min(x as f32);
                    right = right.max(x as f32 + 1.0);
                }
            }
        }
        right - left
    };
    let normal = ink_width(bounds[0]);
    let auto = ink_width(bounds[1]);
    let none = ink_width(bounds[2]);
    assert_eq!(auto, normal);
    assert!(none > normal, "normal={normal} none={none}");
    assert!(
        (bounds[3].width() - bounds[2].width()).abs() < 0.1,
        "inherited={:?} none={:?}",
        bounds[3],
        bounds[2]
    );
    assert!(
        (bounds[4].width() - bounds[0].width()).abs() < 0.1,
        "reset={:?} normal={:?}",
        bounds[4],
        bounds[0]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn all_property_inherits_and_resets_text_color_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { color:red; } .inherited { all:inherit; }\
         .reverted { color:blue; all:revert; } .initial { all:initial; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                let inherited = ui.add(egui::Label::new("inherited").selector(".inherited"));
                let reverted = ui.add(egui::Label::new("reverted").selector(".reverted"));
                let initial = ui.add(egui::Label::new("initial").selector(".initial"));
                *capture.lock().unwrap() = vec![inherited.rect, reverted.rect, initial.rect];
                ui.response()
            })
            .selector(".parent"),
        );
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let red_pixels = |rect: egui::Rect| {
        let left = (rect.left().floor() as i32).clamp(0, WIDTH as i32);
        let right = (rect.right().ceil() as i32).clamp(0, WIDTH as i32);
        let top = (rect.top().floor() as i32).clamp(0, HEIGHT as i32);
        let bottom = (rect.bottom().ceil() as i32).clamp(0, HEIGHT as i32);
        let mut count = 0;
        for y in top..bottom {
            for x in left..right {
                let [red, green, blue, alpha] = pixel(&pixels, x as f32, y as f32);
                if red > 200 && green < 40 && blue < 40 && alpha == 255 {
                    count += 1;
                }
            }
        }
        count
    };

    assert!(red_pixels(bounds[0]) > 0);
    assert!(red_pixels(bounds[1]) > 0);
    assert_eq!(red_pixels(bounds[2]), 0);
}
