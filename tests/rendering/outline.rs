use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn outline_shorthand_and_longhands_paint_outside_the_border_without_resizing() {
    let stylesheet = Stylesheet::parse(
        ".box { box-sizing:border-box; width:32px; height:24px; margin:12px; border:2px solid red; background:white; }
         .short { outline-offset:3px; outline:blue solid 4px; }
         .long { outline-color:blue; outline-style:solid; outline-width:4px; outline-offset:3px; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = [".box.short", ".box.long"]
            .map(|selector| ui.add(empty_response.selector(selector)).rect)
            .to_vec();
    })
    .await;
    let bounds = bounds.lock().unwrap();
    for rect in bounds.iter() {
        assert_eq!(rect.size(), egui::vec2(32.0, 24.0));
        for (x, expected) in [
            (rect.left() + 1.0, [255, 0, 0, 255]),
            (rect.center().x, [255, 255, 255, 255]),
            (rect.right() + 1.0, [255, 255, 255, 255]),
            (rect.right() + 5.0, [0, 0, 255, 255]),
            (rect.right() + 9.0, [255, 255, 255, 255]),
        ] {
            assert_eq!(pixel(&pixels, x, rect.center().y), expected);
        }
    }
    near(bounds[1].top() - bounds[0].bottom(), 24.0);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn negative_outline_offsets_paint_over_widget_content() {
    let stylesheet = Stylesheet::parse(
        ".box { width:48px; height:24px; margin:8px; outline:2px solid blue; outline-offset:calc(-2px - 2px); }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(
            (|ui: &mut egui::Ui| {
                let response = ui.allocate_response(egui::vec2(48.0, 24.0), egui::Sense::hover());
                ui.painter()
                    .rect_filled(response.rect, 0.0, egui::Color32::RED);
                response
            })
            .selector(".box"),
        );
        *capture.lock().unwrap() = Some(response.rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() + 0.5, rect.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() + 3.0, rect.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.center().x, rect.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn outline_styles_currentcolor_and_shorthand_resets_are_rendered() {
    let stylesheet = Stylesheet::parse(
        ".box { width:40px; height:20px; margin:8px; color:blue; outline:3px solid red; }
         .double { outline:6px double currentcolor; }
         .initial { outline-color:initial; }
         .none { outline:none; }
         .zero { outline-width:0; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.horizontal(|ui| {
            *capture.lock().unwrap() = [".box.double", ".box.initial", ".box.none", ".box.zero"]
                .map(|selector| ui.add(empty_response.selector(selector)).rect)
                .to_vec();
        });
    })
    .await;
    let bounds = bounds.lock().unwrap();
    for distance in [1.0, 5.0] {
        assert_eq!(
            pixel(&pixels, bounds[0].right() + distance, bounds[0].center().y),
            [0, 0, 255, 255]
        );
    }
    assert_eq!(
        pixel(&pixels, bounds[0].right() + 3.0, bounds[0].center().y),
        [255, 255, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].right() + 1.0, bounds[1].center().y),
        [0, 0, 255, 255]
    );
    for rect in &bounds[2..] {
        assert_eq!(
            pixel(&pixels, rect.right() + 1.0, rect.center().y),
            [255, 255, 255, 255]
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn outlines_follow_rounded_corners_and_are_not_clipped_by_own_overflow() {
    let stylesheet = Stylesheet::parse(
        ".box { width:48px; height:24px; margin:8px; border-radius:8px; overflow:hidden; outline:2px solid blue; outline-offset:2px; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".box")).rect);
    })
    .await;
    let rect = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, rect.left() - 3.0, rect.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, rect.left() - 3.0, rect.top() - 3.0),
        [255, 255, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn outline_inheritance_resolves_parent_currentcolor_without_implicit_inheritance() {
    let stylesheet = Stylesheet::parse(
        ".parent { display:flex; gap:16px; padding:8px; color:blue; outline:3px solid currentcolor; outline-offset:2px; }
         .child { width:32px; height:24px; color:red; }
         .inherit { outline:inherit; outline-offset:inherit; }
         .reset { outline:inherit; outline-color:unset; }
         .auto { outline:auto; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() =
                    [".child.inherit", ".child.reset", ".child", ".child.auto"]
                        .map(|selector| ui.add(empty_response.selector(selector)).rect)
                        .to_vec();
                ui.response()
            })
            .selector(".parent"),
        );
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(
        pixel(&pixels, bounds[0].right() + 1.0, bounds[0].center().y),
        [255, 255, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[0].right() + 3.0, bounds[0].center().y),
        [0, 0, 255, 255]
    );
    for index in [1, 3] {
        assert_eq!(
            pixel(
                &pixels,
                bounds[index].right() + 1.0,
                bounds[index].center().y
            ),
            [255, 0, 0, 255]
        );
    }
    assert_eq!(
        pixel(&pixels, bounds[2].right() + 1.0, bounds[2].center().y),
        [255, 255, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn invalid_outline_values_are_rejected() {
    for declaration in [
        "outline-width:-1px",
        "outline-width:10%",
        "outline-offset:10%",
        "outline-style:hidden",
        "outline:1px 2px solid",
        "outline:solid dashed",
        "outline:red blue",
        "outline-color:red blue",
        "outline-style:solid dashed",
        "outline-offset:1px 2px",
        "outline:inherit solid",
    ] {
        assert!(
            Stylesheet::parse(&format!(".box {{ {declaration}; }}")).is_err(),
            "{declaration}"
        );
        assert!(InlineStyle::parse(declaration).is_err(), "{declaration}");
    }
}
