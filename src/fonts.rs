use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicU64, Ordering},
    },
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

const FONT_ASSETS_ID: &str = "ecss.font-assets";
const FONT_REVISION_ID: &str = "ecss.font-revision";
const FONT_GENERATION_ID: &str = "ecss.font-generation";
const FONT_REQUEST_ID: &str = "ecss.font-request";
const FONT_CALLBACK_REGISTERED_ID: &str = "ecss.font-callback-registered";
const FONT_INSTALL_CALLBACK_ID: &str = "ecss.font-install";
const FONT_DEFINITIONS_ID: &str = "ecss.font-definitions";
const KERNING_DISABLED_SUFFIX: &str = ":ecss-font-kerning-none";

#[derive(Clone, Debug)]
struct FontDefinitionsFingerprint {
    font_data: Vec<(String, Arc<egui::FontData>)>,
    families: Vec<(FontFamily, Vec<String>)>,
}

impl PartialEq for FontDefinitionsFingerprint {
    fn eq(&self, other: &Self) -> bool {
        self.font_data.len() == other.font_data.len()
            && self.font_data.iter().zip(&other.font_data).all(
                |((name, data), (other_name, other_data))| {
                    name == other_name && Arc::ptr_eq(data, other_data)
                },
            )
            && self.families == other.families
    }
}

