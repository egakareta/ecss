use super::*;

fn image_texture(ui: &egui::Ui) -> egui::TextureHandle {
    let id = egui::Id::unique("object-test-image");
    if let Some(texture) = ui
        .ctx()
        .data(|data| data.get_temp::<egui::TextureHandle>(id))
    {
        return texture;
    }
    let colors = (0..20)
        .flat_map(|y| {
            (0..40).map(move |x| match (x < 20, y < 10) {
                (true, true) => egui::Color32::RED,
                (false, true) => egui::Color32::BLUE,
                (true, false) => egui::Color32::GREEN,
                (false, false) => egui::Color32::YELLOW,
            })
        })
        .collect();
    let texture = ui.ctx().load_texture(
        "object-test-image",
        egui::ColorImage::new([40, 20], colors),
        egui::TextureOptions::NEAREST,
    );
    ui.ctx()
        .data_mut(|data| data.insert_temp(id, texture.clone()));
    texture
}

fn assert_object_pixels(pixels: &[[u8; 4]], area: egui::Rect, object: egui::Rect) {
    for y in area.min.y.ceil() as u32..area.max.y.floor() as u32 {
        for x in area.min.x.ceil() as u32..area.max.x.floor() as u32 {
            let point = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
            let expected = if object.contains(point) {
                match (point.x < object.center().x, point.y < object.center().y) {
                    (true, true) => [255, 0, 0, 255],
                    (false, true) => [0, 0, 255, 255],
                    (true, false) => [0, 255, 0, 255],
                    (false, false) => [255, 255, 0, 255],
                }
            } else {
                [255, 0, 255, 255]
            };
            assert_eq!(
                pixel(pixels, point.x, point.y),
                expected,
                "at {point:?}, area {area:?}, object {object:?}"
            );
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn object_fit_modes_size_and_clip_images_inside_the_content_box() {
    for (fit, box_size, size) in [
        ("fill", egui::vec2(60.0, 60.0), egui::vec2(60.0, 60.0)),
        ("contain", egui::vec2(60.0, 60.0), egui::vec2(60.0, 30.0)),
        ("cover", egui::vec2(60.0, 60.0), egui::vec2(120.0, 60.0)),
        ("none", egui::vec2(60.0, 60.0), egui::vec2(40.0, 20.0)),
        ("scale-down", egui::vec2(60.0, 60.0), egui::vec2(40.0, 20.0)),
        (
            "cover scale-down",
            egui::vec2(60.0, 60.0),
            egui::vec2(40.0, 20.0),
        ),
        ("contain", egui::vec2(20.0, 12.0), egui::vec2(20.0, 10.0)),
        ("scale-down", egui::vec2(20.0, 12.0), egui::vec2(20.0, 10.0)),
        ("none", egui::vec2(20.0, 12.0), egui::vec2(40.0, 20.0)),
        ("fill", egui::vec2(0.0, 12.0), egui::vec2(0.0, 12.0)),
        ("contain", egui::vec2(20.0, 0.0), egui::vec2(0.0, 0.0)),
        ("cover", egui::vec2(20.0, 12.0), egui::vec2(24.0, 12.0)),
        (
            "scale-down cover",
            egui::vec2(20.0, 12.0),
            egui::vec2(24.0, 12.0),
        ),
    ] {
        let stylesheet = Stylesheet::parse(&format!(
            ".image {{ width:{}px; height:{}px; padding:4px; border:3px solid black; background:#f0f; object-fit:{fit}; }}",
            box_size.x, box_size.y,
        )).unwrap();
        let rect = std::sync::Arc::new(Mutex::new(egui::Rect::NOTHING));
        let observed = rect.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            let texture = image_texture(ui);
            *observed.lock().unwrap() = ui
                .add(
                    egui::Image::new(&texture)
                        .fit_to_exact_size(egui::vec2(20.0, 40.0))
                        .maintain_aspect_ratio(false)
                        .selector(".image"),
                )
                .rect;
        })
        .await;
        let border = *rect.lock().unwrap();
        assert_eq!(border.size(), box_size + egui::Vec2::splat(14.0), "{fit}");
        let area = border.shrink(7.0);
        let object = egui::Rect::from_center_size(area.center(), size);
        assert_object_pixels(&pixels, area, object);
        assert_eq!(
            pixel(&pixels, border.center().x, border.min.y + 1.5),
            [0, 0, 0, 255]
        );
        assert_eq!(
            pixel(&pixels, border.min.x + 5.5, area.center().y),
            [255, 0, 255, 255]
        );
        assert_eq!(
            pixel(&pixels, border.max.x + 1.5, area.center().y),
            [255; 4]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn object_position_resolves_keywords_lengths_percentages_and_edge_offsets() {
    for (fit, position, offset, size) in [
        ("none", "top", egui::vec2(10.0, 0.0), egui::vec2(40.0, 20.0)),
        (
            "none",
            "right bottom",
            egui::vec2(20.0, 40.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "none",
            "bottom left",
            egui::vec2(0.0, 40.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "none",
            "25% 75%",
            egui::vec2(5.0, 30.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "none",
            "7px 9px",
            egui::vec2(7.0, 9.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "none",
            "calc(50% + 3px) calc(100% - 4px)",
            egui::vec2(13.0, 36.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "none",
            "calc(NaN * 1px) 0px",
            egui::vec2(0.0, 0.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "none",
            "min(calc(NaN * 1px), 10px) 0px",
            egui::vec2(0.0, 0.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "none",
            "right 3px bottom 5px",
            egui::vec2(17.0, 35.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "none",
            "bottom 10% right 5%",
            egui::vec2(19.0, 36.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "none",
            "-10px 120%",
            egui::vec2(-10.0, 48.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "cover",
            "25% top",
            egui::vec2(-15.0, 0.0),
            egui::vec2(120.0, 60.0),
        ),
        (
            "contain",
            "left bottom",
            egui::vec2(0.0, 30.0),
            egui::vec2(60.0, 30.0),
        ),
        (
            "fill",
            "10px -5px",
            egui::vec2(10.0, -5.0),
            egui::vec2(60.0, 60.0),
        ),
    ] {
        let stylesheet = Stylesheet::parse(&format!(
            ".image {{ width:60px; height:60px; background:#f0f; object-fit:{fit}; object-position:{position}; }}"
        )).unwrap();
        let rect = std::sync::Arc::new(Mutex::new(egui::Rect::NOTHING));
        let observed = rect.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            let texture = image_texture(ui);
            *observed.lock().unwrap() = ui.add(egui::Image::new(&texture).selector(".image")).rect;
        })
        .await;
        let area = *rect.lock().unwrap();
        assert_object_pixels(
            &pixels,
            area,
            egui::Rect::from_min_size(area.min + offset, size),
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn object_properties_inherit_computed_positions_and_obey_css_wide_keywords() {
    for (declaration, offset, size) in [
        ("", egui::vec2(10.0, 10.0), egui::vec2(60.0, 60.0)),
        (
            "object-fit:inherit",
            egui::vec2(10.0, 10.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "object-fit:none; object-position:unset",
            egui::vec2(10.0, 10.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "object-fit:none; object-position:revert",
            egui::vec2(10.0, 10.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "all:inherit",
            egui::vec2(10.0, 10.0),
            egui::vec2(40.0, 20.0),
        ),
        (
            "object-fit:none; object-position:initial",
            egui::vec2(10.0, 20.0),
            egui::vec2(40.0, 20.0),
        ),
        ("all:initial", egui::vec2(0.0, 0.0), egui::vec2(60.0, 60.0)),
        ("all:unset", egui::vec2(10.0, 10.0), egui::vec2(60.0, 60.0)),
        (
            "object-fit:inherit; object-fit:initial; object-position:initial",
            egui::Vec2::ZERO,
            egui::vec2(60.0, 60.0),
        ),
    ] {
        let stylesheet = Stylesheet::parse(&format!(
            ".parent {{ font-size:10px; object-fit:none; object-position:1em 1em; }} \
             .image {{ {declaration}; width:60px; height:60px; background:#f0f; font-size:30px; }}"
        ))
        .unwrap();
        let rect = std::sync::Arc::new(Mutex::new(egui::Rect::NOTHING));
        let observed = rect.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    let texture = image_texture(ui);
                    *observed.lock().unwrap() =
                        ui.add(egui::Image::new(&texture).selector(".image")).rect;
                    ui.response()
                })
                .selector(".parent"),
            );
        })
        .await;
        let area = *rect.lock().unwrap();
        assert_object_pixels(
            &pixels,
            area,
            egui::Rect::from_min_size(area.min + offset, size),
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inline_object_styles_preserve_uvs_tint_and_the_parent_clip() {
    let stylesheet = Stylesheet::parse(".parent {width:30px; height:30px; overflow:hidden;} .image {width:60px; height:60px; background:#f0f;}").unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                let texture = image_texture(ui);
                ui.add(
                    egui::Image::new(&texture)
                        .uv(egui::Rect::from_min_max(
                            egui::pos2(0.0, 0.0),
                            egui::pos2(0.5, 1.0),
                        ))
                        .tint(egui::Color32::from_rgb(128, 255, 255))
                        .selector(".image")
                        .inline_style(
                            InlineStyle::parse("object-fit:contain; object-position:left top;")
                                .unwrap(),
                        ),
                );
                ui.response()
            })
            .selector(".parent"),
        );
    })
    .await;
    assert_eq!(pixel(&pixels, 5.5, 5.5), [128, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 25.5, 25.5), [128, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 35.5, 5.5), [255; 4]);
    assert_eq!(pixel(&pixels, 5.5, 35.5), [255; 4]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn object_position_interpolates_keyword_and_edge_positions_in_animations_and_transitions() {
    for css in [
        "@keyframes move {from {object-position:left top;} to {object-position:right 4px bottom 8px;}} \
         .image {animation:move 1s linear both;}",
        ".image {transition:object-position 1s linear -0.5s;}",
    ] {
        let stylesheet = Stylesheet::parse(&format!(
            ".image {{width:60px; height:60px; background:#f0f; object-fit:none;}} {css}"
        ))
        .unwrap();
        let transition = css.contains("transition");
        let rect = std::sync::Arc::new(Mutex::new(egui::Rect::NOTHING));
        let observed = rect.clone();
        let pixels = render_ui_pixels_at_time(stylesheet, 0.5, move |ui| {
            let texture = image_texture(ui);
            let image = egui::Image::new(&texture).selector(".image");
            let response = if transition {
                let position = if ui.input(|input| input.time) == 0.0 {
                    "object-position:left top"
                } else {
                    "object-position:right 4px bottom 8px"
                };
                ui.add(image.inline_style(InlineStyle::parse(position).unwrap()))
            } else {
                ui.add(image)
            };
            *observed.lock().unwrap() = response.rect;
        })
        .await;
        let area = *rect.lock().unwrap();
        assert_object_pixels(
            &pixels,
            area,
            egui::Rect::from_min_size(area.min + egui::vec2(8.0, 16.0), egui::vec2(40.0, 20.0)),
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn object_properties_do_not_reposition_button_icons_or_text() {
    let mut outputs = Vec::new();
    for properties in ["", "object-fit:cover; object-position:80px 70px;"] {
        let stylesheet = Stylesheet::parse(&format!(
            ".button {{width:100px; height:50px; {properties}}}"
        ))
        .unwrap();
        outputs.push(
            render_ui_pixels(stylesheet, |ui| {
                let texture = image_texture(ui);
                ui.add(
                    egui::Button::image_and_text(
                        egui::Image::new(&texture).fit_to_exact_size(egui::vec2(20.0, 10.0)),
                        "Label",
                    )
                    .selector(".button"),
                );
            })
            .await,
        );
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn object_fit_preserves_logical_image_dimensions_at_high_pixel_density() {
    let stylesheet =
        Stylesheet::parse(".image {width:30px; height:30px; background:#f0f; object-fit:none;}")
            .unwrap();
    let rect = std::sync::Arc::new(Mutex::new(egui::Rect::NOTHING));
    let observed = rect.clone();
    let pixels = render_ui_pixels_at_scale(stylesheet, 2.0, move |ui| {
        let texture = image_texture(ui);
        *observed.lock().unwrap() = ui
            .add(
                egui::Image::new(&texture)
                    .rotate(0.0, egui::Vec2::splat(0.5))
                    .selector(".image"),
            )
            .rect;
    })
    .await;
    let area = *rect.lock().unwrap() * 2.0;
    assert_object_pixels(
        &pixels,
        area,
        egui::Rect::from_center_size(area.center(), egui::vec2(80.0, 40.0)),
    );
}
