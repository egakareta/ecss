use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn column_fragments_paint_in_destination_columns_without_painting_gutters_test() {
    let stylesheet=Stylesheet::parse(".columns {width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto;} .box {height:60px; background-color:red;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".box"));
                ui.response()
            })
            .selector(".columns"),
        );
        *capture.lock().unwrap() = Some(response.rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(rect.size(), egui::vec2(100.0, 30.0));
    assert_eq!(
        pixel(&pixels, rect.left() + 10.0, rect.top() + 10.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 65.0, rect.top() + 10.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 50.0, rect.top() + 10.0),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 10.0, rect.top() + 40.0),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn column_spanners_create_full_width_boxes_between_column_sets_test() {
    let stylesheet=Stylesheet::parse(".columns {width:210px; column-count:2; column-gap:10px;} .item {height:20px; background-color:red;} .span {column-span:all; height:10px; background-color:blue;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut boxes = Vec::new();
        ui.add(
            (|ui: &mut egui::Ui| {
                boxes.push(ui.add(empty_response.selector(".item")).rect);
                boxes.push(ui.add(empty_response.selector(".item")).rect);
                boxes.push(ui.add(empty_response.selector(".span")).rect);
                boxes.push(ui.add(empty_response.selector(".item")).rect);
                boxes.push(ui.add(empty_response.selector(".item")).rect);
                ui.response()
            })
            .selector(".columns"),
        );
        *capture.lock().unwrap() = boxes;
    })
    .await;
    let boxes = bounds.lock().unwrap();
    assert_eq!(boxes[0].size(), egui::vec2(100.0, 20.0));
    assert_eq!(boxes[1].left(), 110.0);
    assert_eq!(
        boxes[2],
        egui::Rect::from_min_size(egui::pos2(0.0, 20.0), egui::vec2(210.0, 10.0))
    );
    assert_eq!(boxes[3].top(), 30.0);
    assert_eq!(boxes[4].left(), 110.0);
    assert_eq!(pixel(&pixels, 105.0, 25.0), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 105.0, 10.0), [255; 4]);
    assert_eq!(pixel(&pixels, 120.0, 40.0), [255, 0, 0, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn column_rules_and_cloned_fragment_borders_render_their_styles_test() {
    let stylesheet = Stylesheet::parse(".columns {width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto; column-rule:6px double green; column-rule-style:none; column-rule-style:double;} .box {height:60px; box-sizing:border-box; border:2px solid blue; background-color:red; box-decoration-break:clone;}").unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".box"));
                ui.response()
            })
            .selector(".columns"),
        );
    })
    .await;
    assert_eq!(pixel(&pixels, 20.0, 28.5), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 70.0, 0.5), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 20.0, 15.0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 50.0, 15.0), [255; 4]);
    let left = pixel(&pixels, 48.0, 15.0);
    let right = pixel(&pixels, 51.0, 15.0);
    assert!(left[1] > left[0] && left[1] > left[2], "{left:?}");
    assert!(right[1] > right[0] && right[1] > right[2], "{right:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn cloned_column_fragments_repeat_border_images_test() {
    let stylesheet = Stylesheet::parse(
        ".columns { width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto; }\
         .box { height:60px; box-sizing:border-box; border:4px solid black; border-image-source:image(red); border-image-slice:2; border-image-width:4; box-decoration-break:clone; }",
    )
    .unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".box"));
                ui.response()
            })
            .selector(".columns"),
        );
    })
    .await;
    assert_eq!(pixel(&pixels, 20.0, 0.5), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 70.0, 0.5), [255, 0, 0, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn cloned_column_fragments_repeat_background_images_test() {
    let stylesheet = Stylesheet::parse(
        ".columns { width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto; }\
         .box { height:60px; background-color:white; background-image:image(red); box-decoration-break:clone; }",
    )
    .unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".box"));
                ui.response()
            })
            .selector(".columns"),
        );
    })
    .await;
    assert_eq!(pixel(&pixels, 20.0, 15.0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 70.0, 15.0), [255, 0, 0, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn cloned_column_fragments_repaint_background_gradients_test() {
    let stylesheet = Stylesheet::parse(
        ".columns { width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto; }\
         .box { height:60px; background-image:linear-gradient(to right, red, blue); box-decoration-break:clone; }",
    )
    .unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".box"));
                ui.response()
            })
            .selector(".columns"),
        );
    })
    .await;
    for x in [10.0, 60.0] {
        let left = pixel(&pixels, x, 15.0);
        assert!(left[0] > left[2], "left fragment pixel at {x} was {left:?}");
    }
    for x in [40.0, 90.0] {
        let right = pixel(&pixels, x, 15.0);
        assert!(
            right[2] > right[0],
            "right fragment pixel at {x} was {right:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn cloned_column_fragments_repaint_radial_gradient_centers_test() {
    let stylesheet = Stylesheet::parse(
        ".columns { width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto; }\
         .box { height:60px; background-image:radial-gradient(circle closest-side, red, blue); box-decoration-break:clone; }",
    )
    .unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".box"));
                ui.response()
            })
            .selector(".columns"),
        );
    })
    .await;
    for x in [22.0, 77.0] {
        let center = pixel(&pixels, x, 15.0);
        assert!(
            center[0] > center[2] * 4,
            "radial center at {x} was {center:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn cloned_column_fragments_repeat_url_background_images_test() {
    let source = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 255, 255]));
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(source)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let data = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        encoded.into_inner(),
    );
    let stylesheet = Stylesheet::load(&format!(
        ".columns {{ width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto; }}\
         .box {{ height:60px; background-image:url(\"data:image/png;base64,{data}\"); box-decoration-break:clone; }}"
    ))
    .await
    .unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".box"));
                ui.response()
            })
            .selector(".columns"),
        );
    })
    .await;
    assert_eq!(pixel(&pixels, 20.0, 15.0), [255, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 70.0, 15.0), [255, 0, 255, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn fragmented_border_image_outset_paints_outside_each_clone_test() {
    async fn render(decoration_break: &str) -> Vec<[u8; 4]> {
        let stylesheet = Stylesheet::parse(&format!(
            ".columns {{ width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto; }} .box {{ height:60px; box-sizing:border-box; border:4px solid black; border-image-source:image(red); border-image-slice:2; border-image-width:4; border-image-outset:4px; box-decoration-break:{decoration_break}; }}"
        ))
        .unwrap();
        render_ui_pixels(stylesheet, move |ui| {
            ui.add_space(40.0);
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(empty_response.selector(".box"));
                    ui.response()
                })
                .selector(".columns"),
            );
        })
        .await
    }

    let cloned = render("clone").await;
    let sliced = render("slice").await;
    assert_eq!(pixel(&cloned, 20.0, 36.5), [255, 0, 0, 255]);
    assert_eq!(pixel(&cloned, 70.0, 36.5), [255, 0, 0, 255]);
    assert_eq!(pixel(&sliced, 20.0, 36.5), [255, 0, 0, 255]);
    assert_eq!(pixel(&sliced, 70.0, 36.5), [255; 4]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn multicolumn_box_shadows_follow_box_decoration_break_test() {
    async fn shadow_pixels(decoration_break: &str, shadow: &str) -> usize {
        let stylesheet = Stylesheet::parse(&format!(
            ".columns {{ width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto; }} .box {{ height:60px; background:white; box-shadow:{shadow}; box-decoration-break:{decoration_break}; }}"
        ))
        .unwrap();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(empty_response.selector(".box"));
                    ui.response()
                })
                .selector(".columns"),
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
            sliced_shadow > 0,
            "slice should keep unbroken shadow edges for {shadow}"
        );
        assert!(
            cloned_shadow > sliced_shadow,
            "clone should paint a shadow around each column fragment for {shadow}: clone={cloned_shadow}, slice={sliced_shadow}"
        );
    }
}
