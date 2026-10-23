use super::*;

pub(super) fn measure_text(
    context: &Context,
    range: ShapeRange,
    measured: Vec2,
    format: &crate::formatting::Formatting,
    text_basis: f32,
    allow_anonymous_content: bool,
) -> Option<TextMeasurement> {
    fn collect(shape: &egui::Shape, texts: &mut Vec<Arc<egui::Galley>>, supported: &mut bool) {
        match shape {
            egui::Shape::Text(shape) if shape.angle == 0.0 => {
                texts.push(shape.galley.clone());
            }
            egui::Shape::Vec(shapes) => shapes
                .iter()
                .for_each(|shape| collect(shape, texts, supported)),
            egui::Shape::Text(_) => *supported = false,
            _ => {}
        }
    }
    let (galleys, fragmentable) = context.graphics(|layers| {
        let list = layers.get(range.layer)?;
        let mut texts = Vec::new();
        let mut fragmentable = true;
        for clipped in list
            .all_entries()
            .skip(range.start.0)
            .take(range.end.0 - range.start.0)
        {
            let mut text_shapes = 0;
            fragmentable &= collect_fragmentable_text(&clipped.shape, &mut text_shapes);
            collect(&clipped.shape, &mut texts, &mut fragmentable);
        }
        Some((texts, fragmentable))
    })?;
    if galleys.is_empty() || galleys.len() > 1 && (!allow_anonymous_content || !fragmentable) {
        return None;
    }
    let (source_job, natural_size) = if let [galley] = galleys.as_slice() {
        (galley.job.as_ref().clone(), galley.size())
    } else {
        let mut job = galleys[0].job.as_ref().clone();
        job.text.clear();
        job.sections.clear();
        for (index, galley) in galleys.iter().enumerate() {
            if index > 0 && !job.text.ends_with('\n') {
                let format = job
                    .sections
                    .last()
                    .or_else(|| galley.job.sections.first())
                    .map(|section| section.format.clone())
                    .unwrap_or_default();
                job.append("\n", 0.0, format);
            }
            let offset = job.text.len();
            job.text.push_str(&galley.job.text);
            job.sections
                .extend(
                    galley
                        .job
                        .sections
                        .iter()
                        .map(|section| egui::text::LayoutSection {
                            byte_range: (offset + section.byte_range.start.0).into()
                                ..(offset + section.byte_range.end.0).into(),
                            ..section.clone()
                        }),
                );
        }
        let size = context.fonts_mut(|fonts| fonts.layout_job(job.clone()).size());
        (job, size)
    };
    let source = prepare_text_job(context, &source_job, format, text_basis);
    let extra = (measured - natural_size).max(Vec2::ZERO);
    Some(TextMeasurement {
        job: source,
        extra,
        minimum: 0.0,
    })
}

pub(super) fn collect_fragmentable_text(shape: &egui::Shape, text_shapes: &mut usize) -> bool {
    match shape {
        egui::Shape::Text(text) if text.angle == 0.0 => {
            *text_shapes += 1;
            true
        }
        egui::Shape::Vec(shapes) => shapes
            .iter()
            .all(|shape| collect_fragmentable_text(shape, text_shapes)),
        egui::Shape::Noop => true,
        egui::Shape::Rect(rect)
            if rect.fill == egui::Color32::TRANSPARENT
                && rect.stroke.width == 0.0
                && rect.stroke.color == egui::Color32::TRANSPARENT =>
        {
            true
        }
        _ => false,
    }
}

fn first_letter_range(text: &str) -> Option<std::ops::Range<usize>> {
    let first_line = text.split_once('\n').map_or(text, |(line, _)| line);
    let indices = first_line.char_indices().collect::<Vec<_>>();
    let characters = indices
        .iter()
        .map(|(_, character)| *character)
        .collect::<Vec<_>>();
    let range = first_letter_character_range(&characters)?;
    let start = indices[range.start].0;
    let (last, character) = indices[range.end - 1];
    Some(start..last + character.len_utf8())
}

pub(super) fn first_text_descendant(frame: &Frame, index: usize) -> Option<usize> {
    for child in &frame.nodes[index].children {
        let node = &frame.nodes[*child];
        if node
            .text
            .as_ref()
            .is_some_and(|text| first_letter_range(&text.job.text).is_some())
        {
            return Some(*child);
        }
        if let Some(descendant) = first_text_descendant(frame, *child) {
            return Some(descendant);
        }
    }
    None
}

