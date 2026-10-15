async fn collapsed_table_borders_choose_one_shared_edge_test() {
    let stylesheet=Stylesheet::parse(".table {display:table; table-layout:fixed; box-sizing:border-box; width:200px; border-collapse:collapse;} .row {display:table-row; border-top:10px solid green;} .a {display:table-cell; height:30px; border:2px solid red; border-top:12px solid red;} .b {display:table-cell; height:30px; border:2px solid red; border-left:8px solid blue; border-top:12px solid red;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut cells = Vec::new();
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        cells = vec![
                            ui.add(empty_box.selector(".a")).rect,
                            ui.add(empty_box.selector(".b")).rect,
                        ];
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
        *capture.lock().unwrap() = Some(cells);
    })
    .await;
    let cells = bounds.lock().unwrap();
    let cells = cells.as_ref().unwrap();
    near(cells[0].right(), cells[1].left());
    assert_eq!(
        pixel(&pixels, cells[0].center().x, cells[0].top() + 8.0),
        [255; 4]
    );
    let x = cells[0].right();
    let y = cells[0].center().y;
    assert_eq!(pixel(&pixels, x - 3.0, y), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, x + 3.0, y), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, x - 6.0, y), [255; 4]);
    assert_eq!(pixel(&pixels, x + 6.0, y), [255; 4]);
}

async fn empty_table_cells_hide_borders_and_backgrounds_without_changing_tracks_test() {
    let stylesheet = Stylesheet::parse(".table {display:table; width:120px; table-layout:fixed; empty-cells:hide;} .row {display:table-row;} .cell {display:table-cell; height:30px; background-color:red; border:2px solid blue;} .visible {display:table-cell; height:30px; background-color:red; border:2px solid blue; empty-cells:show;}").unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(empty_box.selector(".cell"));
                        ui.add(empty_box.selector(".visible"));
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
    })
    .await;
    assert_eq!(pixel(&pixels, 10.0, 10.0), [255; 4]);
    assert_eq!(pixel(&pixels, 0.5, 10.0), [255; 4]);
    assert_eq!(pixel(&pixels, 80.0, 10.0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 60.5, 10.0), [0, 0, 255, 255]);
}

async fn table_cells_align_native_text_within_the_row_height_test() {
    let stylesheet = Stylesheet::parse(".table {display:table;table-layout:fixed;width:100px;} .row {display:table-row;} .middle {display:table-cell;height:40px;vertical-align:middle;color:red;} .top {display:table-cell;vertical-align:top;color:blue;}").unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(egui::Label::new("H").selector(".middle"));
                        ui.add(egui::Label::new("H").selector(".top"));
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
    })
    .await;
    let mut red = 40;
    let mut blue = 40;
    for y in 0..40 {
        for x in 0..100 {
            let p = pixel(&pixels, x as f32, y as f32);
            if p[0] > 128 && p[1] < 128 && p[2] < 128 {
                red = red.min(y);
            }
            if p[2] > 128 && p[0] < 128 && p[1] < 128 {
                blue = blue.min(y);
            }
        }
    }
    assert!(blue < 10 && red > blue + 8 && red < 25, "{red} {blue}");
}
