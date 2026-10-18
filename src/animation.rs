use std::{collections::HashMap, time::Duration};

use cssparser::{Parser as CssParser, ToCss, Token};
use egui::{Color32, Context, Id};

use crate::css::{Declaration, Keyframe};

const PROPERTIES: [&str; 9] = [
    "animation",
    "animation-delay",
    "animation-direction",
    "animation-duration",
    "animation-fill-mode",
    "animation-iteration-count",
    "animation-name",
    "animation-play-state",
    "animation-timing-function",
];

#[derive(Clone, Debug)]
pub(crate) struct Property {
    pub(crate) name: String,
    pub(crate) value: String,
}

pub(crate) struct Sample {
    pub(crate) declarations: Vec<Declaration>,
    pub(crate) layout_active: bool,
    pub(crate) can_defer_layout: bool,
}

#[derive(Clone, Copy, Debug)]
enum Direction {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

#[derive(Clone, Copy, Debug)]
enum FillMode {
    None,
    Forwards,
    Backwards,
    Both,
}

#[derive(Clone, Copy, Debug)]
enum PlayState {
    Running,
    Paused,
}

#[derive(Clone, Copy, Debug)]
enum Iterations {
    Count(f64),
    Infinite,
}

#[derive(Clone, Copy, Debug)]
enum StepPosition {
    Start,
    End,
    JumpStart,
    JumpEnd,
    JumpNone,
    JumpBoth,
}

#[derive(Clone, Copy, Debug)]
struct LinearPoint {
    input: f64,
    output: f64,
}

#[derive(Clone, Debug)]
enum TimingFunction {
    Linear,
    LinearFunction(Vec<LinearPoint>),
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier(f64, f64, f64, f64),
    Steps(f64, StepPosition),
}

#[derive(Clone, Debug)]
struct Config {
    name: String,
    duration: f64,
    delay: f64,
    iterations: Iterations,
    direction: Direction,
    fill: FillMode,
    play_state: PlayState,
    timing: TimingFunction,
}

#[derive(Clone, Debug)]
struct Clock {
    key: String,
    started_at: f64,
    paused_at: Option<f64>,
}

pub(crate) fn is_property(name: &str) -> bool {
    PROPERTIES.contains(&name.to_ascii_lowercase().as_str())
}

pub(crate) fn parse_property(name: &str, value: &str) -> Result<Property, &'static str> {
    let name = name.to_ascii_lowercase();
    if value.trim().eq_ignore_ascii_case("revert-layer") {
        return Err("cascade layers are unsupported");
    }
    if is_css_wide(value) {
        return Ok(Property {
            name,
            value: value.trim().to_owned(),
        });
    }
    if name == "animation" {
        expand_shorthand(value)?;
    } else {
        let values = split_list(value);
        if values.is_empty() || values.iter().any(|value| value.is_empty()) {
            return Err("empty animation value");
        }
        for value in values {
            validate_longhand(&name, value)?;
        }
    }
    Ok(Property {
        name,
        value: value.trim().to_owned(),
    })
}

fn validate_longhand(name: &str, value: &str) -> Result<(), &'static str> {
    match name {
        "animation-delay" => parse_time(value)
            .map(|_| ())
            .ok_or("invalid animation delay"),
        "animation-duration" => parse_time(value)
            .filter(|duration| *duration >= 0.0)
            .map(|_| ())
            .ok_or("invalid animation duration"),
        "animation-direction" => match value.to_ascii_lowercase().as_str() {
            "normal" | "reverse" | "alternate" | "alternate-reverse" => Ok(()),
            _ => Err("invalid animation direction"),
        },
        "animation-fill-mode" => match value.to_ascii_lowercase().as_str() {
            "none" | "forwards" | "backwards" | "both" => Ok(()),
            _ => Err("invalid animation fill mode"),
        },
        "animation-iteration-count" => {
            if value.eq_ignore_ascii_case("infinite") {
                return Ok(());
            }
            value
                .parse::<f64>()
                .ok()
                .filter(|count| count.is_finite() && *count >= 0.0)
                .map(|_| ())
                .ok_or("invalid animation iteration count")
        }
        "animation-name" => {
            if value.eq_ignore_ascii_case("none") || is_animation_name(value) {
                Ok(())
            } else {
                Err("invalid animation name")
            }
        }
        "animation-play-state" => match value.to_ascii_lowercase().as_str() {
            "running" | "paused" => Ok(()),
            _ => Err("invalid animation play state"),
        },
        "animation-timing-function" => parse_timing(value)
            .map(|_| ())
            .ok_or("invalid animation timing function"),
        _ => Err("unsupported animation property"),
    }
}

fn expand_shorthand(value: &str) -> Result<Vec<ConfigValues>, &'static str> {
    split_list(value)
        .into_iter()
        .map(parse_shorthand_group)
        .collect()
}

#[derive(Clone, Debug)]
struct ConfigValues {
    name: String,
    duration: String,
    delay: String,
    iterations: String,
    direction: String,
    fill: String,
    play_state: String,
    timing: String,
}

impl Default for ConfigValues {
    fn default() -> Self {
        Self {
            name: "none".to_owned(),
            duration: "0s".to_owned(),
            delay: "0s".to_owned(),
            iterations: "1".to_owned(),
            direction: "normal".to_owned(),
            fill: "none".to_owned(),
            play_state: "running".to_owned(),
            timing: "ease".to_owned(),
        }
    }
}

fn parse_shorthand_group(group: &str) -> Result<ConfigValues, &'static str> {
    if group.trim().is_empty() {
        return Err("empty animation shorthand item");
    }
    let mut values = ConfigValues::default();
    let mut has_name = false;
    let mut has_duration = false;
    let mut has_delay = false;
    let mut has_timing = false;
    let mut has_iteration = false;
    let mut has_direction = false;
    let mut has_fill = false;
    let mut has_play_state = false;

    let components = crate::layout_style::components(group)?;
    for (index, component) in components.iter().enumerate() {
        let component = component.as_str();
        if let Some(time) = parse_time(component) {
            if time < 0.0 || has_duration {
                if has_delay {
                    return Err("duplicate animation time");
                }
                values.delay = component.to_owned();
                has_delay = true;
            } else {
                values.duration = component.to_owned();
                has_duration = true;
            }
        } else if parse_timing(component).is_some() {
            if has_timing {
                return Err("duplicate animation timing function");
            }
            values.timing = component.to_owned();
            has_timing = true;
        } else if component.eq_ignore_ascii_case("infinite")
            || component
                .parse::<f64>()
                .ok()
                .is_some_and(|count| count.is_finite() && count >= 0.0)
        {
            if has_iteration {
                return Err("duplicate animation iteration count");
            }
            values.iterations = component.to_owned();
            has_iteration = true;
        } else if matches_ignore_ascii_case(
            component,
            &["normal", "reverse", "alternate", "alternate-reverse"],
        ) {
            if has_direction {
                return Err("duplicate animation direction");
            }
            values.direction = component.to_owned();
            has_direction = true;
        } else if matches_ignore_ascii_case(component, &["forwards", "backwards", "both"])
            || component.eq_ignore_ascii_case("none")
                && (has_name
                    || components[index + 1..]
                        .iter()
                        .any(|next| is_custom_ident(next)))
        {
            if has_fill {
                return Err("duplicate animation fill mode");
            }
            values.fill = component.to_owned();
            has_fill = true;
        } else if matches_ignore_ascii_case(component, &["running", "paused"]) {
            if has_play_state {
                return Err("duplicate animation play state");
            }
            values.play_state = component.to_owned();
            has_play_state = true;
        } else if component.eq_ignore_ascii_case("none") || is_custom_ident(component) {
            if has_name {
                return Err("duplicate animation name");
            }
            values.name = component.to_owned();
            has_name = true;
        } else {
            return Err("invalid animation shorthand");
        }
    }

    for (name, value) in [
        ("animation-name", values.name.as_str()),
        ("animation-duration", values.duration.as_str()),
        ("animation-delay", values.delay.as_str()),
        ("animation-iteration-count", values.iterations.as_str()),
        ("animation-direction", values.direction.as_str()),
        ("animation-fill-mode", values.fill.as_str()),
        ("animation-play-state", values.play_state.as_str()),
        ("animation-timing-function", values.timing.as_str()),
    ] {
        validate_longhand(name, value)?;
    }
    Ok(values)
}