pub(super) fn apply_first_letter_inline_spacing(
    context: &Context,
    job: &mut egui::text::LayoutJob,
    style: &BoxStyle,
    basis: f32,
    writing: crate::formatting::Writing,
    direction: taffy::Direction,
) -> Option<(f32, f32)> {
    let range = first_letter_range(&job.text)?;
    let edge = |value: taffy::LengthPercentageAuto, basis: f32| {
        crate::format_layout::raw(value.into_raw(), basis)
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
    };
    let margin = [
        edge(style.layout.margin.top, basis),
        edge(style.layout.margin.right, basis),
        edge(style.layout.margin.bottom, basis),
        edge(style.layout.margin.left, basis),
    ];
    let padding = [
        edge(style.layout.padding.top.into(), basis).max(0.0),
        edge(style.layout.padding.right.into(), basis).max(0.0),
        edge(style.layout.padding.bottom.into(), basis).max(0.0),
        edge(style.layout.padding.left.into(), basis).max(0.0),
    ];
    let border = style.layout.border.map(|value| {
        resolve(value.into_raw(), 0.0)
            .filter(|value| value.is_finite())
            .unwrap_or(0.0)
            .max(0.0)
    });
    let border_widths = [border.top, border.right, border.bottom, border.left];
    let border: [f32; 4] = std::array::from_fn(|side| {
        if matches!(
            style.border_styles[side],
            crate::layout_style::BorderStyle::None | crate::layout_style::BorderStyle::Hidden
        ) {
            0.0
        } else {
            border_widths[side]
        }
    });
    let edges = std::array::from_fn::<_, 4, _>(|side| margin[side] + padding[side] + border[side]);
    let direction = if writing.vertical()
        && style
            .format
            .text_orientation
            .eq_ignore_ascii_case("upright")
    {
        taffy::Direction::Ltr
    } else {
        direction
    };
    let (start_side, end_side, block_edges) = if writing.vertical() {
        if direction == taffy::Direction::Rtl {
            (2, 0, [1, 3])
        } else {
            (0, 2, [1, 3])
        }
    } else if direction == taffy::Direction::Rtl {
        (1, 3, [0, 2])
    } else {
        (3, 1, [0, 2])
    };
    let start_spacing = edges[start_side];
    let end_spacing = edges[end_side];
    if !start_spacing.is_finite() || !end_spacing.is_finite() {
        return None;
    }
    let block_extent = if writing.vertical() {
        block_edges
            .iter()
            .map(|side| padding[*side] + border[*side])
            .sum::<f32>()
    } else {
        0.0
    };
    if start_spacing == 0.0 && end_spacing == 0.0 && block_extent == 0.0 {
        return Some((0.0, 0.0));
    }

    let mut sections = Vec::with_capacity(job.sections.len() + 2);
    for section in job.sections.drain(..) {
        let start = section.byte_range.start.0;
        let end = section.byte_range.end.0;
        let mut boundaries = vec![start];
        for boundary in [range.start, range.end] {
            if start < boundary && boundary < end {
                boundaries.push(boundary);
            }
        }
        boundaries.push(end);
        boundaries.sort_unstable();
        boundaries.dedup();
        if start == end {
            sections.push(section);
            continue;
        }
        for (index, bounds) in boundaries.windows(2).enumerate() {
            let mut part = section.clone();
            part.byte_range = egui::text::ByteIndex(bounds[0])..egui::text::ByteIndex(bounds[1]);
            if index > 0 {
                part.leading_space = 0.0;
            }
            sections.push(part);
        }
    }
    if let Some(section) = sections
        .iter_mut()
        .find(|section| section.byte_range.start.0 == range.start)
    {
        section.leading_space += start_spacing;
        if writing.vertical() && block_extent > 0.0 {
            let line_height = section.format.line_height.unwrap_or_else(|| {
                context.fonts_mut(|fonts| fonts.row_height(&section.format.font_id))
            });
            section.format.line_height = Some(line_height + block_extent);
        }
    }
    let mut trailing = 0.0;
    if range.end < job.text.len() {
        if let Some(section) = sections
            .iter_mut()
            .find(|section| section.byte_range.start.0 == range.end)
        {
            section.leading_space += end_spacing;
        } else {
            trailing = end_spacing;
        }
    } else {
        trailing = end_spacing;
    }
    job.sections = sections;
    Some((trailing, start_spacing + end_spacing))
}

