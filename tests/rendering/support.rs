#[cfg(not(target_arch = "wasm32"))]
use std::sync::{OnceLock, mpsc};

use ecss::Stylesheet;
use eframe::{egui, egui_wgpu, wgpu};

pub(super) const WIDTH: u32 = 256;
pub(super) const HEIGHT: u32 = 128;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[cfg(not(target_arch = "wasm32"))]
struct RenderRequest {
    stylesheet: Stylesheet,
    pixels_per_point: f32,
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

pub(super) fn empty_box(ui: &mut egui::Ui) -> egui::Response {
    ui.allocate_exact_size(egui::Vec2::ZERO, egui::Sense::hover())
        .1
}

pub(super) fn pixel(pixels: &[[u8; 4]], x: f32, y: f32) -> [u8; 4] {
    pixels[y.floor() as usize * WIDTH as usize + x.floor() as usize]
}

pub(super) fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.05,
        "expected {expected}, got {actual}"
    );
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn render_ui_pixels(
    stylesheet: Stylesheet,
    build_ui: impl FnMut(&mut egui::Ui) + Send + 'static,
) -> Vec<[u8; 4]> {
    render_ui_pixels_at_scale(stylesheet, 1.0, build_ui).await
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn render_ui_pixels_at_scale(
    stylesheet: Stylesheet,
    pixels_per_point: f32,
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
                        request.pixels_per_point,
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
            pixels_per_point,
            build_ui: Box::new(build_ui),
            pixels_tx,
        })
        .expect("the pixel-rendering thread exited before accepting a frame");
    pixels_rx
        .recv()
        .expect("the pixel-rendering thread exited before returning an image")
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn render_ui_pixels(
    stylesheet: Stylesheet,
    build_ui: impl FnMut(&mut egui::Ui) + Send + 'static,
) -> Vec<[u8; 4]> {
    render_ui_pixels_at_scale(stylesheet, 1.0, build_ui).await
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn render_ui_pixels_at_scale(
    stylesheet: Stylesheet,
    pixels_per_point: f32,
    build_ui: impl FnMut(&mut egui::Ui) + Send + 'static,
) -> Vec<[u8; 4]> {
    let mut renderer = OffscreenRenderer::new().await;
    render_ui_frame(
        &mut renderer,
        stylesheet,
        pixels_per_point,
        Box::new(build_ui),
    )
    .await
}

async fn render_ui_frame(
    renderer: &mut OffscreenRenderer,
    stylesheet: Stylesheet,
    pixels_per_point: f32,
    mut build_ui: Box<dyn FnMut(&mut egui::Ui) + Send>,
) -> Vec<[u8; 4]> {
    let context = egui::Context::default();
    stylesheet.install(&context);
    let viewport_id = egui::ViewportId::ROOT;
    let mut viewports = egui::ViewportIdMap::default();
    viewports.insert(
        viewport_id,
        egui::ViewportInfo {
            native_pixels_per_point: Some(pixels_per_point),
            ..Default::default()
        },
    );
    let mut output = context.run_ui(
        egui::RawInput {
            viewport_id,
            viewports,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(
                    WIDTH as f32 / pixels_per_point,
                    HEIGHT as f32 / pixels_per_point,
                ),
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