fn matches_ignore_ascii_case(value: &str, values: &[&str]) -> bool {
    values
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn is_css_wide(value: &str) -> bool {
    matches_ignore_ascii_case(value.trim(), &["initial", "inherit", "unset", "revert"])
}

fn is_custom_ident(value: &str) -> bool {
    if value.is_empty()
        || matches_ignore_ascii_case(
            value,
            &[
                "none",
                "initial",
                "inherit",
                "unset",
                "revert",
                "revert-layer",
                "default",
                "normal",
                "reverse",
                "alternate",
                "alternate-reverse",
                "forwards",
                "backwards",
                "both",
                "running",
                "paused",
                "infinite",
                "ease",
                "ease-in",
                "ease-out",
                "ease-in-out",
                "linear",
                "step-start",
                "step-end",
            ],
        )
    {
        return false;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || matches!(first, '_' | '-'))
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn is_animation_name(value: &str) -> bool {
    !matches_ignore_ascii_case(
        value,
        &[
            "none",
            "initial",
            "inherit",
            "unset",
            "revert",
            "revert-layer",
            "default",
        ],
    ) && is_identifier(value)
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || matches!(first, '_' | '-'))
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn split_list(value: &str) -> Vec<&str> {
    let mut values = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                values.push(value[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    values.push(value[start..].trim());
    values
}

fn parse_time(value: &str) -> Option<f64> {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    if let Some(number) = lower.strip_suffix("ms") {
        let milliseconds = number.trim().parse::<f64>().ok()?;
        milliseconds.is_finite().then_some(milliseconds / 1000.0)
    } else if let Some(number) = lower.strip_suffix('s') {
        let seconds = number.trim().parse::<f64>().ok()?;
        seconds.is_finite().then_some(seconds)
    } else {
        None
    }
}

fn parse_timing(value: &str) -> Option<TimingFunction> {
    let mut input = CssParser::new(value);
    let timing = match input.next().ok()?.clone() {
        Token::Ident(name) => match name.to_ascii_lowercase().as_str() {
            "linear" => TimingFunction::Linear,
            "ease" => TimingFunction::Ease,
            "ease-in" => TimingFunction::EaseIn,
            "ease-out" => TimingFunction::EaseOut,
            "ease-in-out" => TimingFunction::EaseInOut,
            "step-start" => TimingFunction::Steps(1.0, StepPosition::Start),
            "step-end" => TimingFunction::Steps(1.0, StepPosition::End),
            _ => return None,
        },
        Token::Function(name) => input
            .parse_nested_block(|arguments| parse_timing_arguments(&name, arguments))
            .ok()?,
        _ => return None,
    };
    input.expect_exhausted().ok()?;
    Some(timing)
}

fn parse_timing_arguments(
    name: &str,
    input: &mut CssParser<'_>,
) -> Result<TimingFunction, cssparser::ParseError<&'static str>> {
    match name.to_ascii_lowercase().as_str() {
        "linear" => parse_linear_function(input),
        "cubic-bezier" => {
            let x1 = timing_number(input)?;
            input.expect_comma()?;
            let y1 = timing_number(input)?;
            input.expect_comma()?;
            let x2 = timing_number(input)?;
            input.expect_comma()?;
            let y2 = timing_number(input)?;
            input.expect_exhausted()?;
            if !(0.0..=1.0).contains(&x1) || !(0.0..=1.0).contains(&x2) {
                return Err(cssparser::ParseError::custom(
                    "cubic-bezier x coordinates must be between zero and one",
                ));
            }
            Ok(TimingFunction::CubicBezier(x1, y1, x2, y2))
        }
        "steps" => {
            input.skip_whitespace();
            let start = input.position();
            if !matches!(input.next()?, Token::Number { .. }) {
                return Err(cssparser::ParseError::custom(
                    "steps requires a positive integer",
                ));
            }
            let count_text = input.slice(start..input.position()).trim();
            let digits = count_text
                .strip_prefix('+')
                .or_else(|| count_text.strip_prefix('-'))
                .unwrap_or(count_text);
            if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(cssparser::ParseError::custom(
                    "steps requires a positive integer",
                ));
            }
            let count = count_text
                .parse::<f64>()
                .ok()
                .filter(|count| count.is_finite() && *count > 0.0)
                .ok_or_else(|| {
                    cssparser::ParseError::custom("steps requires a positive integer")
                })?;
            let position = if input.try_parse(|input| input.expect_comma()).is_ok() {
                let position = input.expect_ident_cloned()?.to_ascii_lowercase();
                match position.as_str() {
                    "start" => StepPosition::Start,
                    "end" => StepPosition::End,
                    "jump-start" => StepPosition::JumpStart,
                    "jump-end" => StepPosition::JumpEnd,
                    "jump-none" => StepPosition::JumpNone,
                    "jump-both" => StepPosition::JumpBoth,
                    _ => {
                        return Err(cssparser::ParseError::custom("invalid steps position"));
                    }
                }
            } else {
                StepPosition::End
            };
            input.expect_exhausted()?;
            if matches!(position, StepPosition::JumpNone) && count <= 1.0 {
                return Err(cssparser::ParseError::custom(
                    "jump-none requires at least two steps",
                ));
            }
            Ok(TimingFunction::Steps(count, position))
        }
        _ => Err(cssparser::ParseError::custom("unsupported easing function")),
    }
}

fn parse_linear_function(
    input: &mut CssParser<'_>,
) -> Result<TimingFunction, cssparser::ParseError<&'static str>> {
    let mut stops = Vec::new();
    let mut output = None;
    let mut positions = Vec::new();
    let mut after_comma = false;

    while !input.is_exhausted() {
        input.skip_whitespace();
        if input.is_exhausted() {
            break;
        }
        let start = input.position();
        let token = input.next()?.clone();
        match token {
            Token::Number { .. } => {
                if output.is_some() {
                    return Err(cssparser::ParseError::custom(
                        "linear() stop must contain one number",
                    ));
                }
                output = Some(linear_component(
                    input.slice(start..input.position()),
                    false,
                )?);
                after_comma = false;
            }
            Token::Percentage { .. } => {
                if output.is_none() {
                    return Err(cssparser::ParseError::custom(
                        "linear() stop position must follow its number",
                    ));
                }
                if positions.len() == 2 {
                    return Err(cssparser::ParseError::custom(
                        "linear() stop accepts at most two percentages",
                    ));
                }
                positions.push(linear_component(
                    input.slice(start..input.position()),
                    true,
                )?);
                after_comma = false;
            }
            Token::Function(_) => {
                input.parse_nested_block(|arguments| {
                    while arguments.next().is_ok() {}
                    Ok::<_, cssparser::ParseError<&'static str>>(())
                })?;
                let value = input.slice(start..input.position()).trim();
                if output.is_none() {
                    if let Ok(value) = linear_component(value, false) {
                        output = Some(value);
                    } else {
                        return Err(cssparser::ParseError::custom(
                            "linear() stop position must follow its number",
                        ));
                    }
                } else {
                    let value = linear_component(value, true)?;
                    if positions.len() == 2 {
                        return Err(cssparser::ParseError::custom(
                            "linear() stop accepts at most two percentages",
                        ));
                    }
                    positions.push(value);
                }
                after_comma = false;
            }
            Token::Comma => {
                push_linear_stop(&mut stops, output.take(), std::mem::take(&mut positions))?;
                after_comma = true;
            }
            _ => return Err(cssparser::ParseError::custom("invalid linear() stop")),
        }
    }

    if after_comma {
        return Err(cssparser::ParseError::custom(
            "linear() cannot end with a comma",
        ));
    }
    push_linear_stop(&mut stops, output, positions)?;
    if stops.len() < 2 {
        return Err(cssparser::ParseError::custom(
            "linear() requires at least two stops",
        ));
    }

    let last_stop = stops.len() - 1;
    let mut largest_input = f64::NEG_INFINITY;
    let mut points = Vec::new();
    for (index, (output, positions)) in stops.into_iter().enumerate() {
        if positions.is_empty() {
            let input = if index == 0 {
                largest_input = 0.0;
                Some(0.0)
            } else if index == last_stop {
                largest_input = largest_input.max(1.0);
                Some(largest_input)
            } else {
                None
            };
            points.push((input, output));
            continue;
        }

        let input = positions[0].max(largest_input);
        largest_input = input;
        points.push((Some(input), output));
        if let Some(input) = positions.get(1) {
            let input = input.max(largest_input);
            largest_input = input;
            points.push((Some(input), output));
        }
    }

    let mut index = 0;
    while index < points.len() {
        if points[index].0.is_some() {
            index += 1;
            continue;
        }
        let start = index;
        while points.get(index).is_some_and(|point| point.0.is_none()) {
            index += 1;
        }
        let left = points[start - 1].0.expect("linear() has a preceding input");
        let right = points[index].0.expect("linear() has a following input");
        let intervals = (index - start + 1) as f64;
        for (point_index, point) in points.iter_mut().enumerate().take(index).skip(start) {
            let distance = (point_index - start + 1) as f64;
            point.0 = Some(left + (right - left) * distance / intervals);
        }
    }

    Ok(TimingFunction::LinearFunction(
        points
            .into_iter()
            .map(|(input, output)| LinearPoint {
                input: input.expect("all linear() inputs are assigned"),
                output,
            })
            .collect(),
    ))
}

fn push_linear_stop(
    stops: &mut Vec<(f64, Vec<f64>)>,
    output: Option<f64>,
    positions: Vec<f64>,
) -> Result<(), cssparser::ParseError<&'static str>> {
    let output =
        output.ok_or_else(|| cssparser::ParseError::custom("linear() stop needs a number"))?;
    stops.push((output, positions));
    Ok(())
}

