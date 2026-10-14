use std::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
use std::sync::{OnceLock, mpsc};

use eframe::{egui, egui_wgpu, wgpu};
use trcss::{SelectorExt as _, Stylesheet};

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

rendering_test!(installed_white_background_renders_every_pixel_white);
rendering_test!(selector_background_color_is_visible_in_rendered_widget_pixels);
rendering_test!(asymmetric_box_model_is_visible_in_rendered_pixels);

#[cfg(not(target_arch = "wasm32"))]
async fn render_ui_pixels(
    stylesheet: Stylesheet,
    build_ui: impl FnMut(&mut egui::Ui) + Send + 'static,
) -> Vec<[u8; 4]> {
    let (pixels_tx, pixels_rx) = mpsc::channel();
    let requests = RENDERER_THREAD.get_or_init(|| {
        let (requests_tx, requests_rx) = mpsc::channel::<RenderRequest>();
        std::thread::Builder::new()
            .name("trcss-test-renderer".to_owned())
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
            label: Some("trcss test render target"),
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
            label: Some("trcss test pixel readback"),
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
                label: Some("trcss test render encoder"),
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
                    label: Some("trcss test render pass"),
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
