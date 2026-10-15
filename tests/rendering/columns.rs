async fn column_fragments_paint_in_destination_columns_without_painting_gutters_test() {
    let stylesheet=Stylesheet::parse(".columns {width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto;} .box {height:60px; background-color:red;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_box.selector(".box"));
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

async fn column_spanners_create_full_width_boxes_between_column_sets_test() {
    let stylesheet=Stylesheet::parse(".columns {width:210px; column-count:2; column-gap:10px;} .item {height:20px; background-color:red;} .span {column-span:all; height:10px; background-color:blue;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut boxes = Vec::new();
        ui.add(
            (|ui: &mut egui::Ui| {
                boxes.push(ui.add(empty_box.selector(".item")).rect);
                boxes.push(ui.add(empty_box.selector(".item")).rect);
                boxes.push(ui.add(empty_box.selector(".span")).rect);
                boxes.push(ui.add(empty_box.selector(".item")).rect);
                boxes.push(ui.add(empty_box.selector(".item")).rect);
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

async fn column_rules_and_cloned_fragment_borders_render_their_styles_test() {
    let stylesheet = Stylesheet::parse(".columns {width:100px; height:30px; column-count:2; column-gap:10px; column-fill:auto; column-rule:6px double green; column-rule-style:none; column-rule-style:double;} .box {height:60px; box-sizing:border-box; border:2px solid blue; background-color:red; box-decoration-break:clone;}").unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_box.selector(".box"));
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
