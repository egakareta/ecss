use super::*;

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
        ".parent { color:red; } .inherited { all:inherit; } .initial { all:initial; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                let inherited = ui.add(egui::Label::new("inherited").selector(".inherited"));
                let initial = ui.add(egui::Label::new("initial").selector(".initial"));
                *capture.lock().unwrap() = vec![inherited.rect, initial.rect];
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
    assert_eq!(red_pixels(bounds[1]), 0);
}
