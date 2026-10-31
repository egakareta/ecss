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