pub(super) fn prepare_text_job(
    context: &Context,
    job: &egui::text::LayoutJob,
    format: &crate::formatting::Formatting,
    text_basis: f32,
) -> egui::text::LayoutJob {
    let preserve = matches!(
        format.white_space.as_str(),
        "pre" | "pre-wrap" | "break-spaces"
    );
    let newlines = preserve || format.white_space == "pre-line";
    let mut result = job.clone();
    result.text.clear();
    result.sections.clear();
    let mut space = false;
    let mut carriage = false;
    let mut line_advance = 0.0_f32;
    let mut word_start = true;
    let mut line_index = 0usize;
    let mut line_start = true;
    let indent = if format.text_indent.1 {
        format.text_indent.0 * text_basis
    } else {
        format.text_indent.0
    };
    for (index, mut c) in job.text.char_indices() {
        if c == '\n' && carriage {
            carriage = false;
            continue;
        }
        carriage = c == '\r';
        if carriage {
            c = '\n';
        }
        if !preserve && matches!(c, ' ' | '\t' | '\n' | '\u{c}') {
            if c == '\n' && newlines {
                space = false;
            } else {
                if space {
                    continue;
                }
                space = true;
                c = ' ';
            }
        } else {
            space = false;
        }
        let mut text_format = job.format_at_byte(egui::text::ByteIndex(index)).clone();
        text_format.font_id.family = crate::fonts::FontAssets::family_for_kerning(
            context,
            &text_format.font_id.family,
            format.font_kerning == crate::formatting::FontKerning::None,
        );
        if format.font_size_set {
            text_format.font_id.size = format.em;
        }
        if let Some(height) = format.line_height {
            text_format.line_height = Some(height);
        }
        let mut output = String::new();
        let space_width = context.fonts_mut(|fonts| fonts.glyph_width(&text_format.font_id, ' '));
        let indent_spaces = if indent > 0.0 && space_width > 0.0 {
            (indent / space_width).round().max(1.0) as usize
        } else {
            0
        };
        let apply_indent = line_start
            && if format.text_indent_hanging {
                line_index > 0
            } else {
                line_index == 0 || format.text_indent_each_line
            };
        if apply_indent {
            output.extend(std::iter::repeat_n(' ', indent_spaces));
        }
        if c == '\t' && preserve {
            let tab_stop = if format.tab_size_length {
                format.tab_size
            } else {
                format.tab_size * space_width
            };
            if tab_stop > 0.0 && space_width > 0.0 {
                let remainder = line_advance.rem_euclid(tab_stop);
                let advance = if remainder <= f32::EPSILON {
                    tab_stop
                } else {
                    tab_stop - remainder
                };
                let count = (advance / space_width).ceil().max(1.0) as usize;
                output.extend(std::iter::repeat_n(' ', count));
            }
        } else {
            match format.text_transform.as_str() {
                "uppercase" => output.extend(c.to_uppercase()),
                "lowercase" => output.extend(c.to_lowercase()),
                "capitalize" if word_start => output.extend(c.to_uppercase()),
                "full-width" => {
                    let transformed = if c == ' ' {
                        '\u{3000}'
                    } else if ('!'..='~').contains(&c) {
                        char::from_u32(c as u32 + 0xFEE0).unwrap_or(c)
                    } else {
                        c
                    };
                    output.push(transformed);
                }
                "full-size-kana" => output.push(full_size_kana(c)),
                _ => output.push(c),
            }
        }
        if matches!(c, '\n' | '\r') {
            line_advance = 0.0;
            word_start = true;
            line_index += 1;
            line_start = true;
        } else {
            for character in output.chars() {
                let glyph_width =
                    context.fonts_mut(|fonts| fonts.glyph_width(&text_format.font_id, character));
                line_advance += glyph_width
                    + format.letter_spacing
                    + if character.is_whitespace() {
                        format.word_spacing
                    } else {
                        0.0
                    };
            }
            word_start = !c.is_alphanumeric() && !is_combining_mark(c);
            line_start = false;
        }
        for character in output.chars() {
            text_format.extra_letter_spacing = format.letter_spacing
                + if character.is_whitespace() {
                    format.word_spacing
                } else {
                    0.0
                };
            result.append(character.encode_utf8(&mut [0; 4]), 0.0, text_format.clone());
        }
    }
    if matches!(format.white_space.as_str(), "pre" | "nowrap") {
        result.wrap.max_width = f32::INFINITY;
    }
    result.wrap.break_anywhere |= matches!(format.word_break.as_str(), "break-all" | "break-word")
        || matches!(format.overflow_wrap.as_str(), "break-word" | "anywhere")
        || format.line_break == "anywhere";
    result
}

fn full_size_kana(character: char) -> char {
    match character {
        'ぁ' => 'あ',
        'ぃ' => 'い',
        'ぅ' => 'う',
        'ぇ' => 'え',
        'ぉ' => 'お',
        'っ' => 'つ',
        'ゃ' => 'や',
        'ゅ' => 'ゆ',
        'ょ' => 'よ',
        'ゎ' => 'わ',
        'ァ' => 'ア',
        'ィ' => 'イ',
        'ゥ' => 'ウ',
        'ェ' => 'エ',
        'ォ' => 'オ',
        'ッ' => 'ツ',
        'ャ' => 'ヤ',
        'ュ' => 'ユ',
        'ョ' => 'ヨ',
        'ヮ' => 'ワ',
        _ => character,
    }
}

fn is_cjk(character: char) -> bool {
    matches!(
        character as u32,
        0x2E80..=0x2FFF
            | 0x3000..=0x30FF
            | 0x3100..=0x312F
            | 0x31A0..=0x31BF
            | 0x31F0..=0x31FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xA960..=0xA97F
            | 0xAC00..=0xD7AF
            | 0xF900..=0xFAFF
            | 0xFE30..=0xFE4F
            | 0xFF00..=0xFFEF
            | 0x20000..=0x323AF
    )
}

