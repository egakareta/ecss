async fn content_and_border_box_sizes_include_the_correct_edges_test() {
    let stylesheet = Stylesheet::parse(
        ".content { width: 60px; height: 20px; padding: 5px 10px; border: 2px solid #00f; background-color: #f00; }\
         .border { box-sizing: border-box; width: 60px; height: 30px; padding: 5px 10px; border: 2px solid #00f; background-color: #f00; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_box.selector(".content")).rect,
            ui.add(empty_box.selector(".border")).rect,
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
            .push(ui.add(empty_box.selector(".layered")).rect);
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_box.selector(".shorthand")).rect);
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

async fn border_shape_circle_clips_background_and_draws_its_stroke_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing: border-box; width: 40px; height: 40px; border-shape: circle(50%); border: 4px solid blue; background-color: red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let stroke = pixel(&pixels, rect.center().x, rect.top() + 1.0);
    assert!(stroke[0] < 20 && stroke[1] < 20 && stroke[2] > 240, "{stroke:?}");
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, rect.left() + 1.0, rect.top() + 1.0), [255; 4]);
}

async fn border_shape_outer_shadow_spread_expands_the_shape_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing:border-box; width:80px; height:80px; border-shape:circle(20px) border-box; background:white; box-shadow:0 0 0 8px green; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let expanded_edge = pixel(&pixels, rect.center().x + 24.0, rect.center().y);
    assert_eq!(expanded_edge, [0, 128, 0, 255]);
}

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
            ui.add(empty_box.selector(".evenodd")).rect,
            ui.add(empty_box.selector(".nonzero")).rect,
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
            ui.add(empty_box.selector(".evenodd")).rect,
            ui.add(empty_box.selector(".nonzero")).rect,
        ];
    })
    .await;
    for rect in bounds.lock().unwrap().iter() {
        let shadow_at_inner_contour = pixel(&pixels, rect.left() + 23.0, rect.center().y);
        assert_eq!(shadow_at_inner_contour, [0, 128, 0, 255]);
        assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
    }
}

async fn basic_shape_positions_support_edge_offsets_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing:border-box; width:40px; height:40px; border:2px solid blue; background-color:red; border-shape:ellipse(5px at right 10px bottom 10px) border-box; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.right() - 10.0, rect.bottom() - 10.0),
        [255, 0, 0, 255]
    );
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
}

async fn border_shape_clips_gradient_background_images_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing:border-box; width:40px; height:40px; border:4px solid blue; background-image:linear-gradient(red, blue); border-shape:circle(50%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.left() + 1.0, rect.top() + 1.0), [255; 4]);
    assert_ne!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
}

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
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255, 0, 255, 255]);
    assert_eq!(pixel(&pixels, rect.left() + 1.0, rect.top() + 1.0), [255; 4]);
}

async fn border_shape_clips_descendant_content_test() {
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
                child = Some(ui.add(empty_box.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some((parent.rect, child.unwrap()));
    })
    .await;
    let (parent, child) = bounds.lock().unwrap().unwrap();
    assert!(child.right() > parent.right());
    assert_eq!(pixel(&pixels, parent.center().x, parent.center().y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, parent.left() + 1.0, parent.top() + 1.0), [255; 4]);
    assert_eq!(pixel(&pixels, parent.right() + 5.0, parent.center().y), [255; 4]);
}

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
                child = Some(ui.add(empty_box.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some((parent.rect, child.unwrap()));
    })
    .await;
    let (parent, child) = bounds.lock().unwrap().unwrap();
    assert!(child.right() > parent.right());
    assert_eq!(pixel(&pixels, parent.center().x, parent.center().y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, parent.left() + 1.0, parent.top() + 1.0), [255; 4]);
    assert_eq!(pixel(&pixels, parent.right() + 5.0, parent.center().y), [255; 4]);
}

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
                child = Some(ui.add(empty_box.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some((parent.rect, child.unwrap()));
    })
    .await;
    let (parent, child) = bounds.lock().unwrap().unwrap();
    assert!(child.right() > parent.right());
    assert_eq!(pixel(&pixels, parent.center().x, parent.center().y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, parent.left() + 1.0, parent.top() + 1.0), [255; 4]);
}

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
                ui.add(empty_box.selector(".child"));
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some(parent.rect);
    })
    .await;
    let parent = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, parent.left() + 4.0, parent.top() + 4.0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, parent.left() - 4.0, parent.center().y), [255, 0, 0, 255]);
}

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
        ".image {{ box-sizing:border-box; width:20px; height:20px; background-image:image-set(url(\"data:image/svg+xml;base64,AA==\") 4x type(\"image/svg+xml\"), url(\"{high_density}\") 2x type(\"image/png\"), url(\"{standard_density}\") 1x); }}"
    );
    let stylesheet = Stylesheet::load(&css).await.unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [0, 0, 255, 255]);
}

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
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let blue_half = pixel(&pixels, rect.left() * 2.0 + 2.5, rect.center().y * 2.0);
    assert!(
        blue_half[2] >= 240 && blue_half[0] <= 15,
        "2x background image was not density-corrected: {blue_half:?}"
    );
}

