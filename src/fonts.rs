use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use egui::{FontData, FontFamily, FontTweak, Id};
use futures_util::future::join_all;

use crate::StylesheetLoadError;

#[derive(Clone, Debug, Default)]
pub(crate) struct FontAssets {
    variants: Vec<LoadedFont>,
}

#[derive(Clone, Debug)]
pub(crate) struct FontRequest {
    pub(crate) families: Vec<String>,
    pub(crate) weight: u16,
    pub(crate) italic: bool,
}

#[derive(Clone, Debug)]
struct LoadedFont {
    family: String,
    weight: u16,
    italic: bool,
    faces: Vec<FontPart>,
}

#[derive(Clone, Debug)]
struct FontPart {
    bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
struct FontFace {
    family: String,
    min_weight: u16,
    max_weight: u16,
    italic: bool,
    url: String,
}

const FONT_ASSETS_ID: &str = "trcss.font-assets";
const FONT_REVISION_ID: &str = "trcss.font-revision";
const FONT_CALLBACK_REGISTERED_ID: &str = "trcss.font-callback-registered";
const FONT_INSTALL_CALLBACK_ID: &str = "trcss.font-install";

impl FontAssets {
    #[cfg(test)]
    pub(crate) fn with_test_font(family: &str, weight: u16, bytes: Vec<u8>) -> Self {
        Self {
            variants: vec![LoadedFont {
                family: family.to_owned(),
                weight,
                italic: false,
                faces: vec![FontPart { bytes }],
            }],
        }
    }

    pub(crate) async fn load(
        import_urls: &[String],
        requests: &[FontRequest],
    ) -> Result<Self, StylesheetLoadError> {
        if import_urls.is_empty() || requests.is_empty() {
            return Ok(Self::default());
        }

        let import_responses = join_all(import_urls.iter().map(|url| fetch(url.clone()))).await;
        let mut font_faces = Vec::new();
        for (url, response) in import_urls.iter().zip(import_responses) {
            let response = response?;
            let css = std::str::from_utf8(&response.bytes).map_err(|error| {
                StylesheetLoadError::Font {
                    url: url.clone(),
                    message: format!("font stylesheet was not UTF-8: {error}"),
                }
            })?;
            font_faces.extend(parse_font_faces(css, url));
        }

        let mut resolved = Vec::new();
        let mut seen_variants = HashSet::new();
        let mut urls = Vec::new();
        let mut seen_urls = HashSet::new();

        for request in requests {
            let Some((family, selected_faces)) = resolve_request(request, &font_faces) else {
                continue;
            };
            let variant_key = (normalize_family(&family), request.weight, request.italic);
            if !seen_variants.insert(variant_key) {
                continue;
            }

            for face in &selected_faces {
                if seen_urls.insert(face.url.clone()) {
                    urls.push(face.url.clone());
                }
            }
            resolved.push((family, request.weight, request.italic, selected_faces));
        }

        let responses = join_all(urls.iter().map(|url| fetch(url.clone()))).await;
        let mut decoded_fonts = HashMap::new();
        for (url, response) in urls.iter().zip(responses) {
            let response = response?;
            decoded_fonts.insert(url.clone(), decode_font(url, response.bytes)?);
        }

        let variants = resolved
            .into_iter()
            .map(|(family, weight, italic, faces)| LoadedFont {
                family,
                weight,
                italic,
                faces: faces
                    .into_iter()
                    .filter_map(|face| {
                        decoded_fonts.get(&face.url).map(|bytes| FontPart {
                            bytes: bytes.clone(),
                        })
                    })
                    .collect(),
            })
            .filter(|font| !font.faces.is_empty())
            .collect();

        Ok(Self { variants })
    }

    pub(crate) fn family(
        &self,
        context: &egui::Context,
        candidates: &[String],
        weight: u16,
        italic: bool,
    ) -> Option<FontFamily> {
        for candidate in candidates {
            if let Some(variant) = self.variants.iter().find(|variant| {
                normalize_family(&variant.family) == normalize_family(candidate)
                    && variant.weight == weight
                    && variant.italic == italic
            }) {
                let family = FontFamily::Name(font_family_id(variant).into());
                if context.fonts(|fonts| fonts.definitions().families.contains_key(&family)) {
                    return Some(family);
                }
            }
        }
        None
    }