fn linear_component(
    value: &str,
    percentage: bool,
) -> Result<f64, cssparser::ParseError<&'static str>> {
    let value = value.trim();
    if percentage {
        if let Some(value) = value.strip_suffix('%') {
            return value
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| value / 100.0)
                .ok_or_else(|| cssparser::ParseError::custom("invalid linear() percentage"));
        }
    } else if !value.contains('(') {
        return value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .ok_or_else(|| cssparser::ParseError::custom("invalid linear() number"));
    }

    let arena = crate::math::Arena::default();
    let units = crate::layout_style::Units {
        math: &arena,
        font: 16.0,
        root_font: 16.0,
        viewport: egui::Vec2::splat(1000.0),
        pixels_per_point: 1.0,
    };
    if percentage {
        let handle = crate::math::percentage(value, units, true)
            .map_err(|_| cssparser::ParseError::custom("invalid linear() percentage"))?
            .ok_or_else(|| {
                cssparser::ParseError::custom("linear() stop position must be a percentage")
            })?;
        Ok(f64::from(crate::math::resolve(handle, 1.0)))
    } else {
        let number = crate::math::number(value, units)
            .map_err(|_| cssparser::ParseError::custom("invalid linear() number"))?
            .ok_or_else(|| cssparser::ParseError::custom("invalid linear() number"))?;
        let number = f64::from(number);
        Ok(if number.is_nan() { 0.0 } else { number })
    }
}

fn timing_number(input: &mut CssParser<'_>) -> Result<f64, cssparser::ParseError<&'static str>> {
    input.skip_whitespace();
    let start = input.position();
    if !matches!(input.next()?, Token::Number { .. }) {
        return Err(cssparser::ParseError::custom(
            "cubic-bezier requires four numbers",
        ));
    }
    input
        .slice(start..input.position())
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| cssparser::ParseError::custom("invalid cubic-bezier number"))
}