async fn image_color_paints_as_a_background_image_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:20px; height:20px; background-image:image(rgb(255, 0, 0)); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255, 0, 0, 255]);
}

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
            .push(ui.add(empty_box.selector(".image")).rect);
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_box.selector(".transparent")).rect);
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let mixed = pixel(&pixels, bounds[0].center().x, bounds[0].center().y);
    assert!(
        (120..=135).contains(&mixed[0])
            && mixed[1] <= 5
            && (120..=135).contains(&mixed[2]),
        "expected a 50/50 premultiplied cross-fade: {mixed:?}"
    );
    let translucent = pixel(&pixels, bounds[1].center().x, bounds[1].center().y);
    assert!(
        translucent[0] > 110
            && translucent[2] > 110
            && translucent[1] <= 5,
        "transparent cross-fade should preserve the red source when composited: {translucent:?}"
    );
}

async fn cross_fade_accepts_percentage_before_image_test() {
    let stylesheet = Stylesheet::parse(
        ".blend { box-sizing:border-box; width:20px; height:20px; background-image:cross-fade(25% red, 75% blue); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".blend")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [64, 0, 191, 255]
    );
}

async fn image_set_selects_cross_fade_candidate_at_output_scale_test() {
    let stylesheet = Stylesheet::parse(
        ".blend { box-sizing:border-box; width:20px; height:20px; background-image:image-set(cross-fade(image(red) 100%) 1x, cross-fade(image(blue) 100%) 2x); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_scale(stylesheet, 2.0, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".blend")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x * 2.0, rect.center().y * 2.0),
        [0, 0, 255, 255]
    );
}

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
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 5.0, rect.center().y),
        [0, 0, 255, 255]
    );
}

async fn cross_fade_rejects_nested_image_set_test() {
    let result = Stylesheet::parse(
        ".invalid { background-image:cross-fade(image-set(image(red) 1x, image(blue) 2x) 100%); }",
    );
    assert!(result.is_err());
}

async fn conic_gradient_background_uses_clockwise_css_angles_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { box-sizing:border-box; width:40px; height:40px; background-image:conic-gradient(red 0%, blue 100%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".gradient")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let right = pixel(&pixels, rect.center().x + 10.0, rect.center().y);
    assert!((160..=225).contains(&right[0]));
    assert!((55..=105).contains(&right[1]));
    assert!((20..=125).contains(&right[2]));
}

async fn conic_gradient_resolves_start_angle_and_center_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { box-sizing:border-box; width:40px; height:40px; background-image:conic-gradient(from 90deg at right 10px bottom 10px, red 0%, blue 100%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".gradient")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let right = pixel(&pixels, rect.left() + 35.0, rect.top() + 30.0);
    assert!(right[0] >= 245 && right[2] <= 30, "unexpected conic start color: {right:?}");
    let top = pixel(&pixels, rect.left() + 30.0, rect.top() + 25.0);
    assert!(top[0] < 100);
    assert!(top[2] > 150);
}

async fn repeating_linear_gradient_repeats_its_stop_interval_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { box-sizing:border-box; width:40px; height:40px; background-image:repeating-linear-gradient(red 0%, blue 25%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".gradient")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let repeated = pixel(&pixels, rect.center().x, rect.center().y);
    assert!(
        repeated[0] >= 220 && repeated[2] <= 35,
        "unexpected repeated stop: {repeated:?}"
    );
}

