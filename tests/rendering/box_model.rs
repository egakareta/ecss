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