pub(super) fn reflow_text(
    context: &Context,
    range: ShapeRange,
    job: &egui::text::LayoutJob,
    selection_color: Color32,
    format: &crate::formatting::Formatting,
    width: f32,
) -> Vec2 {
    fn find_galley(shape: &egui::Shape) -> Option<&std::sync::Arc<egui::Galley>> {
        match shape {
            egui::Shape::Text(text) => Some(&text.galley),
            egui::Shape::Vec(shapes) => shapes.iter().find_map(find_galley),
            _ => None,
        }
    }

    let job = justify_wrapped_text_job(context, job, width, format);
    let current = context.graphics(|layers| {
        let list = layers.get(range.layer)?;
        list.all_entries()
            .skip(range.start.0)
            .take(range.end.0 - range.start.0)
            .find_map(|entry| find_galley(&entry.shape).cloned())
    });
    if let Some(current) = current
        .as_ref()
        .filter(|galley| galley.job.as_ref() == &job)
    {
        return current.size();
    }

    let galley = context.fonts_mut(|fonts| fonts.layout_job(job));
    if current
        .as_ref()
        .is_some_and(|current| galley_has_selection(current, selection_color))
    {
        // Replacing a selected galley here would discard egui's selection vertices.
        return galley.size();
    }
    fn replace(shape: &mut egui::Shape, galley: &std::sync::Arc<egui::Galley>) {
        match shape {
            egui::Shape::Text(text) => text.galley = galley.clone(),
            egui::Shape::Vec(shapes) => shapes.iter_mut().for_each(|shape| replace(shape, galley)),
            _ => {}
        }
    }
    context.graphics_mut(|layers| {
        let list = layers.entry(range.layer);
        for index in range.start.0..range.end.0 {
            list.mutate_shape(egui::layers::ShapeIdx(index), |entry| {
                replace(&mut entry.shape, &galley)
            });
        }
    });
    galley.size()
}

fn galley_has_selection(galley: &egui::Galley, selection_color: Color32) -> bool {
    galley.rows.iter().any(|row| {
        row.row
            .visuals
            .mesh
            .vertices
            .iter()
            .any(|vertex| vertex.color == selection_color)
    })
}

fn slice_job(job: &egui::text::LayoutJob, range: egui::text::ByteRange) -> egui::text::LayoutJob {
    let mut result = job.clone();
    result.text = job.text[range.start.0..range.end.0].to_owned();
    result.sections = job
        .sections
        .iter()
        .filter_map(|section| {
            let start = section.byte_range.start.max(range.start);
            let end = section.byte_range.end.min(range.end);
            (start < end).then(|| egui::text::LayoutSection {
                byte_range: (start.0 - range.start.0).into()..(end.0 - range.start.0).into(),
                ..section.clone()
            })
        })
        .collect();
    result.wrap.max_width = f32::INFINITY;
    result
}

fn leading_space_at_byte(job: &egui::text::LayoutJob, byte: usize) -> f32 {
    job.sections
        .iter()
        .find(|section| section.byte_range.start.0 == byte)
        .map_or(0.0, |section| section.leading_space)
}

pub(super) fn text_runs(
    context: &Context,
    job: &egui::text::LayoutJob,
    format: &crate::formatting::Formatting,
) -> (Vec<crate::layout_tree::TextRun>, f32) {
    let mut start = 0;
    let mut runs = Vec::new();
    let preserve = matches!(
        format.white_space.as_str(),
        "pre" | "pre-wrap" | "break-spaces"
    );
    let break_anywhere = matches!(format.word_break.as_str(), "break-all" | "break-word")
        || matches!(format.overflow_wrap.as_str(), "break-word" | "anywhere")
        || format.line_break == "anywhere";
    let natural_ends = unicode_linebreak::linebreaks(&job.text)
        .filter_map(|(end, _)| {
            if format.word_break == "keep-all" && end < job.text.len() {
                let before = job.text[..end].chars().next_back();
                let after = job.text[end..].chars().next();
                if before.is_some_and(is_cjk) && after.is_some_and(is_cjk) {
                    return None;
                }
            }
            Some(end)
        })
        .collect::<Vec<_>>();
    let line_ends = if break_anywhere {
        let characters = job.text.char_indices().collect::<Vec<_>>();
        characters
            .iter()
            .enumerate()
            .filter_map(|(index, (start, character))| {
                let next = characters.get(index + 1).map(|(_, character)| *character);
                let grapheme_continues = next.is_some_and(|next| {
                    is_combining_mark(next) || next == '\u{200d}' || *character == '\u{200d}'
                });
                (!grapheme_continues).then_some(start + character.len_utf8())
            })
            .collect::<Vec<_>>()
    } else {
        natural_ends.clone()
    };
    let intrinsic_ends = if matches!(format.white_space.as_str(), "nowrap" | "pre") {
        natural_ends
            .into_iter()
            .filter(|end| *end == job.text.len() || job.text[..*end].ends_with('\n'))
            .collect::<Vec<_>>()
    } else if matches!(format.word_break.as_str(), "break-all" | "break-word")
        || format.overflow_wrap == "anywhere"
        || format.line_break == "anywhere"
    {
        line_ends.clone()
    } else {
        natural_ends
    };
    let mut minimum = 0.0_f32;
    for &end in &intrinsic_ends {
        let original = &job.text[start..end];
        let content = if matches!(format.white_space.as_str(), "pre" | "break-spaces") {
            original.trim_end_matches(['\r', '\n'])
        } else {
            original.trim_end_matches(|c: char| c.is_ascii_whitespace())
        };
        let range = egui::text::ByteIndex(start)..egui::text::ByteIndex(start + content.len());
        let width = context.fonts_mut(|fonts| fonts.layout_job(slice_job(job, range)).size().x);
        minimum = minimum.max(width);
        start = end;
    }
    start = 0;
    for end in line_ends {
        let original = &job.text[start..end];
        let content = if preserve {
            original.trim_end_matches(['\r', '\n'])
        } else {
            original.trim_end_matches(|c: char| c.is_ascii_whitespace())
        };
        let range = egui::text::ByteIndex(start)..egui::text::ByteIndex(start + content.len());
        if let Some(section) = job
            .sections
            .iter()
            .find(|section| section.byte_range.end.0 > start)
        {
            let size =
                context.fonts_mut(|fonts| fonts.layout_job(slice_job(job, range.clone())).size());
            let (space, height) = context.fonts_mut(|fonts| {
                (
                    fonts.glyph_width(&section.format.font_id, ' '),
                    fonts.row_height(&section.format.font_id),
                )
            });
            runs.push(crate::layout_tree::TextRun {
                range,
                width: size.x,
                height: format.line_height.unwrap_or(size.y.max(height)),
                gap: if !preserve && content.len() < original.len() {
                    space
                } else {
                    0.0
                },
                marker_width: format.text_overflow[1].marker().map_or(0.0, |marker| {
                    let mut marker_job = egui::text::LayoutJob::default();
                    marker_job.append(marker, 0.0, section.format.clone());
                    context.fonts_mut(|fonts| fonts.layout_job(marker_job).size().x)
                }),
                newline: original.contains('\n'),
                min_content_break: intrinsic_ends.binary_search(&end).is_ok(),
            });
        }
        start = end;
    }
    (runs, minimum)
}
pub(super) struct FragmentedTextInput<'a> {
    pub(super) context: &'a Context,
    pub(super) layer: egui::LayerId,
    pub(super) start: egui::layers::ShapeIdx,
    pub(super) end: egui::layers::ShapeIdx,
    pub(super) geometry: &'a Geometry,
    pub(super) job: &'a egui::text::LayoutJob,
    pub(super) format: &'a crate::formatting::Formatting,
    pub(super) selection_visuals: &'a egui::Visuals,
}

