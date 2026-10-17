use super::*;

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn table_rows_share_columns_and_border_spacing() {
    let context = Context::default();
    Stylesheet::parse(".table { display:table; table-layout:fixed; box-sizing:border-box; width:200px; border-spacing:5px 3px; } .row { display:table-row; } .cell { display:table-cell; height:20px; }").unwrap().install(&context);
    let mut rects = Vec::new();
    frame(&context, 300.0, 0.0, Vec::new(), |ui| {
        ui.add(
            (|ui: &mut Ui| {
                let mut cells = Vec::new();
                for _ in 0..2 {
                    ui.add(
                        (|ui: &mut Ui| {
                            for _ in 0..2 {
                                cells.push(ui.add(empty.selector(".cell")).rect);
                            }
                            ui.response()
                        })
                        .selector(".row"),
                    );
                }
                rects = cells;
                ui.response()
            })
            .selector(".table"),
        );
    });
    assert_eq!(rects[0].min, pos2(5.0, 3.0));
    assert_eq!(rects[0].width(), 92.5);
    assert_eq!(rects[1].left(), 102.5);
    assert_eq!(rects[2].top(), 26.0);
    assert_eq!(rects[2].left(), rects[0].left());
    assert_eq!(rects[3].right(), 195.0);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn collapsed_table_row_does_not_consume_a_row_track() {
    let context = Context::default();
    Stylesheet::parse(
        ".table { display:table; width:100px; border-collapse:collapse; } .row { display:table-row; } .collapsed { display:table-row; visibility:collapse; } .cell { display:table-cell; height:20px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                for class in [".row", ".collapsed", ".row"] {
                    ui.add(
                        (|ui: &mut Ui| {
                            rects.push(ui.add(empty.selector(".cell")).rect);
                            ui.response()
                        })
                        .selector(class),
                    );
                }
                ui.response()
            })
            .selector(".table"),
        );
    });
    assert!((rects[0].height() - 20.0).abs() < 0.05, "{rects:?}");
    assert!(
        (rects[2].top() - rects[0].bottom()).abs() < 0.05,
        "{rects:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn collapsed_table_column_does_not_consume_a_column_track() {
    let context = Context::default();
    Stylesheet::parse(
        ".table { display:table; width:100px; table-layout:fixed; border-collapse:collapse; } .column { display:table-column; width:50px; } .collapsed-column { display:table-column; width:50px; visibility:collapse; } .row { display:table-row; } .cell { display:table-cell; height:20px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                ui.add(empty.selector(".collapsed-column"));
                ui.add(empty.selector(".column"));
                ui.add(
                    (|ui: &mut Ui| {
                        rects.push(ui.add(empty.selector(".cell")).rect);
                        rects.push(ui.add(empty.selector(".cell")).rect);
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
    });
    assert_eq!(rects[0].width(), 0.0, "{rects:?}");
    assert!(
        (rects[1].left() - rects[0].left()).abs() < 0.05,
        "{rects:?}"
    );
    assert!((rects[1].width() - 50.0).abs() < 0.05, "{rects:?}");
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn collapsed_table_row_group_removes_its_tracks() {
    let context = Context::default();
    Stylesheet::parse(
        ".table { display:table; width:100px; border-collapse:collapse; } .row { display:table-row; } .group { display:table-row-group; visibility:collapse; } .cell { display:table-cell; height:20px; }",
    )
    .unwrap()
    .install(&context);
    let mut rects = Vec::new();
    frame(&context, 200.0, 0.0, Vec::new(), |ui| {
        rects.clear();
        ui.add(
            (|ui: &mut Ui| {
                ui.add(
                    (|ui: &mut Ui| {
                        rects.push(ui.add(empty.selector(".cell")).rect);
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.add(
                    (|ui: &mut Ui| {
                        ui.add(
                            (|ui: &mut Ui| {
                                rects.push(ui.add(empty.selector(".cell")).rect);
                                ui.response()
                            })
                            .selector(".row"),
                        );
                        ui.response()
                    })
                    .selector(".group"),
                );
                ui.add(
                    (|ui: &mut Ui| {
                        rects.push(ui.add(empty.selector(".cell")).rect);
                        ui.response()
                    })
                    .selector(".row"),
                );
                ui.response()
            })
            .selector(".table"),
        );
    });
    assert!((rects[0].height() - 20.0).abs() < 0.05, "{rects:?}");
    assert_eq!(rects[1].height(), 0.0, "{rects:?}");
    assert!((rects[2].height() - 20.0).abs() < 0.05, "{rects:?}");
    assert!(
        (rects[2].top() - rects[0].bottom()).abs() < 0.05,
        "{rects:?}"
    );
}
