use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{OnceLock, mpsc};

use ecss::{SelectorExt as _, Stylesheet};
use eframe::{egui, egui_wgpu, wgpu};

const WIDTH: u32 = 256;
const HEIGHT: u32 = 128;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[cfg(not(target_arch = "wasm32"))]
struct RenderRequest {
    stylesheet: Stylesheet,
    build_ui: Box<dyn FnMut(&mut egui::Ui) + Send>,
    pixels_tx: mpsc::Sender<Vec<[u8; 4]>>,
}

struct OffscreenRenderer {
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: egui_wgpu::Renderer,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    readback: wgpu::Buffer,
}

#[cfg(not(target_arch = "wasm32"))]
// Keep graphics work on one long-lived thread because native drivers may keep
// thread-local state that is unsafe to tear down with each test thread.
static RENDERER_THREAD: OnceLock<mpsc::Sender<RenderRequest>> = OnceLock::new();

#[cfg(target_arch = "wasm32")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

async fn installed_white_background_renders_every_pixel_white_test() {
    let pixels = render_ui_pixels(Stylesheet::default(), |_ui| {}).await;
    assert_eq!(pixels.len(), (WIDTH * HEIGHT) as usize);
    let first_non_white = pixels
        .iter()
        .enumerate()
        .find(|(_, pixel)| **pixel != [255; 4]);
    if let Some((index, pixel)) = first_non_white {
        panic!(
            "pixel ({}, {}) was {pixel:?}, expected opaque white",
            index % WIDTH as usize,
            index / WIDTH as usize,
        );
    }
}

async fn selector_background_color_is_visible_in_rendered_widget_pixels_test() {
    let stylesheet = Stylesheet::parse(".primary { background-color: #f00; }").unwrap();
    let widget_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = widget_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(egui::Button::new("X").selector(".primary"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = widget_rect.lock().unwrap().unwrap();
    let x = (rect.min.x + 4.0).floor() as usize;
    let y = (rect.min.y + 3.0).floor() as usize;
    assert_eq!(pixels[y * WIDTH as usize + x], [255, 0, 0, 255]);
}

async fn button_frame_stroke_tracks_the_css_padding_box_test() {
    let stylesheet = Stylesheet::parse(
        ".primary { background-color: #f00; padding: 8px 12px; border-radius: 6px; }",
    )
    .unwrap();
    let button_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = button_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let response = ui.add(egui::Button::new("X").selector(".primary"));
        *rect_from_ui.lock().unwrap() = Some(response.rect);
    })
    .await;

    let rect = button_rect.lock().unwrap().unwrap();
    let pixel_at = |x: f32, y: f32| {
        let x = x.floor() as usize;
        let y = y.floor() as usize;
        pixels[y * WIDTH as usize + x]
    };

    assert_eq!(
        pixel_at(rect.min.x + 0.5, rect.center().y),
        [118, 118, 118, 255]
    );
    assert_eq!(
        pixel_at(rect.min.x + 12.5, rect.center().y),
        [255, 0, 0, 255]
    );
}

async fn asymmetric_box_model_is_visible_in_rendered_pixels_test() {
    let stylesheet = Stylesheet::parse(
        ".box { padding: 2px 6px 4px 10px; margin: 4px 3px 5px 7px; background-color: #f00; }",
    )
    .unwrap();
    let content_rect = std::sync::Arc::new(Mutex::new(None));
    let rect_from_ui = content_rect.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let rect_from_widget = rect_from_ui.clone();
        let content = move |ui: &mut egui::Ui| {
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(12.0, 8.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 0.0, egui::Color32::GREEN);
            *rect_from_widget.lock().unwrap() = Some(rect);
            response
        };
        ui.add(content.selector(".box"));
    })
    .await;

    let rect = content_rect.lock().unwrap().unwrap();
    let pixel_at = |x: f32, y: f32| {
        let x = x.floor() as usize;
        let y = y.floor() as usize;
        pixels[y * WIDTH as usize + x]
    };
    let red = [255, 0, 0, 255];
    let green = [0, 255, 0, 255];
    let white = [255, 255, 255, 255];

    assert_eq!(pixel_at(rect.center().x, rect.center().y), green);
    assert_eq!(pixel_at(rect.min.x - 5.5, rect.center().y), red);
    assert_eq!(pixel_at(rect.max.x + 2.5, rect.center().y), red);
    assert_eq!(pixel_at(rect.center().x, rect.min.y - 1.5), red);
    assert_eq!(pixel_at(rect.center().x, rect.max.y + 2.5), red);

    assert_eq!(pixel_at(rect.min.x - 10.0 - 3.5, rect.center().y), white);
    assert_eq!(pixel_at(rect.max.x + 6.0 + 1.5, rect.center().y), white);
    assert_eq!(pixel_at(rect.center().x, rect.min.y - 2.0 - 2.5), white);
    assert_eq!(pixel_at(rect.center().x, rect.max.y + 4.0 + 2.5), white);
}

