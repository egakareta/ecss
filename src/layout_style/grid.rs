use super::*;

pub(super) fn track(value: &str, units: Units) -> Result<TrackSizingFunction, &'static str> {
    match value {
        "*" => return Ok(fr(1.0)),
        "auto" => return Ok(auto()),
        "min-content" => return Ok(min_content()),
        "max-content" => return Ok(max_content()),
        "fit-content" => return Ok(minmax(min_content(), max_content())),
        _ => {}
    }
    if let Some(n) = value.strip_suffix("fr") {
        return Ok(fr(number(n, units)?));
    }
    if let Some(inner) = value
        .strip_prefix("minmax(")
        .and_then(|v| v.strip_suffix(')'))
    {
        let (a, b) = inner.split_once(',').ok_or("invalid minmax")?;
        if a.trim().ends_with("fr") {
            return Err("flexible minimum track");
        }
        return Ok(minmax(
            track(a.trim(), units)?.min,
            track(b.trim(), units)?.max,
        ));
    }
    if let Some(inner) = value
        .strip_prefix("fit-content(")
        .and_then(|v| v.strip_suffix(')'))
    {
        return Ok(fit_content(gap(inner.trim(), units)?));
    }
    if let Some(handle) = crate::math::length(value, units, false, true)? {
        return Ok(taffy::LengthPercentage::calc(handle).into());
    }
    let (v, p) = length_value(value, units, false)?;
    Ok(if p { percent(v) } else { length(v) })
}

pub(super) fn tracks(
    value: &str,
    units: Units,
) -> Result<Vec<GridTemplateComponent<String>>, &'static str> {
    if value == "none" {
        return Ok(Vec::new());
    }
    components(value)?
        .iter()
        .map(|v| {
            if let Some(inner) = v.strip_prefix("repeat(").and_then(|v| v.strip_suffix(')')) {
                let (count, values) = inner.split_once(',').ok_or("invalid repeat")?;
                let tracks: Vec<_> = components(values)?
                    .iter()
                    .map(|v| track(v, units))
                    .collect::<Result<_, _>>()?;
                if tracks.is_empty() {
                    return Err("empty repeat");
                }
                match count.trim() {
                    "auto-fill" => Ok(repeat(taffy::RepetitionCount::AutoFill, tracks)),
                    "auto-fit" => Ok(repeat(taffy::RepetitionCount::AutoFit, tracks)),
                    n => {
                        let count: u16 = n.parse().map_err(|_| "invalid repeat count")?;
                        if count == 0 {
                            return Err("zero repeat count");
                        }
                        Ok(repeat(count, tracks))
                    }
                }
            } else {
                Ok(GridTemplateComponent::Single(track(v, units)?))
            }
        })
        .collect()
}

pub(super) fn placement(value: &str) -> Result<GridPlacement<String>, &'static str> {
    value.parse().map_err(|_| "invalid grid placement")
}
pub(super) fn placement_pair(
    value: &str,
) -> Result<taffy::Line<GridPlacement<String>>, &'static str> {
    let (a, b) = value.split_once('/').unwrap_or((value, "auto"));
    Ok(taffy::Line {
        start: placement(a.trim())?,
        end: placement(b.trim())?,
    })
}

pub(super) fn parse_flow(value: &str) -> Result<Option<String>, &'static str> {
    if value.eq_ignore_ascii_case("normal") {
        return Ok(None);
    }
    if value == "*" {
        return Ok(Some(value.to_owned()));
    }
    let mut parser = Parser::new(value);
    let name = parser
        .expect_ident_cloned()
        .map_err(|_| "invalid flow name")?;
    parser.expect_exhausted().map_err(|_| "invalid flow name")?;
    Ok(Some(name.to_string()))
}

pub(super) fn parse_chains(value: &str) -> Result<Vec<Vec<String>>, &'static str> {
    if value.eq_ignore_ascii_case("none") {
        return Ok(Vec::new());
    }
    let mut chains = vec![Vec::new()];
    let mut names = std::collections::HashSet::new();
    for part in components(value)? {
        if part == "," {
            if chains.last().is_none_or(Vec::is_empty) {
                return Err("empty grid chain");
            }
            chains.push(Vec::new());
            continue;
        }
        let Some(name) = parse_flow(&part)? else {
            return Err("invalid grid chain name");
        };
        if !names.insert(name.clone()) {
            return Err("duplicate grid chain name");
        }
        chains.last_mut().unwrap().push(name);
    }
    if chains.last().is_none_or(Vec::is_empty) {
        return Err("empty grid chain");
    }
    Ok(chains)
}