fn configs(properties: &[Property]) -> Vec<Config> {
    let mut values: HashMap<String, Vec<String>> = HashMap::new();
    for property in properties {
        if property.name == "animation" {
            if is_css_wide(&property.value) {
                reset_properties(&mut values);
            } else if let Ok(groups) = expand_shorthand(&property.value) {
                values.insert(
                    "animation-name".to_owned(),
                    groups.iter().map(|group| group.name.clone()).collect(),
                );
                values.insert(
                    "animation-duration".to_owned(),
                    groups.iter().map(|group| group.duration.clone()).collect(),
                );
                values.insert(
                    "animation-delay".to_owned(),
                    groups.iter().map(|group| group.delay.clone()).collect(),
                );
                values.insert(
                    "animation-iteration-count".to_owned(),
                    groups
                        .iter()
                        .map(|group| group.iterations.clone())
                        .collect(),
                );
                values.insert(
                    "animation-direction".to_owned(),
                    groups.iter().map(|group| group.direction.clone()).collect(),
                );
                values.insert(
                    "animation-fill-mode".to_owned(),
                    groups.iter().map(|group| group.fill.clone()).collect(),
                );
                values.insert(
                    "animation-play-state".to_owned(),
                    groups
                        .iter()
                        .map(|group| group.play_state.clone())
                        .collect(),
                );
                values.insert(
                    "animation-timing-function".to_owned(),
                    groups.iter().map(|group| group.timing.clone()).collect(),
                );
            }
        } else {
            let property_values = if is_css_wide(&property.value) {
                vec![initial_value(&property.name).to_owned()]
            } else {
                split_list(&property.value)
                    .into_iter()
                    .map(str::to_owned)
                    .collect()
            };
            values.insert(property.name.clone(), property_values);
        }
    }

    let names = values
        .get("animation-name")
        .cloned()
        .unwrap_or_else(|| vec!["none".to_owned()]);
    names
        .iter()
        .enumerate()
        .map(|(index, name)| Config {
            name: name.clone(),
            duration: selected(&values, "animation-duration", index, "0s")
                .and_then(parse_time)
                .unwrap_or(0.0),
            delay: selected(&values, "animation-delay", index, "0s")
                .and_then(parse_time)
                .unwrap_or(0.0),
            iterations: selected(&values, "animation-iteration-count", index, "1")
                .and_then(parse_iterations)
                .unwrap_or(Iterations::Count(1.0)),
            direction: selected(&values, "animation-direction", index, "normal")
                .and_then(parse_direction)
                .unwrap_or(Direction::Normal),
            fill: selected(&values, "animation-fill-mode", index, "none")
                .and_then(parse_fill)
                .unwrap_or(FillMode::None),
            play_state: selected(&values, "animation-play-state", index, "running")
                .and_then(parse_play_state)
                .unwrap_or(PlayState::Running),
            timing: selected(&values, "animation-timing-function", index, "ease")
                .and_then(parse_timing)
                .unwrap_or(TimingFunction::Ease),
        })
        .collect()
}

fn reset_properties(values: &mut HashMap<String, Vec<String>>) {
    for name in [
        "animation-name",
        "animation-duration",
        "animation-delay",
        "animation-iteration-count",
        "animation-direction",
        "animation-fill-mode",
        "animation-play-state",
        "animation-timing-function",
    ] {
        values.insert(name.to_owned(), vec![initial_value(name).to_owned()]);
    }
}

fn initial_value(name: &str) -> &'static str {
    match name {
        "animation-delay" | "animation-duration" => "0s",
        "animation-direction" => "normal",
        "animation-fill-mode" => "none",
        "animation-iteration-count" => "1",
        "animation-name" => "none",
        "animation-play-state" => "running",
        "animation-timing-function" => "ease",
        _ => "none",
    }
}

fn selected<'a>(
    values: &'a HashMap<String, Vec<String>>,
    name: &str,
    index: usize,
    default: &'a str,
) -> Option<&'a str> {
    values
        .get(name)
        .and_then(|values| values.get(index % values.len().max(1)))
        .map(String::as_str)
        .or(Some(default))
}

fn parse_iterations(value: &str) -> Option<Iterations> {
    if value.eq_ignore_ascii_case("infinite") {
        Some(Iterations::Infinite)
    } else {
        value
            .parse::<f64>()
            .ok()
            .filter(|count| count.is_finite() && *count >= 0.0)
            .map(Iterations::Count)
    }
}

fn parse_direction(value: &str) -> Option<Direction> {
    match value.to_ascii_lowercase().as_str() {
        "normal" => Some(Direction::Normal),
        "reverse" => Some(Direction::Reverse),
        "alternate" => Some(Direction::Alternate),
        "alternate-reverse" => Some(Direction::AlternateReverse),
        _ => None,
    }
}

fn parse_fill(value: &str) -> Option<FillMode> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Some(FillMode::None),
        "forwards" => Some(FillMode::Forwards),
        "backwards" => Some(FillMode::Backwards),
        "both" => Some(FillMode::Both),
        _ => None,
    }
}

fn parse_play_state(value: &str) -> Option<PlayState> {
    match value.to_ascii_lowercase().as_str() {
        "running" => Some(PlayState::Running),
        "paused" => Some(PlayState::Paused),
        _ => None,
    }
}

pub(crate) fn sample(
    context: &Context,
    id: Id,
    now: f64,
    properties: &[Property],
    keyframes: &HashMap<String, Vec<Keyframe>>,
    underlying: &[Declaration],
) -> Sample {
    if properties.is_empty() {
        return Sample {
            declarations: Vec::new(),
            layout_active: false,
            can_defer_layout: true,
        };
    }

    let configs = configs(properties);
    let state_id = id.with("ecss.animation");
    let previous = context
        .data(|data| data.get_temp::<Vec<Clock>>(state_id))
        .unwrap_or_default();
    let mut clocks = Vec::with_capacity(configs.len());
    let mut declarations = Vec::new();
    let mut layout_active = false;
    let mut can_defer_layout = true;

    for (index, config) in configs.iter().enumerate() {
        if config.name.eq_ignore_ascii_case("none") {
            continue;
        }
        let key = format!("{index}:{}", config.name);
        let mut clock = previous
            .iter()
            .find(|clock| clock.key == key)
            .cloned()
            .unwrap_or(Clock {
                key,
                started_at: now,
                paused_at: None,
            });
        if matches!(config.play_state, PlayState::Paused) {
            clock.paused_at.get_or_insert(now);
        } else if let Some(paused_at) = clock.paused_at.take() {
            clock.started_at += now - paused_at;
        }
        let effective_now = clock.paused_at.unwrap_or(now);
        clocks.push(clock.clone());

        let Some(frames) = keyframes.get(&config.name) else {
            continue;
        };
        if !frames
            .iter()
            .any(|frame| frame.declarations.iter().any(is_interpolable))
        {
            continue;
        }
        let elapsed = effective_now - clock.started_at - config.delay;
        let (progress, active, before) = animation_progress(config, elapsed);
        if matches!(config.play_state, PlayState::Running) {
            if active {
                context.request_repaint_after(Duration::from_millis(16));
            } else if elapsed < 0.0 {
                context.request_repaint_after(Duration::from_secs_f64(
                    (-elapsed).clamp(0.001, 3600.0),
                ));
            }
        }
        let Some(progress) = progress else {
            continue;
        };
        let sampled = sample_keyframes(frames, progress, before, underlying);
        if active {
            can_defer_layout &= sampled.iter().all(|declaration| {
                matches!(
                    declaration,
                    Declaration::Layout(_) | Declaration::Color(_) | Declaration::BorderRadius(_)
                )
            });
            layout_active |= sampled
                .iter()
                .any(|declaration| matches!(declaration, Declaration::Layout(_)))
                && matches!(config.play_state, PlayState::Running);
        }
        declarations.extend(sampled);
    }

    context.data_mut(|data| data.insert_temp(state_id, clocks));
    Sample {
        declarations,
        layout_active,
        can_defer_layout,
    }
}