pub(super) fn paint_named_flow_texts(frame: &Frame, cache: &Cache, context: &Context) {
    for node in &frame.nodes {
        let Some(geometry) = cache
            .boxes
            .get(&node.id)
            .filter(|geometry| geometry.flow_source)
        else {
            continue;
        };
        let (Some(paint), Some(text)) = (node.paint.as_ref(), node.text.as_ref()) else {
            continue;
        };
        let geometry =
            geometry.translated(frame.origin.to_vec2() - node.ancestor_scroll + node.sticky);
        let selection_visuals = paint.ui.visuals().clone();
        replace_fragmented_text(FragmentedTextInput {
            context,
            layer: paint.ui.layer_id(),
            start: paint.start,
            end: paint.end,
            geometry: &geometry,
            job: &text.job,
            format: &node.style.format,
            selection_visuals: &selection_visuals,
        });
    }
}

pub(super) fn replace_fragmented_text(input: FragmentedTextInput<'_>) {
    let FragmentedTextInput {
        context,
        layer,
        start,
        end,
        geometry,
        job,
        format,
        selection_visuals,
    } = input;
    let original = context.graphics(|layers| {
        layers
            .get(layer)?
            .all_entries()
            .skip(start.0)
            .take(end.0 - start.0)
            .find_map(|entry| {
                if let egui::Shape::Text(text) = &entry.shape {
                    Some((text.clone(), entry.clip_rect))
                } else {
                    None
                }
            })
    });
    let Some((template, clip)) = original else {
        return;
    };
    let selection_color = selection_visuals.selection.bg_fill;
    let selection_by_range = template.galley.job.text == job.text;
    let selected_ranges = if selection_by_range {
        galley_selection_ranges(&template.galley, selection_color)
    } else {
        Default::default()
    };
    let selection_rects = galley_selection_rects(&template.galley, template.pos, selection_color);
    context.graphics_mut(|layers| {
        let list = layers.entry(layer);
        for index in start.0..end.0 {
            list.mutate_shape(egui::layers::ShapeIdx(index), |shape| {
                if matches!(shape.shape, egui::Shape::Text(_)) {
                    shape.shape = egui::Shape::Noop;
                }
            });
        }
    });
    let painter = egui::Painter::new(context.clone(), layer, clip);
    for rect in &selection_rects {
        painter.rect_filled(*rect, 0.0, selection_color);
    }
    for fragment in &geometry.fragments {
        if let Some(range) = &fragment.range {
            let fragment_clip = fragment.clip.map_or(clip, |fragment_clip| {
                clip.intersect(fragment_clip.translate(geometry.border.min.to_vec2()))
            });
            let fragment_painter = egui::Painter::new(context.clone(), layer, fragment_clip);
            let mut text = template.clone();
            let mut visible_range = range.clone();
            let mut layout = if let Some(marker) = &fragment.marker {
                let content_width = (fragment.text_rect.width() - fragment.marker_width).max(0.0);
                let mut fitting = slice_job(job, range.clone());
                fitting.wrap = egui::text::TextWrapping::truncate_at_width(content_width);
                fitting.wrap.overflow_character = None;
                let visible_chars = context.fonts_mut(|fonts| {
                    fonts
                        .layout_job(fitting)
                        .rows
                        .first()
                        .map_or(0, |row| row.row.glyphs.len())
                });
                let source = &job.text[range.start.0..range.end.0];
                let visible_bytes = source
                    .char_indices()
                    .nth(visible_chars)
                    .map_or(source.len(), |(offset, _)| offset);
                let visible_end = egui::text::ByteIndex(range.start.0 + visible_bytes);
                visible_range.end = visible_end;
                let mut layout = slice_job(job, range.start..visible_end);
                if let Some(section) = layout.sections.last().or_else(|| {
                    job.sections
                        .iter()
                        .find(|section| section.byte_range.end > range.start)
                }) {
                    layout.append(marker, 0.0, section.format.clone());
                }
                layout
            } else {
                slice_job(job, range.clone())
            };
            if fragment.marker.is_some() {
                layout.wrap = egui::text::TextWrapping::no_max_width();
            }
            if fragment.marker.is_none()
                && (matches!(format.text_align.as_str(), "justify" | "justify-all")
                    || format.text_align_last == "justify")
            {
                layout = justify_text_job(context, &layout, fragment.text_rect.width(), format);
            }
            text.galley = context.fonts_mut(|fonts| fonts.layout_job(layout));
            text.pos = geometry.border.min + fragment.text_rect.min.to_vec2();
            if selection_by_range {
                let source_start = job.text[..visible_range.start.0].chars().count();
                let source_end = job.text[..visible_range.end.0].chars().count();
                for selected in &selected_ranges {
                    let start = selected.start.max(source_start);
                    let end = selected.end.min(source_end);
                    if start < end {
                        paint_galley_selection_range(
                            &mut text.galley,
                            start - source_start..end - source_start,
                            selection_visuals.selection.stroke.color,
                        );
                    }
                }
            } else {
                paint_galley_selection(
                    &mut text.galley,
                    text.pos,
                    &selection_rects,
                    selection_visuals.selection.stroke.color,
                );
            }
            if format.writing.vertical() {
                vertical_text(&mut text, fragment.text_rect.size(), format);
            }
            fragment_painter.add(egui::Shape::Text(text));
        }
    }
}