pub(super) type NamedTracks = (Vec<GridTemplateComponent<String>>, Vec<Vec<String>>);
pub(super) fn named_tracks(value: &str, units: Units) -> Result<NamedTracks, &'static str> {
    if value.contains('[') {
        let parsed: taffy::GridTemplateTracks<String, GridTemplateComponent<String>> =
            value.parse().map_err(|_| "invalid named grid tracks")?;
        Ok((parsed.tracks, parsed.line_names))
    } else {
        Ok((tracks(value, units)?, Vec::new()))
    }
}

pub(super) fn apply_grid_shorthand(
    value: &str,
    style: &mut Style,
    format: &mut crate::formatting::Formatting,
    units: Units,
) -> Result<(), &'static str> {
    apply_grid_template("none", style, format, units)?;
    style.grid_auto_rows.clear();
    style.grid_auto_columns.clear();
    style.grid_auto_flow = GridAutoFlow::Row;
    if value == "none" {
        return Ok(());
    }
    let parts = components(value)?;
    let slash = parts.iter().position(|part| part == "/");
    if slash.is_some_and(|index| parts[index + 1..].contains(&"/".to_owned())) {
        return Err("invalid grid slash");
    }
    let Some(slash) = slash else {
        return apply_grid_template(value, style, format, units);
    };
    let first = &parts[..slash];
    let second = &parts[slash + 1..];
    if first.is_empty() || second.is_empty() {
        return Err("invalid grid shorthand");
    }
    let first_flow = first
        .iter()
        .filter(|part| part.as_str() == "auto-flow")
        .count();
    let second_flow = second
        .iter()
        .filter(|part| part.as_str() == "auto-flow")
        .count();
    if first_flow + second_flow > 1 {
        return Err("duplicate auto-flow");
    }
    if first_flow == 0 && second_flow == 0 {
        return apply_grid_template(value, style, format, units);
    }
    let parse_auto_tracks =
        |values: &[String]| -> Result<(Vec<TrackSizingFunction>, bool), &'static str> {
            let mut dense = false;
            let mut tracks = Vec::new();
            let mut flow_seen = false;
            for value in values {
                match value.as_str() {
                    "auto-flow" if !flow_seen => flow_seen = true,
                    "dense" if !dense => dense = true,
                    "auto-flow" | "dense" => return Err("duplicate grid auto-flow keyword"),
                    _ => tracks.push(value.clone()),
                }
            }
            if !flow_seen {
                return Err("missing auto-flow");
            }
            if tracks.is_empty() {
                tracks.push("auto".to_owned());
            }
            Ok((
                tracks
                    .iter()
                    .map(|track_value| track(track_value, units))
                    .collect::<Result<_, _>>()?,
                dense,
            ))
        };
    if first_flow == 1 {
        set_grid_template_axis(&second.join(" "), 0, style, format, units)?;
        let (tracks, dense) = parse_auto_tracks(first)?;
        style.grid_auto_rows = tracks;
        style.grid_auto_flow = if dense {
            GridAutoFlow::RowDense
        } else {
            GridAutoFlow::Row
        };
    } else {
        set_grid_template_axis(&first.join(" "), 1, style, format, units)?;
        let (tracks, dense) = parse_auto_tracks(second)?;
        style.grid_auto_columns = tracks;
        style.grid_auto_flow = if dense {
            GridAutoFlow::ColumnDense
        } else {
            GridAutoFlow::Column
        };
    }
    Ok(())
}

