use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn installed_white_background_renders_every_pixel_white_test() {
    let pixels = render_ui_pixels(Stylesheet::default(), |_ui| {}).await;
    assert_eq!(pixels.len(), (WIDTH * HEIGHT) as usize);
    let first_non_white = pixels
        .iter()
        .enumerate()
        .find(|(_, pixel)| **pixel != [255; 4]);
    if let Some((index, pixel)) = first_non_white {
        panic!(
            "pixel ({}, {}) was {pixel:?}, expected opaque white",
            index % WIDTH as usize,
            index / WIDTH as usize,
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn selector_background_color_is_visible_in_rendered_widget_pixels_test() {
    let stylesheet = Stylesheet::parse(".primary { background-color: #f00; }").unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(egui::Button::new("X").selector(".primary"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    let x = (rect.min.x + 4.0).floor() as usize;
    let y = (rect.min.y + 3.0).floor() as usize;
    assert_eq!(pixels[y * WIDTH as usize + x], [255, 0, 0, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_shorthand_gradient_and_content_clip_render_correctly_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { width: 60px; height: 24px; padding: 4px; border: 4px solid black; background: linear-gradient(to right, red, blue); background-clip: content-box; background-attachment: scroll; }",
    )
    .unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(empty_box.selector(".gradient"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    let center_y = rect.center().y;
    assert_eq!(
        pixel(&pixels, rect.min.x + 5.5, center_y),
        [255, 255, 255, 255]
    );
    assert_eq!(pixel(&pixels, rect.min.x + 1.5, center_y), [0, 0, 0, 255]);

    let left = pixel(&pixels, rect.min.x + 9.5, center_y);
    let right = pixel(&pixels, rect.max.x - 9.5, center_y);
    assert!(left[0] > left[2], "left gradient pixel was {left:?}");
    assert!(right[2] > right[0], "right gradient pixel was {right:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn gradient_background_respects_border_radius_and_padding_clip_test() {
    let stylesheet = Stylesheet::parse(
        ".gradient { width: 60px; height: 24px; border: 1px solid black; border-radius: 12px; background: linear-gradient(to right, red, blue); background-clip: padding-box; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_box.selector(".gradient")).rect);
    })
    .await;

    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.min.x + 2.5, rect.min.y + 2.5),
        [255, 255, 255, 255]
    );
    let center = pixel(&pixels, rect.center().x, rect.center().y);
    assert!(
        center[0] > 0 && center[2] > 0,
        "center gradient pixel was {center:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn background_image_data_url_is_decoded_and_tiled_test() {
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
        ".image {{ width: 16px; height: 12px; background-image: url(\"data:image/png;base64,{data}\"); }}"
    );
    let stylesheet = Stylesheet::load(&css).await.unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(empty_box.selector(".image"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [255, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn functional_colors_render_rgb_rgba_hsl_and_hsla_values_test() {
    let stylesheet = Stylesheet::parse(
        ".rgb-comma { background-color: rgb(255, 0, 0); }\
         .rgb-space { background-color: rgb(100% 0% 0%); }\
         .rgba-comma { background-color: rgba(0, 0, 255, 50%); }\
         .hsl-comma { background-color: hsl(120, 100%, 50%); }\
         .hsl-angle { background-color: hsl(0.5turn 100% 50%); }\
         .hsla-space { background-color: hsla(120 100% 50% / 25%); }",
    )
    .unwrap();
    let widget_rects = std::sync::Arc::new(Mutex::new(Vec::new()));
    let rects_from_ui = widget_rects.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut rects = Vec::new();
        for class in [
            ".rgb-comma",
            ".rgb-space",
            ".rgba-comma",
            ".hsl-comma",
            ".hsl-angle",
            ".hsla-space",
        ] {
            let widget = |ui: &mut egui::Ui| {
                ui.allocate_exact_size(egui::vec2(30.0, 18.0), egui::Sense::hover())
                    .1
            };
            let response = ui.add(widget.selector(class));
            rects.push(response.rect);
        }
        *rects_from_ui.lock().unwrap() = rects;
    })
    .await;

    let expected = [
        [255, 0, 0, 255],
        [255, 0, 0, 255],
        [127, 127, 255, 255],
        [0, 255, 0, 255],
        [0, 255, 255, 255],
        [191, 255, 191, 255],
    ];
    let rects = widget_rects.lock().unwrap();
    assert_eq!(rects.len(), expected.len());
    for (rect, expected) in rects.iter().zip(expected) {
        let actual = pixel(&pixels, rect.min.x + 4.0, rect.min.y + 3.0);
        assert!(
            actual
                .into_iter()
                .zip(expected)
                .all(|(actual, expected)| actual.abs_diff(expected) <= 1),
            "pixel at {rect:?} was {actual:?}, expected {expected:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn button_frame_stroke_tracks_the_css_padding_box_test() {
    let stylesheet = Stylesheet::parse(
        ".primary { background-color: #f00; padding: 8px 12px; border-radius: 6px; }",
    )
    .unwrap();
    let button_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = button_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(egui::Button::new("X").selector(".primary"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = button_rect.lock().unwrap().unwrap();
    let pixel_at = |x: f32, y: f32| {
        let x = x.floor() as usize;
        let y = y.floor() as usize;
        pixels[y * WIDTH as usize + x]
    };

    assert_eq!(
        pixel_at(rect.min.x + 0.5, rect.center().y),
        [118, 118, 118, 255]
    );
    assert_eq!(
        pixel_at(rect.min.x + 12.5, rect.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn asymmetric_box_model_is_visible_in_rendered_pixels_test() {
    let stylesheet = Stylesheet::parse(
        ".box { padding: 2px 6px 4px 10px; margin: 4px 3px 5px 7px; background-color: #f00; }",
    )
    .unwrap();
    let content_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = content_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let rect_from_widget = rect_from_ui.clone();
        let content = move |ui: &mut egui::Ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(12.0, 8.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, egui::Color32::GREEN);
            *rect_from_widget.lock().unwrap() = Some(rect);
            response
        };
        ui.add(content.selector(".box"));
    })
    .await;

    let rect = content_rect.lock().unwrap().unwrap();
    let pixel_at = |x: f32, y: f32| {
        let x = x.floor() as usize;
        let y = y.floor() as usize;
        pixels[y * WIDTH as usize + x]
    };
    let red = [255, 0, 0, 255];
    let green = [0, 255, 0, 255];
    let white = [255, 255, 255, 255];

    assert_eq!(pixel_at(rect.center().x, rect.center().y), green);
    assert_eq!(pixel_at(rect.min.x - 5.5, rect.center().y), red);
    assert_eq!(pixel_at(rect.max.x + 2.5, rect.center().y), red);
    assert_eq!(pixel_at(rect.center().x, rect.min.y - 1.5), red);
    assert_eq!(pixel_at(rect.center().x, rect.max.y + 2.5), red);

    assert_eq!(pixel_at(rect.min.x - 10.0 - 3.5, rect.center().y), white);
    assert_eq!(pixel_at(rect.max.x + 6.0 + 1.5, rect.center().y), white);
    assert_eq!(pixel_at(rect.center().x, rect.min.y - 2.0 - 2.5), white);
    assert_eq!(pixel_at(rect.center().x, rect.max.y + 4.0 + 2.5), white);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn percentage_and_auto_sizes_affect_styled_widget_bounds_test() {
    let stylesheet = Stylesheet::parse(
        ".percentage { width: 50%; height: 50%; background-color: #f00; }\
         .automatic { width: auto; height: auto; }\
         .fixed { width: 24px; height: 16px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let bounds_from_ui = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let available_rect = ui.available_rect_before_wrap();
        let inner_rects = std::sync::Arc::new(Mutex::new(Vec::new()));
        let make_box = || {
            let inner_rects = inner_rects.clone();
            move |ui: &mut egui::Ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(12.0, 8.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, egui::Color32::GREEN);
                inner_rects.lock().unwrap().push(rect);
                response
            }
        };

        let percentage = ui.add(make_box().selector(".percentage"));
        let automatic = ui.add(make_box().selector(".automatic"));
        let fixed = ui.add(make_box().selector(".fixed"));
        *bounds_from_ui.lock().unwrap() = Some((
            available_rect,
            percentage.rect,
            automatic.rect,
            fixed.rect,
            inner_rects.lock().unwrap().clone(),
        ));
    })
    .await;

    let (available_rect, percentage, automatic, fixed, inner_rects) =
        bounds.lock().unwrap().take().unwrap();
    assert_eq!(percentage.width(), available_rect.width() * 0.5);
    assert_eq!(percentage.height(), available_rect.height() * 0.5);
    assert_eq!(automatic.size(), egui::vec2(12.0, 8.0));
    assert_eq!(fixed.size(), egui::vec2(24.0, 16.0));

    let pixel_at =
        |x: f32, y: f32| pixels[y.floor() as usize * WIDTH as usize + x.floor() as usize];
    assert_eq!(
        pixel_at(inner_rects[0].center().x, inner_rects[0].center().y),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel_at(percentage.max.x - 4.0, percentage.max.y - 4.0),
        [255, 0, 0, 255]
    );
}