fn paint_galley_selection_range(
    galley: &mut std::sync::Arc<egui::Galley>,
    selected: std::ops::Range<usize>,
    selection_text_color: Color32,
) {
    let galley = std::sync::Arc::make_mut(galley);
    let mut char_offset = 0;
    for placed in &mut galley.rows {
        let row = std::sync::Arc::make_mut(&mut placed.row);
        for (index, glyph) in row.glyphs.iter().enumerate() {
            if !selected.contains(&(char_offset + index)) {
                continue;
            }
            let start = glyph.first_vertex as usize;
            let end = row
                .glyphs
                .get(index + 1)
                .map_or(row.visuals.glyph_vertex_range.end, |next| {
                    next.first_vertex as usize
                });
            for vertex in &mut row.visuals.mesh.vertices[start..end] {
                vertex.color = selection_text_color;
            }
        }
        char_offset += row.glyphs.len() + usize::from(placed.ends_with_newline);
    }
}

fn justify_text_job(
    context: &Context,
    job: &egui::text::LayoutJob,
    width: f32,
    format: &crate::formatting::Formatting,
) -> egui::text::LayoutJob {
    if format.text_justify == "none" {
        return job.clone();
    }

    let natural_width = context.fonts_mut(|fonts| fonts.layout_job(job.clone()).size().x);
    let target = width.max(natural_width);
    let is_inter_character = match format.text_justify.as_str() {
        "inter-character" => true,
        "auto" => job.text.chars().any(is_cjk),
        _ => false,
    };
    let characters = job.text.char_indices().collect::<Vec<_>>();
    let opportunities = if is_inter_character {
        characters
            .iter()
            .enumerate()
            .filter_map(|(index, (byte_index, character))| {
                let next = characters.get(index + 1).map(|(_, character)| *character)?;
                let continues_cluster =
                    is_combining_mark(next) || next == '\u{200d}' || *character == '\u{200d}';
                (!character.is_whitespace() && !next.is_whitespace() && !continues_cluster)
                    .then_some(*byte_index)
            })
            .collect::<Vec<_>>()
    } else {
        let mut seen_word = false;
        characters
            .iter()
            .enumerate()
            .filter_map(|(index, (byte_index, character))| {
                if !character.is_whitespace() {
                    seen_word = true;
                    return None;
                }
                let followed_by_word = characters
                    .get(index + 1)
                    .is_some_and(|(_, next)| !next.is_whitespace());
                (*character == ' ' && seen_word && followed_by_word).then_some(*byte_index)
            })
            .collect::<Vec<_>>()
    };
    if opportunities.is_empty() || target <= natural_width + 0.01 {
        return job.clone();
    }
    let extra = (target - natural_width) / opportunities.len() as f32;
    let mut result = job.clone();
    result.text.clear();
    result.sections.clear();
    let mut next_opportunity = 0;
    for &(index, character) in &characters {
        let mut text_format = job.format_at_byte(egui::text::ByteIndex(index)).clone();
        if opportunities.get(next_opportunity) == Some(&index) {
            text_format.extra_letter_spacing += extra;
            next_opportunity += 1;
        }
        result.append(
            character.encode_utf8(&mut [0; 4]),
            leading_space_at_byte(job, index),
            text_format,
        );
    }
    result.halign = egui::Align::Min;
    result
}