pub(super) fn apply_legacy_grid_dimensions(
    column_value: &str,
    row_value: &str,
    style: &mut Style,
    units: Units,
) -> Result<(), &'static str> {
    let columns = tracks(column_value, units)?;
    let rows = tracks(row_value, units)?;
    if columns.is_empty() || rows.is_empty() {
        return Err("grid template requires at least one row and column");
    }
    let column_count = components(column_value)?.len();
    let row_count = components(row_value)?.len();
    let cells = vec!["*"; column_count].join(" ");
    let area_rows = (0..row_count)
        .map(|_| format!("\"{cells}\""))
        .collect::<Vec<_>>()
        .join(" ");
    style.grid_template_columns = columns;
    style.grid_template_rows = rows;
    style.grid_template_areas = areas(&area_rows)?;
    Ok(())
}

pub(super) fn apply_legacy_grid_areas(
    parts: &[String],
    area_start: usize,
    style: &mut Style,
    format: &mut crate::formatting::Formatting,
    units: Units,
) -> Result<(), &'static str> {
    let columns = tracks(&parts[..area_start].join(" "), units)?;
    if columns.is_empty() {
        return Err("grid template requires at least one column");
    }
    let mut area_rows = Vec::new();
    let mut row_sizes = Vec::new();
    let mut index = area_start;
    while index < parts.len() {
        if quoted_string(&parts[index]).is_none() {
            return Err("expected grid area row");
        }
        area_rows.push(parts[index].clone());
        index += 1;
        if parts
            .get(index)
            .is_some_and(|part| quoted_string(part).is_none())
        {
            row_sizes.push(parts[index].clone());
            index += 1;
        } else {
            row_sizes.push("fit-content".to_owned());
        }
    }
    style.grid_template_areas = areas(&area_rows.join(" "))?;
    style.grid_template_columns = columns;
    set_grid_template_axis(&row_sizes.join(" "), 1, style, format, units)
}

pub(super) fn apply_grid_template(
    value: &str,
    style: &mut Style,
    format: &mut crate::formatting::Formatting,
    units: Units,
) -> Result<(), &'static str> {
    style.grid_template_rows.clear();
    style.grid_template_columns.clear();
    style.grid_template_row_names.clear();
    style.grid_template_column_names.clear();
    style.grid_template_areas = None;
    format.subgrid = [false; 2];
    format.subgrid_names = Default::default();
    if value == "none" {
        return Ok(());
    }

    let parts = components(value)?;
    let slash = parts.iter().position(|part| part == "/");
    if slash.is_some_and(|index| parts[index + 1..].contains(&"/".to_owned())) {
        return Err("invalid grid-template slash");
    }
    let rows = &parts[..slash.unwrap_or(parts.len())];
    let columns = slash.map(|index| &parts[index + 1..]);
    let area_start = rows.iter().position(|part| quoted_string(part).is_some());
    if let Some(area_start) = area_start
        && area_start > 0
        && slash.is_none()
    {
        return apply_legacy_grid_areas(rows, area_start, style, format, units);
    }
    let legacy_dimension_syntax = parts
        .iter()
        .any(|part| matches!(part.as_str(), "*" | "fit-content"));
    if area_start.is_none()
        && legacy_dimension_syntax
        && let Some(columns) = columns
    {
        return apply_legacy_grid_dimensions(&rows.join(" "), &columns.join(" "), style, units);
    }
    if rows.is_empty() || columns.is_some_and(|columns| columns.is_empty()) {
        return Err("invalid grid-template");
    }
    let area_rows: Vec<_> = rows
        .iter()
        .filter(|part| quoted_string(part).is_some())
        .cloned()
        .collect();
    let (row_tracks, row_names) = if !area_rows.is_empty() {
        let area_value = area_rows.join(" ");
        style.grid_template_areas = areas(&area_value)?;
        let mut row_parts = Vec::new();
        let mut skip_track = None;
        for (index, part) in rows.iter().enumerate() {
            if skip_track == Some(index) {
                continue;
            }
            if quoted_string(part).is_some() {
                let next = index + 1;
                if rows
                    .get(next)
                    .is_some_and(|next| !next.starts_with('[') && quoted_string(next).is_none())
                {
                    row_parts.push(rows[next].clone());
                    skip_track = Some(next);
                } else {
                    row_parts.push("auto".to_owned());
                }
            } else {
                row_parts.push(part.clone());
            }
        }
        let rows_value = row_parts.join(" ");
        set_grid_template_axis(&rows_value, 1, style, format, units)?;
        (
            style.grid_template_rows.clone(),
            style.grid_template_row_names.clone(),
        )
    } else if columns.is_some() {
        let rows_value = rows.join(" ");
        set_grid_template_axis(&rows_value, 1, style, format, units)?;
        (
            style.grid_template_rows.clone(),
            style.grid_template_row_names.clone(),
        )
    } else {
        return Err("grid-template requires a slash or area strings");
    };
    style.grid_template_rows = row_tracks;
    style.grid_template_row_names = row_names;

    if let Some(columns) = columns {
        set_grid_template_axis(&columns.join(" "), 0, style, format, units)?;
    }
    Ok(())
}