async fn percentage_and_auto_sizes_affect_styled_widget_bounds_test() {
    let stylesheet = Stylesheet::parse(
        ".percentage { width: 50%; height: 50%; background-color: #f00; }\
         .automatic { width: auto; height: auto; }\
         .fixed { width: 24px; height: 16px; }",
    )
    .unwrap();
    let bounds = std::sync::Arc::new(Mutex::new(None));
    let bounds_from_ui = bounds.clone();
    let pixels = render_ui_pixels(stylesheet, move |ui| {
        let available_rect = ui.available_rect_before_wrap();
        let inner_rects = std::sync::Arc::new(Mutex::new(Vec::new()));
        let make_box = || {
            let inner_rects = inner_rects.clone();
            move |ui: &mut egui::Ui| {
                let (rect, response) =
                    ui.allocate_exact_size(egui::vec2(12.0, 8.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, egui::Color32::GREEN);
                inner_rects.lock().unwrap().push(rect);
                response
            }
        };

        let percentage = ui.add(make_box().selector(".percentage"));
        let automatic = ui.add(make_box().selector(".automatic"));
        let fixed = ui.add(make_box().selector(".fixed"));
        *bounds_from_ui.lock().unwrap() = Some((
            available_rect,
            percentage.rect,
            automatic.rect,
            fixed.rect,
            inner_rects.lock().unwrap().clone(),
        ));
    })
    .await;

    let (available_rect, percentage, automatic, fixed, inner_rects) =
        bounds.lock().unwrap().take().unwrap();
    assert_eq!(percentage.width(), available_rect.width() * 0.5);
    assert_eq!(percentage.height(), available_rect.height() * 0.5);
    assert_eq!(automatic.size(), egui::vec2(12.0, 8.0));
    assert_eq!(fixed.size(), egui::vec2(24.0, 16.0));

    let pixel_at =
        |x: f32, y: f32| pixels[y.floor() as usize * WIDTH as usize + x.floor() as usize];
    assert_eq!(
        pixel_at(inner_rects[0].center().x, inner_rects[0].center().y),
        [0, 255, 0, 255]
    );
    assert_eq!(
        pixel_at(percentage.max.x - 4.0, percentage.max.y - 4.0),
        [255, 0, 0, 255]
    );
}

macro_rules! rendering_test {
    ($name:ident) => {
        paste::paste! {
            #[cfg(not(target_arch = "wasm32"))]
            #[test]
            fn $name() {
                pollster::block_on([<$name _test>]());
            }

            #[cfg(target_arch = "wasm32")]
            #[wasm_bindgen_test::wasm_bindgen_test]
            async fn $name() {
                [<$name _test>]().await;
            }
        }
    };
}

fn empty_box(ui: &mut egui::Ui) -> egui::Response {
    ui.allocate_exact_size(egui::Vec2::ZERO, egui::Sense::hover())
        .1
}

fn pixel(pixels: &[[u8; 4]], x: f32, y: f32) -> [u8; 4] {
    pixels[y.floor() as usize * WIDTH as usize + x.floor() as usize]
}

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.05,
        "expected {expected}, got {actual}"
    );
}

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

rendering_test!(content_and_border_box_sizes_include_the_correct_edges);
rendering_test!(nested_selectors_distribute_flex_space_and_gap);
rendering_test!(grid_tracks_spans_and_gaps_control_selector_boxes);
rendering_test!(percentages_auto_margins_and_min_max_use_the_containing_block);
rendering_test!(absolute_position_skips_static_ancestors_and_does_not_consume_flow);
rendering_test!(overflow_and_display_none_clip_paint_and_remove_layout_space);

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

rendering_test!(flow_root_keeps_visible_overflow);

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

rendering_test!(named_grid_areas_position_children_independently_of_source_order);
rendering_test!(floats_and_clear_position_selector_boxes);
rendering_test!(flex_order_wrap_and_contents_use_only_selector_composition);

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

rendering_test!(visible_descendants_override_hidden_ancestors_without_removing_layout);

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

rendering_test!(stacking_contexts_contain_descendant_z_indices);

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

rendering_test!(logical_border_pairs_resolve_against_direction_and_affect_box_size);

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

rendering_test!(inline_padding_and_borders_paint_around_text_without_expanding_line_height);

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

rendering_test!(subgrid_inherits_tracks_gaps_and_named_lines_on_both_axes);

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

rendering_test!(table_cells_align_native_text_within_the_row_height);

rendering_test!(vertical_text_positions_lines_and_honors_text_orientation);
rendering_test!(column_rules_and_cloned_fragment_borders_render_their_styles);
rendering_test!(empty_table_cells_hide_borders_and_backgrounds_without_changing_tracks);

rendering_test!(inline_text_continues_across_selectors_and_paints_line_fragments);
rendering_test!(column_fragments_paint_in_destination_columns_without_painting_gutters);
rendering_test!(column_spanners_create_full_width_boxes_between_column_sets);
rendering_test!(collapsed_table_borders_choose_one_shared_edge);
rendering_test!(vertical_writing_paints_boxes_in_the_declared_block_direction);