fn animation_progress(config: &Config, elapsed: f64) -> (Option<f32>, bool, bool) {
    let (count, infinite) = match config.iterations {
        Iterations::Count(count) => (count, false),
        Iterations::Infinite => (f64::INFINITY, true),
    };
    let before = elapsed < 0.0;
    if before && !matches!(config.fill, FillMode::Backwards | FillMode::Both) {
        return (None, false, before);
    }
    if count == 0.0 {
        return (None, false, before);
    }

    let duration = config.duration;
    let (iteration, fraction, active) = if before {
        (0.0, 0.0, false)
    } else if duration == 0.0 {
        (0.0, 1.0, false)
    } else {
        let total = duration * count;
        let finished = !infinite && elapsed >= total;
        if finished && !matches!(config.fill, FillMode::Forwards | FillMode::Both) {
            return (None, false, before);
        }
        if finished {
            let final_iteration = (count.ceil() - 1.0).max(0.0);
            let partial = count - final_iteration;
            (
                final_iteration,
                if partial < 1.0 { partial } else { 1.0 },
                false,
            )
        } else {
            let iteration = (elapsed / duration).floor();
            let fraction = (elapsed / duration - iteration).clamp(0.0, 1.0);
            (iteration, fraction, true)
        }
    };

    let reversed = match config.direction {
        Direction::Normal => false,
        Direction::Reverse => true,
        Direction::Alternate => !(iteration as u64).is_multiple_of(2),
        Direction::AlternateReverse => (iteration as u64).is_multiple_of(2),
    };
    let directed = if reversed { 1.0 - fraction } else { fraction };
    (
        Some(ease(&config.timing, directed, before) as f32),
        active,
        before,
    )
}

fn ease(timing: &TimingFunction, progress: f64, before: bool) -> f64 {
    match timing {
        TimingFunction::Linear => progress,
        TimingFunction::LinearFunction(points) => linear_function(points, progress, before),
        TimingFunction::Ease => cubic_bezier(0.25, 0.1, 0.25, 1.0, progress),
        TimingFunction::EaseIn => cubic_bezier(0.42, 0.0, 1.0, 1.0, progress),
        TimingFunction::EaseOut => cubic_bezier(0.0, 0.0, 0.58, 1.0, progress),
        TimingFunction::EaseInOut => cubic_bezier(0.42, 0.0, 0.58, 1.0, progress),
        TimingFunction::CubicBezier(x1, y1, x2, y2) => cubic_bezier(*x1, *y1, *x2, *y2, progress),
        TimingFunction::Steps(count, position) => {
            let count = *count;
            let scaled = progress * count;
            let mut current_step = scaled.floor();
            if matches!(
                *position,
                StepPosition::Start | StepPosition::JumpStart | StepPosition::JumpBoth
            ) {
                current_step += 1.0;
            }
            if before && scaled.fract() == 0.0 {
                current_step -= 1.0;
            }
            if progress >= 0.0 && current_step < 0.0 {
                current_step = 0.0;
            }
            let jumps = match *position {
                StepPosition::JumpNone => count - 1.0,
                StepPosition::JumpBoth => count + 1.0,
                StepPosition::Start
                | StepPosition::End
                | StepPosition::JumpStart
                | StepPosition::JumpEnd => count,
            };
            if progress <= 1.0 && current_step > jumps {
                current_step = jumps;
            }
            current_step / jumps
        }
    }
}

fn linear_function(points: &[LinearPoint], progress: f64, before: bool) -> f64 {
    if points.len() == 1 {
        return points[0].output;
    }
    if before && progress == points[0].input {
        return points[0].output;
    }

    let last_matching = points.partition_point(|point| point.input <= progress);
    if last_matching > 0 && points[last_matching - 1].input == progress {
        return points[last_matching - 1].output;
    }

    let (point_a, point_b) = if last_matching == 0 {
        (points[0], points[1])
    } else if last_matching == points.len() {
        (points[points.len() - 2], points[points.len() - 1])
    } else {
        (points[last_matching - 1], points[last_matching])
    };
    if point_a.input == point_b.input {
        return if progress < point_a.input {
            point_a.output
        } else {
            point_b.output
        };
    }
    let progress_between_points = (progress - point_a.input) / (point_b.input - point_a.input);
    point_a.output + progress_between_points * (point_b.output - point_a.output)
}

fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, x: f64) -> f64 {
    let sample = |a: f64, b: f64, t: f64| {
        3.0 * (1.0 - t).powi(2) * t * a + 3.0 * (1.0 - t) * t.powi(2) * b + t.powi(3)
    };
    if x == 0.0 {
        return 0.0;
    }
    if x == 1.0 {
        return 1.0;
    }
    if x < 0.0 {
        if x1 > 0.0 {
            let slope = y1 / x1;
            return if slope == 0.0 { 0.0 } else { x * slope };
        }
        if x2 > 0.0 {
            let slope = y2 / x2;
            return if slope == 0.0 { 0.0 } else { x * slope };
        }
        return 0.0;
    }
    if x > 1.0 {
        if x2 < 1.0 {
            let slope = (1.0 - y2) / (1.0 - x2);
            return if slope == 0.0 {
                1.0
            } else {
                1.0 + (x - 1.0) * slope
            };
        }
        if x1 < 1.0 {
            let slope = (1.0 - y1) / (1.0 - x1);
            return if slope == 0.0 {
                1.0
            } else {
                1.0 + (x - 1.0) * slope
            };
        }
        return 1.0;
    }
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..24 {
        let middle = (low + high) * 0.5;
        if sample(x1, x2, middle) < x {
            low = middle;
        } else {
            high = middle;
        }
    }
    sample(y1, y2, (low + high) * 0.5)
}