fn justify_wrapped_text_job(
    context: &Context,
    job: &egui::text::LayoutJob,
    width: f32,
    format: &crate::formatting::Formatting,
) -> egui::text::LayoutJob {
    if format.text_justify == "none"
        || !matches!(format.text_align.as_str(), "justify" | "justify-all")
            && format.text_align_last != "justify"
    {
        return job.clone();
    }

    let galley = context.fonts_mut(|fonts| fonts.layout_job(job.clone()));
    if galley.elided {
        return job.clone();
    }

    let characters = job.text.char_indices().collect::<Vec<_>>();
    let mut result = job.clone();
    result.text.clear();
    result.sections.clear();
    let mut character_start = 0;
    let mut changed = false;

    for (row_index, placed_row) in galley.rows.iter().enumerate() {
        let character_end = (character_start + placed_row.row.glyphs.len()).min(characters.len());
        let byte_start = characters
            .get(character_start)
            .map_or(job.text.len(), |(byte, _)| *byte);
        let byte_end = characters
            .get(character_end)
            .map_or(job.text.len(), |(byte, _)| *byte);
        let paragraph_end = placed_row.ends_with_newline || row_index + 1 == galley.rows.len();
        let line_align = if paragraph_end && format.text_align_last != "auto" {
            format.text_align_last.as_str()
        } else {
            format.text_align.as_str()
        };
        let should_justify = line_align == "justify-all"
            || line_align == "justify" && (!paragraph_end || format.text_align_last == "justify");
        let line_range = egui::text::ByteIndex(byte_start)..egui::text::ByteIndex(byte_end);
        let source_line = slice_job(job, line_range);
        let justified_line = (should_justify && byte_start < byte_end)
            .then(|| justify_text_job(context, &source_line, width, format));
        changed |= justified_line
            .as_ref()
            .is_some_and(|justified| justified != &source_line);

        for (byte, character) in source_line.text.char_indices() {
            let text_format = justified_line
                .as_ref()
                .map(|line| line.format_at_byte(egui::text::ByteIndex(byte)).clone())
                .unwrap_or_else(|| {
                    job.format_at_byte(egui::text::ByteIndex(byte_start + byte))
                        .clone()
                });
            result.append(
                character.encode_utf8(&mut [0; 4]),
                leading_space_at_byte(justified_line.as_ref().unwrap_or(&source_line), byte),
                text_format,
            );
        }
        character_start = character_end;

        if placed_row.ends_with_newline
            && characters
                .get(character_start)
                .is_some_and(|(_, character)| *character == '\n')
        {
            let (byte, character) = characters[character_start];
            let text_format = job.format_at_byte(egui::text::ByteIndex(byte)).clone();
            result.append(
                character.encode_utf8(&mut [0; 4]),
                leading_space_at_byte(job, byte),
                text_format,
            );
            character_start += 1;
        }
    }

    for &(byte, character) in characters.iter().skip(character_start) {
        let text_format = job.format_at_byte(egui::text::ByteIndex(byte)).clone();
        result.append(
            character.encode_utf8(&mut [0; 4]),
            leading_space_at_byte(job, byte),
            text_format,
        );
    }
    if !changed {
        return job.clone();
    }
    result.halign = egui::Align::Min;
    result
}

fn galley_selection_ranges(
    galley: &egui::Galley,
    selection_color: Color32,
) -> Vec<std::ops::Range<usize>> {
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let mut char_offset = 0;
    for placed in &galley.rows {
        let row = &placed.row;
        let selected = row
            .visuals
            .mesh
            .vertices
            .iter()
            .filter(|vertex| vertex.color == selection_color)
            .fold(Rect::NOTHING, |rect, vertex| {
                rect.union(Rect::from_min_size(vertex.pos, Vec2::ZERO))
            });
        if selected.is_positive() {
            let start = char_offset + row.char_at(selected.left()).0;
            let end = char_offset + row.char_at(selected.right()).0;
            if start < end {
                if let Some(previous) = ranges.last_mut()
                    && previous.end == start
                {
                    previous.end = end;
                } else {
                    ranges.push(start..end);
                }
            }
        }
        char_offset += row.glyphs.len() + usize::from(placed.ends_with_newline);
    }
    ranges
}

