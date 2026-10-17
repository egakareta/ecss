use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::Duration,
};

use egui::{ColorImage, Context, Id, TextureHandle, TextureId, TextureOptions, Vec2};
use futures_util::future::join_all;

use crate::StylesheetLoadError;

const TEXTURE_IMAGES: &str = "ecss.texture-images";

pub(crate) fn remember_texture_image(context: &Context, id: TextureId, image: ColorImage) {
    context.data_mut(|data| {
        let key = Id::new(TEXTURE_IMAGES);
        let mut images = data
            .get_temp::<HashMap<TextureId, Arc<ColorImage>>>(key)
            .unwrap_or_default();
        images.insert(id, Arc::new(image));
        data.insert_temp(key, images);
    });
}

pub(crate) fn texture_image(context: &Context, id: TextureId) -> Option<Arc<ColorImage>> {
    context.data(|data| {
        data.get_temp::<HashMap<TextureId, Arc<ColorImage>>>(Id::new(TEXTURE_IMAGES))
            .and_then(|images| images.get(&id).cloned())
    })
}

#[derive(Clone, Default)]
pub(crate) struct ImageAssets(Arc<ImageAssetsInner>);

impl std::fmt::Debug for ImageAssets {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ImageAssets")
            .field(
                "loaded_images",
                &self.0.images.read().map(|images| images.len()).unwrap_or(0),
            )
            .finish()
    }
}

#[derive(Default)]
struct ImageAssetsInner {
    images: RwLock<HashMap<String, Arc<ColorImage>>>,
    svg_sources: RwLock<HashMap<String, Arc<[u8]>>>,
}

struct LoadedImage {
    image: ColorImage,
    svg_source: Option<Arc<[u8]>>,
}

impl ImageAssets {
    pub(crate) async fn load(&self, urls: &[String]) -> Result<(), StylesheetLoadError> {
        if let Some(url) = urls.iter().find(|url| {
            !(url.starts_with("https://")
                || url.starts_with("http://")
                || url.starts_with("data:image/"))
        }) {
            return Err(StylesheetLoadError::Image {
                url: url.clone(),
                message: "only HTTP(S) and base64 image data URLs are supported".to_owned(),
            });
        }
        let urls = urls
            .iter()
            .cloned()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let responses = join_all(urls.iter().map(|url| load_image(url))).await;
        let mut images = HashMap::new();
        let mut svg_sources = HashMap::new();
        for (url, image) in urls.into_iter().zip(responses) {
            let image = image?;
            images.insert(url.clone(), Arc::new(image.image));
            if let Some(svg_source) = image.svg_source {
                svg_sources.insert(url, svg_source);
            }
        }
        self.insert(images, svg_sources);
        Ok(())
    }

    pub(crate) fn texture(&self, context: &Context, url: &str) -> Option<TextureHandle> {
        let id = Id::new(("ecss.background-image", url));
        if let Some(texture) = context.data(|data| data.get_temp::<TextureHandle>(id)) {
            return Some(texture);
        }
        let image = self.0.images.read().ok()?.get(url)?.clone();
        let texture =
            context.load_texture(url.to_owned(), (*image).clone(), TextureOptions::LINEAR);
        remember_texture_image(context, texture.id(), (*image).clone());
        context.data_mut(|data| data.insert_temp(id, texture.clone()));
        Some(texture)
    }

    pub(crate) fn size(&self, url: &str) -> Option<[usize; 2]> {
        self.0.images.read().ok()?.get(url).map(|image| image.size)
    }

    pub(crate) fn border_image_texture(
        &self,
        context: &Context,
        url: &str,
        default_size: Vec2,
    ) -> Option<(TextureHandle, [usize; 2])> {
        let svg_source = self.0.svg_sources.read().ok()?.get(url).cloned();
        let Some(svg_source) = svg_source else {
            let texture = self.texture(context, url)?;
            return Some((texture.clone(), texture.size()));
        };

        let id = Id::new((
            "ecss.border-image.svg",
            url,
            default_size.x.to_bits(),
            default_size.y.to_bits(),
        ));
        if let Some(texture) = context.data(|data| data.get_temp::<TextureHandle>(id)) {
            return Some((texture.clone(), texture.size()));
        }
        let image = rasterize_svg(&svg_source, default_size).ok()?;
        let size = image.size;
        let texture = context.load_texture(
            format!("{url}#ecss-border-image-{}x{}", size[0], size[1]),
            image.clone(),
            TextureOptions::LINEAR,
        );
        remember_texture_image(context, texture.id(), image);
        context.data_mut(|data| data.insert_temp(id, texture.clone()));
        Some((texture, size))
    }

