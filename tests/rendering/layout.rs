async fn nested_selectors_distribute_flex_space_and_gap_test() {
    let stylesheet = Stylesheet::parse(
        ".row { display: flex; width: 200px; height: 30px; gap: 10px; }\
         .first { flex: 1 1 0%; min-width: 0; background-color: #f00; }\
         .second { flex: 2 1 0%; min-width: 0; background-color: #00f; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = vec![
                    ui.add(empty_box.selector(".first")).rect,
                    ui.add(empty_box.selector(".second")).rect,
                ];
                ui.response()
            })
            .selector(".row"),
        );
    })
    .await;
    let bounds = bounds.lock().unwrap();
    near(bounds[0].width(), 190.0 / 3.0);
    near(bounds[1].width(), 380.0 / 3.0);
    near(bounds[1].left() - bounds[0].right(), 10.0);
    near(bounds[0].height(), 30.0);
    near(bounds[1].top(), bounds[0].top());
    assert_eq!(
        pixel(&pixels, bounds[0].center().x, bounds[0].center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[1].center().x, bounds[1].center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, bounds[0].right() + 5.0, bounds[0].center().y),
        [255; 4]
    );
}

async fn grid_tracks_spans_and_gaps_control_selector_boxes_test() {
    let stylesheet = Stylesheet::parse(
        ".grid { display: grid; width: 200px; grid-template-columns: repeat(2, minmax(0, 1fr)); grid-template-rows: 20px 30px; gap: 5px 10px; }\
         .first { background-color: #f00; } .second { background-color: #00f; }\
         .wide { grid-column: 1 / -1; background-color: #0f0; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = vec![
                    ui.add(empty_box.selector(".first")).rect,
                    ui.add(empty_box.selector(".second")).rect,
                    ui.add(empty_box.selector(".wide")).rect,
                ];
                ui.response()
            })
            .selector(".grid"),
        );
    })
    .await;
    let bounds = bounds.lock().unwrap();
    assert_eq!(bounds[0].size(), egui::vec2(95.0, 20.0));
    assert_eq!(bounds[1].size(), egui::vec2(95.0, 20.0));
    assert_eq!(bounds[2].size(), egui::vec2(200.0, 30.0));
    near(bounds[2].top() - bounds[0].bottom(), 5.0);
    for (rect, color) in bounds
        .iter()
        .zip([[255, 0, 0, 255], [0, 0, 255, 255], [0, 255, 0, 255]])
    {
        assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), color);
    }
}

async fn overflow_and_display_none_clip_paint_and_remove_layout_space_test() {
    let stylesheet=Stylesheet::parse(
        ".parent { display: flex; width: 60px; height: 30px; overflow: hidden; background-color: #00f; }\
         .absent { display: none; width: 100px; height: 100px; background-color: #0f0; }\
         .child { flex-shrink: 0; width: 100px; height: 50px; background-color: #f00; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut child = None;
        let parent = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_box.selector(".absent"));
                child = Some(ui.add(empty_box.selector(".child")).rect);
                ui.response()
            })
            .selector(".parent"),
        );
        *capture.lock().unwrap() = Some((parent.rect, child.unwrap()));
    })
    .await;
    let (parent, child) = bounds.lock().unwrap().unwrap();
    near(child.left(), parent.left());
    assert_eq!(parent.size(), egui::vec2(60.0, 30.0));
    assert_eq!(
        pixel(&pixels, parent.center().x, parent.center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, parent.right() + 4.0, parent.center().y),
        [255; 4]
    );
    assert_eq!(
        pixel(&pixels, parent.center().x, parent.bottom() + 4.0),
        [255; 4]
    );
}

async fn flow_root_keeps_visible_overflow_test() {
    let stylesheet = Stylesheet::parse(
        ".root { display:flow-root; width:40px; height:20px; background-color:#00f; } .child { width:60px; height:10px; background-color:#f00; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut child = None;
        let root = ui.add(
            (|ui: &mut egui::Ui| {
                child = Some(ui.add(empty_box.selector(".child")).rect);
                ui.response()
            })
            .selector(".root"),
        );
        *capture.lock().unwrap() = Some((root.rect, child.unwrap()));
    })
    .await;
    let (root, child) = bounds.lock().unwrap().unwrap();
    assert!(child.right() > root.right(), "{root:?} {child:?}");
    assert_eq!(
        pixel(&pixels, root.right() + 5.0, child.center().y),
        [255, 0, 0, 255]
    );
}