fn sample_keyframes(
    frames: &[Keyframe],
    progress: f32,
    before: bool,
    underlying: &[Declaration],
) -> Vec<Declaration> {
    let mut properties: Vec<(String, Vec<(f32, Declaration)>)> = Vec::new();
    for frame in frames {
        for declaration in &frame.declarations {
            let Some((name, declaration)) = normalized_declaration(declaration) else {
                continue;
            };
            let anchors =
                if let Some((_, anchors)) = properties.iter_mut().find(|(key, _)| *key == name) {
                    anchors
                } else {
                    properties.push((name, Vec::new()));
                    &mut properties.last_mut().expect("property was inserted").1
                };
            if let Some((_, existing)) = anchors
                .iter_mut()
                .find(|(offset, _)| *offset == frame.offset)
            {
                *existing = declaration;
            } else {
                anchors.push((frame.offset, declaration));
            }
        }
    }

    let mut result = Vec::new();
    for (name, mut anchors) in properties {
        anchors.sort_by(|first, second| first.0.total_cmp(&second.0));
        let base = underlying
            .iter()
            .filter_map(normalized_declaration)
            .filter(|(property, _)| *property == name)
            .map(|(_, declaration)| declaration)
            .next_back();
        if let Some(base) = base {
            if anchors.first().is_some_and(|(offset, _)| *offset > 0.0) {
                anchors.insert(0, (0.0, base.clone()));
            }
            if anchors.last().is_some_and(|(offset, _)| *offset < 1.0) {
                anchors.push((1.0, base));
            }
        }
        let Some((first_offset, _)) = anchors.first() else {
            continue;
        };
        if progress < *first_offset && anchors.len() < 2 {
            continue;
        }
        let pair = if progress < *first_offset {
            Some((&anchors[0], &anchors[1]))
        } else if progress > anchors.last().expect("anchors are non-empty").0 {
            Some((
                &anchors[anchors.len() - 2],
                anchors.last().expect("anchors are non-empty"),
            ))
        } else {
            let mut previous = &anchors[0];
            let mut next = None;
            for anchor in anchors.iter().skip(1) {
                if progress < anchor.0 {
                    next = Some((previous, anchor));
                    break;
                }
                previous = anchor;
            }
            next
        };
        let sampled = if let Some((previous, next)) = pair {
            let span = next.0 - previous.0;
            let local = if span <= 0.0 {
                1.0
            } else {
                (progress - previous.0) / span
            };
            let local = keyframe_timing(frames, previous.0)
                .map(|timing| ease(&timing, f64::from(local), before) as f32)
                .unwrap_or(local);
            interpolate(&previous.1, &next.1, local)
        } else {
            anchors.last().expect("anchors are non-empty").1.clone()
        };
        result.push(sampled);
    }
    result
}

fn keyframe_timing(frames: &[Keyframe], offset: f32) -> Option<TimingFunction> {
    frames
        .iter()
        .rev()
        .find(|frame| frame.offset == offset)
        .and_then(|frame| frame.timing_function.as_deref())
        .and_then(parse_timing)
}

fn normalized_declaration(declaration: &Declaration) -> Option<(String, Declaration)> {
    match declaration {
        Declaration::Color(_) => Some(("color".to_owned(), declaration.clone())),
        Declaration::Background(_) => Some(("background-color".to_owned(), declaration.clone())),
        Declaration::BackgroundShorthand(background) => Some((
            "background-color".to_owned(),
            Declaration::Background(background.color),
        )),
        Declaration::BackgroundImage(_)
        | Declaration::BackgroundAttachment(_)
        | Declaration::BackgroundClip(_)
        | Declaration::BackgroundOrigin(_)
        | Declaration::BackgroundPosition(_)
        | Declaration::BackgroundRepeat(_)
        | Declaration::BackgroundSize(_)
        | Declaration::FontFamily(_)
        | Declaration::FontWeight(_)
        | Declaration::FontStyle(_) => None,
        Declaration::Layout(layout) => Some((layout.name.clone(), declaration.clone())),
        Declaration::BorderRadius(_) => Some(("border-radius".to_owned(), declaration.clone())),
        Declaration::Animation(_)
        | Declaration::CustomProperty { .. }
        | Declaration::Deferred { .. } => None,
    }
}

fn is_interpolable(declaration: &Declaration) -> bool {
    matches!(
        declaration,
        Declaration::Color(_)
            | Declaration::Background(_)
            | Declaration::BackgroundShorthand(_)
            | Declaration::Layout(_)
            | Declaration::BorderRadius(_)
    )
}

fn interpolate(from: &Declaration, to: &Declaration, progress: f32) -> Declaration {
    match (from, to) {
        (Declaration::Color(from), Declaration::Color(to)) => {
            Declaration::Color(interpolate_color(*from, *to, progress))
        }
        (Declaration::Background(from), Declaration::Background(to)) => {
            Declaration::Background(interpolate_color(*from, *to, progress))
        }
        (Declaration::BorderRadius(from), Declaration::BorderRadius(to)) => {
            Declaration::BorderRadius(
                (f32::from(*from) + (f32::from(*to) - f32::from(*from)) * progress)
                    .round()
                    .clamp(0.0, f32::from(u8::MAX)) as u8,
            )
        }
        (Declaration::Layout(from), Declaration::Layout(to)) if from.name == to.name => {
            let value = interpolate_css_values(&from.name, &from.value, &to.value, progress)
                .unwrap_or_else(|| discrete(&from.value, &to.value, progress));
            Declaration::Layout(crate::layout_style::Declaration {
                name: from.name.clone(),
                value,
            })
        }
        _ => {
            if progress < 0.5 {
                from.clone()
            } else {
                to.clone()
            }
        }
    }
}

fn discrete(from: &str, to: &str, progress: f32) -> String {
    if progress < 0.5 {
        from.to_owned()
    } else {
        to.to_owned()
    }
}

