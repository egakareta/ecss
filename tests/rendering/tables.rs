use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_shared_edges_preserve_the_winning_stripes_test() {
    let stylesheet = Stylesheet::parse(
        ".table { display:table; table-layout:fixed; box-sizing:border-box; width:100px; border-collapse:collapse; }\
         .row { display:table-row; }\
         .a { display:table-cell; height:40px; color:blue; border:4px solid black; }\
         .b { display:table-cell; height:40px; color:lime; border:8px solid green; border-left-color:stripes(red 1fr, currentcolor 1fr, blue 1fr); }",
    )
    .unwrap();
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
    let x = cells[0].right();
    let y = cells[0].center().y;
    assert_eq!(pixel(&pixels, x - 3.0, y), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, x, y), [0, 255, 0, 255]);
    assert_eq!(pixel(&pixels, x + 3.0, y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, x - 5.0, y), [255; 4]);
    assert_eq!(pixel(&pixels, x + 5.0, y), [255; 4]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_column_and_group_borders_compete_with_cell_borders_test() {
    let stylesheet = Stylesheet::parse(
        ".table { display:table; table-layout:fixed; box-sizing:border-box; width:150px; border-collapse:collapse; }\
         .column-group { display:table-column-group; border-right:14px solid red; }\
         .first-column { display:table-column; border-right:12px solid blue; }\
         .second-group { display:table-column-group; border-right:12px solid yellow; }\
         .second-column { display:table-column; border-right:12px solid lime; }\
         .third-column { display:table-column; }\
         .row { display:table-row; }\
         .cell { display:table-cell; height:40px; border:1px solid black; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut cells = Vec::new();
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(empty_box.selector(".first-column"));
                        ui.response()
                    })
                    .selector(".column-group"),
                );
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(empty_box.selector(".second-column"));
                        ui.response()
                    })
                    .selector(".second-group"),
                );
                ui.add(empty_box.selector(".third-column"));
                ui.add(
                    (|ui: &mut egui::Ui| {
                        cells = vec![
                            ui.add(empty_box.selector(".cell")).rect,
                            ui.add(empty_box.selector(".cell")).rect,
                            ui.add(empty_box.selector(".cell")).rect,
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
    near(cells[1].right(), cells[2].left());
    assert_eq!(
        pixel(&pixels, cells[0].right() - 5.0, cells[0].center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, cells[0].right() + 5.0, cells[0].center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, cells[1].right() - 3.0, cells[1].center().y),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, cells[1].right() + 3.0, cells[1].center().y),
        [0, 255, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_column_and_group_borders_follow_rtl_columns_test() {
    let stylesheet = Stylesheet::parse(
        ".table { display:table; table-layout:fixed; box-sizing:border-box; width:150px; border-collapse:collapse; direction:rtl; }\
         .column-group { display:table-column-group; border-left:14px solid red; }\
         .first-column { display:table-column; border-left:12px solid blue; }\
         .second-group { display:table-column-group; border-left:12px solid yellow; }\
         .second-column { display:table-column; border-left:12px solid lime; }\
         .third-column { display:table-column; }\
         .row { display:table-row; }\
         .cell { display:table-cell; height:40px; border:1px solid black; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut cells = Vec::new();
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(empty_box.selector(".first-column"));
                        ui.response()
                    })
                    .selector(".column-group"),
                );
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(empty_box.selector(".second-column"));
                        ui.response()
                    })
                    .selector(".second-group"),
                );
                ui.add(empty_box.selector(".third-column"));
                ui.add(
                    (|ui: &mut egui::Ui| {
                        cells = (0..3)
                            .map(|_| ui.add(empty_box.selector(".cell")).rect)
                            .collect();
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
    let bounds = bounds.lock().unwrap();
    let cells = bounds.as_ref().unwrap();
    near(cells[0].left(), cells[1].right());
    near(cells[1].left(), cells[2].right());
    let y = cells[0].center().y;
    assert_eq!(pixel(&pixels, cells[0].left() - 5.0, y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, cells[0].left() + 5.0, y), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, cells[1].left() - 3.0, y), [0, 255, 0, 255]);
    assert_eq!(pixel(&pixels, cells[1].left() + 3.0, y), [0, 255, 0, 255]);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_equal_cell_borders_follow_table_direction_test() {
    async fn shared_border(direction: &str) -> ([u8; 4], Vec<egui::Rect>) {
        let stylesheet = Stylesheet::parse(&format!(
            ".table {{ display:table; table-layout:fixed; box-sizing:border-box; width:100px; border-collapse:collapse; direction:{direction}; }}\
             .row {{ display:table-row; }}\
             .a {{ display:table-cell; height:30px; border:4px solid black; border-left-color:blue; border-right-color:red; }}\
             .b {{ display:table-cell; height:30px; border:4px solid black; border-left-color:lime; border-right-color:yellow; }}"
        ))
        .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    let mut cells = Vec::new();
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
                    *capture.lock().unwrap() = Some(cells);
                    ui.response()
                })
                .selector(".table"),
            );
        })
        .await;
        let cells = bounds.lock().unwrap();
        let cells = cells.as_ref().unwrap();
        let boundary = (cells[0].center().x + cells[1].center().x) * 0.5;
        (
            pixel(&pixels, boundary, cells[0].center().y),
            cells.to_vec(),
        )
    }

    let (ltr, ltr_cells) = shared_border("ltr").await;
    let (rtl, rtl_cells) = shared_border("rtl").await;
    assert_eq!(ltr, [255, 0, 0, 255], "{ltr_cells:?}");
    assert_eq!(rtl, [0, 0, 255, 255], "{rtl_cells:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_outer_border_widths_follow_table_direction_test() {
    let stylesheet = Stylesheet::parse(
        ".table { display:table; table-layout:fixed; box-sizing:border-box; width:100px; border-collapse:collapse; direction:rtl; }\
         .row { display:table-row; }\
         .a { display:table-cell; height:30px; border:4px solid black; border-left-width:2px; border-right-width:8px; }\
         .b { display:table-cell; height:30px; border:4px solid black; border-left-width:6px; border-right-width:3px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    render_ui_pixels(stylesheet, move |ui| {
        let mut cells = Vec::new();
        let table = ui.add(
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
        *capture.lock().unwrap() = Some((cells, table.rect));
    })
    .await;

    let bounds = bounds.lock().unwrap();
    let (cells, table) = bounds.as_ref().unwrap();
    near(cells[0].right(), table.right() - 4.0);
    near(cells[1].left(), table.left() + 3.0);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_missing_cells_do_not_pull_outer_borders_inward_test() {
    let stylesheet = Stylesheet::parse(
        ".table { display:table; table-layout:fixed; box-sizing:border-box; width:100px; border-collapse:collapse; border:8px solid blue; }\
         .row { display:table-row; }\
         .cell { display:table-cell; height:30px; border:2px solid red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut rows = Vec::new();
        ui.add(
            (|ui: &mut egui::Ui| {
                for count in [2, 1] {
                    let mut cells = Vec::new();
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            cells = (0..count)
                                .map(|_| ui.add(empty_box.selector(".cell")).rect)
                                .collect();
                            ui.response()
                        })
                        .selector(".row"),
                    );
                    rows.push(cells);
                }
                ui.response()
            })
            .selector(".table"),
        );
        *capture.lock().unwrap() = Some(rows);
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let rows = bounds.as_ref().unwrap();
    let short_row_cell = rows[1][0];
    assert_eq!(
        pixel(&pixels, short_row_cell.right() + 3.0, short_row_cell.center().y),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, rows[0][1].right() + 3.0, short_row_cell.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_missing_cells_respect_rtl_outer_border_test() {
    let stylesheet = Stylesheet::parse(
        ".table { display:table; table-layout:fixed; box-sizing:border-box; width:100px; border-collapse:collapse; border:8px solid blue; direction:rtl; }\
         .row { display:table-row; }\
         .cell { display:table-cell; height:30px; border:2px solid red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut rows = Vec::new();
        ui.add(
            (|ui: &mut egui::Ui| {
                for count in [2, 1] {
                    let mut cells = Vec::new();
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            cells = (0..count)
                                .map(|_| ui.add(empty_box.selector(".cell")).rect)
                                .collect();
                            ui.response()
                        })
                        .selector(".row"),
                    );
                    rows.push(cells);
                }
                ui.response()
            })
            .selector(".table"),
        );
        *capture.lock().unwrap() = Some(rows);
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let rows = bounds.as_ref().unwrap();
    let short_row_cell = rows[1][0];
    assert_eq!(
        pixel(&pixels, short_row_cell.left() - 3.0, short_row_cell.center().y),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, rows[0][1].left() - 3.0, short_row_cell.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_anonymous_rows_paint_missing_outer_border_segments_test() {
    let stylesheet = Stylesheet::parse(
        ".table { display:table; table-layout:fixed; box-sizing:border-box; width:100px; border-collapse:collapse; border:8px solid blue; }\
         .row { display:table-row; }\
         .cell { display:table-cell; height:30px; border:2px solid red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut cells = Vec::new();
        let mut full_row = Vec::new();
        ui.add(
            (|ui: &mut egui::Ui| {
                cells.push(ui.add(empty_box.selector(".cell")).rect);
                ui.add(
                    (|ui: &mut egui::Ui| {
                        full_row = (0..2)
                            .map(|_| ui.add(empty_box.selector(".cell")).rect)
                            .collect();
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
        *capture.lock().unwrap() = Some((cells, full_row));
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let (anonymous_row, full_row) = bounds.as_ref().unwrap();
    assert_eq!(
        pixel(
            &pixels,
            anonymous_row[0].right() + 3.0,
            anonymous_row[0].center().y
        ),
        [255; 4]
    );
    assert_eq!(
        pixel(
            &pixels,
            full_row[1].right() + 3.0,
            anonymous_row[0].center().y
        ),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_missing_declared_columns_keep_the_outer_border_at_the_grid_edge_test() {
    let stylesheet = Stylesheet::parse(
        ".table { display:table; table-layout:fixed; box-sizing:border-box; width:100px; border-collapse:collapse; border:8px solid blue; }\
         .column { display:table-column; width:40px; }\
         .row { display:table-row; }\
         .cell { display:table-cell; height:30px; border:2px solid red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut cell = None;
        let table = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_box.selector(".column"));
                ui.add(empty_box.selector(".column"));
                ui.add(
                    (|ui: &mut egui::Ui| {
                        cell = Some(ui.add(empty_box.selector(".cell")).rect);
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
        *capture.lock().unwrap() = Some((cell.unwrap(), table.rect));
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let (cell, table) = bounds.as_ref().unwrap();
    assert_eq!(
        pixel(&pixels, cell.right() + 3.0, cell.center().y),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, table.right() - 3.0, cell.center().y),
        [0, 0, 255, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_vertical_table_anonymous_rows_paint_missing_outer_border_segments_test() {
    let stylesheet = Stylesheet::parse(
        ".table { display:table; table-layout:fixed; box-sizing:border-box; width:100px; height:100px; writing-mode:vertical-rl; border-collapse:collapse; border:8px solid blue; }\
         .row { display:table-row; }\
         .cell { display:table-cell; width:30px; height:30px; border:2px solid red; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut anonymous = None;
        let mut full_row = Vec::new();
        ui.add(
            (|ui: &mut egui::Ui| {
                anonymous = Some(ui.add(empty_box.selector(".cell")).rect);
                ui.add(
                    (|ui: &mut egui::Ui| {
                        full_row = (0..2)
                            .map(|_| ui.add(empty_box.selector(".cell")).rect)
                            .collect();
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
        *capture.lock().unwrap() = Some((anonymous.unwrap(), full_row));
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let (anonymous, full_row) = bounds.as_ref().unwrap();
    assert_eq!(
        pixel(&pixels, anonymous.center().x, anonymous.bottom() + 3.0),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, anonymous.center().x, full_row[1].bottom() + 3.0),
        [0, 0, 255, 255],
        "{anonymous:?} {full_row:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_cells_ignore_border_radius_test() {
    let stylesheet = Stylesheet::parse(
        ".collapsed { display:table; table-layout:fixed; box-sizing:border-box; width:64px; border-collapse:collapse; }\
         .separate { display:table; table-layout:fixed; box-sizing:border-box; width:64px; border-collapse:separate; border-spacing:0; }\
         .row { display:table-row; }\
         .collapsed-cell, .separate-cell { display:table-cell; box-sizing:border-box; height:64px; overflow:hidden; background:red; border-radius:20px; }\
         .fill { box-sizing:border-box; width:64px; height:64px; background:blue; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 2]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for (index, table, cell) in [
            (0, ".collapsed", ".collapsed-cell"),
            (1, ".separate", ".separate-cell"),
        ] {
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            let response = ui.add(
                                (|ui: &mut egui::Ui| {
                                    ui.add(empty_box.selector(".fill"));
                                    ui.response()
                                })
                                .selector(cell),
                            );
                            capture.lock().unwrap()[index] = Some(response.rect);
                            ui.response()
                        })
                        .selector(".row"),
                    );
                    ui.response()
                })
                .selector(table),
            );
        }
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let collapsed = bounds[0].unwrap();
    let separate = bounds[1].unwrap();
    assert_eq!(
        pixel(&pixels, collapsed.left() + 2.5, collapsed.top() + 2.5),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, separate.left() + 2.5, separate.top() + 2.5),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_cell_borders_ignore_border_radius_test() {
    let stylesheet = Stylesheet::parse(
        ".collapsed { display:table; table-layout:fixed; box-sizing:border-box; width:64px; border-collapse:collapse; }\
         .separate { display:table; table-layout:fixed; box-sizing:border-box; width:64px; border-collapse:separate; border-spacing:0; }\
         .row { display:table-row; }\
         .collapsed-cell, .separate-cell { display:table-cell; box-sizing:border-box; height:64px; background:red; border:4px solid blue; border-radius:20px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 2]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for (index, table, cell) in [
            (0, ".collapsed", ".collapsed-cell"),
            (1, ".separate", ".separate-cell"),
        ] {
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            capture.lock().unwrap()[index] =
                                Some(ui.add(empty_box.selector(cell)).rect);
                            ui.response()
                        })
                        .selector(".row"),
                    );
                    ui.response()
                })
                .selector(table),
            );
        }
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let collapsed = bounds[0].unwrap();
    let separate = bounds[1].unwrap();
    assert_ne!(
        pixel(&pixels, collapsed.left() + 1.0, collapsed.top() + 1.0),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, separate.left() + 1.0, separate.top() + 1.0),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_box_background_ignores_border_radius_test() {
    let stylesheet = Stylesheet::parse(
        ".collapsed { display:table; table-layout:fixed; box-sizing:border-box; width:64px; border-collapse:collapse; background:red; border-radius:20px; }\
         .separate { display:table; table-layout:fixed; box-sizing:border-box; width:64px; border-collapse:separate; border-spacing:0; background:red; border-radius:20px; }\
         .row { display:table-row; }\
         .cell { display:table-cell; box-sizing:border-box; height:64px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 2]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for (index, table) in [(0, ".collapsed"), (1, ".separate")] {
            let response = ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            ui.add(empty_box.selector(".cell"));
                            ui.response()
                        })
                        .selector(".row"),
                    );
                    ui.response()
                })
                .selector(table),
            );
            capture.lock().unwrap()[index] = Some(response.rect);
        }
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let collapsed = bounds[0].unwrap();
    let separate = bounds[1].unwrap();
    assert_eq!(
        pixel(&pixels, collapsed.left() + 2.5, collapsed.top() + 2.5),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, separate.left() + 2.5, separate.top() + 2.5),
        [255; 4]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_box_shadow_ignores_border_radius_test() {
    let stylesheet = Stylesheet::parse(
        ".table { display:table; table-layout:fixed; box-sizing:border-box; width:64px; margin:24px; border-collapse:collapse; border-radius:20px; box-shadow:0 0 0 6px green; }\
         .row { display:table-row; }\
         .cell { display:table-cell; box-sizing:border-box; height:64px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(empty_box.selector(".cell"));
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
        *capture.lock().unwrap() = Some(response.rect);
    })
    .await;
    let table = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, table.left() - 1.5, table.top() - 1.5),
        [0, 128, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_table_internal_cells_do_not_paint_outer_shadows_test() {
    let stylesheet = Stylesheet::parse(
        ".collapsed { display:table; table-layout:fixed; width:80px; border-collapse:collapse; box-shadow:0 0 0 6px green; }\
         .separate { display:table; table-layout:fixed; width:80px; border-collapse:separate; }\
         .row { display:table-row; }\
         .collapsed-cell, .separate-cell { display:table-cell; height:40px; background:white; border:2px solid blue; border-image:image(red) 2; box-shadow:0 0 0 6px red, 0 0 0 6px green inset; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 2]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for (index, table, cell) in [
            (0, ".collapsed", ".collapsed-cell"),
            (1, ".separate", ".separate-cell"),
        ] {
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            capture.lock().unwrap()[index] =
                                Some(ui.add(empty_box.selector(cell)).rect);
                            ui.response()
                        })
                        .selector(".row"),
                    );
                    ui.response()
                })
                .selector(table),
            );
        }
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let collapsed = bounds[0].unwrap();
    let separate = bounds[1].unwrap();
    assert_eq!(
        pixel(&pixels, collapsed.right() + 2.0, collapsed.center().y),
        [0, 128, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, separate.right() + 2.0, separate.center().y),
        [255, 0, 0, 255]
    );
    assert_ne!(
        pixel(&pixels, collapsed.center().x, collapsed.top() + 0.5),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, separate.center().x, separate.top() + 0.5),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, collapsed.left() + 4.0, collapsed.top() + 4.0),
        [0, 128, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn collapsed_tables_do_not_paint_border_images_test() {
    let stylesheet = Stylesheet::parse(
        ".collapsed { display:table; table-layout:fixed; box-sizing:border-box; width:80px; border-collapse:collapse; border:4px solid blue; border-image:image(red) 2; }\
         .separate { display:table; table-layout:fixed; box-sizing:border-box; width:80px; border-collapse:separate; border:4px solid blue; border-image:image(red) 2; }\
         .row { display:table-row; }\
         .cell { display:table-cell; height:40px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new([None; 2]));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        for (index, table_class) in [(0, ".collapsed"), (1, ".separate")] {
            let table = ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            ui.add(empty_box.selector(".cell"));
                            ui.response()
                        })
                        .selector(".row"),
                    );
                    ui.response()
                })
                .selector(table_class),
            );
            capture.lock().unwrap()[index] = Some(table.rect);
        }
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let collapsed = bounds[0].unwrap();
    let separate = bounds[1].unwrap();
    assert_eq!(
        pixel(&pixels, collapsed.left() + 1.0, collapsed.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, separate.left() + 1.0, separate.center().y),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