async fn named_grid_areas_position_children_independently_of_source_order_test() {
    let stylesheet=Stylesheet::parse(
        ".grid { display: grid; width: 150px; grid-template-columns: 50px 100px; grid-template-rows: 20px 30px; grid-template-areas: 'head head' 'nav main'; }\
         .main { grid-area: main; background-color: #f00; }\
         .head { grid-area: head; background-color: #00f; }\
         .nav { grid-area: nav; background-color: #0f0; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = vec![
                    ui.add(empty_box.selector(".main")).rect,
                    ui.add(empty_box.selector(".head")).rect,
                    ui.add(empty_box.selector(".nav")).rect,
                ];
                ui.response()
            })
            .selector(".grid"),
        );
    })
    .await;
    let boxes = bounds.lock().unwrap();
    assert_eq!(boxes[0].size(), egui::vec2(100.0, 30.0));
    assert_eq!(boxes[1].size(), egui::vec2(150.0, 20.0));
    assert_eq!(boxes[2].size(), egui::vec2(50.0, 30.0));
    near(boxes[0].left() - boxes[2].left(), 50.0);
    near(boxes[0].top() - boxes[1].top(), 20.0);
    for (rect, color) in boxes
        .iter()
        .zip([[255, 0, 0, 255], [0, 0, 255, 255], [0, 255, 0, 255]])
    {
        assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), color);
    }
}

async fn floats_and_clear_position_selector_boxes_test() {
    let stylesheet = Stylesheet::parse(
        ".flow { display: flow-root; width: 160px; }\
         .left { float: left; width: 40px; height: 30px; background-color: #f00; }\
         .right { float: right; width: 50px; height: 20px; background-color: #00f; }\
         .clear { clear: both; height: 10px; background-color: #0f0; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = vec![
                    ui.add(empty_box.selector(".left")).rect,
                    ui.add(empty_box.selector(".right")).rect,
                    ui.add(empty_box.selector(".clear")).rect,
                ];
                ui.response()
            })
            .selector(".flow"),
        );
    })
    .await;
    let boxes = bounds.lock().unwrap();
    near(boxes[1].left() - boxes[0].left(), 110.0);
    near(boxes[1].top(), boxes[0].top());
    near(boxes[2].top(), boxes[0].bottom());
    near(boxes[2].width(), 160.0);
    for (rect, color) in boxes
        .iter()
        .zip([[255, 0, 0, 255], [0, 0, 255, 255], [0, 255, 0, 255]])
    {
        assert_eq!(pixel(&pixels, rect.center().x, rect.center().y), color);
    }
}

async fn flex_order_wrap_and_contents_use_only_selector_composition_test() {
    let stylesheet = Stylesheet::parse(
        ".flex { display: flex; flex-wrap: wrap; width: 100px; gap: 5px; }\
         .contents { display: contents; padding: 50px; border: 20px solid #000; }\
         .first { order: 2; width: 60px; height: 20px; background-color: #f00; }\
         .second { order: 1; width: 60px; height: 20px; background-color: #00f; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                let first = ui.add(empty_box.selector(".first")).rect;
                let mut second = None;
                ui.add(
                    (|ui: &mut egui::Ui| {
                        second = Some(ui.add(empty_box.selector(".second")).rect);
                        ui.response()
                    })
                    .selector(".contents"),
                );
                *capture.lock().unwrap() = vec![first, second.unwrap()];
                ui.response()
            })
            .selector(".flex"),
        );
    })
    .await;
    let boxes = bounds.lock().unwrap();
    near(boxes[0].top() - boxes[1].bottom(), 5.0);
    near(boxes[0].left(), boxes[1].left());
    assert_eq!(
        pixel(&pixels, boxes[0].center().x, boxes[0].center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, boxes[1].center().x, boxes[1].center().y),
        [0, 0, 255, 255]
    );
}

