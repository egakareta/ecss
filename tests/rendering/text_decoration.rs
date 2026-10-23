use super::*;

fn blue(pixel: &[u8; 4]) -> bool {
    pixel[2] > 240 && pixel[0] < 20 && pixel[1] < 20
}

fn bands(values: &[usize]) -> Vec<std::ops::Range<usize>> {
    let mut result = Vec::new();
    let mut start = None;
    for (index, count) in values.iter().copied().chain([0]).enumerate() {
        if count > 0 {
            start.get_or_insert(index);
        } else if let Some(start) = start.take() {
            result.push(start..index);
        }
    }
    result
}

fn blue_rows(pixels: &[[u8; 4]]) -> Vec<usize> {
    pixels
        .chunks(WIDTH as usize)
        .map(|row| row.iter().filter(|pixel| blue(pixel)).count())
        .collect()
}

async fn decorated_label(declarations: &str) -> (Vec<[u8; 4]>, egui::Rect) {
    let stylesheet = Stylesheet::parse(&format!(
        ".text {{ font-size:32px; line-height:42px; color:red; {declarations} }}"
    ))
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(egui::Rect::NOTHING));
    let capture = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        ui.add_space(16.0);
        *capture.lock().unwrap() = ui
            .add(egui::Label::new("hello world").selector(".text"))
            .rect;
    })
    .await;
    let rect = *bounds.lock().unwrap();
    (pixels, rect)
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_decoration_shorthand_matches_longhands_without_changing_layout() {
    let (short, short_rect) =
        decorated_label("text-decoration:blue underline overline line-through solid 2px;").await;
    let (long, long_rect) = decorated_label("text-decoration-line:underline overline line-through; text-decoration-color:blue; text-decoration-style:solid; text-decoration-thickness:2px;").await;
    let (plain, plain_rect) = decorated_label("text-decoration:none;").await;
    assert_eq!(short_rect, plain_rect);
    assert_eq!(short_rect, long_rect);
    assert_eq!(short, long);
    assert!(!plain.iter().any(blue));
    let rows = blue_rows(&short);
    let lines = bands(&rows);
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines[0].end < short_rect.center().y as usize);
    assert!(lines[2].start > short_rect.center().y as usize);
    assert!(rows.iter().copied().max().unwrap() > 100);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_decoration_styles_and_thickness_produce_distinct_line_patterns() {
    let mut profiles = Vec::new();
    for style in ["solid", "double", "dashed", "dotted", "wavy"] {
        let (pixels, _) =
            decorated_label(&format!("text-decoration:underline blue {style} 3px;")).await;
        let rows = blue_rows(&pixels);
        assert!(rows.iter().sum::<usize>() > 5, "{style}");
        profiles.push(rows);
    }
    assert_eq!(bands(&profiles[0]).len(), 1);
    assert_eq!(bands(&profiles[1]).len(), 2);
    assert!(profiles[2].iter().max() < profiles[0].iter().max());
    assert!(profiles[3].iter().max() < profiles[0].iter().max());
    assert!(profiles[3].iter().sum::<usize>() < profiles[2].iter().sum::<usize>());
    assert_ne!(profiles[3], profiles[2]);
    assert!(bands(&profiles[4])[0].len() > bands(&profiles[0])[0].len());
    let (percentage, _) = decorated_label("text-decoration:underline blue 12.5%;").await;
    let (length, _) = decorated_label("text-decoration:underline blue calc(2px + 2px);").await;
    assert_eq!(percentage, length);
    assert!(blue_rows(&length).iter().sum::<usize>() > profiles[0].iter().sum::<usize>());
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_decoration_resets_restore_currentcolor_and_default_style() {
    let (expected, _) = decorated_label("color:blue; text-decoration:underline;").await;
    for declaration in [
        "color:blue; text-decoration:overline red wavy 4px; text-decoration:underline;",
        "color:blue; text-decoration:underline red; text-decoration-color:initial;",
        "color:blue; text-decoration:underline red; text-decoration-color:unset;",
        "color:blue; text-decoration:underline red; text-decoration-color:revert;",
    ] {
        let (pixels, _) = decorated_label(declaration).await;
        assert_eq!(pixels, expected, "{declaration}");
    }
    let (plain, _) = decorated_label("").await;
    for reset in ["none", "initial", "unset", "revert", "blink"] {
        let (pixels, _) = decorated_label(&format!(
            "text-decoration:underline blue; text-decoration:{reset};"
        ))
        .await;
        assert_eq!(pixels, plain, "{reset}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn ancestor_text_decorations_propagate_without_being_cancelled_or_recolored() {
    for (child, decorated) in [
        (
            "display:inline; text-decoration:none; text-decoration-color:lime;",
            true,
        ),
        ("display:block; text-decoration:none;", true),
        ("display:inline-block;", false),
        ("position:absolute;", false),
        ("float:left;", false),
        ("display:inline-block; text-decoration:inherit;", true),
        ("display:inline; text-decoration:overline lime;", true),
    ] {
        let stylesheet = Stylesheet::parse(&format!(
            ".parent {{ width:220px; font-size:24px; color:blue; text-decoration:underline 2px; }} .child {{ color:red; {child} }}"
        )).unwrap();
        let pixels = render_ui_pixels(stylesheet, |ui| {
            ui.add_space(16.0);
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(egui::Label::new("hello world").selector(".child"));
                    ui.response()
                })
                .selector(".parent"),
            );
        })
        .await;
        assert!(
            pixels
                .iter()
                .any(|pixel| pixel[0] > 200 && pixel[1] < 40 && pixel[2] < 40),
            "{child}"
        );
        assert_eq!(pixels.iter().any(blue), decorated, "{child}");
        if decorated {
            assert_eq!(bands(&blue_rows(&pixels)).len(), 1, "{child}");
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inline_text_decoration_spans_internal_word_spaces() {
    let stylesheet = Stylesheet::parse(
        ".flow { width:220px; } .text { display:inline; font-size:24px; color:red; text-decoration:underline blue 2px; }"
    ).unwrap();
    let pixels = render_ui_pixels(stylesheet, |ui| {
        ui.add_space(16.0);
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("hello world").selector(".text"));
                ui.response()
            })
            .selector(".flow"),
        );
    })
    .await;
    let rows = blue_rows(&pixels);
    let (y, count) = rows
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| *count)
        .unwrap();
    assert!(*count > 80);
    let coverage = pixels[y * WIDTH as usize..(y + 1) * WIDTH as usize]
        .iter()
        .map(|pixel| usize::from(pixel[2] > 190 && pixel[0] < 200 && pixel[1] < 200))
        .collect::<Vec<_>>();
    assert_eq!(bands(&coverage).len(), 1);
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn inherited_decoration_thickness_percentages_use_the_child_font_size() {
    async fn render(parent: &str, child: &str) -> Vec<[u8; 4]> {
        let stylesheet = Stylesheet::parse(&format!(
            ".parent {{ width:220px; font-size:16px; text-decoration-thickness:{parent}; }} .child {{ display:inline-block; font-size:32px; line-height:42px; color:red; text-decoration-line:underline; text-decoration-color:blue; text-decoration-thickness:{child}; }}"
        )).unwrap();
        render_ui_pixels(stylesheet, |ui| {
            ui.add_space(16.0);
            ui.add(
                (|ui: &mut egui::Ui| {
                    ui.add(egui::Label::new("hello world").selector(".child"));
                    ui.response()
                })
                .selector(".parent"),
            );
        })
        .await
    }
    for (parent, expected) in [("12.5%", "4px"), ("calc(1px + 12.5%)", "5px")] {
        let inherited = render(parent, "inherit").await;
        assert!(inherited.iter().any(blue));
        assert_eq!(inherited, render(parent, expected).await, "{parent}");
    }
}