fn interpolate_color(from: Color32, to: Color32, progress: f32) -> Color32 {
    let from_alpha = f32::from(from.a()) / 255.0;
    let to_alpha = f32::from(to.a()) / 255.0;
    let alpha = from_alpha + (to_alpha - from_alpha) * progress;
    let channel = |first: u8, second: u8| {
        if alpha <= f32::EPSILON {
            return 0;
        }
        let from = f32::from(first) / 255.0 * from_alpha;
        let to = f32::from(second) / 255.0 * to_alpha;
        ((from + (to - from) * progress) / alpha * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_unmultiplied(
        channel(from.r(), to.r()),
        channel(from.g(), to.g()),
        channel(from.b(), to.b()),
        (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn parse_color(value: &str) -> Option<Color32> {
    let mut parser = CssParser::new(value);
    crate::css::parse_color(&mut parser).ok()
}

#[derive(Clone, Debug)]
enum CssComponent {
    Number(f32),
    Percentage(f32),
    Dimension(f32, String),
    CalcLengthPercentage {
        length: f32,
        unit: String,
        percentage: f32,
    },
    CalcSum(Vec<(f32, String)>),
    Function(String, Vec<CssComponent>),
    Block(char, char, Vec<CssComponent>),
    Raw(String),
}

fn interpolate_css_values(property: &str, from: &str, to: &str, progress: f32) -> Option<String> {
    if let (Some(from), Some(to)) = (parse_color(from), parse_color(to)) {
        return Some(color_to_css(interpolate_color(from, to, progress)));
    }
    let from = normalize_shorthand(property, from);
    let to = normalize_shorthand(property, to);
    let mut from_parser = CssParser::new(&from);
    let mut to_parser = CssParser::new(&to);
    let from = parse_components(&mut from_parser).ok()?;
    let to = parse_components(&mut to_parser).ok()?;
    let interpolated = interpolate_component_lists(property, &from, &to, progress)?;
    let mut value = String::new();
    for component in interpolated {
        component.write_css(&mut value);
    }
    Some(value)
}

fn parse_components(
    parser: &mut CssParser<'_>,
) -> Result<Vec<CssComponent>, cssparser::ParseError<()>> {
    let mut components = Vec::new();
    while let Ok(token) = parser.next_including_whitespace_and_comments() {
        let token = token.clone();
        components.push(match token {
            Token::Number { value, .. } => CssComponent::Number(value),
            Token::Percentage { unit_value, .. } => CssComponent::Percentage(unit_value),
            Token::Dimension { value, unit, .. } => {
                CssComponent::Dimension(value, unit.to_string())
            }
            Token::Function(name) => CssComponent::Function(
                name.to_string(),
                parser.parse_nested_block(parse_components)?,
            ),
            Token::ParenthesisBlock => {
                CssComponent::Block('(', ')', parser.parse_nested_block(parse_components)?)
            }
            Token::SquareBracketBlock => {
                CssComponent::Block('[', ']', parser.parse_nested_block(parse_components)?)
            }
            Token::CurlyBracketBlock => {
                CssComponent::Block('{', '}', parser.parse_nested_block(parse_components)?)
            }
            token => CssComponent::Raw(token.to_css_string()),
        });
    }
    Ok(components)
}

fn interpolate_component_lists(
    property: &str,
    from: &[CssComponent],
    to: &[CssComponent],
    progress: f32,
) -> Option<Vec<CssComponent>> {
    if from.len() != to.len() {
        return None;
    }
    from.iter()
        .zip(to)
        .map(|(from, to)| interpolate_component(property, from, to, progress))
        .collect()
}

fn interpolate_component(
    property: &str,
    from: &CssComponent,
    to: &CssComponent,
    progress: f32,
) -> Option<CssComponent> {
    let from_color = component_color(from);
    let to_color = component_color(to);
    if let (Some(from), Some(to)) = (from_color, to_color) {
        return Some(CssComponent::Raw(color_to_css(interpolate_color(
            from, to, progress,
        ))));
    }
    match (from, to) {
        (CssComponent::Number(from), CssComponent::Number(to)) => {
            Some(CssComponent::Number(lerp(*from, *to, progress)))
        }
        (CssComponent::Number(0.0), CssComponent::Dimension(to, unit))
            if is_length_property(property) =>
        {
            Some(CssComponent::Dimension(
                lerp(0.0, *to, progress),
                unit.clone(),
            ))
        }
        (CssComponent::Dimension(from, unit), CssComponent::Number(0.0))
            if is_length_property(property) =>
        {
            Some(CssComponent::Dimension(
                lerp(*from, 0.0, progress),
                unit.clone(),
            ))
        }
        (CssComponent::Number(0.0), CssComponent::Percentage(to))
            if is_length_property(property) =>
        {
            Some(CssComponent::Percentage(lerp(0.0, *to, progress)))
        }
        (CssComponent::Percentage(from), CssComponent::Number(0.0))
            if is_length_property(property) =>
        {
            Some(CssComponent::Percentage(lerp(*from, 0.0, progress)))
        }
        (CssComponent::Percentage(from), CssComponent::Percentage(to)) => {
            Some(CssComponent::Percentage(lerp(*from, *to, progress)))
        }
        (CssComponent::Dimension(from, from_unit), CssComponent::Dimension(to, to_unit))
            if dimension_category(from_unit).is_some()
                && dimension_category(from_unit) == dimension_category(to_unit) =>
        {
            interpolate_dimensions(*from, from_unit, *to, to_unit, progress)
        }
        (CssComponent::Dimension(value, unit), CssComponent::Percentage(percentage))
            if is_length_percentage_property(property) && is_length_unit(unit) =>
        {
            Some(CssComponent::CalcLengthPercentage {
                length: lerp(*value, 0.0, progress),
                unit: unit.clone(),
                percentage: lerp(0.0, *percentage, progress),
            })
        }
        (CssComponent::Percentage(percentage), CssComponent::Dimension(value, unit))
            if is_length_percentage_property(property) && is_length_unit(unit) =>
        {
            Some(CssComponent::CalcLengthPercentage {
                length: lerp(0.0, *value, progress),
                unit: unit.clone(),
                percentage: lerp(*percentage, 0.0, progress),
            })
        }
        (CssComponent::Function(from_name, from), CssComponent::Function(to_name, to))
            if from_name.eq_ignore_ascii_case(to_name) =>
        {
            Some(CssComponent::Function(
                from_name.clone(),
                interpolate_component_lists(property, from, to, progress)?,
            ))
        }
        (
            CssComponent::Block(from_open, from_close, from),
            CssComponent::Block(to_open, to_close, to),
        ) if from_open == to_open && from_close == to_close => Some(CssComponent::Block(
            *from_open,
            *from_close,
            interpolate_component_lists(property, from, to, progress)?,
        )),
        (CssComponent::Raw(from), CssComponent::Raw(to)) if from == to => {
            Some(CssComponent::Raw(from.clone()))
        }
        _ => None,
    }
}

fn component_color(component: &CssComponent) -> Option<Color32> {
    let mut value = String::new();
    component.write_css(&mut value);
    parse_color(&value)
}

fn color_to_css(color: Color32) -> String {
    format!(
        "#{:02x}{:02x}{:02x}{:02x}",
        color.r(),
        color.g(),
        color.b(),
        color.a()
    )
}

fn normalize_shorthand(property: &str, value: &str) -> String {
    let (maximum, expand_quad) = match property {
        "margin" | "padding" | "inset" | "border-width" | "border-style" | "border-color" => {
            (4, true)
        }
        "margin-block"
        | "margin-inline"
        | "padding-block"
        | "padding-inline"
        | "inset-block"
        | "inset-inline"
        | "border-block-width"
        | "border-inline-width"
        | "border-block-style"
        | "border-inline-style"
        | "border-block-color"
        | "border-inline-color" => (2, false),
        _ => return value.to_owned(),
    };
    let Ok(components) = crate::layout_style::components(value) else {
        return value.to_owned();
    };
    if components.is_empty() || components.len() > maximum {
        return value.to_owned();
    }
    let expanded = match (expand_quad, components.as_slice()) {
        (true, [all]) => vec![all.clone(), all.clone(), all.clone(), all.clone()],
        (true, [vertical, horizontal]) => vec![
            vertical.clone(),
            horizontal.clone(),
            vertical.clone(),
            horizontal.clone(),
        ],
        (true, [top, horizontal, bottom]) => vec![
            top.clone(),
            horizontal.clone(),
            bottom.clone(),
            horizontal.clone(),
        ],
        (false, [one]) => vec![one.clone(), one.clone()],
        (false, [first, second]) => vec![first.clone(), second.clone()],
        _ => components,
    };
    expanded.join(" ")
}

fn is_length_property(property: &str) -> bool {
    matches!(
        property,
        "width"
            | "height"
            | "inline-size"
            | "block-size"
            | "min-width"
            | "min-height"
            | "min-inline-size"
            | "min-block-size"
            | "max-width"
            | "max-height"
            | "max-inline-size"
            | "max-block-size"
            | "margin"
            | "margin-top"
            | "margin-right"
            | "margin-bottom"
            | "margin-left"
            | "margin-block"
            | "margin-inline"
            | "margin-block-start"
            | "margin-block-end"
            | "margin-inline-start"
            | "margin-inline-end"
            | "padding"
            | "padding-top"
            | "padding-right"
            | "padding-bottom"
            | "padding-left"
            | "padding-block"
            | "padding-inline"
            | "padding-block-start"
            | "padding-block-end"
            | "padding-inline-start"
            | "padding-inline-end"
            | "inset"
            | "inset-block"
            | "inset-inline"
            | "top"
            | "right"
            | "bottom"
            | "left"
            | "gap"
            | "row-gap"
            | "column-gap"
            | "border-width"
            | "border-top-width"
            | "border-right-width"
            | "border-bottom-width"
            | "border-left-width"
            | "border-radius"
            | "border-top-left-radius"
            | "border-top-right-radius"
            | "border-bottom-right-radius"
            | "border-bottom-left-radius"
            | "border-start-start-radius"
            | "border-start-end-radius"
            | "border-end-start-radius"
            | "border-end-end-radius"
            | "font-size"
            | "column-width"
    )
}

fn is_length_percentage_property(property: &str) -> bool {
    is_length_property(property)
        && !matches!(
            property,
            "border-width"
                | "border-top-width"
                | "border-right-width"
                | "border-bottom-width"
                | "border-left-width"
                | "column-width"
        )
}

fn is_length_unit(unit: &str) -> bool {
    dimension_category(unit) == Some(DimensionCategory::Length)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DimensionCategory {
    Length,
    Angle,
    Time,
    Frequency,
    Resolution,
}

fn dimension_category(unit: &str) -> Option<DimensionCategory> {
    Some(match unit.to_ascii_lowercase().as_str() {
        "px" | "in" | "cm" | "mm" | "q" | "pt" | "pc" | "em" | "rem" | "ex" | "rex" | "ch"
        | "rch" | "cap" | "rcap" | "ic" | "ric" | "lh" | "rlh" | "vw" | "vh" | "vi" | "vb"
        | "vmin" | "vmax" | "svw" | "svh" | "svi" | "svb" | "svmin" | "svmax" | "lvw" | "lvh"
        | "lvi" | "lvb" | "lvmin" | "lvmax" | "dvw" | "dvh" | "dvi" | "dvb" | "dvmin" | "dvmax" => {
            DimensionCategory::Length
        }
        "deg" | "grad" | "rad" | "turn" => DimensionCategory::Angle,
        "s" | "ms" => DimensionCategory::Time,
        "hz" | "khz" => DimensionCategory::Frequency,
        "dpi" | "dpcm" | "dppx" | "x" => DimensionCategory::Resolution,
        _ => return None,
    })
}

fn interpolate_dimensions(
    from: f32,
    from_unit: &str,
    to: f32,
    to_unit: &str,
    progress: f32,
) -> Option<CssComponent> {
    if from_unit.eq_ignore_ascii_case(to_unit) {
        return Some(CssComponent::Dimension(
            lerp(from, to, progress),
            from_unit.to_owned(),
        ));
    }
    let from_factor = canonical_unit_factor(from_unit);
    let to_factor = canonical_unit_factor(to_unit);
    if let (Some((from_factor, canonical)), Some((to_factor, to_canonical))) =
        (from_factor, to_factor)
        && canonical == to_canonical
    {
        return Some(CssComponent::Dimension(
            lerp(from * from_factor, to * to_factor, progress),
            canonical.to_owned(),
        ));
    }
    Some(CssComponent::CalcSum(vec![
        (lerp(from, 0.0, progress), from_unit.to_owned()),
        (lerp(0.0, to, progress), to_unit.to_owned()),
    ]))
}

fn canonical_unit_factor(unit: &str) -> Option<(f32, &'static str)> {
    Some(match unit.to_ascii_lowercase().as_str() {
        "px" => (1.0, "px"),
        "in" => (96.0, "px"),
        "cm" => (96.0 / 2.54, "px"),
        "mm" => (96.0 / 25.4, "px"),
        "q" => (96.0 / 101.6, "px"),
        "pt" => (96.0 / 72.0, "px"),
        "pc" => (16.0, "px"),
        "deg" => (1.0, "deg"),
        "grad" => (0.9, "deg"),
        "rad" => (180.0 / std::f32::consts::PI, "deg"),
        "turn" => (360.0, "deg"),
        "s" => (1.0, "s"),
        "ms" => (0.001, "s"),
        "hz" => (1.0, "hz"),
        "khz" => (1000.0, "hz"),
        "dpi" => (1.0 / 96.0, "dppx"),
        "dpcm" => (2.54 / 96.0, "dppx"),
        "dppx" | "x" => (1.0, "dppx"),
        _ => return None,
    })
}

fn lerp(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

impl CssComponent {
    fn write_css(&self, output: &mut String) {
        match self {
            Self::Number(value) => output.push_str(&format_number(f64::from(*value))),
            Self::Percentage(value) => {
                output.push_str(&format_number(f64::from(*value * 100.0)));
                output.push('%');
            }
            Self::Dimension(value, unit) => {
                output.push_str(&format_number(f64::from(*value)));
                output.push_str(unit);
            }
            Self::CalcLengthPercentage {
                length,
                unit,
                percentage,
            } => {
                output.push_str("calc(");
                output.push_str(&format_number(f64::from(*length)));
                output.push_str(unit);
                output.push_str(" + ");
                output.push_str(&format_number(f64::from(*percentage * 100.0)));
                output.push_str("%)");
            }
            Self::CalcSum(components) => {
                output.push_str("calc(");
                for (index, (value, unit)) in components.iter().enumerate() {
                    let magnitude = if *value < 0.0 { -*value } else { *value };
                    if index == 0 {
                        if *value < 0.0 {
                            output.push('-');
                        }
                    } else if *value < 0.0 {
                        output.push_str(" - ");
                    } else {
                        output.push_str(" + ");
                    }
                    output.push_str(&format_number(f64::from(magnitude)));
                    output.push_str(unit);
                }
                output.push(')');
            }
            Self::Function(name, components) => {
                output.push_str(name);
                output.push('(');
                write_components(components, output);
                output.push(')');
            }
            Self::Block(open, close, components) => {
                output.push(*open);
                write_components(components, output);
                output.push(*close);
            }
            Self::Raw(value) => output.push_str(value),
        }
    }
}

fn write_components(components: &[CssComponent], output: &mut String) {
    for component in components {
        component.write_css(output);
    }
}

fn format_number(number: f64) -> String {
    let formatted = format!("{number:.6}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}