    pub(crate) fn install(&self, context: &egui::Context) {
        if self.variants.is_empty() {
            return;
        }

        let should_register = context.data_mut(|data| {
            let revision = data
                .get_temp::<u64>(Id::new(FONT_REVISION_ID))
                .unwrap_or_default()
                .wrapping_add(1);
            data.insert_temp(Id::new(FONT_REVISION_ID), revision);
            data.insert_temp(Id::new(FONT_ASSETS_ID), Arc::new(self.clone()));
            let callback_registered = data
                .get_temp::<bool>(Id::new(FONT_CALLBACK_REGISTERED_ID))
                .unwrap_or(false);
            if !callback_registered {
                data.insert_temp(Id::new(FONT_CALLBACK_REGISTERED_ID), true);
            }
            !callback_registered
        });

        if !should_register {
            return;
        }
        let installed_revision = Arc::new(Mutex::new(0_u64));
        context.on_begin_pass(
            FONT_INSTALL_CALLBACK_ID,
            Arc::new(move |ui| {
                let context = ui.ctx();
                let (revision, assets) = context.data(|data| {
                    (
                        data.get_temp::<u64>(Id::new(FONT_REVISION_ID)),
                        data.get_temp::<Arc<FontAssets>>(Id::new(FONT_ASSETS_ID)),
                    )
                });
                let (Some(revision), Some(assets)) = (revision, assets) else {
                    return;
                };
                let Ok(mut installed_revision) = installed_revision.lock() else {
                    return;
                };
                if *installed_revision == revision {
                    return;
                }

                assets.add_to_definitions(context);
                *installed_revision = revision;
            }),
        );
    }

    fn add_to_definitions(&self, context: &egui::Context) {
        let mut definitions = context.fonts(|fonts| fonts.definitions().clone());
        let mut fallbacks = definitions
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
        for fallback in definitions
            .families
            .get(&FontFamily::Monospace)
            .into_iter()
            .flatten()
        {
            if !fallbacks.contains(fallback) {
                fallbacks.push(fallback.clone());
            }
        }

        for variant in &self.variants {
            let alias = font_family_id(variant);
            let mut names = Vec::new();
            for (index, part) in variant.faces.iter().enumerate() {
                let name = format!("{alias}:{index}");
                let mut data = FontData::from_owned(part.bytes.clone());
                let axes = data.variation_axes();
                let mut tweak = FontTweak::default();
                if axes.iter().any(|axis| axis.tag.to_be_bytes() == *b"wght") {
                    tweak.coords.push(*b"wght", f32::from(variant.weight));
                }
                if axes.iter().any(|axis| axis.tag.to_be_bytes() == *b"ital") {
                    tweak
                        .coords
                        .push(*b"ital", if variant.italic { 1.0 } else { 0.0 });
                }
                data = data.tweak(tweak);
                definitions.font_data.insert(name.clone(), Arc::new(data));
                names.push(name);
            }

            names.extend(fallbacks.iter().cloned());
            definitions
                .families
                .insert(FontFamily::Name(alias.into()), names);
        }

        context.set_fonts(definitions);
    }
}

async fn fetch(url: String) -> Result<ehttp::Response, StylesheetLoadError> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(StylesheetLoadError::Font {
            url,
            message: "only HTTP and HTTPS font URLs are supported".to_owned(),
        });
    }

    let response =
        ehttp::fetch_async(ehttp::Request::get(&url).with_timeout(Some(Duration::from_secs(30))))
            .await
            .map_err(|error| StylesheetLoadError::Font {
                url: url.clone(),
                message: error.to_string(),
            })?;
    if !response.ok {
        return Err(StylesheetLoadError::Font {
            url,
            message: format!("server returned HTTP status {}", response.status),
        });
    }
    Ok(response)
}