pub(super) fn set_grid_template_axis(
    value: &str,
    axis: usize,
    style: &mut Style,
    format: &mut crate::formatting::Formatting,
    units: Units,
) -> Result<(), &'static str> {
    let parts = components(value)?;
    if parts.first().is_some_and(|part| part == "subgrid") {
        format.subgrid[axis] = true;
        let mut names = Vec::new();
        for part in &parts[1..] {
            let inner = part
                .strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
                .ok_or("expected subgrid line names")?;
            let mut parser = Parser::new(inner);
            let mut line_names = Vec::new();
            while !parser.is_exhausted() {
                line_names.push(
                    parser
                        .expect_ident_cloned()
                        .map_err(|_| "invalid subgrid line name")?
                        .to_string(),
                );
            }
            names.push(line_names);
        }
        format.subgrid_names[axis] = names;
        return Ok(());
    }
    let (tracks, names) = named_tracks(value, units)?;
    if axis == 0 {
        style.grid_template_columns = tracks;
        style.grid_template_column_names = names;
    } else {
        style.grid_template_rows = tracks;
        style.grid_template_row_names = names;
    }
    Ok(())
}

pub(super) fn quoted_string(value: &str) -> Option<String> {
    let mut parser = Parser::new(value);
    let string = parser.expect_string().ok()?.to_string();
    parser.expect_exhausted().ok()?;
    Some(string)
}

pub(super) fn areas(value: &str) -> Result<Option<taffy::GridTemplateAreas<String>>, &'static str> {
    if value == "none" {
        return Ok(None);
    }
    let mut parser = Parser::new(value);
    let mut rows = Vec::new();
    while !parser.is_exhausted() {
        let row = parser
            .expect_string()
            .map_err(|_| "expected grid area row")?;
        let cells: Vec<String> = row.split_whitespace().map(str::to_owned).collect();
        if cells.is_empty() || cells.len() > u16::MAX as usize {
            return Err("invalid grid area row");
        }
        rows.push(cells);
    }
    if rows.is_empty() || rows.len() > u16::MAX as usize {
        return Err("invalid grid template areas");
    }
    let columns = rows[0].len();
    let mut areas: Vec<(String, (usize, usize, usize, usize))> = Vec::new();
    for (y, row) in rows.iter().enumerate() {
        if row.len() != columns {
            return Err("unequal grid template rows");
        }
        for (x, name) in row.iter().enumerate() {
            if name.chars().all(|c| c == '.') {
                continue;
            }
            let area = if let Some((_, area)) = areas.iter_mut().find(|(area, _)| area == name) {
                area
            } else {
                areas.push((name.clone(), (x, y, x + 1, y + 1)));
                &mut areas.last_mut().unwrap().1
            };
            area.0 = area.0.min(x);
            area.1 = area.1.min(y);
            area.2 = area.2.max(x + 1);
            area.3 = area.3.max(y + 1);
        }
    }
    let mut result = Vec::new();
    for (name, (left, top, right, bottom)) in areas {
        if rows[top..bottom]
            .iter()
            .any(|row| row[left..right].iter().any(|cell| *cell != name))
        {
            return Err("nonrectangular grid area");
        }
        result.push(taffy::GridTemplateArea {
            name,
            column_start: left as u16 + 1,
            column_end: right as u16 + 1,
            row_start: top as u16 + 1,
            row_end: bottom as u16 + 1,
        });
    }
    Ok(Some(taffy::GridTemplateAreas {
        areas: result,
        row_count: rows.len() as u16,
        column_count: columns as u16,
    }))
}