async fn gradient_color_stops_can_specify_a_hard_stop_interval_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { box-sizing:border-box; width:40px; height:20px; background-image:linear-gradient(to right, red 0%, blue 40% 60%, red 100%); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".gradient")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let hard_stop = pixel(&pixels, rect.center().x, rect.center().y);
    assert!(
        hard_stop[2] >= 240 && hard_stop[0] <= 15,
        "expected the specified blue hard-stop interval, got {hard_stop:?}"
    );
}

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
            .push(ui.add(empty_box.selector(".hint")).rect);
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_box.selector(".linear")).rect);
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
            .push(ui.add(empty_box.selector(".oklab")).rect);
        capture
            .lock()
            .unwrap()
            .push(ui.add(empty_box.selector(".srgb")).rect);
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let oklab = pixel(&pixels, bounds[0].center().x, bounds[0].center().y);
    let srgb = pixel(&pixels, bounds[1].center().x, bounds[1].center().y);
    assert!(oklab[1] >= 55, "expected Oklab interpolation: {oklab:?}");
    assert!(
        (120..=135).contains(&srgb[0])
            && srgb[1] <= 5
            && (120..=135).contains(&srgb[2]),
        "expected sRGB interpolation: {srgb:?}"
    );
}

async fn radial_gradient_resolves_ellipse_radii_and_position_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { box-sizing:border-box; width:40px; height:40px; background-image:radial-gradient(ellipse 50% 25% at right 10px bottom 10px, red, blue); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".gradient")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let center = pixel(&pixels, rect.left() + 30.0, rect.top() + 30.0);
    assert!(center[0] >= 220 && center[2] <= 55, "unexpected radial center: {center:?}");
    let edge = pixel(&pixels, rect.left() + 9.0, rect.top() + 30.0);
    assert!(edge[2] >= 235 && edge[0] <= 20, "unexpected radial edge: {edge:?}");
}

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
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.center().x, rect.top() + 2.0), [0, 0, 255, 255]);
}

async fn image_color_paints_as_a_border_image_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:20px; height:20px; border:4px solid black; border-image-source:image(rgb(255, 0, 0)); border-image-slice:1; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.center().x, rect.top() + 2.0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
}

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
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
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
            ui.add(empty_box.selector(".number")).rect,
            ui.add(empty_box.selector(".percentage")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    for rect in bounds.iter() {
        assert_eq!(pixel(&pixels, rect.left() + 2.0, rect.top() + 2.0), [0, 255, 0, 255]);
        assert_eq!(pixel(&pixels, rect.left() + 8.0, rect.top() + 8.0), [255; 4]);
    }
}

async fn border_image_slice_rejects_duplicate_fill_keywords_test() {
    assert!(Stylesheet::parse(".image { border-image-slice:1 fill fill; }").is_err());
}

async fn conic_gradient_paints_as_a_border_image_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:20px; height:20px; border:4px solid black; border-image-source:conic-gradient(red, blue); border-image-slice:5; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let top = pixel(&pixels, rect.center().x, rect.top() + 2.0);
    assert!(top[0] >= 240 && top[2] <= 45, "unexpected conic border image: {top:?}");
}

async fn border_shape_clips_border_image_paint_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:40px; height:40px; border:4px solid black; border-shape:circle(50%); border-image-source:linear-gradient(to right, red, blue); border-image-slice:5; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let corner = pixel(&pixels, rect.left() + 1.0, rect.top() + 1.0);
    let top = pixel(&pixels, rect.center().x, rect.top() + 2.0);
    assert_eq!(corner, [255, 255, 255, 255]);
    assert_ne!(top, [255, 255, 255, 255], "expected the border image: {top:?}");
}

async fn cross_fade_paints_as_a_border_image_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:20px; height:20px; border:4px solid black; border-image-source:cross-fade(red, blue 75%); border-image-slice:5; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let top = pixel(&pixels, rect.center().x, rect.top() + 2.0);
    assert!(
        (55..=75).contains(&top[0]) && (180..=205).contains(&top[2]),
        "cross-fade border image was not blended: {top:?}"
    );
}

async fn radial_gradient_paints_as_a_border_image_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:20px; height:20px; border:4px solid black; border-image-source:radial-gradient(circle 15px, red, blue); border-image-slice:5; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let top = pixel(&pixels, rect.center().x, rect.top() + 2.0);
    assert!(
        top[0] > 80 && top[2] > 80,
        "radial border image did not interpolate its source: {top:?}"
    );
}