fn galley_selection_rects(
    galley: &egui::Galley,
    pos: egui::Pos2,
    selection_color: Color32,
) -> Vec<Rect> {
    galley
        .rows
        .iter()
        .filter_map(|placed| {
            let rect = placed
                .row
                .visuals
                .mesh
                .vertices
                .iter()
                .filter(|vertex| vertex.color == selection_color)
                .fold(Rect::NOTHING, |rect, vertex| {
                    rect.union(Rect::from_min_size(vertex.pos, Vec2::ZERO))
                });
            rect.is_positive()
                .then_some(rect.translate(pos.to_vec2() + placed.pos.to_vec2()))
        })
        .collect()
}

fn paint_galley_selection(
    galley: &mut std::sync::Arc<egui::Galley>,
    pos: egui::Pos2,
    selection_rects: &[Rect],
    selection_text_color: Color32,
) {
    if selection_rects.is_empty() {
        return;
    }
    let galley = std::sync::Arc::make_mut(galley);
    for placed in &mut galley.rows {
        let row = std::sync::Arc::make_mut(&mut placed.row);
        for (index, glyph) in row.glyphs.iter().enumerate() {
            let glyph_center = pos + placed.pos.to_vec2() + glyph.logical_rect().center().to_vec2();
            if !selection_rects
                .iter()
                .any(|rect| rect.contains(glyph_center))
            {
                continue;
            }
            let start = glyph.first_vertex as usize;
            let end = row
                .glyphs
                .get(index + 1)
                .map_or(row.visuals.glyph_vertex_range.end, |next| {
                    next.first_vertex as usize
                });
            for vertex in &mut row.visuals.mesh.vertices[start..end] {
                vertex.color = selection_text_color;
            }
        }
    }
}

pub(super) fn rotate_text(
    context: &Context,
    layer: egui::LayerId,
    start: egui::layers::ShapeIdx,
    end: egui::layers::ShapeIdx,
    geometry: &Geometry,
    format: &crate::formatting::Formatting,
) {
    context.graphics_mut(|layers| {
        let list = layers.entry(layer);
        for index in start.0..end.0 {
            list.mutate_shape(egui::layers::ShapeIdx(index), |entry| {
                if let egui::Shape::Text(text) = &mut entry.shape {
                    text.pos = geometry.content().min;
                    vertical_text(text, geometry.content().size(), format);
                }
            });
        }
    });
}

pub(super) fn vertical_text(
    text: &mut egui::epaint::TextShape,
    size: Vec2,
    format: &crate::formatting::Formatting,
) {
    use crate::formatting::Writing;
    let galley = std::sync::Arc::make_mut(&mut text.galley);
    let mut bounds = Rect::NOTHING;
    for placed in &mut galley.rows {
        let origin = placed.pos;
        let row = std::sync::Arc::make_mut(&mut placed.row);
        let original = row.visuals.mesh.vertices.clone();
        for vertex in &mut row.visuals.mesh.vertices {
            let p = vertex.pos;
            vertex.pos = match format.writing {
                Writing::SidewaysLr => pos2(origin.y + p.y, size.y - origin.x - p.x),
                writing if writing.reverse() => pos2(size.x - origin.y - p.y, origin.x + p.x),
                _ => pos2(origin.y + row.size.y - p.y, origin.x + p.x),
            };
        }
        if matches!(format.writing, Writing::VerticalRl | Writing::VerticalLr)
            && format.text_orientation != "sideways"
        {
            for glyph in &row.glyphs {
                if glyph.uv_rect.is_nothing()
                    || format.text_orientation == "mixed" && !upright_character(glyph.chr)
                {
                    continue;
                }
                let column = if format.writing.reverse() {
                    size.x - origin.y - row.size.y
                } else {
                    origin.y
                };
                let cell = glyph.logical_rect();
                let target = pos2(
                    column + (row.size.y - glyph.advance_width) * 0.5,
                    origin.x + glyph.pos.x,
                );
                let start = glyph.first_vertex as usize;
                for (vertex, original) in row
                    .visuals
                    .mesh
                    .vertices
                    .iter_mut()
                    .zip(&original)
                    .skip(start)
                    .take(4)
                {
                    vertex.pos = original.pos + (target - cell.min);
                }
            }
        }
        row.visuals.mesh_bounds = row
            .visuals
            .mesh
            .vertices
            .iter()
            .fold(Rect::NOTHING, |bounds, vertex| {
                bounds.union(Rect::from_min_max(vertex.pos, vertex.pos))
            });
        bounds = bounds.union(row.visuals.mesh_bounds);
        row.size = vec2(row.size.y, row.size.x);
        placed.pos = Pos2::ZERO;
    }
    galley.rect = Rect::from_min_size(Pos2::ZERO, vec2(galley.rect.height(), galley.rect.width()));
    galley.mesh_bounds = bounds;
    text.angle = 0.0;
}

fn upright_character(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x11ff | 0x2e80..=0xa4cf | 0xac00..=0xd7ff |
        0xf900..=0xfaff | 0xfe10..=0xfe1f | 0xfe30..=0xfe4f |
        0xff01..=0xff60 | 0xffe0..=0xffe6 | 0x1b000..=0x1b2ff |
        0x1f000..=0x1faff | 0x20000..=0x3ffff)
}