    pub(crate) fn image(&self, url: &str) -> Option<Arc<ColorImage>> {
        self.0.images.read().ok()?.get(url).cloned()
    }

    pub(crate) fn contains_all(&self, urls: &[String]) -> bool {
        self.0
            .images
            .read()
            .is_ok_and(|images| urls.iter().all(|url| images.contains_key(url)))
    }

    pub(crate) fn mark_load_started(&self, context: &Context) -> bool {
        let id = Id::new(("ecss.background-load", Arc::as_ptr(&self.0) as usize));
        if context
            .data(|data| data.get_temp::<bool>(id))
            .unwrap_or(false)
        {
            return false;
        }
        context.data_mut(|data| data.insert_temp(id, true));
        true
    }

    fn insert(
        &self,
        images: HashMap<String, Arc<ColorImage>>,
        svg_sources: HashMap<String, Arc<[u8]>>,
    ) {
        if images.is_empty() && svg_sources.is_empty() {
            return;
        }
        if let Ok(mut stored) = self.0.images.write() {
            stored.extend(images);
        }
        if let Ok(mut stored) = self.0.svg_sources.write() {
            stored.extend(svg_sources);
        }
    }
}

async fn load_image(url: &str) -> Result<LoadedImage, StylesheetLoadError> {
    if let Some(data) = url.strip_prefix("data:") {
        let (metadata, encoded) =
            data.split_once(',')
                .ok_or_else(|| StylesheetLoadError::Image {
                    url: url.to_owned(),
                    message: "invalid image data URL".to_owned(),
                })?;
        if !metadata.to_ascii_lowercase().ends_with(";base64") {
            return Err(StylesheetLoadError::Image {
                url: url.to_owned(),
                message: "image data URLs must use base64 encoding".to_owned(),
            });
        }
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .map_err(|error| StylesheetLoadError::Image {
                url: url.to_owned(),
                message: error.to_string(),
            })?;
        return decode_image(url, &bytes);
    }

    let response =
        ehttp::fetch_async(ehttp::Request::get(url).with_timeout(Some(Duration::from_secs(30))))
            .await
            .map_err(|error| StylesheetLoadError::Image {
                url: url.to_owned(),
                message: error.to_string(),
            })?;
    if !response.ok {
        return Err(StylesheetLoadError::Image {
            url: url.to_owned(),
            message: format!("server returned HTTP status {}", response.status),
        });
    }
    decode_image(url, &response.bytes)
}

fn decode_image(url: &str, bytes: &[u8]) -> Result<LoadedImage, StylesheetLoadError> {
    if let Ok(image) = image::load_from_memory(bytes) {
        let rgba = image.to_rgba8();
        let width = usize::try_from(rgba.width()).unwrap_or(usize::MAX);
        let height = usize::try_from(rgba.height()).unwrap_or(usize::MAX);
        return Ok(LoadedImage {
            image: ColorImage::from_rgba_unmultiplied([width, height], rgba.as_raw()),
            svg_source: None,
        });
    }

    let svg_source: Arc<[u8]> = Arc::from(bytes);
    let image = rasterize_svg(&svg_source, Vec2::splat(100.0)).map_err(|message| {
        StylesheetLoadError::Image {
            url: url.to_owned(),
            message,
        }
    })?;
    Ok(LoadedImage {
        image,
        svg_source: Some(svg_source),
    })
}

fn rasterize_svg(bytes: &[u8], default_size: Vec2) -> Result<ColorImage, String> {
    let default_size = resvg::usvg::Size::from_wh(default_size.x, default_size.y)
        .ok_or_else(|| "SVG default dimensions must be positive".to_owned())?;
    let mut options = resvg::usvg::Options {
        default_size,
        ..Default::default()
    };
    options.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_data(bytes, &options).map_err(|error| error.to_string())?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| "SVG dimensions are invalid or too large".to_owned())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect::<Vec<_>>();
    Ok(ColorImage::from_rgba_unmultiplied(
        [size.width() as usize, size.height() as usize],
        &rgba,
    ))
}