async fn partial_borders_draw_only_the_limited_or_clipped_segments_test() {
    let stylesheet = Stylesheet::parse(
        ".limited { box-sizing: border-box; width: 60px; height: 30px; border: 2px solid blue; border-limit: sides 50%; }\
         .clipped { box-sizing: border-box; width: 60px; height: 30px; border: 2px solid red; border-top-clip: 0 1fr; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_box.selector(".limited")).rect,
            ui.add(empty_box.selector(".clipped")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(pixel(&pixels, bounds[0].left() + 5.0, bounds[0].top() + 0.5), [255; 4]);
    assert_eq!(pixel(&pixels, bounds[0].center().x, bounds[0].top() + 0.5), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, bounds[1].center().x, bounds[1].top() + 0.5), [255; 4]);
    assert_eq!(pixel(&pixels, bounds[1].left() + 0.5, bounds[1].center().y), [255, 0, 0, 255]);
}

async fn striped_border_color_paints_across_the_border_thickness_test() {
    let stylesheet = Stylesheet::parse(
        ".striped { box-sizing: border-box; width: 60px; height: 30px; border: 8px solid stripes(red 1fr, blue 1fr); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".striped")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.center().x, rect.top() + 1.0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, rect.center().x, rect.top() + 5.0), [0, 0, 255, 255]);
}

async fn linear_gradient_border_image_is_sliced_and_painted_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing: border-box; width: 60px; height: 30px; border: 8px solid black; border-image-source: linear-gradient(red, blue); border-image-slice: 10; border-image-width: 8; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let top = pixel(&pixels, rect.center().x, rect.top() + 4.0);
    let bottom = pixel(&pixels, rect.center().x, rect.bottom() - 4.0);
    assert!(top[0] > top[2], "{top:?}");
    assert!(bottom[2] > bottom[0], "{bottom:?}");
}

async fn border_image_width_auto_uses_border_width_for_gradients_test() {
    let stylesheet = Stylesheet::parse(
        ".image { box-sizing:border-box; width:80px; height:60px; border:4px solid black; border-image-source:linear-gradient(to bottom, red, blue); border-image-slice:20; border-image-width:auto; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let inside_border = pixel(&pixels, rect.center().x, rect.top() + 2.0);
    let outside_border = pixel(&pixels, rect.center().x, rect.top() + 10.0);
    assert!(inside_border[0] > inside_border[2], "{inside_border:?}");
    assert_eq!(outside_border, [255, 255, 255, 255]);
}

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
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".image")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let beyond_natural_slice = pixel(
        &pixels,
        rect.center().x * 2.0,
        rect.top() * 2.0 + 6.0,
    );
    assert_eq!(beyond_natural_slice, [255, 255, 255, 255]);
}

async fn svg_path_border_shape_uses_its_closed_contour_test() {
    let stylesheet = Stylesheet::parse(
        ".diamond { box-sizing: border-box; width: 40px; height: 40px; border: 4px solid blue; background-color: red; border-shape: path(\"M 20 0 L 40 20 L 20 40 L 0 20 Z\"); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".diamond")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, rect.left() + 1.0, rect.top() + 1.0), [255; 4]);
    let border = pixel(&pixels, rect.center().x, rect.top() + 1.0);
    assert!(border[0] < 20 && border[1] < 20 && border[2] > 240, "{border:?}");
}

async fn svg_path_border_shape_preserves_subpaths_and_evenodd_holes_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing: border-box; width: 60px; height: 40px; border: 4px solid blue; background-color: red; border-shape: path(evenodd, \"M 0 0 H 60 V 40 H 0 Z M 15 10 H 45 V 30 H 15 Z\"); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.left() + 7.0, rect.center().y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
    let inner_border = pixel(&pixels, rect.center().x, rect.top() + 10.0);
    assert!(
        inner_border[0] < 20 && inner_border[1] < 20 && inner_border[2] > 240,
        "{inner_border:?}"
    );
}

async fn shape_border_shape_handles_curves_and_evenodd_subpaths_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing: border-box; width: 60px; height: 40px; border: 2px solid blue; background-color: red; border-shape: shape(evenodd from 0 0, hline to 60px, vline to 40px, hline to 0, close, move to 15px 10px, curve to 45px 10px with 25px 10px / 35px 10px, vline by 20px, curve to 15px 30px with 35px 30px / 25px 30px, close); }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.left() + 5.0, rect.center().y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255; 4]);
    let inner_border = pixel(&pixels, rect.center().x, rect.top() + 10.0);
    assert!(
        inner_border[0] < 20 && inner_border[1] < 20 && inner_border[2] > 240,
        "{inner_border:?}"
    );
}