fn resolve_request(request: &FontRequest, faces: &[FontFace]) -> Option<(String, Vec<FontFace>)> {
    for candidate in &request.families {
        let family_faces = faces
            .iter()
            .filter(|face| normalize_family(&face.family) == normalize_family(candidate))
            .collect::<Vec<_>>();
        if family_faces.is_empty() {
            continue;
        }

        let style_faces = family_faces
            .iter()
            .copied()
            .filter(|face| face.italic == request.italic)
            .collect::<Vec<_>>();
        if style_faces.is_empty() {
            continue;
        }

        let in_range = style_faces
            .iter()
            .copied()
            .filter(|face| face.min_weight <= request.weight && request.weight <= face.max_weight)
            .collect::<Vec<_>>();
        let selected = if in_range.is_empty() {
            let nearest_distance = style_faces
                .iter()
                .map(|face| weight_distance(request.weight, face.min_weight, face.max_weight))
                .min()?;
            style_faces
                .into_iter()
                .filter(|face| {
                    weight_distance(request.weight, face.min_weight, face.max_weight)
                        == nearest_distance
                })
                .collect::<Vec<_>>()
        } else {
            in_range
        };

        let family = selected.first()?.family.clone();
        let mut seen_urls = HashSet::new();
        let selected = selected
            .into_iter()
            .filter(|face| seen_urls.insert(face.url.clone()))
            .cloned()
            .collect();
        return Some((family, selected));
    }
    None
}

fn weight_distance(weight: u16, min: u16, max: u16) -> u16 {
    if weight < min {
        min - weight
    } else {
        weight.saturating_sub(max)
    }
}

fn parse_font_faces(source: &str, base_url: &str) -> Vec<FontFace> {
    let source = strip_css_comments(source);
    let mut faces = Vec::new();
    let mut offset = 0;

    while let Some(start_relative) = source[offset..].find("@font-face") {
        let start = offset + start_relative;
        let Some(open_relative) = source[start..].find('{') else {
            break;
        };
        let open = start + open_relative;
        let Some(close_relative) = source[open + 1..].find('}') else {
            break;
        };
        let close = open + 1 + close_relative;
        let declarations = descriptor_values(&source[open + 1..close]);
        if let (Some(family), Some((min_weight, max_weight)), Some(src)) = (
            declarations
                .get("font-family")
                .and_then(|value| unquote(value)),
            declarations
                .get("font-weight")
                .and_then(|value| parse_weight_range(value)),
            declarations.get("src"),
        ) {
            let italic = declarations
                .get("font-style")
                .is_some_and(|value| value.trim().eq_ignore_ascii_case("italic"));
            if let Some(url) = first_font_url(src, base_url) {
                faces.push(FontFace {
                    family,
                    min_weight,
                    max_weight,
                    italic,
                    url,
                });
            }
        }
        offset = close + 1;
    }

    faces
}

fn descriptor_values(body: &str) -> HashMap<String, String> {
    body.split(';')
        .filter_map(|part| {
            let (name, value) = part.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        })
        .collect()
}

fn parse_weight_range(value: &str) -> Option<(u16, u16)> {
    let weights = value
        .split_whitespace()
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "normal" => Some(400),
            "bold" => Some(700),
            _ => value
                .parse::<u16>()
                .ok()
                .filter(|weight| (1..=1000).contains(weight)),
        })
        .collect::<Option<Vec<_>>>()?;
    match weights.as_slice() {
        [weight] => Some((*weight, *weight)),
        [min, max] if min <= max => Some((*min, *max)),
        _ => None,
    }
}

fn first_font_url(src: &str, base_url: &str) -> Option<String> {
    let lower = src.to_ascii_lowercase();
    let mut offset = 0;
    while let Some(start_relative) = lower[offset..].find("url(") {
        let start = offset + start_relative + 4;
        let end = start + src[start..].find(')')?;
        let Some(value) = unquote(src[start..end].trim()) else {
            offset = end + 1;
            continue;
        };
        let value = resolve_url(base_url, &value);
        if [".woff2", ".woff", ".ttf", ".otf"]
            .iter()
            .any(|extension| value.to_ascii_lowercase().contains(extension))
        {
            return Some(value);
        }
        offset = end + 1;
    }
    None
}