async fn visible_descendants_override_hidden_ancestors_without_removing_layout_test() {
    let stylesheet = Stylesheet::parse(
        ".hidden { visibility:hidden; width:80px; height:40px; background-color:red; }\
         .visible { visibility:visible; width:20px; height:10px; background-color:blue; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut child = None;
        let parent = ui.add(
            (|ui: &mut egui::Ui| {
                child = Some(ui.add(empty_box.selector(".visible")).rect);
                ui.response()
            })
            .selector(".hidden"),
        );
        *capture.lock().unwrap() = Some((parent.rect, child.unwrap()));
    })
    .await;
    let (parent, child) = bounds.lock().unwrap().unwrap();
    assert_eq!(parent.size(), egui::vec2(80.0, 40.0));
    assert_eq!(
        pixel(&pixels, child.center().x, child.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, parent.right() - 5.0, parent.bottom() - 5.0),
        [255; 4]
    );
}

async fn stacking_contexts_contain_descendant_z_indices_test() {
    let stylesheet=Stylesheet::parse(
        ".root { position:relative; width:80px; height:40px; }\
         .wrapper { position:relative; } .context { position:relative; z-index:0; }\
         .child { position:absolute; left:0; top:0; width:40px; height:40px; z-index:2; background-color:green; }\
         .sibling { position:absolute; left:0; top:0; width:40px; height:40px; z-index:1; background-color:red; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut rects = Vec::new();
        for parent in [".wrapper", ".context"] {
            let root = ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            ui.add(empty_box.selector(".child"));
                            ui.response()
                        })
                        .selector(parent),
                    );
                    ui.add(empty_box.selector(".sibling"));
                    ui.response()
                })
                .selector(".root"),
            );
            rects.push(root.rect);
        }
        *capture.lock().unwrap() = rects;
    })
    .await;
    let rects = bounds.lock().unwrap();
    assert_eq!(
        pixel(&pixels, rects[0].left() + 10.0, rects[0].top() + 10.0),
        [0, 128, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rects[1].left() + 10.0, rects[1].top() + 10.0),
        [255, 0, 0, 255]
    );
}

async fn vertical_writing_paints_boxes_in_the_declared_block_direction_test() {
    let stylesheet=Stylesheet::parse(".flow {writing-mode:vertical-rl; width:80px; height:40px;} .red {block-size:20px; inline-size:30px; background-color:red;} .blue {block-size:20px; inline-size:30px; background-color:blue;}").unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_box.selector(".red"));
                ui.add(empty_box.selector(".blue"));
                ui.response()
            })
            .selector(".flow"),
        );
    })
    .await;
    assert_eq!(pixel(&pixels, 70.0, 15.0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 50.0, 15.0), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 20.0, 15.0), [255; 4]);
}

async fn subgrid_inherits_tracks_gaps_and_named_lines_on_both_axes_test() {
    let stylesheet = Stylesheet::parse(".grid {display:grid; width:210px; grid-template-columns:[start] 1fr [middle] 1fr [end]; grid-template-rows:20px 20px; gap:10px;} .sub {display:grid; grid-column:start / end; grid-row:1 / 3; grid-template-columns:subgrid [a] [b] [c]; grid-template-rows:subgrid;} .red {grid-column:a / b; grid-row:1; background-color:red;} .blue {grid-column:b / c; grid-row:2; background-color:blue;}").unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(empty_box.selector(".red"));
                        ui.add(empty_box.selector(".blue"));
                        ui.response()
                    })
                    .selector(".sub"),
                );
                ui.response()
            })
            .selector(".grid"),
        );
    })
    .await;
    assert_eq!(pixel(&pixels, 50.0, 10.0), [255, 0, 0, 255]);
    assert_eq!(pixel(&pixels, 160.0, 40.0), [0, 0, 255, 255]);
    assert_eq!(pixel(&pixels, 105.0, 10.0), [255; 4]);
    assert_eq!(pixel(&pixels, 50.0, 25.0), [255; 4]);
    assert_eq!(pixel(&pixels, 50.0, 40.0), [255; 4]);
}