#[cfg_attr(not(target_arch = "wasm32"), pollster::test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
async fn text_decorations_follow_wrapping_clipping_and_vertical_writing() {
    let stylesheet = Stylesheet::parse(
        ".flow { width:90px; } .text { display:inline; font-size:24px; color:red; text-decoration:underline blue 2px; }"
    ).unwrap();
    let pixels = render_ui_pixels(stylesheet, |ui| {
        ui.add_space(16.0);
        ui.add(
            (|ui: &mut egui::Ui| {
                ui.add(egui::Label::new("hello world hello world").selector(".text"));
                ui.response()
            })
            .selector(".flow"),
        );
    })
    .await;
    assert!(bands(&blue_rows(&pixels)).len() >= 2);

    let (pixels, rect) = decorated_label(
        "width:60px; white-space:nowrap; overflow:hidden; text-decoration:underline blue 2px;",
    )
    .await;
    assert!(pixels.iter().any(blue));
    assert!(pixels.chunks(WIDTH as usize).all(|row| {
        row.iter()
            .enumerate()
            .all(|(x, pixel)| !blue(pixel) || x < rect.right().ceil() as usize)
    }));

    let (pixels, _) = decorated_label(
        "width:42px; height:180px; writing-mode:vertical-rl; text-decoration:underline blue 2px;",
    )
    .await;
    let rows = blue_rows(&pixels);
    let mut columns = vec![0; WIDTH as usize];
    for (index, pixel) in pixels.iter().enumerate() {
        if blue(pixel) {
            columns[index % WIDTH as usize] += 1;
        }
    }
    assert!(rows.iter().filter(|count| **count > 0).count() > 50);
    assert!(columns.iter().filter(|count| **count > 0).count() < 10);
}

#[cfg_attr(not(target_arch = "wasm32"), test)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
fn text_decoration_value_grammar_is_validated() {
    for declaration in [
        "text-decoration:underline underline",
        "text-decoration:none underline",
        "text-decoration:underline none",
        "text-decoration:red blue",
        "text-decoration:solid wavy",
        "text-decoration:1px 2px",
        "text-decoration:underline -1px",
        "text-decoration:inherit underline",
        "text-decoration-line:underline red",
        "text-decoration-line:none none",
        "text-decoration-style:groove",
        "text-decoration-color:red blue",
        "text-decoration-thickness:-1%",
        "text-decoration-thickness:1",
    ] {
        assert!(
            Stylesheet::parse(&format!(".text {{ {declaration}; }}")).is_err(),
            "{declaration}"
        );
        assert!(InlineStyle::parse(declaration).is_err(), "{declaration}");
    }
    for declaration in [
        "text-decoration:UNDERLINE OVERLINE LINE-THROUGH BLINK DOUBLE #00f 0.1em",
        "text-decoration:rgb(0, 0, 255) dotted underline",
        "text-decoration-color:currentcolor",
        "text-decoration-thickness:from-font",
        "text-decoration-thickness:calc(1px + 2%)",
        "text-decoration-style:inherit",
        "text-decoration-line:initial",
    ] {
        assert!(
            Stylesheet::parse(&format!(".text {{ {declaration}; }}")).is_ok(),
            "{declaration}"
        );
        assert!(InlineStyle::parse(declaration).is_ok(), "{declaration}");
    }
}