async fn shape_border_shape_arcs_resolve_css_radii_and_sweep_test() {
    let stylesheet = Stylesheet::parse(
        ".shape { box-sizing: border-box; width: 50px; height: 40px; border: 2px solid blue; background-color: red; border-shape: shape(from 0 0, hline to 40px, arc to 50px 10px of 10px cw small, vline to 40px, hline to 0, close) border-box; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".shape")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, rect.right() - 5.0, rect.top() + 5.0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, rect.right() - 1.0, rect.top() + 1.0), [255; 4]);
}

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
                    ui.add(empty_box.selector(".even")).rect,
                    ui.add(empty_box.selector(".nonzero")).rect,
                ];
                ui.response()
            })
            .selector(".row"),
        );
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(pixel(&pixels, bounds[0].center().x, bounds[0].center().y), [255; 4]);
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
        [255, 0, 0, 255]
    );
}

async fn outer_and_inset_box_shadows_paint_on_their_respective_sides_test() {
    let stylesheet = Stylesheet::parse(
        ".outer { box-sizing: border-box; width: 60px; height: 30px; background-color: red; box-shadow: 4px 4px 0 blue; }\
         .inner { box-sizing: border-box; width: 60px; height: 30px; background-color: red; box-shadow: 4px 4px 0 blue inset; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = vec![
            ui.add(empty_box.selector(".outer")).rect,
            ui.add(empty_box.selector(".inner")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(pixel(&pixels, bounds[0].right() + 2.0, bounds[0].center().y + 4.0), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, bounds[1].center().x, bounds[1].top() + 1.0), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, bounds[1].center().x, bounds[1].center().y), [255, 0, 0, 255]);
}

async fn inset_box_shadow_blur_follows_gaussian_edge_profile_test() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:220px; height:100px; border:8px solid blue; border-radius:20px; background-color:white; box-shadow:inset 0 0 20px red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".box")).rect);
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
            ui.add(empty_box.selector(".spread")).rect,
            ui.add(empty_box.selector(".offset")).rect,
        ];
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let spread = bounds[0];
    assert_eq!(pixel(&pixels, spread.center().x, spread.top() + 6.0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, spread.center().x, spread.top() + 9.0), [255; 4]);
    assert_eq!(pixel(&pixels, spread.center().x, spread.top() + 2.0), [0, 0, 255, 255]);

    let offset = bounds[1];
    assert_eq!(pixel(&pixels, offset.center().x, offset.top() + 6.0), [0, 128, 0, 255]);
    assert_eq!(pixel(&pixels, offset.center().x, offset.bottom() - 6.0), [255; 4]);
}

async fn box_shadow_offset_none_is_case_insensitive_test() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:20px; height:20px; box-shadow-offset:NONE; box-shadow-blur:4px; box-shadow-color:red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".box")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.right() + 1.0, rect.center().y),
        [255; 4]
    );
}

async fn box_shadow_blur_scales_beyond_128_pixels_test() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:60px; height:40px; background-color:white; box-shadow:0 0 200px red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".box")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    let distant_shadow = pixel(&pixels, rect.right() + 125.0, rect.center().y);
    assert!(
        distant_shadow[1] < 240,
        "large blur radius was clipped: {distant_shadow:?}"
    );
}

async fn box_shadow_blur_approximates_gaussian_profile_test() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:60px; height:40px; background-color:white; box-shadow:0 0 100px red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".box")).rect);
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
                child = Some(ui.add(empty_box.selector(".child")).rect);
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
                        absolute = Some(ui.add(empty_box.selector(".absolute")).rect);
                        normal = Some(ui.add(empty_box.selector(".normal")).rect);
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

async fn logical_border_pairs_resolve_against_direction_and_affect_box_size_test() {
    let stylesheet=Stylesheet::parse(
        ".box { direction:rtl; width:20px; height:10px; border-inline-width:2px 4px; border-inline-style:solid; border-inline-color:red blue; border-block:3px solid green; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".box")).rect);
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