rendering_test!(installed_white_background_renders_every_pixel_white);
rendering_test!(selector_background_color_is_visible_in_rendered_widget_pixels);
rendering_test!(button_frame_stroke_tracks_the_css_padding_box);
rendering_test!(asymmetric_box_model_is_visible_in_rendered_pixels);
rendering_test!(percentage_and_auto_sizes_affect_styled_widget_bounds);

#[cfg(not(target_arch = "wasm32"))]
async fn render_ui_pixels(
    stylesheet: Stylesheet,
    build_ui: impl FnMut(&mut egui::Ui) + Send + 'static,
) -> Vec<[u8; 4]> {
    let (pixels_tx, pixels_rx) = mpsc::channel();
    let requests = RENDERER_THREAD.get_or_init(|| {
        let (requests_tx, requests_rx) = mpsc::channel::<RenderRequest>();
        std::thread::Builder::new()
            .name("ecss-test-renderer".to_owned())
            .spawn(move || {
                let mut renderer = pollster::block_on(OffscreenRenderer::new());
                while let Ok(request) = requests_rx.recv() {
                    let pixels = pollster::block_on(render_ui_frame(
                        &mut renderer,
                        request.stylesheet,
                        request.build_ui,
                    ));
                    let _ = request.pixels_tx.send(pixels);
                }
            })
            .expect("could not start the pixel-rendering thread");
        requests_tx
    });
    requests
        .send(RenderRequest {
            stylesheet,
            build_ui: Box::new(build_ui),
            pixels_tx,
        })
        .expect("the pixel-rendering thread exited before accepting a frame");
    pixels_rx
        .recv()
        .expect("the pixel-rendering thread exited before returning an image")
}

#[cfg(target_arch = "wasm32")]
async fn render_ui_pixels(
    stylesheet: Stylesheet,
    build_ui: impl FnMut(&mut egui::Ui) + Send + 'static,
) -> Vec<[u8; 4]> {
    let mut renderer = OffscreenRenderer::new().await;
    render_ui_frame(&mut renderer, stylesheet, Box::new(build_ui)).await
}

async fn render_ui_frame(
    renderer: &mut OffscreenRenderer,
    stylesheet: Stylesheet,
    mut build_ui: Box<dyn FnMut(&mut egui::Ui) + Send>,
) -> Vec<[u8; 4]> {
    let context = egui::Context::default();
    stylesheet.install(&context);
    let mut output = context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(WIDTH as f32, HEIGHT as f32),
            )),
            ..Default::default()
        },
        &mut build_ui,
    );
    renderer.render(&context, &mut output).await
}

impl OffscreenRenderer {
    async fn new() -> Self {
        let descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::new(descriptor);
        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::util::new_instance_with_webgpu_detection(descriptor).await;
        let adapter_options = wgpu::RequestAdapterOptions {
            force_fallback_adapter: true,
            ..Default::default()
        };
        let adapter = match instance.request_adapter(&adapter_options).await {
            Ok(adapter) => adapter,
            Err(_) => instance
                .request_adapter(&Default::default())
                .await
                .expect("an available wgpu adapter is required for the pixel-rendering test"),
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("could not create a wgpu device for the pixel-rendering test");
        let renderer =
            egui_wgpu::Renderer::new(&device, FORMAT, egui_wgpu::RendererOptions::PREDICTABLE);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ecss test render target"),
            size: wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ecss test pixel readback"),
            size: u64::from(WIDTH * HEIGHT * 4),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            renderer,
            texture,
            texture_view,
            readback,
        }
    }

    async fn render(
        &mut self,
        context: &egui::Context,
        output: &mut egui::FullOutput,
    ) -> Vec<[u8; 4]> {
        for (id, deltas) in &output.textures_delta.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&self.device, &self.queue, *id, delta);
            }
        }
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [WIDTH, HEIGHT],
            pixels_per_point: output.pixels_per_point,
        };
        let paint_jobs =
            context.tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ecss test render encoder"),
            });
        let user_buffers = self.renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen,
        );
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ecss test render pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            // A non-white clear color makes the test fail if the stylesheet's
                            // begin-pass background paint is absent or does not cover the screen.
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    ..Default::default()
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, &paint_jobs, &screen);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(WIDTH * 4),
                    rows_per_image: Some(HEIGHT),
                },
            },
            wgpu::Extent3d {
                width: WIDTH,
                height: HEIGHT,
                depth_or_array_layers: 1,
            },
        );
        self.queue
            .submit(user_buffers.into_iter().chain([encoder.finish()]));

        let slice = self.readback.slice(..);
        let (mapped_tx, mapped_rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = mapped_tx.send(result);
        });
        #[cfg(not(target_arch = "wasm32"))]
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("wgpu failed while rendering the test frame");
        mapped_rx
            .await
            .expect("wgpu did not complete the pixel readback")
            .expect("wgpu could not map the rendered pixels");

        let mapped = self
            .readback
            .get_mapped_range(..)
            .expect("could not access the rendered pixel buffer");
        let (pixels, remainder) = mapped.as_chunks::<4>();
        assert!(remainder.is_empty());
        let pixels = pixels.to_vec();
        drop(mapped);
        self.readback.unmap();
        output.textures_delta.clear();
        pixels
    }
}
