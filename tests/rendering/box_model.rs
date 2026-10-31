use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn rounded_border_corner_tracks_the_curve_without_faceting_test() {
    let stylesheet = Stylesheet::parse(
        ".round { box-sizing:border-box; width:48px; height:48px; border:1px solid black; border-radius:12px; background:white; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_scale(stylesheet, 4.0, move |ui| {
        ui.add_space(4.0);
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".round")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    // At 4×, this point is inside the ideal arc's antialias fringe but outside coarse chords.
    let diagonal = 12.0 * (1.0 - std::f32::consts::FRAC_1_SQRT_2) * 4.0;
    let sample_x = (rect.left() * 4.0 + diagonal).floor() as usize - 1;
    let sample_y = (rect.top() * 4.0 + diagonal).floor() as usize;
    let sample = pixels[sample_y * WIDTH as usize + sample_x];
    assert!(
        sample[0] <= 160,
        "the corner edge is too far inside its ideal curve: {sample:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn content_and_border_box_sizes_include_the_correct_edges_test() {
    let stylesheet = Stylesheet::parse(
        ".content { width: 60px; height: 20px; padding: 5px 10px; border: 2px solid #00f; background-color: #f00; }\
         .border { box-sizing: border-box; width: 60px; height: 30px; padding: 5px 10px; border: 2px solid #00f; background-color: #f00; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".content")).rect,
            ui.add(empty_response.selector(".border")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(bounds[0].size(), egui::vec2(84.0, 34.0));
    assert_eq!(bounds[1].size(), egui::vec2(60.0, 30.0));
    for rect in bounds.iter() {
        assert_eq!(
            pixel(&pixels, rect.left() + 0.5, rect.center().y),
            [0, 0, 255, 255]
        );
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.center().y),
            [255, 0, 0, 255]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inline_style_attribute_overrides_the_matching_stylesheet_rule_test() {
    let stylesheet = Stylesheet::parse(
        ".inline { box-sizing:border-box; width:48px; height:24px; background-color:red; }",
    )
    .unwrap();
    let inline = ecss::InlineStyle::parse(
        "/* declaration list */ width:24px !important; background-color:blue",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(
            ui.add(
                empty_response
                    .selector(".inline")
                    .inline_style(inline.clone()),
            )
            .rect,
        );
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(rect.size(), egui::vec2(24.0, 24.0));
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn svg_url_paints_as_a_border_image_test() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="9" height="9"><rect width="9" height="9" fill="#ff0000"/></svg>"##;
    let data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, svg);
    let stylesheet = Stylesheet::load(&format!(
        ".image {{ box-sizing:border-box; width:24px; height:24px; border:4px solid black; background:white; border-image-source:url(\"data:image/svg+xml;base64,{data}\"); border-image-slice:3; border-image-width:1; }}"
    ))
    .await
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 2.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [255, 255, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn svg_image_set_type_hint_paints_as_a_border_image_test() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="9" height="9"><rect width="9" height="9" fill="#ff0000"/></svg>"##;
    let data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, svg);
    let stylesheet = Stylesheet::load(&format!(
        ".image {{ box-sizing:border-box; width:24px; height:24px; border:4px solid black; background:white; border-image-source:image-set(url(\"data:image/svg+xml;base64,{data}\") 1x type(\"image/svg+xml\")); border-image-slice:3; border-image-width:1; }}"
    ))
    .await
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 2.0),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn svg_border_image_without_intrinsic_dimensions_uses_the_border_area_test() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 9 9"><rect width="9" height="9" fill="#ff0000"/></svg>"##;
    let data = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, svg);
    let stylesheet = Stylesheet::load(&format!(
        ".image {{ box-sizing:border-box; width:24px; height:24px; border:4px solid black; background:white; border-image-source:url(\"data:image/svg+xml;base64,{data}\"); border-image-slice:25%; border-image-width:auto; }}"
    ))
    .await
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 1.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 8.0),
        [255, 255, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_source_paints_as_a_background_test() {
    let stylesheet = Stylesheet::parse(
        "#source { position:relative; z-index:0; box-sizing:border-box; width:16px; height:16px; background:red; }\n         .target { box-sizing:border-box; width:32px; height:32px; background-image:element(#source); }\n         .fade { box-sizing:border-box; width:32px; height:32px; background-image:cross-fade(element(#source) 50%, blue 50%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(empty_response.selector("#source"));
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".target")).rect,
            ui.add(empty_response.selector(".fade")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let target = bounds[0];
    assert_eq!(
        pixel(&pixels, target.center().x, target.center().y),
        [255, 0, 0, 255]
    );
    let fade = bounds[1];
    let color = pixel(&pixels, fade.center().x, fade.center().y);
    assert!(
        color[0].abs_diff(127) < 3 && color[2].abs_diff(128) < 3,
        "unexpected cross-fade pixel {color:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_source_requires_a_stacking_context_test() {
    let stylesheet = Stylesheet::parse(
        "#source { box-sizing:border-box; width:16px; height:16px; background:green; }\n         #hidden-source { display:none; width:16px; height:16px; background:green; }\n         .target { box-sizing:border-box; width:24px; height:24px; background:red; background-image:element(#source); }\n         .hidden-target { box-sizing:border-box; width:24px; height:24px; background:blue; background-image:element(#hidden-source); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(empty_response.selector("#source"));
        ui.add(empty_response.selector("#hidden-source"));
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".target")).rect,
            ui.add(empty_response.selector(".hidden-target")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(
        pixel(&pixels, bounds[0].center().x, bounds[0].center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_source_flex_item_with_z_index_is_eligible_test() {
    let stylesheet = Stylesheet::parse(
        "#container { display:flex; box-sizing:border-box; width:24px; height:16px; }\n         #source { z-index:0; box-sizing:border-box; width:16px; height:16px; background:green; }\n         .target { box-sizing:border-box; width:24px; height:16px; background:red; background-image:element(#source); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector("#source"));
                ui.response()
            })
            .selector("#container"),
        );
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".target")).rect);
    })
    .await;
    let target = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, target.center().x, target.center().y),
        [0, 128, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_source_fixed_positioned_with_auto_z_index_is_eligible_test() {
    let stylesheet = Stylesheet::parse(
        "#source { position:fixed; top:0; left:100px; z-index:auto; box-sizing:border-box; width:16px; height:16px; background:green; }\n         .target { box-sizing:border-box; width:24px; height:24px; background:red; background-image:element(#source); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(empty_response.selector("#source"));
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".target")).rect);
    })
    .await;
    let target = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, target.center().x, target.center().y),
        [0, 128, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_hidden_context_keeps_visible_descendants_test() {
    let stylesheet = Stylesheet::parse(
        "#parent { position:relative; z-index:0; box-sizing:border-box; width:32px; height:32px; visibility:hidden; }\n         .child { box-sizing:border-box; width:32px; height:32px; visibility:visible; background:green; }\n         .target { box-sizing:border-box; width:32px; height:32px; background:red; background-image:element(#parent); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".child"));
                ui.response()
            })
            .selector("#parent"),
        );
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".target")).rect);
    })
    .await;
    let target = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, target.center().x, target.center().y),
        [0, 128, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_ignores_references_in_display_none_descendants_test() {
    let stylesheet = Stylesheet::parse(
        "#holder { position:relative; z-index:0; box-sizing:border-box; width:32px; height:32px; background:green; }\n         #inert { display:none; background-image:element(#target); }\n         #target { position:relative; z-index:0; box-sizing:border-box; width:24px; height:24px; background:red; background-image:element(#holder); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector("#inert"));
                ui.response()
            })
            .selector("#holder"),
        );
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector("#target")).rect);
    })
    .await;
    let target = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, target.center().x, target.center().y),
        [0, 128, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_uses_the_first_matching_id_test() {
    let stylesheet = Stylesheet::parse(
        "#duplicate { position:relative; z-index:0; box-sizing:border-box; width:16px; height:16px; }\n         .target { box-sizing:border-box; width:16px; height:16px; background-image:element(#duplicate); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for color in [egui::Color32::RED, egui::Color32::BLUE] {
            ui.add(
                (move |ui: &mut egui::Ui| {
                    let rect = ui.max_rect();
                    ui.painter()
                        .rect_filled(rect, egui::CornerRadius::ZERO, color);
                    ui.allocate_response(rect.size(), egui::Sense::hover())
                })
                .selector("#duplicate"),
            );
        }
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".target")).rect);
    })
    .await;
    let target = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, target.center().x, target.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn opposite_concave_corner_hulls_constrain_border_radii_test() {
    let stylesheet = Stylesheet::parse(
        ".scoops { box-sizing:border-box; width:64px; height:64px; background:red; corner-top-left-shape:notch; corner-bottom-right-shape:notch; border-top-left-radius:100%; border-bottom-right-radius:100%; }\n         .reference { box-sizing:border-box; width:64px; height:64px; background:red; corner-top-left-shape:notch; corner-bottom-right-shape:notch; border-top-left-radius:50%; border-bottom-right-radius:50%; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".scoops")).rect,
            ui.add(empty_response.selector(".reference")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(
                pixel(
                    &pixels,
                    bounds[0].left() + x as f32,
                    bounds[0].top() + y as f32
                ),
                pixel(
                    &pixels,
                    bounds[1].left() + x as f32,
                    bounds[1].top() + y as f32
                ),
                "used radius differs from the non-overlapping reference at ({x}, {y})"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn corner_shorthands_match_their_longhand_expansions_test() {
    let stylesheet = Stylesheet::parse(
        ".all-short, .all-long, .side-short, .side-long, .single-short, .single-long { box-sizing:border-box; width:36px; height:28px; border:2px solid blue; background:red; }\
         .all-short { corner:scoop 16px 4px / 12px 2px; }\
         .all-long { corner-shape:scoop; border-radius:16px 4px / 12px 2px; }\
         .side-short { corner-inline-start:notch 12px 3px; }\
         .side-long { corner-inline-start-shape:notch; border-inline-start-radius:12px 3px; }\
         .single-short { corner-end-start:bevel 9px / 6px; }\
         .single-long { corner-end-start-shape:bevel; border-end-start-radius:9px / 6px; }",
    )
    .unwrap();
    let short_pixels = render_ui_pixels(stylesheet.clone(), move |ui| {
        for short in [".all-short", ".side-short", ".single-short"] {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.add(empty_response.selector(short));
            });
        }
    })
    .await;
    let long_pixels = render_ui_pixels(stylesheet, move |ui| {
        for long in [".all-long", ".side-long", ".single-long"] {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.add(empty_response.selector(long));
            });
        }
    })
    .await;
    for y in 0..128 {
        for x in 0..256 {
            assert_eq!(
                pixel(&short_pixels, x as f32, y as f32),
                pixel(&long_pixels, x as f32, y as f32),
                "corner shorthands differ from longhand expansions at ({x}, {y})"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn calculated_superellipse_parameter_matches_its_keyword_test() {
    let stylesheet = Stylesheet::parse(
        ".keyword { box-sizing:border-box; width:48px; height:48px; background:red; border-radius:24px; corner-shape:squircle; }\
         .calculated { box-sizing:border-box; width:48px; height:48px; background:red; border-radius:24px; corner-shape:superellipse(calc(1 + 1)); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".keyword")).rect,
            ui.add(empty_response.selector(".calculated")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    for y in 0..48 {
        for x in 0..48 {
            assert_eq!(
                pixel(
                    &pixels,
                    bounds[0].left() + x as f32,
                    bounds[0].top() + y as f32
                ),
                pixel(
                    &pixels,
                    bounds[1].left() + x as f32,
                    bounds[1].top() + y as f32
                ),
                "calculated superellipse differs from squircle at ({x}, {y})"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn outer_shadow_does_not_paint_inside_a_transparent_box_test() {
    let stylesheet = Stylesheet::parse(
        ".shadow { box-sizing:border-box; width:32px; height:32px; box-shadow:0 0 0 8px red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shadow")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn box_shadow_spread_preserves_square_corners_test() {
    let stylesheet = Stylesheet::parse(
        ".shadow { box-sizing:border-box; width:32px; height:32px; margin:30px; box-shadow:0 0 0 8px red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shadow")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() - 6.5, rect.top() - 6.5),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_source_paints_as_a_border_image_test() {
    let stylesheet = Stylesheet::parse(
        "#source { position:relative; z-index:0; box-sizing:border-box; width:12px; height:12px; background:red; }\n         .target { box-sizing:border-box; width:24px; height:24px; border:4px solid black; background:white; border-image:element(#source) 4 / 1; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(empty_response.selector("#source"));
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".target")).rect);
    })
    .await;
    let target = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, target.center().x, target.top() + 1.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, target.center().x, target.center().y),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn cyclic_element_image_references_are_invalid_test() {
    let stylesheet = Stylesheet::parse(
        "#first { position:relative; z-index:0; box-sizing:border-box; width:16px; height:16px; background:red; background-image:element(#second); }\n         #second { position:relative; z-index:0; box-sizing:border-box; width:16px; height:16px; background:blue; background-image:element(#first); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector("#first")).rect,
            ui.add(empty_response.selector("#second")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(
        pixel(&pixels, bounds[0].center().x, bounds[0].center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_cycle_through_a_descendant_is_invalid_test() {
    let stylesheet = Stylesheet::parse(
        "#parent { position:relative; z-index:0; box-sizing:border-box; width:64px; height:64px; background:green; }\n         #child { box-sizing:border-box; width:16px; height:16px; background:blue; background-image:element(#parent); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let capture = capture.clone();
        ui.add(
            (move |ui: &mut egui::Ui| {
                *capture.lock().unwrap() = Some(ui.add(empty_response.selector("#child")).rect);
                ui.response()
            })
            .selector("#parent"),
        );
    })
    .await;
    let child = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, child.center().x, child.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_cycle_through_an_unidentified_descendant_is_invalid_test() {
    let stylesheet = Stylesheet::parse(
        "#parent { position:relative; z-index:0; box-sizing:border-box; width:64px; height:64px; background:green; }\n         .child { box-sizing:border-box; width:16px; height:16px; background:blue; background-image:element(#parent); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let capture = capture.clone();
        ui.add(
            (move |ui: &mut egui::Ui| {
                *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".child")).rect);
                ui.response()
            })
            .selector("#parent"),
        );
    })
    .await;
    let child = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, child.center().x, child.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn element_image_cycles_across_root_selectors_include_cached_descendants_test() {
    let stylesheet = Stylesheet::parse(
        "#parent { position:relative; z-index:0; box-sizing:border-box; width:64px; height:64px; background:green; }\n         .child { box-sizing:border-box; width:16px; height:16px; background:blue; background-image:element(#other); }\n         #other { position:relative; z-index:0; box-sizing:border-box; width:16px; height:16px; background:yellow; background-image:element(#parent); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".child"));
                ui.response()
            })
            .selector("#parent"),
        );
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector("#other")).rect);
    })
    .await;
    let other = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, other.center().x, other.center().y),
        [255, 255, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn overridden_element_image_reference_does_not_create_a_cycle_test() {
    let stylesheet = Stylesheet::parse(
        "#first { position:relative; z-index:0; box-sizing:border-box; width:16px; height:16px; background:red; background-image:element(#second); background-image:none; }\n         #second { box-sizing:border-box; width:16px; height:16px; background:blue; background-image:element(#first); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(empty_response.selector("#first"));
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector("#second")).rect);
    })
    .await;
    let second = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, second.center().x, second.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn all_reset_element_image_reference_does_not_create_a_cycle_test() {
    let stylesheet = Stylesheet::parse(
        "#first { background-image:element(#second); all:initial; position:relative; z-index:0; box-sizing:border-box; width:16px; height:16px; background:red; }\n         #second { position:relative; z-index:0; box-sizing:border-box; width:16px; height:16px; background:blue; background-image:element(#first); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(empty_response.selector("#first"));
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector("#second")).rect);
    })
    .await;
    let second = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, second.center().x, second.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_shorthand_reset_does_not_create_an_element_image_cycle_test() {
    for reset in [
        "border:2px solid transparent",
        "border-image-source:initial",
        "border-image-source:unset",
        "border-image-source:revert",
        "border-image-source:inherit",
        "border-image:initial",
    ] {
        let stylesheet = Stylesheet::parse(&format!(
        "#first {{ position:relative; z-index:0; box-sizing:border-box; width:16px; height:16px; background:red; border-image-source:element(#second); {reset}; }}\n         #second {{ position:relative; z-index:0; box-sizing:border-box; width:16px; height:16px; background:blue; background-image:element(#first); }}",
    ))
    .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            ui.add(empty_response.selector("#first"));
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector("#second")).rect);
        })
        .await;
        let second = bounds.lock().unwrap().unwrap();
        assert_eq!(
            pixel(&pixels, second.center().x, second.center().y),
            [255, 0, 0, 255]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn first_letter_border_image_participates_in_element_cycle_detection_test() {
    let stylesheet = Stylesheet::parse(
        "#first { position:relative; z-index:0; color:white; background:red; font-size:24px; }\n         #first::first-letter { border:3px solid lime; border-image-source:element(#second); border-image-slice:1; }\n         #second { position:relative; z-index:0; box-sizing:border-box; width:24px; height:24px; background:blue; background-image:element(#first); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(egui::Label::new("A").selector("#first"));
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector("#second")).rect);
    })
    .await;
    let second = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, second.center().x, second.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn first_letter_border_image_resets_remove_element_cycles() {
    for reset in ["initial", "unset", "revert", "inherit"] {
        let stylesheet = Stylesheet::parse(&format!(
            "#first {{ position:relative; z-index:0; width:32px; height:24px; color:transparent; background:red; }} \
             #first::first-letter {{ border-image-source:element(#second); border-image-source:{reset}; }} \
             #second {{ position:relative; z-index:0; width:32px; height:24px; background:blue; background-image:element(#first); }}"
        )).unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            ui.add(egui::Label::new("X").selector("#first"));
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector("#second")).rect);
        })
        .await;
        let rect = bounds.lock().unwrap().unwrap();
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.center().y),
            [255, 0, 0, 255],
            "{reset}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inherited_border_images_participate_in_element_cycles() {
    let stylesheet = Stylesheet::parse(
        ".parent { border-image-source:element(#second); } \
         #first { position:relative; z-index:0; box-sizing:border-box; width:24px; height:24px; border:3px solid black; background:red; border-image-source:inherit; border-image-slice:1; } \
         #second { position:relative; z-index:0; width:24px; height:24px; background:blue; background-image:element(#first); }"
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector("#first"));
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector("#second")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn animated_and_variable_border_images_participate_in_element_cycles() {
    for animated in [false, true] {
        let sources = if animated {
            "@keyframes first { from,to { border-image-source:element(#second); } } \
             @keyframes second { from,to { border-image-source:element(#first); } } \
             #first { animation:first 1s linear infinite; } \
             #second { animation:second 1s linear infinite; }"
        } else {
            "#first { --image:element(#second); border-image-source:var(--image); } \
             #second { --image:element(#first); border-image-source:var(--image); }"
        };
        let stylesheet = Stylesheet::parse(&format!(
            "#first, #second {{ position:relative; z-index:0; box-sizing:border-box; width:24px; height:24px; border:3px solid transparent; border-image-slice:1; }} \
             #first {{ background:red; }} #second {{ background:blue; }} {sources}"
        )).unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
            ui.add(empty_response.selector("#first"));
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector("#second")).rect);
        })
        .await;
        let rect = bounds.lock().unwrap().unwrap();
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.top() + 1.0),
            [0, 0, 255, 255],
            "animated={animated}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_hairline_resolves_to_one_device_pixel_test() {
    let stylesheet = Stylesheet::parse(
        ".hairline { box-sizing:border-box; width:20px; height:20px; border:hairline solid red; background:white; }",
    )
    .unwrap();
    for scale in [1.0, 2.0] {
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_at_scale(stylesheet.clone(), scale, move |ui| {
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".hairline")).rect);
        })
        .await;
        let rect = bounds.lock().unwrap().unwrap();
        let y = rect.center().y * scale;
        assert_eq!(
            pixel(&pixels, (rect.left() + 0.25 / scale) * scale, y),
            [255, 0, 0, 255]
        );
        assert_eq!(
            pixel(&pixels, (rect.left() + 1.25 / scale) * scale, y),
            [255, 255, 255, 255]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_layers_use_their_matching_clip_values_test() {
    let stylesheet = Stylesheet::parse(
        ".layered { box-sizing:border-box; width:40px; height:40px; padding:6px; border:4px solid black; background-image:linear-gradient(red, red), linear-gradient(blue, blue); background-clip:content-box, border-box; } .shorthand { box-sizing:border-box; width:40px; height:40px; padding:6px; border:4px solid black; background:linear-gradient(red, red) content-box, linear-gradient(blue, blue) border-box yellow; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_response.selector(".layered")).rect);
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_response.selector(".shorthand")).rect);
    })
    .await;
    for rect in bounds.lock().unwrap().iter() {
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.center().y),
            [255, 0, 0, 255]
        );
        assert_eq!(
            pixel(&pixels, rect.left() + 7.0, rect.center().y),
            [0, 0, 255, 255]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_shape_circle_clips_background_and_draws_its_stroke_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing: border-box; width: 40px; height: 40px; border-shape: circle(50%); border: 4px solid blue; background-color: red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let stroke = pixel(&pixels, rect.center().x, rect.top() + 1.0);
    assert!(
        stroke[0] < 20 && stroke[1] < 20 && stroke[2] > 240,
        "{stroke:?}"
    );
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 1.0, rect.top() + 1.0),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_shape_outer_shadow_spread_expands_the_shape_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing:border-box; width:80px; height:80px; border-shape:circle(20px) border-box; background:white; box-shadow:0 0 0 8px green; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let expanded_edge = pixel(&pixels, rect.center().x + 24.0, rect.center().y);
    assert_eq!(expanded_edge, [0, 128, 0, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_shape_outer_shadow_preserves_evenodd_holes_test() {
    let stylesheet = Stylesheet::parse(
        ".evenodd { box-sizing:border-box; width:60px; height:40px; border-shape:path(evenodd, \"M 0 0 H 60 V 40 H 0 Z M 15 10 H 45 V 30 H 15 Z\"); background:white; box-shadow:0 0 0 4px green; }\
         .nonzero { box-sizing:border-box; width:60px; height:40px; border-shape:path(nonzero, \"M 0 0 H 60 V 40 H 0 Z M 15 10 V 30 H 45 V 10 Z\"); background:white; box-shadow:0 0 0 4px green; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".evenodd")).rect,
            ui.add(empty_response.selector(".nonzero")).rect,
        ];
    })
    .await;
    for rect in bounds.lock().unwrap().iter() {
        assert_eq!(
            pixel(&pixels, rect.left() + 16.0, rect.center().y),
            [0, 128, 0, 255]
        );
        assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_shape_inset_shadow_uses_inner_path_subcontours_test() {
    let stylesheet = Stylesheet::parse(
        ".evenodd { box-sizing:border-box; width:80px; height:60px; border:5px solid blue; border-shape:path(\"M 0 0 H 80 V 60 H 0 Z\") path(evenodd, \"M 0 0 H 70 V 50 H 0 Z M 20 10 H 50 V 40 H 20 Z\"); background:white; box-shadow:inset 0 0 0 4px green; }\
         .nonzero { box-sizing:border-box; width:80px; height:60px; border:5px solid blue; border-shape:path(\"M 0 0 H 80 V 60 H 0 Z\") path(nonzero, \"M 0 0 H 70 V 50 H 0 Z M 20 10 V 40 H 50 V 10 Z\"); background:white; box-shadow:inset 0 0 0 4px green; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".evenodd")).rect,
            ui.add(empty_response.selector(".nonzero")).rect,
        ];
    })
    .await;
    for rect in bounds.lock().unwrap().iter() {
        let shadow_at_inner_contour = pixel(&pixels, rect.left() + 23.0, rect.center().y);
        assert_eq!(shadow_at_inner_contour, [0, 128, 0, 255]);
        assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn basic_shape_positions_support_edge_offsets_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing:border-box; width:40px; height:40px; border:2px solid blue; background-color:red; border-shape:ellipse(5px at right 10px bottom 10px) border-box; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.right() - 10.0, rect.bottom() - 10.0),
        [255, 0, 0, 255]
    );
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_shape_clips_gradient_background_images_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing:border-box; width:40px; height:40px; border:4px solid blue; background-image:linear-gradient(red, blue); border-shape:circle(50%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 1.0, rect.top() + 1.0),
        [255; 4]
    );
    assert_ne!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_shape_clips_tiled_url_background_images_test() {
    let source = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 255, 255]));
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    let css = format!(
        ".shape {{ box-sizing:border-box; width:40px; height:40px; border:4px solid blue; background-image:url(\"data:image/png;base64,{data}\"); border-shape:circle(50%); }}"
    );
    let stylesheet = Stylesheet::load(&css).await.unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [255, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 1.0, rect.top() + 1.0),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_shape_clips_descendant_content_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { box-sizing:border-box; width:40px; height:40px; overflow:hidden; border-shape:circle(50%); }\
         .child { box-sizing:border-box; width:60px; height:60px; background-color:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut child = None;
        let parent = ui.add(
            (|ui: &mut egui::Ui| {
                child = Some(ui.add(empty_response.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some((parent.rect, child.unwrap()));
    })
    .await;
    let (parent, child) = bounds.lock().unwrap().unwrap();
    assert!(child.right() > parent.right());
    assert_eq!(
        pixel(&pixels, parent.center().x, parent.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, parent.left() + 1.0, parent.top() + 1.0),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, parent.right() + 5.0, parent.center().y),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_shape_preserves_visible_descendant_overflow_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { box-sizing:border-box; width:40px; height:40px; overflow:visible; border-shape:circle(50%); }\
         .child { box-sizing:border-box; width:60px; height:60px; background-color:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut child = None;
        let parent = ui.add(
            (|ui: &mut egui::Ui| {
                child = Some(ui.add(empty_response.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some((parent.rect, child.unwrap()));
    })
    .await;
    let (parent, child) = bounds.lock().unwrap().unwrap();
    assert!(child.right() > parent.right());
    assert_eq!(
        pixel(&pixels, parent.left() + 1.0, parent.top() + 1.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, parent.right() + 5.0, parent.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_limit_percentages_use_border_box_dimensions_test() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:100px; height:60px; border:4px solid black; border-radius:20px; border-limit:sides 50%; background:white; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 26.0, rect.top() + 1.0),
        [0, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 24.0, rect.top() + 1.0),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_radius_clips_overflow_content_to_the_curved_padding_edge_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { box-sizing:border-box; width:40px; height:40px; overflow:hidden; border-radius:50%; }\
         .child { box-sizing:border-box; width:60px; height:60px; background-color:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut child = None;
        let parent = ui.add(
            (|ui: &mut egui::Ui| {
                child = Some(ui.add(empty_response.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some((parent.rect, child.unwrap()));
    })
    .await;
    let (parent, child) = bounds.lock().unwrap().unwrap();
    assert!(child.right() > parent.right());
    assert_eq!(
        pixel(&pixels, parent.center().x, parent.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, parent.left() + 1.0, parent.top() + 1.0),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, parent.right() + 5.0, parent.center().y),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_radius_clips_when_visible_overflow_computes_to_auto_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { box-sizing:border-box; width:40px; height:40px; overflow-x:visible; overflow-y:hidden; border-radius:50%; }\
         .child { box-sizing:border-box; width:60px; height:60px; background-color:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut child = None;
        let parent = ui.add(
            (|ui: &mut egui::Ui| {
                child = Some(ui.add(empty_response.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some((parent.rect, child.unwrap()));
    })
    .await;
    let (parent, child) = bounds.lock().unwrap().unwrap();
    assert!(child.right() > parent.right());
    assert_eq!(
        pixel(&pixels, parent.center().x, parent.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, parent.left() + 1.0, parent.top() + 1.0),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn overflow_clip_margin_expands_rounded_descendant_clipping_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { box-sizing:border-box; width:40px; height:40px; overflow:clip; overflow-clip-margin:5px; border-radius:50%; }\
         .child { box-sizing:border-box; width:60px; height:60px; background-color:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let parent = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".child"));
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some(parent.rect);
    })
    .await;
    let parent = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, parent.left() + 4.0, parent.top() + 4.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, parent.left() - 4.0, parent.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn overflow_clip_margin_expands_border_shape_descendant_clipping_test() {
    let stylesheet = Stylesheet::parse(
        ".parent { box-sizing:border-box; width:40px; height:40px; overflow:clip; overflow-clip-margin:5px; border-shape:circle(50%); }\
         .child { box-sizing:border-box; width:60px; height:60px; background-color:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let parent = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".child"));
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some(parent.rect);
    })
    .await;
    let parent = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, parent.left() + 4.0, parent.top() + 4.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, parent.left() - 4.0, parent.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn image_set_selects_the_nearest_supported_resolution_test() {
    let data_url = |color| {
        let source = image::RgbaImage::from_pixel(1, 1, image::Rgba(color));
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(source)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let data = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            encoded.into_inner(),
        );
        format!("data:image/png;base64,{data}")
    };
    let high_density = data_url([255, 0, 0, 255]);
    let standard_density = data_url([0, 0, 255, 255]);
    let css = format!(
        ".image {{ box-sizing:border-box; width:20px; height:20px; background-image:image-set(url(\"data:image/svg+xml;base64,AA==\") 4x type(\"image/unsupported\"), url(\"{high_density}\") 2x type(\"image/png\"), url(\"{standard_density}\") 1x); }}"
    );
    let stylesheet = Stylesheet::load(&css).await.unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn image_set_resolution_scales_background_intrinsic_size_test() {
    let mut source = image::RgbaImage::new(4, 4);
    for (x, _, pixel) in source.enumerate_pixels_mut() {
        *pixel = if x < 2 {
            image::Rgba([255, 0, 0, 255])
        } else {
            image::Rgba([0, 0, 255, 255])
        };
    }
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    let stylesheet = Stylesheet::load(&format!(
        ".image {{ box-sizing:border-box; width:20px; height:20px; background-image:image-set(url(\"data:image/png;base64,{data}\") 2x); }}"
    ))
    .await
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_scale(stylesheet, 2.0, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let blue_half = pixel(&pixels, rect.left() * 2.0 + 2.5, rect.center().y * 2.0);
    assert!(
        blue_half[2] >= 240 && blue_half[0] <= 15,
        "2x background image was not density-corrected: {blue_half:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn image_color_paints_as_a_background_image_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:20px; height:20px; background-image:image(rgb(255, 0, 0)); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn cross_fade_combines_loaded_images_and_colors_test() {
    let source = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    let stylesheet = Stylesheet::load(&format!(
        ".image {{ box-sizing:border-box; width:20px; height:20px; background-image:cross-fade(url(\"data:image/png;base64,{data}\") 50%, blue 50%); }} .transparent {{ box-sizing:border-box; width:20px; height:20px; background-color:blue; background-image:cross-fade(red 50%, transparent 50%); }}"
    ))
    .await
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_response.selector(".image")).rect);
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_response.selector(".transparent")).rect);
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let mixed = pixel(&pixels, bounds[0].center().x, bounds[0].center().y);
    assert!(
        (120..=135).contains(&mixed[0]) && mixed[1] <= 5 && (120..=135).contains(&mixed[2]),
        "expected a 50/50 premultiplied cross-fade: {mixed:?}"
    );
    let translucent = pixel(&pixels, bounds[1].center().x, bounds[1].center().y);
    assert!(
        translucent[0] > 110 && translucent[2] > 110 && translucent[1] <= 5,
        "transparent cross-fade should preserve the red source when composited: {translucent:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn cross_fade_accepts_percentage_before_image_test() {
    let stylesheet = Stylesheet::parse(
        ".blend { box-sizing:border-box; width:20px; height:20px; background-image:cross-fade(25% red, 75% blue); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".blend")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [64, 0, 191, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn image_set_selects_cross_fade_candidate_at_output_scale_test() {
    let stylesheet = Stylesheet::parse(
        ".blend { box-sizing:border-box; width:20px; height:20px; background-image:image-set(cross-fade(image(red) 100%) 1x, cross-fade(image(blue) 100%) 2x); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_scale(stylesheet, 2.0, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".blend")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x * 2.0, rect.center().y * 2.0),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn image_set_keeps_generated_cross_fade_natural_size_test() {
    let mut source = image::RgbaImage::new(8, 1);
    for (x, _, pixel) in source.enumerate_pixels_mut() {
        *pixel = if x < 4 {
            image::Rgba([255, 0, 0, 255])
        } else {
            image::Rgba([0, 0, 255, 255])
        };
    }
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    let stylesheet = Stylesheet::load(&format!(
        ".image {{ box-sizing:border-box; width:16px; height:4px; background-image:image-set(cross-fade(url(\"data:image/png;base64,{data}\") 100%) 2x); }}"
    ))
    .await
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 5.0, rect.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn cross_fade_rejects_nested_image_set_test() {
    let result = Stylesheet::parse(
        ".invalid { background-image:cross-fade(image-set(image(red) 1x, image(blue) 2x) 100%); }",
    );
    assert!(result.is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn conic_gradient_background_uses_clockwise_css_angles_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { box-sizing:border-box; width:40px; height:40px; background-image:conic-gradient(red 0%, blue 100%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".gradient")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let right = pixel(&pixels, rect.center().x + 10.0, rect.center().y);
    assert!((160..=225).contains(&right[0]));
    assert!((55..=105).contains(&right[1]));
    assert!((20..=125).contains(&right[2]));
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn conic_gradient_resolves_start_angle_and_center_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { box-sizing:border-box; width:40px; height:40px; background-image:conic-gradient(from 90deg at right 10px bottom 10px, red 0%, blue 100%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".gradient")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let right = pixel(&pixels, rect.left() + 35.0, rect.top() + 30.0);
    assert!(
        right[0] >= 245 && right[2] <= 30,
        "unexpected conic start color: {right:?}"
    );
    let top = pixel(&pixels, rect.left() + 30.0, rect.top() + 25.0);
    assert!(top[0] < 100);
    assert!(top[2] > 150);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn repeating_linear_gradient_repeats_its_stop_interval_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { box-sizing:border-box; width:40px; height:40px; background-image:repeating-linear-gradient(red 0%, blue 25%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".gradient")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let repeated = pixel(&pixels, rect.center().x, rect.center().y);
    assert!(
        repeated[0] >= 220 && repeated[2] <= 35,
        "unexpected repeated stop: {repeated:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn gradient_color_stops_can_specify_a_hard_stop_interval_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { box-sizing:border-box; width:40px; height:20px; background-image:linear-gradient(to right, red 0%, blue 40% 60%, red 100%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".gradient")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let hard_stop = pixel(&pixels, rect.center().x, rect.center().y);
    assert!(
        hard_stop[2] >= 240 && hard_stop[0] <= 15,
        "expected the specified blue hard-stop interval, got {hard_stop:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn gradient_transition_hints_place_the_midpoint_color_test() {
    let stylesheet = Stylesheet::parse(
        ".hint { box-sizing:border-box; width:40px; height:20px; background-image:linear-gradient(to right, red 0%, 25%, blue 100%); } .linear { box-sizing:border-box; width:40px; height:20px; background-image:linear-gradient(to right, red 0%, blue 100%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_response.selector(".hint")).rect);
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_response.selector(".linear")).rect);
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let hinted = pixel(&pixels, bounds[0].left() + 10.0, bounds[0].center().y);
    let linear = pixel(&pixels, bounds[1].left() + 10.0, bounds[1].center().y);
    assert!(
        hinted[0] + 40 < linear[0] && hinted[2] > linear[2] + 40,
        "hint should move the midpoint toward 25%: hinted={hinted:?}, linear={linear:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn gradients_use_oklab_by_default_and_can_select_srgb_test() {
    let stylesheet = Stylesheet::parse(
        ".oklab { box-sizing:border-box; width:40px; height:20px; background-image:linear-gradient(to right, red, blue); } .srgb { box-sizing:border-box; width:40px; height:20px; background-image:linear-gradient(in srgb to right, red, blue); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_response.selector(".oklab")).rect);
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_response.selector(".srgb")).rect);
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let oklab = pixel(&pixels, bounds[0].center().x, bounds[0].center().y);
    let srgb = pixel(&pixels, bounds[1].center().x, bounds[1].center().y);
    assert!(oklab[1] >= 55, "expected Oklab interpolation: {oklab:?}");
    assert!(
        (120..=135).contains(&srgb[0]) && srgb[1] <= 5 && (120..=135).contains(&srgb[2]),
        "expected sRGB interpolation: {srgb:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn radial_gradient_resolves_ellipse_radii_and_position_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { box-sizing:border-box; width:40px; height:40px; background-image:radial-gradient(ellipse 50% 25% at right 10px bottom 10px, red, blue); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".gradient")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let center = pixel(&pixels, rect.left() + 30.0, rect.top() + 30.0);
    assert!(
        center[0] >= 220 && center[2] <= 55,
        "unexpected radial center: {center:?}"
    );
    let edge = pixel(&pixels, rect.left() + 9.0, rect.top() + 30.0);
    assert!(
        edge[2] >= 235 && edge[0] <= 20,
        "unexpected radial edge: {edge:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn image_set_border_image_source_uses_the_selected_resolution_test() {
    let data_url = |color| {
        let source = image::RgbaImage::from_pixel(1, 1, image::Rgba(color));
        let mut encoded = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(source)
            .write_to(&mut encoded, image::ImageFormat::Png)
            .unwrap();
        let data = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            encoded.into_inner(),
        );
        format!("data:image/png;base64,{data}")
    };
    let high_density = data_url([255, 0, 0, 255]);
    let standard_density = data_url([0, 0, 255, 255]);
    let css = format!(
        ".image {{ box-sizing:border-box; width:20px; height:20px; border:4px solid black; border-image-source:image-set(url(\"{high_density}\") 2x, url(\"{standard_density}\") 1x); border-image-slice:1; border-image-width:4; }}"
    );
    let stylesheet = Stylesheet::load(&css).await.unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 2.0),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn image_color_paints_as_a_border_image_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:20px; height:20px; border:4px solid black; border-image-source:image(rgb(255, 0, 0)); border-image-slice:1; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 2.0),
        [255, 0, 0, 255]
    );
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn solid_color_border_image_honors_space_repeat_test() {
    let stylesheet = Stylesheet::parse(
        ".space { box-sizing:border-box; width:30px; height:20px; border:4px solid black; background:white; border-image-source:image(rgb(255, 0, 0)); border-image-slice:1; border-image-width:4; border-image-repeat:space; }\
         .stretch { box-sizing:border-box; width:30px; height:20px; border:4px solid black; background:white; border-image-source:image(rgb(255, 0, 0)); border-image-slice:1; border-image-width:4; border-image-repeat:stretch; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".space")).rect,
            ui.add(empty_response.selector(".stretch")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(
        pixel(&pixels, bounds[0].center().x, bounds[0].top() + 2.0),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, bounds[0].left() + 1.0, bounds[0].top() + 1.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].top() + 2.0),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_image_repeat_tiles_the_center_region_test() {
    let mut source = image::RgbaImage::from_pixel(6, 6, image::Rgba([0, 255, 0, 255]));
    for y in 1..5 {
        for x in 1..5 {
            source.put_pixel(
                x,
                y,
                if x < 3 {
                    image::Rgba([255, 0, 0, 255])
                } else {
                    image::Rgba([0, 0, 255, 255])
                },
            );
        }
    }
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    let stylesheet = Stylesheet::load(&format!(
        ".image {{ box-sizing:border-box; width:20px; height:20px; border:2px solid black; border-image-source:url(\"data:image/png;base64,{data}\"); border-image-slice:1 fill; border-image-width:1; border-image-repeat:repeat; }}"
    ))
    .await
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let repeated_tile = pixel(&pixels, rect.left() + 15.0, rect.center().y);
    assert!(
        repeated_tile[0] > 240 && repeated_tile[2] < 15,
        "expected the center source tile to repeat, got {repeated_tile:?}"
    );
    let leading_tile = pixel(&pixels, rect.left() + 2.0, rect.center().y);
    assert!(
        leading_tile != [255; 4],
        "expected the centered repeat sequence to cover the leading region, got {leading_tile:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_image_slice_values_clamp_to_image_dimensions_test() {
    let source = image::RgbaImage::from_pixel(6, 6, image::Rgba([0, 255, 0, 255]));
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    let stylesheet = Stylesheet::load(&format!(
        ".number {{ box-sizing:border-box; width:30px; height:30px; border:2px solid black; border-image-source:url(\"data:image/png;base64,{data}\"); border-image-slice:100; border-image-width:auto; }}\
         .percentage {{ box-sizing:border-box; width:30px; height:30px; border:2px solid black; border-image-source:url(\"data:image/png;base64,{data}\"); border-image-slice:200%; border-image-width:auto; }}"
    ))
    .await
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".number")).rect,
            ui.add(empty_response.selector(".percentage")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    for rect in bounds.iter() {
        assert_eq!(
            pixel(&pixels, rect.left() + 2.0, rect.top() + 2.0),
            [0, 255, 0, 255]
        );
        assert_eq!(
            pixel(&pixels, rect.left() + 8.0, rect.top() + 8.0),
            [255; 4]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_image_slice_rejects_duplicate_fill_keywords_test() {
    assert!(Stylesheet::parse(".image { border-image-slice:1 fill fill; }").is_err());
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn conic_gradient_paints_as_a_border_image_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:20px; height:20px; border:4px solid black; border-image-source:conic-gradient(red, blue); border-image-slice:5; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let top = pixel(&pixels, rect.center().x, rect.top() + 2.0);
    assert!(
        top[0] >= 240 && top[2] <= 45,
        "unexpected conic border image: {top:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_shape_clips_border_image_paint_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:40px; height:40px; border:4px solid black; border-shape:circle(50%); border-image-source:linear-gradient(to right, red, blue); border-image-slice:5; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let corner = pixel(&pixels, rect.left() + 1.0, rect.top() + 1.0);
    let top = pixel(&pixels, rect.center().x, rect.top() + 2.0);
    assert_eq!(corner, [255, 255, 255, 255]);
    assert_ne!(
        top,
        [255, 255, 255, 255],
        "expected the border image: {top:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn cross_fade_paints_as_a_border_image_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:20px; height:20px; border:4px solid black; border-image-source:cross-fade(red, blue 75%); border-image-slice:5; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let top = pixel(&pixels, rect.center().x, rect.top() + 2.0);
    assert!(
        (55..=75).contains(&top[0]) && (180..=205).contains(&top[2]),
        "cross-fade border image was not blended: {top:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn radial_gradient_paints_as_a_border_image_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:20px; height:20px; border:4px solid black; border-image-source:radial-gradient(circle 15px, red, blue); border-image-slice:5; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let top = pixel(&pixels, rect.center().x, rect.top() + 2.0);
    assert!(
        top[0] > 80 && top[2] > 80,
        "radial border image did not interpolate its source: {top:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn partial_borders_draw_only_the_limited_or_clipped_segments_test() {
    let stylesheet = Stylesheet::parse(
        ".limited { box-sizing: border-box; width: 60px; height: 30px; border: 2px solid blue; border-limit: sides 50%; }\
         .clipped { box-sizing: border-box; width: 60px; height: 30px; border: 2px solid red; border-top-clip: 0 1fr; }\
         .fractional { box-sizing: border-box; width: 60px; height: 30px; border: 2px solid black; border-top-clip: 0.25fr; }\
         .fractional-fixed { box-sizing: border-box; width: 60px; height: 30px; border: 2px solid black; border-top-clip: 0.25fr 10px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".limited")).rect,
            ui.add(empty_response.selector(".clipped")).rect,
            ui.add(empty_response.selector(".fractional")).rect,
            ui.add(empty_response.selector(".fractional-fixed")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(
        pixel(&pixels, bounds[0].left() + 5.0, bounds[0].top() + 0.5),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, bounds[0].center().x, bounds[0].top() + 0.5),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].top() + 0.5),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].left() + 0.5, bounds[1].center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[2].left() + 10.0, bounds[2].top() + 0.5),
        [0, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[2].center().x, bounds[2].top() + 0.5),
        [0, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[3].left() + 8.0, bounds[3].top() + 0.5),
        [0, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[3].left() + 17.0, bounds[3].top() + 0.5),
        [0, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[3].left() + 55.0, bounds[3].top() + 0.5),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn zero_width_adjacent_border_takes_the_entire_corner_transition_test() {
    let stylesheet = Stylesheet::parse(
        ".corner { box-sizing:border-box; width:50px; height:50px; border:0 solid blue; border-left:10px solid red; border-radius:20px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".corner")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let transition = pixel(&pixels, rect.left() + 10.0, rect.top() + 5.0);
    assert!(
        transition[0] > 200 && transition[1] < 64 && transition[2] < 64,
        "zero-width adjacent border left an unpainted corner transition: {transition:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_corner_color_transition_tracks_border_width_ratio_test() {
    let stylesheet = Stylesheet::parse(
        ".corner { box-sizing:border-box; width:50px; height:50px; border:0 solid; border-top:2px solid blue; border-left:10px solid red; border-radius:20px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".corner")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let transition = pixel(&pixels, rect.left() + 11.0, rect.top() + 3.0);
    assert!(
        transition[0] > 200 && transition[1] < 64 && transition[2] < 64,
        "wider adjacent border did not retain the larger corner share: {transition:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn zero_flex_border_clip_does_not_create_an_implicit_flex_test() {
    let stylesheet = Stylesheet::parse(
        ".zero { box-sizing:border-box; width:60px; height:30px; border:2px solid black; border-top-clip:0fr; }\
         .fixed { box-sizing:border-box; width:60px; height:30px; border:2px solid black; border-top-clip:0fr 10px; }\
         .implicit { box-sizing:border-box; width:60px; height:30px; border:2px solid black; border-top-clip:0 10px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 3]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = [
            Some(ui.add(empty_response.selector(".zero")).rect),
            Some(ui.add(empty_response.selector(".fixed")).rect),
            Some(ui.add(empty_response.selector(".implicit")).rect),
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    for rect in bounds[..2].iter().flatten() {
        assert_eq!(pixel(&pixels, rect.center().x, rect.top() + 0.5), [255; 4]);
        assert_eq!(
            pixel(&pixels, rect.left() + 0.5, rect.center().y),
            [0, 0, 0, 255]
        );
    }
    let implicit = bounds[2].unwrap();
    assert_eq!(
        pixel(&pixels, implicit.center().x, implicit.top() + 0.5),
        [0, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn striped_border_color_paints_from_the_padding_edge_outward_test() {
    let stylesheet = Stylesheet::parse(
        ".striped { box-sizing:border-box; width:60px; height:30px; border:8px solid stripes(red 1fr, blue 1fr); }\
         .partial { box-sizing:border-box; width:60px; height:30px; border:8px solid stripes(red 0.25fr, blue 0.25fr); background:white; }\
         .partial-fixed { box-sizing:border-box; width:80px; height:80px; border:40px solid stripes(red 0.1fr, blue 0.2fr, green 10px); background:white; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 3]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = [
            Some(ui.add(empty_response.selector(".striped")).rect),
            Some(ui.add(empty_response.selector(".partial")).rect),
            Some(ui.add(empty_response.selector(".partial-fixed")).rect),
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let rect = bounds[0].unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 1.0),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 5.0),
        [255, 0, 0, 255]
    );
    let partial = bounds[1].unwrap();
    assert_eq!(
        pixel(&pixels, partial.center().x, partial.top() + 0.25),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, partial.center().x, partial.top() + 4.25),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, partial.center().x, partial.top() + 7.25),
        [255, 0, 0, 255]
    );
    let partial_fixed = bounds[2].unwrap();
    assert_eq!(
        pixel(
            &pixels,
            partial_fixed.center().x,
            partial_fixed.top() + 0.25
        ),
        [255; 4]
    );
    assert_eq!(
        pixel(
            &pixels,
            partial_fixed.center().x,
            partial_fixed.top() + 22.25
        ),
        [0, 128, 0, 255]
    );
    assert_eq!(
        pixel(
            &pixels,
            partial_fixed.center().x,
            partial_fixed.top() + 32.25
        ),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(
            &pixels,
            partial_fixed.center().x,
            partial_fixed.top() + 38.25
        ),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn striped_border_shape_color_starts_at_the_padding_edge_test() {
    let stylesheet = Stylesheet::parse(
        ".shaped { box-sizing:border-box; width:60px; height:60px; border:8px solid stripes(red 1fr, blue 1fr); border-shape:circle(50%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shaped")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let outer = pixel(&pixels, rect.center().x, rect.top() + 1.0);
    let inner = pixel(&pixels, rect.center().x, rect.top() + 7.0);
    assert!(
        outer[2] > outer[0] * 4,
        "unexpected outer stripe color: {outer:?}"
    );
    assert!(
        inner[0] > inner[2] * 4,
        "unexpected inner stripe color: {inner:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn linear_gradient_border_image_is_sliced_and_painted_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing: border-box; width: 60px; height: 30px; border: 8px solid black; border-image-source: linear-gradient(red, blue); border-image-slice: calc(20% + 20%); border-image-width: 8; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let top = pixel(&pixels, rect.center().x, rect.top() + 4.0);
    let bottom = pixel(&pixels, rect.center().x, rect.bottom() - 4.0);
    assert!(top[0] > top[2], "{top:?}");
    assert!(bottom[2] > bottom[0], "{bottom:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_image_width_auto_uses_border_width_for_gradients_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:80px; height:60px; border:4px solid black; border-image-source:linear-gradient(to bottom, red, blue); border-image-slice:20; border-image-width:auto; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let inside_border = pixel(&pixels, rect.center().x, rect.top() + 2.0);
    let outside_border = pixel(&pixels, rect.center().x, rect.top() + 10.0);
    assert!(inside_border[0] > inside_border[2], "{inside_border:?}");
    assert_eq!(outside_border, [255, 255, 255, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_image_width_and_outset_use_computed_border_widths_when_style_is_none_test() {
    let stylesheet = Stylesheet::parse(
        ".width { box-sizing:border-box; width:60px; height:40px; background:white; border-style:none; border-width:4px; border-image:image(red) calc(0.5 + 0.5) / calc(1 + 1); }\
         .outset { box-sizing:border-box; width:60px; height:40px; background:white; border-style:none; border-width:4px; border-image:image(red) 1 / calc(2px + 2px) / calc(1 + 1); }\
         .length-width { box-sizing:border-box; width:60px; height:40px; background:white; border-style:none; border-width:4px; border-image:image(red) 1 / calc(4px + 10%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 3]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = [
            Some(ui.add(empty_response.selector(".width")).rect),
            Some(ui.add(empty_response.selector(".outset")).rect),
            Some(ui.add(empty_response.selector(".length-width")).rect),
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let width = bounds[0].unwrap();
    assert_eq!(
        pixel(&pixels, width.center().x, width.top() + 2.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, width.center().x, width.top() + 10.0),
        [255; 4]
    );
    let outset = bounds[1].unwrap();
    assert_eq!(
        pixel(&pixels, outset.right() + 6.0, outset.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, outset.right() + 2.0, outset.center().y),
        [255; 4]
    );
    let length_width = bounds[2].unwrap();
    assert_eq!(
        pixel(&pixels, length_width.center().x, length_width.top() + 2.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, length_width.center().x, length_width.top() + 10.0),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_image_width_auto_resolves_density_corrected_slices_test() {
    let source = image::RgbaImage::from_pixel(16, 16, image::Rgba([255, 0, 0, 255]));
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    let stylesheet = Stylesheet::load(&format!(
        ".image {{ box-sizing:border-box; width:30px; height:30px; border:2px solid black; border-image-source:image-set(url(\"data:image/png;base64,{data}\") 2x); border-image-slice:4; border-image-width:auto; }}"
    ))
    .await
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_scale(stylesheet, 2.0, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let beyond_natural_slice = pixel(&pixels, rect.center().x * 2.0, rect.top() * 2.0 + 6.0);
    assert_eq!(beyond_natural_slice, [255, 255, 255, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn svg_path_border_shape_uses_its_closed_contour_test() {
    let stylesheet = Stylesheet::parse(
        ".diamond { box-sizing: border-box; width: 40px; height: 40px; border: 4px solid blue; background-color: red; border-shape: path(\"M 20 0 L 40 20 L 20 40 L 0 20 Z\"); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".diamond")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 1.0, rect.top() + 1.0),
        [255; 4]
    );
    let border = pixel(&pixels, rect.center().x, rect.top() + 1.0);
    assert!(
        border[0] < 20 && border[1] < 20 && border[2] > 240,
        "{border:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn svg_path_border_shape_preserves_subpaths_and_evenodd_holes_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing: border-box; width: 60px; height: 40px; border: 4px solid blue; background-color: red; border-shape: path(evenodd, \"M 0 0 H 60 V 40 H 0 Z M 15 10 H 45 V 30 H 15 Z\"); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 7.0, rect.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
    let inner_border = pixel(&pixels, rect.center().x, rect.top() + 10.0);
    assert!(
        inner_border[0] < 20 && inner_border[1] < 20 && inner_border[2] > 240,
        "{inner_border:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn shape_border_shape_handles_curves_and_evenodd_subpaths_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing: border-box; width: 60px; height: 40px; border: 2px solid blue; background-color: red; border-shape: shape(evenodd from 0 0, hline to 60px, vline to 40px, hline to 0, close, move to 15px 10px, curve to 45px 10px with 25px 10px / 35px 10px, vline by 20px, curve to 15px 30px with 35px 30px / 25px 30px, close); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 5.0, rect.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
    let inner_border = pixel(&pixels, rect.center().x, rect.top() + 10.0);
    assert!(
        inner_border[0] < 20 && inner_border[1] < 20 && inner_border[2] > 240,
        "{inner_border:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn shape_border_shape_arcs_resolve_css_radii_and_sweep_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing: border-box; width: 50px; height: 40px; border: 2px solid blue; background-color: red; border-shape: shape(from 0 0, hline to 40px, arc to 50px 10px of 10px cw small, vline to 40px, hline to 0, close) border-box; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() - 5.0, rect.top() + 5.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() - 1.0, rect.top() + 1.0),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn polygon_border_shape_respects_fill_rules_on_retraced_contours_test() {
    let stylesheet = Stylesheet::parse(
        ".row { display:flex; width:120px; height:60px; }\
         .even, .nonzero { box-sizing:border-box; flex-shrink:0; width:60px; height:60px; border:2px solid blue; background-color:red; }\
         .even { border-shape:polygon(evenodd, 0 0, 100% 0, 100% 100%, 0 100%, 0 0, 100% 0, 100% 100%, 0 100%) border-box; }\
         .nonzero { border-shape:polygon(nonzero, 0 0, 100% 0, 100% 100%, 0 100%, 0 0, 100% 0, 100% 100%, 0 100%) border-box; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = vec![
                    ui.add(empty_response.selector(".even")).rect,
                    ui.add(empty_response.selector(".nonzero")).rect,
                ];
                ui.response()
            })
            .selector(".row"),
        );
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(
        pixel(&pixels, bounds[0].center().x, bounds[0].center().y),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn outer_and_inset_box_shadows_paint_on_their_respective_sides_test() {
    let stylesheet = Stylesheet::parse(
        ".outer { box-sizing: border-box; width: 60px; height: 30px; background-color: red; box-shadow: calc(2px + 2px) calc(1px + 3px) calc(0px + 0px) blue; }\
         .inner { box-sizing: border-box; width: 60px; height: 30px; background-color: red; box-shadow: 4px 4px 0 blue inset; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".outer")).rect,
            ui.add(empty_response.selector(".inner")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(
        pixel(&pixels, bounds[0].right() + 2.0, bounds[0].center().y + 4.0),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].top() + 1.0),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inset_box_shadow_blur_follows_gaussian_edge_profile_test() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:220px; height:100px; border:8px solid blue; border-radius:20px; background-color:white; box-shadow:inset 0 0 20px red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    for (distance, expected_green) in [
        (2.0, 153),
        (5.0, 181),
        (10.0, 218),
        (15.0, 240),
        (20.0, 250),
        (25.0, 254),
    ] {
        let sample = pixel(&pixels, rect.center().x, rect.top() + 8.0 + distance);
        let tolerance = (f32::from(expected_green) * 0.05).ceil() as u8;
        assert_eq!(sample[0], 255);
        assert!(
            sample[1].abs_diff(expected_green) <= tolerance,
            "inset blur differs from a Gaussian edge at {distance}px: expected green {expected_green}, got {sample:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inset_box_shadow_spread_and_offset_follow_padding_edge_test() {
    let stylesheet = Stylesheet::parse(
        ".spread { box-sizing:border-box; width:80px; height:60px; border:4px solid blue; background:white; box-shadow:inset 0 0 0 4px red; }\
         .offset { box-sizing:border-box; width:80px; height:60px; border:4px solid blue; background:white; box-shadow:inset 0 5px 0 green; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_response.selector(".spread")).rect,
            ui.add(empty_response.selector(".offset")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let spread = bounds[0];
    assert_eq!(
        pixel(&pixels, spread.center().x, spread.top() + 6.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, spread.center().x, spread.top() + 9.0),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, spread.center().x, spread.top() + 2.0),
        [0, 0, 255, 255]
    );

    let offset = bounds[1];
    assert_eq!(
        pixel(&pixels, offset.center().x, offset.top() + 6.0),
        [0, 128, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, offset.center().x, offset.bottom() - 6.0),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn box_shadow_offset_none_is_case_insensitive_test() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:20px; height:20px; box-shadow-offset:NONE; box-shadow-blur:4px; box-shadow-color:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.right() + 1.0, rect.center().y),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn box_shadow_longhand_lists_repeat_and_truncate_to_offsets_test() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:20px; height:20px; background:white; box-shadow-offset:calc(2px + 3px) 0, 25px 0, 45px 0; box-shadow-blur:calc(0px + 0px), 0px, 0px, 100px; box-shadow-spread:calc(0px + 0px); box-shadow-color:red, blue; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.right() + 2.0, rect.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() + 22.0, rect.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() + 42.0, rect.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() + 110.0, rect.center().y),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn box_shadow_blur_scales_beyond_128_pixels_test() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:60px; height:40px; background-color:white; box-shadow:0 0 200px red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let distant_shadow = pixel(&pixels, rect.right() + 125.0, rect.center().y);
    assert!(
        distant_shadow[1] < 240,
        "large blur radius was clipped: {distant_shadow:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn box_shadow_blur_approximates_gaussian_profile_test() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:60px; height:40px; background-color:white; box-shadow:0 0 100px red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    for (distance, expected_green) in [
        (5.0, 138),
        (20.0, 167),
        (45.0, 208),
        (70.0, 234),
        (95.0, 248),
        (120.0, 253),
    ] {
        let sample = pixel(&pixels, rect.right() + distance, rect.center().y);
        assert_eq!(sample[0], 255);
        let tolerance = (f32::from(expected_green) * 0.05).ceil() as u8;
        assert!(
            sample[1].abs_diff(expected_green) <= tolerance,
            "blur profile differs from a Gaussian edge at {distance}px: expected green {expected_green}, got {sample:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn percentages_auto_margins_and_min_max_use_the_containing_block_test() {
    let stylesheet=Stylesheet::parse(
        ".parent { display: flow-root; width: 200px; height: 80px; }\
         .child { width: 80%; max-width: 80px; min-height: 20px; height: 10%; margin: 5% auto; padding: 5%; box-sizing: border-box; background-color: #f00; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut child = None;
        let parent = ui.add(
            (|ui: &mut egui::Ui| {
                child = Some(ui.add(empty_response.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some((parent.rect, child.unwrap()));
    })
    .await;
    let (parent, child) = bounds.lock().unwrap().unwrap();
    assert_eq!(child.size(), egui::vec2(80.0, 20.0));
    near(child.left() - parent.left(), 60.0);
    near(child.top() - parent.top(), 10.0);
    assert_eq!(
        pixel(&pixels, child.center().x, child.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn absolute_position_skips_static_ancestors_and_does_not_consume_flow_test() {
    let stylesheet=Stylesheet::parse(
        ".outer { position: relative; width: 180px; height: 80px; padding: 5px; border: 2px solid #000; }\
         .static { margin: 10px; width: 80px; height: 30px; }\
         .absolute { position: absolute; right: 10px; bottom: 10px; width: 20px; height: 15px; background-color: #f00; }\
         .normal { width: 20px; height: 10px; background-color: #00f; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut absolute = None;
        let mut normal = None;
        let outer = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        absolute = Some(ui.add(empty_response.selector(".absolute")).rect);
                        normal = Some(ui.add(empty_response.selector(".normal")).rect);
                        ui.response()
                    })
                    .selector(".static"),
                );
                ui.response()
            })
            .selector(".outer"),
        );
        *capture.lock().unwrap() = Some((outer.rect, absolute.unwrap(), normal.unwrap()));
    })
    .await;
    let (outer, absolute, normal) = bounds.lock().unwrap().unwrap();
    near(outer.right() - absolute.right(), 12.0);
    near(outer.bottom() - absolute.bottom(), 12.0);
    near(normal.top() - outer.top(), 17.0);
    assert_eq!(
        pixel(&pixels, absolute.center().x, absolute.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, normal.center().x, normal.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn logical_border_pairs_resolve_against_direction_and_affect_box_size_test() {
    let stylesheet=Stylesheet::parse(
        ".box { direction:rtl; width:20px; height:10px; border-inline-width:2px 4px; border-inline-style:solid; border-inline-color:red blue; border-block:3px solid green; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(rect.size(), egui::vec2(26.0, 16.0));
    assert_eq!(
        pixel(&pixels, rect.left() + 1.5, rect.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.right() - 0.5, rect.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 1.5),
        [0, 128, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn upright_text_orientation_maps_logical_border_corners_with_ltr_direction_test() {
    let stylesheet = Stylesheet::parse(
        ".upright, .mixed { box-sizing:border-box; width:48px; height:48px; writing-mode:vertical-rl; direction:rtl; border:4px solid blue; border-block-start-color:green; border-inline-start-color:red; background:white; border-start-start-radius:16px; }\
         .upright { text-orientation:upright; }\
         .mixed { text-orientation:mixed; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 2]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = [
            Some(ui.add(empty_response.selector(".upright")).rect),
            Some(ui.add(empty_response.selector(".mixed")).rect),
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let upright = bounds[0].unwrap();
    let mixed = bounds[1].unwrap();
    assert_eq!(
        pixel(&pixels, upright.center().x, upright.top() + 2.5),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, upright.right() - 2.5, upright.center().y),
        [0, 128, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, mixed.center().x, mixed.bottom() - 2.5),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, mixed.right() - 2.5, mixed.center().y),
        [0, 128, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, upright.right() - 2.5, upright.top() + 2.5),
        [255; 4]
    );
    assert_ne!(
        pixel(&pixels, upright.right() - 2.5, upright.bottom() - 2.5),
        [255; 4]
    );
    assert_ne!(
        pixel(&pixels, mixed.right() - 2.5, mixed.top() + 2.5),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, mixed.right() - 2.5, mixed.bottom() - 2.5),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn corner_block_start_shorthand_maps_radii_in_inline_start_end_order_test() {
    let stylesheet = Stylesheet::parse(
        ".upright, .mixed { box-sizing:border-box; width:48px; height:48px; writing-mode:vertical-rl; direction:rtl; border:4px solid blue; corner-block-start:round 16px 0; }\
         .upright { text-orientation:upright; }\
         .mixed { text-orientation:mixed; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 2]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = [
            Some(ui.add(empty_response.selector(".upright")).rect),
            Some(ui.add(empty_response.selector(".mixed")).rect),
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let upright = bounds[0].unwrap();
    let mixed = bounds[1].unwrap();
    assert_eq!(
        pixel(&pixels, upright.right() - 2.5, upright.top() + 2.5),
        [255; 4]
    );
    assert_ne!(
        pixel(&pixels, upright.right() - 2.5, upright.bottom() - 2.5),
        [255; 4]
    );
    assert_ne!(
        pixel(&pixels, mixed.right() - 2.5, mixed.top() + 2.5),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, mixed.right() - 2.5, mixed.bottom() - 2.5),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn upright_text_orientation_maps_logical_border_clips_with_ltr_direction_test() {
    let stylesheet = Stylesheet::parse(
        ".upright, .mixed { box-sizing:border-box; width:48px; height:48px; writing-mode:vertical-rl; direction:rtl; border:2px solid black; border-inline-start-clip:0 1fr; }\
         .upright { text-orientation:upright; }\
         .mixed { text-orientation:mixed; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 2]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = [
            Some(ui.add(empty_response.selector(".upright")).rect),
            Some(ui.add(empty_response.selector(".mixed")).rect),
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let upright = bounds[0].unwrap();
    let mixed = bounds[1].unwrap();
    assert_eq!(
        pixel(&pixels, upright.center().x, upright.top() + 0.5),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, upright.center().x, upright.bottom() - 0.5),
        [0, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, mixed.center().x, mixed.top() + 0.5),
        [0, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, mixed.center().x, mixed.bottom() - 0.5),
        [255; 4]
    );
}
