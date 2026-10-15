async fn inline_text_continues_across_selectors_and_paints_line_fragments_test() {
    let stylesheet=Stylesheet::parse(".flow {width:80px;} .one {display:inline; background-color:red; color:white;} .two {display:inline; background-color:blue; color:white;} .probe {width:max-content;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut a = None;
        let mut b = None;
        ui.add(
            (|ui: &mut egui::Ui| {
                a = Some(
                    ui.add(egui::Label::new("aa bb ccccccc ").selector(".one"))
                        .rect,
                );
                b = Some(ui.add(egui::Label::new("dd").selector(".two")).rect);
                ui.response()
            })
            .selector(".flow"),
        );
        let reference = ui.add(egui::Label::new("aa bb").selector(".probe")).rect;
        *capture.lock().unwrap() = Some((a.unwrap(), b.unwrap(), reference));
    })
    .await;
    let (a, b, reference) = bounds.lock().unwrap().unwrap();
    assert!(a.height() > reference.height(), "{a:?} {b:?}");
    assert!(b.top() > a.top() && b.top() < a.bottom(), "{a:?} {b:?}");
    assert_eq!(
        pixel(&pixels, a.left() + 0.5, a.top() + 0.5),
        [255, 0, 0, 255]
    );
    assert_eq!(
        pixel(&pixels, b.left() + 0.5, b.top() + 0.5),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, a.left() + reference.width() + 1.5, a.top() + 0.5),
        [255; 4]
    );
}

async fn vertical_text_positions_lines_and_honors_text_orientation_test() {
    let stylesheet = Stylesheet::parse(".row {display:flex; width:200px; gap:10px;} .rl {writing-mode:vertical-rl; white-space:pre; width:32px; height:60px; color:red;} .lr {writing-mode:vertical-lr; white-space:pre; width:32px; height:60px; color:red;} .upright {writing-mode:vertical-lr; text-orientation:upright; width:20px; height:20px; color:red;} .sideways {writing-mode:vertical-lr; text-orientation:sideways; width:20px; height:20px; color:red;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut job = egui::text::LayoutJob::default();
        for (text, color) in [("W\n", egui::Color32::RED), ("I", egui::Color32::BLUE)] {
            job.append(
                text,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(14.0),
                    color,
                    ..Default::default()
                },
            );
        }
        ui.add(
            (|ui: &mut egui::Ui| {
                *capture.lock().unwrap() = vec![
                    ui.add(egui::Label::new(job.clone()).selector(".rl")).rect,
                    ui.add(egui::Label::new(job).selector(".lr")).rect,
                    ui.add(egui::Label::new("L").selector(".upright")).rect,
                    ui.add(egui::Label::new("L").selector(".sideways")).rect,
                ];
                ui.response()
            })
            .selector(".row"),
        );
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let ink = |rect: egui::Rect, red: bool| {
        let mut count = 0;
        let mut area = egui::Rect::NOTHING;
        for y in rect.top() as u32..rect.bottom() as u32 {
            for x in rect.left() as u32..rect.right() as u32 {
                let p = pixel(&pixels, x as f32, y as f32);
                if p[1] < 128
                    && if red {
                        p[0] > 128 && p[2] < 128
                    } else {
                        p[2] > 128 && p[0] < 128
                    }
                {
                    count += 1;
                    area = area.union(egui::Rect::from_min_size(
                        egui::pos2(x as f32, y as f32),
                        egui::Vec2::splat(1.0),
                    ));
                }
            }
        }
        (count, area)
    };
    for (index, rect) in bounds[..2].iter().enumerate() {
        let red = ink(*rect, true).1;
        let blue = ink(*rect, false).1;
        assert!(
            red.is_positive() && blue.is_positive(),
            "{index}: {red:?} {blue:?}"
        );
        assert!(
            if index == 0 {
                red.center().x > blue.center().x
            } else {
                red.center().x < blue.center().x
            },
            "{index}: {red:?} {blue:?}"
        );
    }
    let upright = ink(bounds[2], true).1;
    let sideways = ink(bounds[3], true).1;
    assert!(
        upright.is_positive() && upright.height() > upright.width(),
        "{upright:?}"
    );
    assert!(
        sideways.is_positive() && sideways.width() > sideways.height(),
        "{sideways:?}"
    );
}

async fn inline_padding_and_borders_paint_around_text_without_expanding_line_height_test() {
    let stylesheet = Stylesheet::parse(".flow {width:100px; margin-top:16px;} .text {display:inline; padding:4px 8px; border:2px solid blue; margin-left:3px; background-color:red; color:white;} .next {display:inline-block; width:10px; height:10px; background-color:green;} .probe {width:max-content;}").unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(Vec::new()));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let mut boxes = Vec::new();
        let flow = ui
            .add(
                (|ui: &mut egui::Ui| {
                    boxes.push(ui.add(egui::Label::new("X").selector(".text")).rect);
                    boxes.push(ui.add(empty_box.selector(".next")).rect);
                    ui.response()
                })
                .selector(".flow"),
            )
            .rect;
        boxes.push(flow);
        boxes.push(ui.add(egui::Label::new("X").selector(".probe")).rect);
        *capture.lock().unwrap() = boxes;
    })
    .await;
    let bounds = bounds.lock().unwrap();
    let text = bounds[0];
    assert!((text.width() - bounds[3].width() - 20.0).abs() < 0.5);
    assert!((text.height() - bounds[3].height() - 12.0).abs() < 0.5);
    near(text.left(), 3.0);
    near(bounds[1].left(), text.right());
    assert!((bounds[2].height() - bounds[3].height()).abs() < 0.5);
    assert_eq!(
        pixel(&pixels, text.left() + 0.5, text.center().y),
        [0, 0, 255, 255]
    );
    assert_eq!(
        pixel(&pixels, text.left() + 3.0, text.center().y),
        [255, 0, 0, 255]
    );
}