fn resolve_url(base_url: &str, url: &str) -> String {
    if url.starts_with("https://") || url.starts_with("http://") {
        return url.to_owned();
    }
    let Some((scheme, rest)) = base_url.split_once("://") else {
        return url.to_owned();
    };
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let origin = &base_url[..scheme.len() + 3 + authority_end];
    if url.starts_with("//") {
        return format!("{scheme}:{url}");
    }
    if url.starts_with('/') {
        return format!("{origin}{url}");
    }
    let directory = base_url
        .rsplit_once('/')
        .map_or(base_url, |(directory, _)| directory);
    format!("{directory}/{url}")
}

fn decode_font(url: &str, bytes: Vec<u8>) -> Result<Vec<u8>, StylesheetLoadError> {
    match bytes.get(..4) {
        Some(b"wOF2") => {
            wuff::decompress_woff2(&bytes).map_err(|error| StylesheetLoadError::Font {
                url: url.to_owned(),
                message: format!("could not decode WOFF2 font: {error}"),
            })
        }
        Some(b"wOFF") => {
            wuff::decompress_woff1(&bytes).map_err(|error| StylesheetLoadError::Font {
                url: url.to_owned(),
                message: format!("could not decode WOFF font: {error}"),
            })
        }
        Some(b"\0\x01\0\0") | Some(b"OTTO") | Some(b"true") | Some(b"typ1") => Ok(bytes),
        _ => Err(StylesheetLoadError::Font {
            url: url.to_owned(),
            message: "response was not a supported TTF, OTF, WOFF, or WOFF2 font".to_owned(),
        }),
    }
}

fn strip_css_comments(source: &str) -> String {
    let mut bytes = source.as_bytes().to_vec();
    let mut offset = 0;
    while let Some(start_relative) = bytes[offset..].windows(2).position(|pair| pair == b"/*") {
        let start = offset + start_relative;
        let Some(end_relative) = bytes[start + 2..].windows(2).position(|pair| pair == b"*/")
        else {
            break;
        };
        let end = start + 2 + end_relative + 2;
        for byte in &mut bytes[start..end] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
        offset = end;
    }
    String::from_utf8(bytes).expect("replacing comment bytes preserves UTF-8")
}

fn unquote(value: &str) -> Option<String> {
    let value = value.trim();
    let quote = value.as_bytes().first().copied()?;
    if (quote == b'\'' || quote == b'"') && value.as_bytes().last() == Some(&quote) {
        Some(value[1..value.len() - 1].to_owned())
    } else if !value.is_empty() {
        Some(value.to_owned())
    } else {
        None
    }
}

pub(crate) fn normalize_family(family: &str) -> String {
    family.trim().to_ascii_lowercase()
}

fn font_family_id(font: &LoadedFont) -> String {
    format!(
        "trcss:{}:{}:{}",
        normalize_family(&font.family),
        font.weight,
        if font.italic { "italic" } else { "normal" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_variable_and_static_font_face_descriptors() {
        let faces = parse_font_faces(
            "@font-face { font-family: 'Outfit'; font-style: normal; font-weight: 100 900; src: url(https://fonts.test/outfit.woff2) format('woff2'); }\
             @font-face { font-family: 'SUSE Mono'; font-style: italic; font-weight: 500; src: url(https://fonts.test/mono.ttf) format('truetype'); }",
            "https://fonts.test/theme.css",
        );

        assert_eq!(faces.len(), 2);
        assert_eq!(faces[0].family, "Outfit");
        assert_eq!((faces[0].min_weight, faces[0].max_weight), (100, 900));
        assert!(!faces[0].italic);
        assert_eq!(faces[1].family, "SUSE Mono");
        assert_eq!((faces[1].min_weight, faces[1].max_weight), (500, 500));
        assert!(faces[1].italic);
    }
}
