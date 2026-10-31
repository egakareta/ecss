use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn nested_descendant_specificity_states_importance_and_inline_styles_render() {
    let stylesheet = Stylesheet::parse(
        ".panel { display:block; width:160px; height:100px; }
         .panel, #alternate {
             .child { width:64px !important; height:24px; background:red !important; }
         }
         .child.extra { width:48px !important; background:blue !important; }
         .button { width:48px; height:24px; background:blue;
             &:disabled { width:64px; background:lime; }
         }",
    )
    .unwrap();
    let normal_style = InlineStyle::parse("width:32px; background:lime").unwrap();
    let important_style =
        InlineStyle::parse("width:32px !important; background:yellow !important").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                let normal = ui.add(
                    empty_response
                        .selector(".child.extra")
                        .inline_style(normal_style.clone()),
                );
                let important = ui.add(
                    empty_response
                        .selector(".child.extra")
                        .inline_style(important_style.clone()),
                );
                *capture.lock().unwrap() = vec![normal.rect, important.rect];
                ui.response()
            })
            .selector(".panel"),
        );
        let disabled = ui.add_enabled(false, empty_response.selector(".button"));
        capture.lock().unwrap().push(disabled.rect);
    })
    .await;

    let bounds = bounds.lock().unwrap();
    assert_eq!(bounds.len(), 3);
    assert_eq!(bounds[0].size(), egui::vec2(64.0, 24.0));
    assert_eq!(bounds[1].size(), egui::vec2(32.0, 24.0));
    assert_eq!(bounds[2].size(), egui::vec2(64.0, 24.0));
    assert_eq!(
        pixel(&pixels, bounds[0].center().x, bounds[0].center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
        [255, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[2].center().x, bounds[2].center().y),
        [127, 255, 127, 255]
    );
}
