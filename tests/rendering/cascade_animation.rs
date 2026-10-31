use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_padding_side_keeps_other_animated_sides_at_midpoint_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes expand { from { width:20px; padding:0; } to { width:80px; padding:20px 40px; } }\
         .physical { box-sizing:content-box; width:40px; height:24px; padding:0; animation:expand 1s linear both; }\
         .physical { padding-left:7px !important; }\
         .vertical { box-sizing:content-box; writing-mode:vertical-rl; width:40px; height:24px; padding:0; animation:expand 1s linear both; }\
         .vertical { padding-inline-start:7px !important; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        let mut current = Vec::new();
        for selector in [".physical", ".vertical"] {
            current.push(ui.add(egui::Button::new("X").selector(selector)).rect);
        }
        *capture.lock().unwrap() = current;
    })
    .await;

    let bounds = bounds.lock().unwrap();
    near(bounds[0].width(), 77.0);
    near(bounds[0].height(), 44.0);
    near(bounds[1].width(), 90.0);
    near(bounds[1].height(), 41.0);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_margin_side_keeps_other_animated_sides_at_midpoint_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes spread { from { width:20px; margin:0; } to { width:80px; margin:20px 40px; } }\
         .item { width:20px; height:10px; margin:0; animation:spread 1s linear both; }\
         .item { margin-left:7px !important; }\
         .sibling { width:10px; height:10px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        let mut item = None;
        let mut sibling = None;
        let parent = ui.horizontal(|ui| {
            item = Some(ui.add(egui::Button::new("X").selector(".item")).rect);
            sibling = Some(ui.add(egui::Button::new("X").selector(".sibling")).rect);
        });
        *capture.lock().unwrap() = Some((parent.response.rect, item.unwrap(), sibling.unwrap()));
    })
    .await;

    let (parent, item, sibling) = bounds.lock().unwrap().unwrap();
    near(item.left() - parent.left(), 7.0);
    near(item.top() - parent.top(), 10.0);
    near(sibling.left() - item.right(), 20.0);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_border_longhand_does_not_suppress_other_animated_border_components_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes border-color { from { border:2px solid red; } to { border:10px solid blue; } }\
         .box { box-sizing:border-box; width:60px; height:40px; background:white; border:4px solid black; animation:border-color 1s linear both; }\
         .box { border-left-color:lime !important; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 5.5),
        [128, 0, 128, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 5.5, rect.center().y),
        [0, 255, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_border_suppresses_animated_border_image_source_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes change-image { from { border-image-source:image(rgb(255, 0, 0)); } to { border-image-source:image(rgb(0, 0, 255)); } }\
         .box { box-sizing:border-box; width:60px; height:40px; background:white; border:4px solid black; border-image-source:image(rgb(255, 0, 0)); border-image-slice:1; border-image-width:4; animation:change-image 1s linear both; border:4px solid black !important; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 1.5),
        [0, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_border_suppresses_animated_border_image_width_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes widen-image { from { border-image-width:0; } to { border-image-width:4; } }\
         .box { box-sizing:border-box; width:60px; height:40px; background:white; border-image-width:0; animation:widen-image 1s linear both; border:4px solid black !important; border-image-source:image(rgb(0, 255, 0)) !important; border-image-slice:1 !important; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 1.5),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 5.5),
        [255, 255, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn animated_border_resets_underlying_border_image_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes redraw-border { from { border:2px solid black; } to { border:10px solid black; } }\
         .box { box-sizing:border-box; width:60px; height:40px; background:white; border:4px solid black; border-image-source:image(rgb(255, 0, 0)); border-image-slice:1; border-image-width:4; animation:redraw-border 1s linear both; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 5.5),
        [0, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_border_image_source_does_not_block_other_shorthand_animation_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes widen-image { from { border-image:image(rgb(255, 0, 0)) 1 / 0; } to { border-image:image(rgb(0, 0, 255)) 1 / 4; } }\
         .box { box-sizing:border-box; width:60px; height:40px; background:white; border:4px solid black; border-image-source:image(rgb(0, 255, 0)) !important; border-image-slice:1; border-image-width:0; animation:widen-image 1s linear both; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.top() + 2.0),
        [0, 255, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn animated_border_image_shorthand_keeps_trailing_repeat_test() {
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
    let url = format!("data:image/png;base64,{data}");
    for shorthand in [
        format!("url(\"{url}\") 1 fill round"),
        format!("url(\"{url}\") 1 fill / 1 round round"),
        format!("url(\"{url}\") 1 fill / 1 / 0 round"),
        format!("round 1 fill / / 0 url(\"{url}\")"),
    ] {
        let stylesheet = Stylesheet::load(&format!(
            "@keyframes tile {{ from {{ border-image:{shorthand}; }} to {{ border-image:{shorthand}; }} }}\
             .animated, .static, .expected {{ box-sizing:border-box; width:30px; height:20px; border:4px solid black; border-image-source:url(\"{url}\"); border-image-slice:1 fill; border-image-width:1; border-image-repeat:round; }}\
             .animated {{ animation:tile 1s linear both; }}\
             .static {{ border-image:{shorthand}; }}"
        ))
        .await
        .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
            *capture.lock().unwrap() = [".animated", ".static", ".expected"]
                .map(|selector| ui.add(empty_response.selector(selector)).rect)
                .to_vec();
        })
        .await;

        let bounds = bounds.lock().unwrap();
        let offsets = [2.0, 6.0, 10.0, 15.0, 20.0, 27.0];
        let expected =
            offsets.map(|offset| pixel(&pixels, bounds[2].left() + offset, bounds[2].center().y));
        assert!(expected.contains(&[255, 0, 0, 255]));
        assert!(expected.contains(&[0, 0, 255, 255]));
        for rect in &bounds[..2] {
            for (offset, expected) in offsets.into_iter().zip(expected) {
                assert_eq!(
                    pixel(&pixels, rect.left() + offset, rect.center().y),
                    expected,
                    "border-image-repeat differs at x offset {offset} for {shorthand}"
                );
            }
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_inherit_preserves_parent_border_image_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes inherit-border { from { border:inherit; } to { border:inherit; } }\
         .parent { border:4px solid black; border-image-source:image(rgb(0, 255, 0)); border-image-slice:1; border-image-width:1; }\
         .child { box-sizing:border-box; width:30px; height:24px; background:white; border:2px solid black; }\
         .animated { animation:inherit-border 1s linear both; }\
         .static { border:inherit; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = [".child.animated", ".child.static"]
                    .map(|selector| ui.add(empty_response.selector(selector)).rect)
                    .to_vec();
                ui.response()
            })
            .selector(".parent"),
        );
    })
    .await;

    for rect in bounds.lock().unwrap().iter() {
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.top() + 1.5),
            [0, 255, 0, 255]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_css_wide_resets_border_image_test() {
    for keyword in ["initial", "unset", "revert"] {
        let stylesheet = Stylesheet::parse(&format!(
            ".box {{ box-sizing:border-box; width:30px; height:24px; background:white; border:4px solid black; border-image:image(red) 1 / 1; border:{keyword}; }}"
        ))
        .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
        })
        .await;
        let rect = bounds.lock().unwrap().unwrap();
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.top() + 1.5),
            [255, 255, 255, 255],
            "border:{keyword} did not reset the image"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn border_image_urls_used_only_in_keyframes_are_loaded_test() {
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        6,
        6,
        image::Rgba([255, 0, 0, 255]),
    ))
    .write_to(&mut encoded, image::ImageFormat::Png)
    .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    for declaration in [
        format!("border-image:url(\"data:image/png;base64,{data}\") 1 / 1"),
        format!("border-image-source:url(\"data:image/png;base64,{data}\")"),
    ] {
        let stylesheet = Stylesheet::load(&format!(
            "@keyframes paint {{ from,to {{ {declaration}; }} }}\
             .box {{ box-sizing:border-box; width:30px; height:24px; background:white; border:4px solid black; border-image-slice:1; border-image-width:1; animation:paint 1s linear both; }}"
        ))
        .await
        .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
            *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
        })
        .await;
        let rect = bounds.lock().unwrap().unwrap();
        assert_eq!(
            pixel(&pixels, rect.center().x, rect.top() + 1.5),
            [255, 0, 0, 255],
            "keyframe-only image was not loaded for {declaration}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_background_shorthand_resets_animated_background_color_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes paint { from { background-color:#000; } to { background-color:#fff; } }\
         .animated { width:40px; height:30px; background-color:#000; animation:paint 1s linear both; }\
         .important { background-image:linear-gradient(red, red); background:#0f0 !important; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        let mut current = Vec::new();
        for selector in [".animated", ".animated.important"] {
            current.push(ui.add(egui::Button::new("X").selector(selector)).rect);
        }
        *capture.lock().unwrap() = current;
    })
    .await;

    let bounds = bounds.lock().unwrap();
    assert_eq!(
        pixel(&pixels, bounds[0].center().x, bounds[0].center().y),
        [128, 128, 128, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
        [0, 255, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn important_font_shorthand_suppresses_animated_font_size_and_line_height_test() {
    let stylesheet = Stylesheet::parse(
        "@keyframes enlarge { from { font-size:8px; line-height:8px; } to { font-size:32px; line-height:32px; } }\
         .text { display:block; font:16px/18px monospace; animation:enlarge 1s linear both; }\
         .important { font:16px/18px monospace !important; }\
         .control { display:block; font:16px/18px monospace; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
        for selector in [".text", ".text.important", ".control"] {
            capture
                .lock()
                .unwrap()
                .push(ui.add(egui::Label::new("MMMM").selector(selector)).rect);
        }
    })
    .await;

    let bounds = bounds.lock().unwrap();
    near(bounds[1].width(), bounds[2].width());
    near(bounds[1].height(), bounds[2].height());
    assert!((bounds[0].height() - bounds[1].height()).abs() > 1.0);
}
