use super::*;

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
                    ui.add(empty_response.selector(".first")).rect,
                    ui.add(empty_response.selector(".second")).rect,
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
                    ui.add(empty_response.selector(".first")).rect,
                    ui.add(empty_response.selector(".second")).rect,
                    ui.add(empty_response.selector(".wide")).rect,
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn chained_grid_slots_continue_text_at_a_line_break_test() {
    let stylesheet = Stylesheet::parse(
        ".grid { display:grid; width:100px; height:20px; grid-template-columns:30px 70px; grid-template-rows:20px; grid-template-areas:\"first second\"; chains:first second; } .text { flow:first; font-size:12px; color:#f00; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let grid = ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("one two").selector(".text"));
                ui.response()
            })
            .selector(".grid"),
        );
        *capture.lock().unwrap() = Some(grid.rect);
    })
    .await;
    let grid = bounds.lock().unwrap().unwrap();
    let second_slot_has_text = ((grid.left() + 31.0).floor() as i32..grid.right().ceil() as i32)
        .any(|x| {
            (grid.top().floor() as i32..grid.bottom().ceil() as i32).any(|y| {
                let pixel = pixel(&pixels, x as f32, y as f32);
                pixel[0] > 150 && pixel[1] < 100 && pixel[2] < 100
            })
        });
    assert!(
        second_slot_has_text,
        "text did not continue into slot two: grid={grid:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
                ui.add(empty_response.selector(".absent"));
                child = Some(ui.add(empty_response.selector(".child")).rect);
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
                child = Some(ui.add(empty_response.selector(".child")).rect);
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
                    ui.add(empty_response.selector(".main")).rect,
                    ui.add(empty_response.selector(".head")).rect,
                    ui.add(empty_response.selector(".nav")).rect,
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
                    ui.add(empty_response.selector(".left")).rect,
                    ui.add(empty_response.selector(".right")).rect,
                    ui.add(empty_response.selector(".clear")).rect,
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
                let first = ui.add(empty_response.selector(".first")).rect;
                let mut second = None;
                ui.add(
                    (|ui: &mut egui::Ui| {
                        second = Some(ui.add(empty_response.selector(".second")).rect);
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
                child = Some(ui.add(empty_response.selector(".visible")).rect);
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn stacking_contexts_contain_descendant_z_indices_test() {
    let stylesheet=Stylesheet::parse(
        ".root { position:relative; width:80px; height:24px; }\
         .wrapper { position:relative; } .context { position:relative; z-index:0; }\
         .will-context { position:relative; will-change:z-index; }\
         .will-position-context { will-change:position; }\
         .child { position:absolute; left:0; top:0; width:40px; height:24px; z-index:2; background-color:green; }\
         .sibling { position:absolute; left:0; top:0; width:40px; height:24px; z-index:1; background-color:red; }",
    ).unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut rects = Vec::new();
        for parent in [
            ".wrapper",
            ".context",
            ".will-context",
            ".will-position-context",
        ] {
            let root = ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            ui.add(empty_response.selector(".child"));
                            ui.response()
                        })
                        .selector(parent),
                    );
                    ui.add(empty_response.selector(".sibling"));
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
    assert_eq!(
        pixel(&pixels, rects[2].left() + 10.0, rects[2].top() + 10.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, rects[3].left() + 10.0, rects[3].top() + 10.0),
        [255, 0, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn will_change_position_establishes_absolute_and_fixed_containing_blocks_test() {
    let stylesheet = Stylesheet::parse(
        ".root { position:relative; width:120px; height:80px; }\
         .anchor { will-change:position; margin-left:20px; margin-top:10px; width:60px; height:50px; background-color:blue; }\
         .absolute { position:absolute; left:0; top:0; width:10px; height:10px; background-color:red; }\
         .fixed { position:fixed; left:10px; top:10px; width:10px; height:10px; background-color:green; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let capture = capture.clone();
        ui.add(
            (move |ui: &mut egui::Ui| {
                let mut child_rects = Vec::new();
                let anchor = ui.add(
                    (|ui: &mut egui::Ui| {
                        child_rects.push(ui.add(empty_response.selector(".absolute")).rect);
                        child_rects.push(ui.add(empty_response.selector(".fixed")).rect);
                        ui.response()
                    })
                    .selector(".anchor"),
                );
                *capture.lock().unwrap() = Some((anchor.rect, child_rects));
                ui.response()
            })
            .selector(".root"),
        );
    })
    .await;
    let (anchor, children) = bounds.lock().unwrap().clone().unwrap();
    assert_eq!(children[0].left(), anchor.left());
    assert_eq!(children[0].top(), anchor.top());
    assert_eq!(children[1].left(), anchor.left() + 10.0);
    assert_eq!(children[1].top(), anchor.top() + 10.0);
    assert_eq!(
        pixel(&pixels, children[0].center().x, children[0].center().y),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, children[1].center().x, children[1].center().y),
        [0, 128, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn vertical_writing_paints_boxes_in_the_declared_block_direction_test() {
    let stylesheet=Stylesheet::parse(".flow {writing-mode:vertical-rl; width:80px; height:40px;} .red {block-size:20px; inline-size:30px; background-color:red;} .blue {block-size:20px; inline-size:30px; background-color:blue;}").unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(empty_response.selector(".red"));
                ui.add(empty_response.selector(".blue"));
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn subgrid_inherits_tracks_gaps_and_named_lines_on_both_axes_test() {
    let stylesheet = Stylesheet::parse(".grid {display:grid; width:210px; grid-template-columns:[start] 1fr [middle] 1fr [end]; grid-template-rows:20px 20px; gap:10px;} .sub {display:grid; grid-column:start / end; grid-row:1 / 3; grid-template-columns:subgrid [a] [b] [c]; grid-template-rows:subgrid;} .red {grid-column:a / b; grid-row:1; background-color:red;} .blue {grid-column:b / c; grid-row:2; background-color:blue;}").unwrap();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(empty_response.selector(".red"));
                        ui.add(empty_response.selector(".blue"));
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

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn named_flow_text_continues_across_document_ordered_regions_test() {
    let stylesheet = Stylesheet::parse(
        ".first { display:block; width:50px; height:14px; flow-from:article; background-color:#00f; }\
         .second { display:block; width:80px; height:28px; flow-from:article; background-color:#00f; }\
         .region-child { width:50px; height:14px; background-color:#0f0; }\
         .source { flow-into:article content; font-size:12px; color:#f00; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("one two three four five six seven").selector(".source"));
                *capture.lock().unwrap() = vec![
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            ui.add(empty_response.selector(".region-child"));
                            ui.response()
                        })
                        .selector(".first"),
                    )
                    .rect,
                    ui.add(empty_response.selector(".second")).rect,
                ];
                ui.response()
            })
            .selector(".container"),
        );
    })
    .await;
    let regions = bounds.lock().unwrap();
    for x in regions[0].left().floor() as i32..regions[0].right().ceil() as i32 {
        for y in regions[0].top().floor() as i32..regions[0].bottom().ceil() as i32 {
            assert_ne!(pixel(&pixels, x as f32, y as f32), [0, 255, 0, 255]);
        }
    }
    for region in regions.iter() {
        let has_red_text = (region.left().floor() as i32..region.right().ceil() as i32).any(|x| {
            (region.top().floor() as i32..region.bottom().ceil() as i32).any(|y| {
                let color = pixel(&pixels, x as f32, y as f32);
                color[0] > 150 && color[1] < 100 && color[2] < 100
            })
        });
        assert!(has_red_text, "region received no text: {region:?}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_justify_modes_select_word_character_and_script_specific_spacing_test() {
    async fn render(mode: &str, text: &str) -> Vec<[u8; 4]> {
        let stylesheet = Stylesheet::parse(&format!(
            ".region {{ display:block; width:100px; height:60px; flow-from:article; }}\
             .source {{ flow-into:article content; font-size:14px; \
             color:#f00; text-align:justify-all; text-justify:{mode}; }}"
        ))
        .unwrap();
        let text = text.to_owned();
        render_ui_pixels(stylesheet, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(egui::Label::new(text.clone()).selector(".source"));
                    ui.add(empty_response.selector(".region"));
                    ui.response()
                })
                .selector(".container"),
            );
        })
        .await
    }
    async fn render_widget(
        mode: &str,
        alignment: &str,
        last_alignment: &str,
        text: &str,
    ) -> Vec<[u8; 4]> {
        let stylesheet = Stylesheet::parse(&format!(
            ".text {{ display:block; width:100px; height:60px; font-size:14px; color:#f00; \
             text-align:{alignment}; text-align-last:{last_alignment}; text-justify:{mode}; }}"
        ))
        .unwrap();
        let text = text.to_owned();
        render_ui_pixels(stylesheet, move |ui| {
            ui.add(egui::Label::new(text.clone()).selector(".text"));
        })
        .await
    }

    let latin = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu";
    let none = render("none", latin).await;
    let inter_word = render("inter-word", latin).await;
    let inter_character = render("inter-character", latin).await;
    let auto_latin = render("auto", latin).await;
    assert_ne!(none, inter_word);
    assert_ne!(inter_character, inter_word);
    assert_eq!(auto_latin, inter_word);

    let cjk = "日本語日本語日本語日本語日本語日本語";
    let cjk_inter_word = render("inter-word", cjk).await;
    let cjk_inter_character = render("inter-character", cjk).await;
    let auto_cjk = render("auto", cjk).await;
    assert_eq!(auto_cjk, cjk_inter_character);
    assert_ne!(auto_cjk, cjk_inter_word);

    let generic_none = render_widget("none", "justify-all", "auto", latin).await;
    let generic_inter_word = render_widget("inter-word", "justify-all", "auto", latin).await;
    assert_ne!(generic_none, generic_inter_word);

    let last_line_words = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda";
    let plain_last = render_widget("inter-word", "left", "auto", last_line_words).await;
    let justified_last = render_widget("inter-word", "left", "justify", last_line_words).await;
    assert_ne!(plain_last, justified_last);

    let justify = render_widget("inter-word", "justify", "auto", last_line_words).await;
    let justify_all = render_widget("inter-word", "justify-all", "auto", last_line_words).await;
    assert_ne!(justify, justify_all);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn anonymous_text_in_content_flow_continues_across_regions_test() {
    let stylesheet = Stylesheet::parse(
        ".first { display:block; width:60px; height:14px; flow-from:article; overflow:hidden; background-color:#00f; }\
         .second { display:block; width:60px; height:14px; flow-from:article; overflow:hidden; background-color:#00f; }\
         .source { display:block; width:60px; flow-into:article content; font-size:12px; color:#f00; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.label("anonymous one");
                        ui.label("anonymous two");
                        ui.response()
                    })
                    .selector(".source"),
                );
                *capture.lock().unwrap() = vec![
                    ui.add(empty_response.selector(".first")).rect,
                    ui.add(empty_response.selector(".second")).rect,
                ];
                ui.response()
            })
            .selector(".container"),
        );
    })
    .await;
    let regions = bounds.lock().unwrap();
    for region in regions.iter() {
        let has_red_text = (region.left().floor() as i32..region.right().ceil() as i32).any(|x| {
            (region.top().floor() as i32..region.bottom().ceil() as i32).any(|y| {
                let color = pixel(&pixels, x as f32, y as f32);
                color[0] > 150 && color[1] < 100 && color[2] < 100
            })
        });
        assert!(
            has_red_text,
            "region received no anonymous text: {region:?}"
        );
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn auto_height_region_expands_to_flow_content_test() {
    let stylesheet = Stylesheet::parse(
        ".region { display:block; width:55px; flow-from:article; box-sizing:content-box; padding:2px; border:1px solid black; background-color:#00f; }\
         .after { display:block; width:55px; height:5px; background-color:#0f0; }\
         .source { flow-into:article; font-size:12px; color:#f00; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    egui::Label::new("one two three four five six seven eight nine")
                        .selector(".source"),
                );
                let region = ui.add(empty_response.selector(".region")).rect;
                let after = ui.add(empty_response.selector(".after")).rect;
                *capture.lock().unwrap() = Some((region, after));
                ui.response()
            })
            .selector(".container"),
        );
    })
    .await;
    let (region, after) = bounds.lock().unwrap().unwrap();
    assert!(
        region.height() > 14.0,
        "auto region did not grow: {region:?}"
    );
    assert!(
        after.top() >= region.bottom() - 0.01,
        "following block overlaps region: {region:?} {after:?}"
    );
    let has_red_text = (region.left().floor() as i32..region.right().ceil() as i32).any(|x| {
        (region.top().floor() as i32..region.bottom().ceil() as i32).any(|y| {
            let color = pixel(&pixels, x as f32, y as f32);
            color[0] > 150 && color[1] < 100 && color[2] < 100
        })
    });
    assert!(
        has_red_text,
        "auto region did not receive flow content: {region:?}"
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn region_fragment_break_discards_overflow_from_the_last_region_test() {
    async fn render(fragment: &str) -> (Vec<[u8; 4]>, egui::Rect) {
        let stylesheet = Stylesheet::parse(&format!(
            ".first {{ display:block; width:40px; height:14px; flow-from:article; }}\
             .last {{ display:block; width:40px; height:14px; flow-from:article; region-fragment:{fragment}; }}\
             .source {{ flow-into:article; font-size:12px; color:#f00; }}"
        ))
        .unwrap();
        let bounds = std::sync::Arc::new(Mutex::new(None));
        let capture = bounds.clone();
        let pixels = render_ui_pixels(stylesheet, move |ui| {
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(empty_response.selector(".first"));
                    let last = ui.add(empty_response.selector(".last")).rect;
                    ui.add(egui::Label::new("one two three four five six").selector(".source"));
                    ui.add(egui::Label::new("continued").selector(".source"));
                    *capture.lock().unwrap() = Some(last);
                    ui.response()
                })
                .selector(".container"),
            );
        })
        .await;
        (pixels, bounds.lock().unwrap().unwrap())
    }

    let (auto_pixels, last) = render("auto").await;
    let (break_pixels, _) = render("break").await;
    let overflow_has_text = (last.left().floor() as i32..last.right().ceil() as i32).any(|x| {
        (last.bottom().floor() as i32..(last.bottom() + 14.0).ceil() as i32).any(|y| {
            let color = pixel(&auto_pixels, x as f32, y as f32);
            color[0] > 150 && color[1] < 100 && color[2] < 100
        })
    });
    let break_overflow_has_text =
        (last.left().floor() as i32..last.right().ceil() as i32).any(|x| {
            (last.bottom().floor() as i32..(last.bottom() + 14.0).ceil() as i32).any(|y| {
                let color = pixel(&break_pixels, x as f32, y as f32);
                color[0] > 150 && color[1] < 100 && color[2] < 100
            })
        });
    assert!(overflow_has_text);
    assert!(!break_overflow_has_text);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn region_break_before_advances_to_the_next_region_test() {
    let stylesheet = Stylesheet::parse(
        ".first { display:block; width:50px; height:28px; flow-from:article; }\
         .second { display:block; width:50px; height:14px; flow-from:article; }\
         .source { flow-into:article; font-size:12px; color:#f00; }\
         .next { flow-into:article; font-size:12px; color:#f00; break-before:region; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = vec![
                    ui.add(empty_response.selector(".first")).rect,
                    ui.add(empty_response.selector(".second")).rect,
                ];
                ui.add(egui::Label::new("one").selector(".source"));
                ui.add(egui::Label::new("two").selector(".next"));
                ui.response()
            })
            .selector(".container"),
        );
    })
    .await;
    let regions = bounds.lock().unwrap();
    for region in regions.iter() {
        let has_red_text = (region.left().floor() as i32..region.right().ceil() as i32).any(|x| {
            (region.top().floor() as i32..region.bottom().ceil() as i32).any(|y| {
                let color = pixel(&pixels, x as f32, y as f32);
                color[0] > 150 && color[1] < 100 && color[2] < 100
            })
        });
        assert!(has_red_text, "region received no text: {region:?}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn region_break_after_advances_following_content_test() {
    let stylesheet = Stylesheet::parse(
        ".first { display:block; width:50px; height:28px; flow-from:article; }\
         .second { display:block; width:50px; height:14px; flow-from:article; }\
         .source { flow-into:article; font-size:12px; color:#f00; }\
         .previous { flow-into:article; font-size:12px; color:#f00; break-after:region; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = vec![
                    ui.add(empty_response.selector(".first")).rect,
                    ui.add(empty_response.selector(".second")).rect,
                ];
                ui.add(egui::Label::new("one").selector(".previous"));
                ui.add(egui::Label::new("two").selector(".source"));
                ui.response()
            })
            .selector(".container"),
        );
    })
    .await;
    let regions = bounds.lock().unwrap();
    for region in regions.iter() {
        let has_red_text = (region.left().floor() as i32..region.right().ceil() as i32).any(|x| {
            (region.top().floor() as i32..region.bottom().ceil() as i32).any(|y| {
                let color = pixel(&pixels, x as f32, y as f32);
                color[0] > 150 && color[1] < 100 && color[2] < 100
            })
        });
        assert!(has_red_text, "region received no text: {region:?}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn compound_flow_elements_move_to_regions_in_document_order_test() {
    let stylesheet = Stylesheet::parse(
        ".first { display:block; width:50px; height:14px; flow-from:story; background-color:#00f; }\
         .second { display:block; width:50px; height:14px; flow-from:story; background-color:#00f; }\
         .source { display:block; width:50px; flow-into:story; }\
         .content-source { display:block; width:50px; flow-into:story content; }\
         .red { display:block; width:30px; height:14px; background-color:#f00; }\
         .green { display:block; width:30px; height:14px; background-color:#0f0; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                for (child, source) in [(".red", ".source"), (".green", ".content-source")] {
                    ui.add(
                        (|ui: &mut egui::Ui| {
                            ui.add(empty_response.selector(child));
                            ui.response()
                        })
                        .selector(source),
                    );
                }
                *capture.lock().unwrap() = vec![
                    ui.add(empty_response.selector(".first")).rect,
                    ui.add(empty_response.selector(".second")).rect,
                ];
                ui.response()
            })
            .selector(".container"),
        );
    })
    .await;
    let regions = bounds.lock().unwrap();
    assert_eq!(
        pixel(&pixels, regions[0].left() + 5.0, regions[0].top() + 5.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, regions[1].left() + 5.0, regions[1].top() + 5.0),
        [0, 255, 0, 255]
    );
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn region_overflow_clips_compound_flow_content_test() {
    let stylesheet = Stylesheet::parse(
        ".region { display:block; width:40px; height:14px; flow-from:story; overflow:hidden; }\
         .source { display:block; width:60px; flow-into:story; }\
         .child { display:block; width:60px; height:14px; background-color:#f00; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(
                    (|ui: &mut egui::Ui| {
                        ui.add(empty_response.selector(".child"));
                        ui.response()
                    })
                    .selector(".source"),
                );
                *capture.lock().unwrap() = Some(ui.add(empty_response.selector(".region")).rect);
                ui.response()
            })
            .selector(".container"),
        );
    })
    .await;
    let region = bounds.lock().unwrap().unwrap();
    assert_eq!(
        pixel(&pixels, region.left() + 35.0, region.top() + 5.0),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, region.right() + 5.0, region.top() + 5.0),
        [255; 4]
    );
}
