use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::Duration,
};

use egui::{ColorImage, Context, Id, TextureHandle, TextureOptions};
use futures_util::future::join_all;

use crate::StylesheetLoadError;

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
        for (url, image) in urls.into_iter().zip(responses) {
            images.insert(url, Arc::new(image?));
        }
        self.insert(images);
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
        context.data_mut(|data| data.insert_temp(id, texture.clone()));
        Some(texture)
    }

    pub(crate) fn size(&self, url: &str) -> Option<[usize; 2]> {
        self.0.images.read().ok()?.get(url).map(|image| image.size)
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

    fn insert(&self, images: HashMap<String, Arc<ColorImage>>) {
        if images.is_empty() {
            return;
        }
        if let Ok(mut stored) = self.0.images.write() {
            stored.extend(images);
        }
    }
}

async fn load_image(url: &str) -> Result<ColorImage, StylesheetLoadError> {
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

fn decode_image(url: &str, bytes: &[u8]) -> Result<ColorImage, StylesheetLoadError> {
    let image = image::load_from_memory(bytes).map_err(|error| StylesheetLoadError::Image {
        url: url.to_owned(),
        message: error.to_string(),
    })?;
    let rgba = image.to_rgba8();
    let width = usize::try_from(rgba.width()).unwrap_or(usize::MAX);
    let height = usize::try_from(rgba.height()).unwrap_or(usize::MAX);
    Ok(ColorImage::from_rgba_unmultiplied(
        [width, height],
        rgba.as_raw(),
    ))
}