impl Eq for FontDefinitionsFingerprint {}

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
        let family = self.family_name(candidates, weight, italic)?;
        context
            .fonts(|fonts| fonts.definitions().families.contains_key(&family))
            .then_some(family)
    }

    pub(crate) fn family_name(
        &self,
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
                return Some(FontFamily::Name(font_family_id(variant).into()));
            }
        }
        None
    }

    pub(crate) fn family_for_kerning(
        context: &egui::Context,
        family: &FontFamily,
        disabled: bool,
    ) -> FontFamily {
        let family = normal_kerning_family(family);
        if !disabled {
            return family;
        }

        let disabled_family = disabled_kerning_family(&family);
        context
            .fonts(|fonts| fonts.definitions().families.contains_key(&disabled_family))
            .then_some(disabled_family)
            .unwrap_or(family)
    }

    pub(crate) fn install_shared(
        assets: Arc<RwLock<Self>>,
        revision: Arc<AtomicU64>,
        global_request: Option<FontRequest>,
        context: &egui::Context,
    ) {
        let should_register = context.data_mut(|data| {
            let generation = data
                .get_temp::<u64>(Id::new(FONT_GENERATION_ID))
                .unwrap_or_default()
                .wrapping_add(1);
            data.insert_temp(Id::new(FONT_GENERATION_ID), generation);
            data.insert_temp(Id::new(FONT_ASSETS_ID), assets);
            data.insert_temp(Id::new(FONT_REVISION_ID), revision);
            data.insert_temp(Id::new(FONT_REQUEST_ID), global_request);

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
        let installed = Arc::new(Mutex::new((0_u64, 0_u64)));
        context.on_begin_pass(
            FONT_INSTALL_CALLBACK_ID,
            Arc::new(move |ui| {
                let context = ui.ctx();
                let current_definitions =
                    context.fonts(|fonts| font_definitions_fingerprint(fonts.definitions()));
                let (generation, assets, revision, global_request, installed_definitions) = context
                    .data(|data| {
                        (
                            data.get_temp::<u64>(Id::new(FONT_GENERATION_ID)),
                            data.get_temp::<Arc<RwLock<FontAssets>>>(Id::new(FONT_ASSETS_ID)),
                            data.get_temp::<Arc<AtomicU64>>(Id::new(FONT_REVISION_ID)),
                            data.get_temp::<Option<FontRequest>>(Id::new(FONT_REQUEST_ID))
                                .flatten(),
                            data.get_temp::<FontDefinitionsFingerprint>(Id::new(
                                FONT_DEFINITIONS_ID,
                            )),
                        )
                    });
                let (Some(generation), Some(assets), Some(revision)) =
                    (generation, assets, revision)
                else {
                    return;
                };
                let revision = revision.load(Ordering::Acquire);
                let Ok(mut installed) = installed.lock() else {
                    return;
                };
                let copies_present = context.fonts(|fonts| {
                    let definitions = fonts.definitions();
                    definitions
                        .font_data
                        .keys()
                        .any(|name| name.ends_with(KERNING_DISABLED_SUFFIX))
                        && definitions.families.keys().any(is_kerning_disabled_family)
                });
                if *installed == (generation, revision)
                    && installed_definitions.as_ref() == Some(&current_definitions)
                    && copies_present
                {
                    return;
                }

                let Ok(assets) = assets.read() else {
                    return;
                };
                assets.add_to_definitions(context);
                if !assets.variants.is_empty() {
                    let family = global_request.as_ref().and_then(|request| {
                        assets.family_name(&request.families, request.weight, request.italic)
                    });
                    if let Some(family) = family {
                        let is_registered = context
                            .fonts(|fonts| fonts.definitions().families.contains_key(&family));
                        if !is_registered {
                            context.request_repaint();
                            return;
                        }
                        context.all_styles_mut(|style| {
                            for font_id in style.text_styles.values_mut() {
                                font_id.family = family.clone();
                            }
                        });
                    }
                }
                *installed = (generation, revision);
                context.request_repaint();
            }),
        );
    }

    pub(crate) fn add_to_definitions(&self, context: &egui::Context) {
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

        let fingerprint = font_definitions_fingerprint(&definitions);
        add_kerning_disabled_copies(&mut definitions);

        context.set_fonts(definitions);
        context.data_mut(|data| {
            data.insert_temp(Id::new(FONT_DEFINITIONS_ID), fingerprint);
        });
    }
}

fn font_definitions_fingerprint(definitions: &egui::FontDefinitions) -> FontDefinitionsFingerprint {
    FontDefinitionsFingerprint {
        font_data: definitions
            .font_data
            .iter()
            .filter(|(name, _)| !name.ends_with(KERNING_DISABLED_SUFFIX))
            .map(|(name, data)| (name.clone(), data.clone()))
            .collect(),
        families: definitions
            .families
            .iter()
            .filter(|(family, _)| !is_kerning_disabled_family(family))
            .map(|(family, names)| (family.clone(), names.clone()))
            .collect(),
    }
}

fn add_kerning_disabled_copies(definitions: &mut egui::FontDefinitions) {
    let font_data = definitions
        .font_data
        .iter()
        .filter(|(name, _)| !name.ends_with(KERNING_DISABLED_SUFFIX))
        .map(|(name, data)| (name.clone(), data.clone()))
        .collect::<Vec<_>>();
    for (name, data) in font_data {
        let mut disabled = (*data).clone();
        disabled.font = Cow::Owned(disable_font_kerning(&data.font, data.index));
        definitions
            .font_data
            .insert(disabled_kerning_font_name(&name), Arc::new(disabled));
    }

    let families = definitions
        .families
        .iter()
        .filter(|(family, _)| !is_kerning_disabled_family(family))
        .map(|(family, names)| (family.clone(), names.clone()))
        .collect::<Vec<_>>();
    for (family, names) in families {
        let disabled_names = names
            .into_iter()
            .map(|name| {
                let disabled_name = disabled_kerning_font_name(&name);
                if definitions.font_data.contains_key(&disabled_name) {
                    disabled_name
                } else {
                    name
                }
            })
            .collect();
        definitions
            .families
            .insert(disabled_kerning_family(&family), disabled_names);
    }
}

fn disabled_kerning_font_name(name: &str) -> String {
    format!("{name}{KERNING_DISABLED_SUFFIX}")
}

fn disabled_kerning_family(family: &FontFamily) -> FontFamily {
    FontFamily::Name(format!("{}{KERNING_DISABLED_SUFFIX}", family).into())
}

fn is_kerning_disabled_family(family: &FontFamily) -> bool {
    matches!(family, FontFamily::Name(name) if name.ends_with(KERNING_DISABLED_SUFFIX))
}

fn normal_kerning_family(family: &FontFamily) -> FontFamily {
    let FontFamily::Name(name) = family else {
        return family.clone();
    };
    let Some(name) = name.strip_suffix(KERNING_DISABLED_SUFFIX) else {
        return family.clone();
    };
    match name {
        "Proportional" => FontFamily::Proportional,
        "Monospace" => FontFamily::Monospace,
        _ => FontFamily::Name(name.into()),
    }
}

fn disable_font_kerning(bytes: &[u8], face_index: u32) -> Vec<u8> {
    let mut font = bytes.to_vec();
    let Some((directory, collection)) = sfnt_directory(&font, face_index) else {
        return font;
    };
    let Some(signature_end) = directory.checked_add(4) else {
        return font;
    };
    let Some(signature) = font.get(directory..signature_end) else {
        return font;
    };
    if !matches!(signature, b"\0\x01\0\0" | b"OTTO" | b"true" | b"typ1") {
        return font;
    }
    let Some(count_offset) = directory.checked_add(4) else {
        return font;
    };
    let Some(table_count) = read_u16(&font, count_offset).map(usize::from) else {
        return font;
    };
    let Some(records_start) = directory.checked_add(12) else {
        return font;
    };
    let Some(records_end) = records_start.checked_add(table_count * 16) else {
        return font;
    };
    if records_end > font.len() {
        return font;
    }

    let mut gpos = None;
    let mut head = None;
    let mut removed_records = Vec::new();
    for index in 0..table_count {
        let record = records_start + index * 16;
        let tag = &font[record..record + 4];
        let Some(offset) = read_u32(&font, record + 8).map(|offset| offset as usize) else {
            return bytes.to_vec();
        };
        let Some(length) = read_u32(&font, record + 12).map(|length| length as usize) else {
            return bytes.to_vec();
        };
        let Some(end) = offset.checked_add(length) else {
            return bytes.to_vec();
        };
        if end > font.len() {
            return bytes.to_vec();
        }
        match tag {
            b"GPOS" => gpos = Some((offset, length)),
            b"head" => head = Some(offset),
            b"kern" | b"kerx" => removed_records.push(index),
            _ => {}
        }
    }

    let mut changed = false;
    if let Some((offset, length)) = gpos {
        changed |= disable_gpos_kerning(&mut font, offset, length);
    }

    if !removed_records.is_empty() {
        let mut next_record = 0;
        for index in 0..table_count {
            if removed_records.contains(&index) {
                continue;
            }
            if next_record != index {
                font.copy_within(
                    records_start + index * 16..records_start + (index + 1) * 16,
                    records_start + next_record * 16,
                );
            }
            next_record += 1;
        }
        font[records_start + next_record * 16..records_end].fill(0);
        let new_count = next_record as u16;
        font[directory + 4..directory + 6].copy_from_slice(&new_count.to_be_bytes());
        let entry_selector = if next_record == 0 {
            0
        } else {
            (usize::BITS - 1 - next_record.leading_zeros()) as u16
        };
        let search_range = if next_record == 0 {
            0
        } else {
            (1_u16 << entry_selector) * 16
        };
        let range_shift = new_count * 16 - search_range;
        font[directory + 6..directory + 8].copy_from_slice(&search_range.to_be_bytes());
        font[directory + 8..directory + 10].copy_from_slice(&entry_selector.to_be_bytes());
        font[directory + 10..directory + 12].copy_from_slice(&range_shift.to_be_bytes());
        changed = true;
    }

    if changed {
        update_font_checksums(&mut font, directory, collection, head);
    }
    font
}

fn sfnt_directory(bytes: &[u8], face_index: u32) -> Option<(usize, bool)> {
    if bytes.get(..4)? == b"ttcf" {
        let count = read_u32(bytes, 8)?;
        if face_index >= count {
            return None;
        }
        let offset = (face_index as usize).checked_mul(4)?.checked_add(12)?;
        Some((read_u32(bytes, offset)? as usize, true))
    } else if face_index == 0 {
        Some((0, false))
    } else {
        None
    }
}

fn disable_gpos_kerning(font: &mut [u8], offset: usize, length: usize) -> bool {
    if length < 10 {
        return false;
    }
    let Some(feature_offset) = read_u16(font, offset + 6).map(usize::from) else {
        return false;
    };
    if feature_offset == 0 || feature_offset + 2 > length {
        return false;
    }
    let feature_list = offset + feature_offset;
    let Some(count) = read_u16(font, feature_list).map(usize::from) else {
        return false;
    };
    let records_start = feature_list + 2;
    if records_start
        .checked_add(count * 6)
        .is_none_or(|end| end > offset + length)
    {
        return false;
    }

    let mut tags = (0..count)
        .map(|index| {
            let start = records_start + index * 6;
            <[u8; 4]>::try_from(&font[start..start + 4]).unwrap()
        })
        .collect::<Vec<_>>();
    let mut changed = false;
    for index in 0..count {
        let replacement = match &tags[index] {
            b"kern" => Some(available_feature_tag(&tags, index)),
            b"vkrn" => Some(available_feature_tag(&tags, index)),
            _ => None,
        };
        let Some(Some(replacement)) = replacement else {
            continue;
        };
        let start = records_start + index * 6;
        font[start..start + 4].copy_from_slice(&replacement);
        tags[index] = replacement;
        changed = true;
    }
    changed
}

fn available_feature_tag(tags: &[[u8; 4]], index: usize) -> Option<[u8; 4]> {
    let tag = tags[index];
    let previous = index.checked_sub(1).map(|index| tags[index]);
    let next = tags.get(index + 1).copied();
    let available = |last| {
        let candidate = [tag[0], tag[1], tag[2], last];
        if previous.is_none_or(|previous| previous < candidate)
            && next.is_none_or(|next| candidate < next)
            && !tags.contains(&candidate)
        {
            return Some(candidate);
        }
        None
    };
    for last in tag[3].saturating_add(1)..=b'~' {
        if let Some(candidate) = available(last) {
            return Some(candidate);
        }
    }
    for last in (b' '..tag[3]).rev() {
        if let Some(candidate) = available(last) {
            return Some(candidate);
        }
    }
    None
}

fn update_font_checksums(font: &mut [u8], directory: usize, collection: bool, head: Option<usize>) {
    let Some(count) = read_u16(font, directory + 4).map(usize::from) else {
        return;
    };
    let records_start = directory + 12;
    if let Some(head) = head
        && head.checked_add(12).is_some_and(|end| end <= font.len())
    {
        font[head + 8..head + 12].fill(0);
    }
    for index in 0..count {
        let record = records_start + index * 16;
        let Some(offset) = read_u32(font, record + 8).map(|offset| offset as usize) else {
            continue;
        };
        let Some(length) = read_u32(font, record + 12).map(|length| length as usize) else {
            continue;
        };
        let Some(end) = offset.checked_add(length) else {
            continue;
        };
        if end > font.len() {
            continue;
        }
        let checksum = font_checksum(&font[offset..end]);
        font[record + 4..record + 8].copy_from_slice(&checksum.to_be_bytes());
    }
    if !collection
        && let Some(head) = head
        && head + 12 <= font.len()
    {
        let adjustment = 0xB1B0_AFBA_u32.wrapping_sub(font_checksum(font));
        font[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    let end = offset.checked_add(2)?;
    Some(u16::from_be_bytes(bytes.get(offset..end)?.try_into().ok()?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    Some(u32::from_be_bytes(bytes.get(offset..end)?.try_into().ok()?))
}

fn font_checksum(bytes: &[u8]) -> u32 {
    bytes.chunks(4).fold(0_u32, |sum, chunk| {
        let mut word = [0; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
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
        "ecss:{}:{}:{}",
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
