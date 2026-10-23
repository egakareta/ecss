use std::sync::{Arc, LazyLock};

use cssparser::Parser;

use crate::{
    animation::{TimingFunction, parse_time, parse_timing, split_list},
    css::AllKeyword,
};

#[derive(Clone, Debug)]
pub(crate) struct Settings(Arc<Lists>);

#[derive(Clone, Debug)]
struct Lists {
    properties: Vec<String>,
    durations: Vec<f64>,
    delays: Vec<f64>,
    timings: Vec<TimingFunction>,
}

impl Default for Settings {
    fn default() -> Self {
        static INITIAL: LazyLock<Arc<Lists>> = LazyLock::new(|| {
            Arc::new(Lists {
                properties: vec!["all".into()],
                durations: vec![0.0],
                delays: vec![0.0],
                timings: vec![TimingFunction::Ease],
            })
        });
        Self(INITIAL.clone())
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Longhand {
    All,
    Property,
    Duration,
    Delay,
    Timing,
}

#[derive(Clone, Debug)]
pub(crate) enum Property {
    Shorthand(Settings),
    Names(Vec<String>),
    Durations(Vec<f64>),
    Delays(Vec<f64>),
    Timings(Vec<TimingFunction>),
    Wide(Longhand, AllKeyword),
}

#[derive(Clone, Debug)]
pub(super) struct Config {
    pub(super) duration: f64,
    pub(super) delay: f64,
    pub(super) timing: TimingFunction,
}

impl Settings {
    pub(crate) fn apply(&mut self, property: &Property, parent: &Self) {
        if let Property::Shorthand(value) = property {
            *self = value.clone();
            return;
        }
        if let Property::Wide(Longhand::All, keyword) = property {
            *self = if *keyword == AllKeyword::Inherit {
                parent.clone()
            } else {
                Self::default()
            };
            return;
        }
        let output = Arc::make_mut(&mut self.0);
        match property {
            Property::Shorthand(_) => unreachable!(),
            Property::Names(value) => output.properties.clone_from(value),
            Property::Durations(value) => output.durations.clone_from(value),
            Property::Delays(value) => output.delays.clone_from(value),
            Property::Timings(value) => output.timings.clone_from(value),
            Property::Wide(name, keyword) => {
                let initial = Self::default();
                let source = if *keyword == AllKeyword::Inherit {
                    &parent.0
                } else {
                    &initial.0
                };
                match name {
                    Longhand::All => unreachable!(),
                    Longhand::Property => output.properties.clone_from(&source.properties),
                    Longhand::Duration => output.durations.clone_from(&source.durations),
                    Longhand::Delay => output.delays.clone_from(&source.delays),
                    Longhand::Timing => output.timings.clone_from(&source.timings),
                }
            }
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        let values = &self.0;
        values
            .properties
            .iter()
            .enumerate()
            .any(|(index, property)| {
                property != "none"
                    && values.durations[index % values.durations.len()]
                        + values.delays[index % values.delays.len()]
                        > 0.0
            })
    }

    pub(super) fn matching(&self, matches: impl Fn(&str) -> bool) -> Option<Config> {
        let values = &self.0;
        let index = values
            .properties
            .iter()
            .rposition(|name| name == "all" || matches(name))?;
        Some(Config {
            duration: values.durations[index % values.durations.len()],
            delay: values.delays[index % values.delays.len()],
            timing: values.timings[index % values.timings.len()].clone(),
        })
    }
}

pub(crate) fn is_property(name: &str) -> bool {
    longhand(name).is_some()
}

fn longhand(name: &str) -> Option<Longhand> {
    Some(match name.to_ascii_lowercase().as_str() {
        "transition" => Longhand::All,
        "transition-property" => Longhand::Property,
        "transition-duration" => Longhand::Duration,
        "transition-delay" => Longhand::Delay,
        "transition-timing-function" => Longhand::Timing,
        _ => return None,
    })
}

pub(crate) fn parse_property(name: &str, value: &str) -> Result<Property, &'static str> {
    let name = longhand(name).ok_or("unsupported transition property")?;
    let keyword = match value.trim().to_ascii_lowercase().as_str() {
        "inherit" => Some(AllKeyword::Inherit),
        "initial" => Some(AllKeyword::Initial),
        "unset" => Some(AllKeyword::Unset),
        "revert" => Some(AllKeyword::Revert),
        "revert-layer" => return Err("cascade layers are unsupported"),
        _ => None,
    };
    if let Some(keyword) = keyword {
        return Ok(Property::Wide(name, keyword));
    }
    let values = split_list(value);
    if values.iter().any(|value| value.is_empty()) {
        return Err("empty transition value");
    }
    Ok(match name {
        Longhand::All => {
            let mut settings = Lists {
                properties: Vec::new(),
                durations: Vec::new(),
                delays: Vec::new(),
                timings: Vec::new(),
            };
            for value in values {
                let mut property = None;
                let mut duration = None;
                let mut delay = None;
                let mut timing = None;
                for component in crate::layout_style::components(value)? {
                    if let Some(time) = parse_time(&component) {
                        if duration.is_none() {
                            if time < 0.0 {
                                return Err("negative transition duration");
                            }
                            duration = Some(time);
                        } else if delay.replace(time).is_some() {
                            return Err("duplicate transition time");
                        }
                    } else if let Some(parsed) = parse_timing(&component) {
                        if timing.replace(parsed).is_some() {
                            return Err("duplicate transition timing function");
                        }
                    } else if property.replace(property_name(&component)?).is_some() {
                        return Err("duplicate transition property");
                    }
                }
                settings
                    .properties
                    .push(property.unwrap_or_else(|| "all".into()));
                settings.durations.push(duration.unwrap_or(0.0));
                settings.delays.push(delay.unwrap_or(0.0));
                settings
                    .timings
                    .push(timing.unwrap_or(TimingFunction::Ease));
            }
            validate_names(&settings.properties)?;
            Property::Shorthand(Settings(Arc::new(settings)))
        }
        Longhand::Property => {
            let names = values
                .into_iter()
                .map(property_name)
                .collect::<Result<Vec<_>, _>>()?;
            validate_names(&names)?;
            Property::Names(names)
        }
        Longhand::Duration | Longhand::Delay => {
            let times = values
                .into_iter()
                .map(|value| {
                    parse_time(value)
                        .filter(|time| !matches!(name, Longhand::Duration) || *time >= 0.0)
                        .ok_or("invalid transition time")
                })
                .collect::<Result<Vec<_>, _>>()?;
            if matches!(name, Longhand::Duration) {
                Property::Durations(times)
            } else {
                Property::Delays(times)
            }
        }
        Longhand::Timing => Property::Timings(
            values
                .into_iter()
                .map(|value| parse_timing(value).ok_or("invalid transition timing function"))
                .collect::<Result<Vec<_>, _>>()?,
        ),
    })
}

fn property_name(value: &str) -> Result<String, &'static str> {
    let mut parser = Parser::new(value);
    let name = parser
        .expect_ident_cloned()
        .map_err(|_| "invalid transition property name")?;
    parser
        .expect_exhausted()
        .map_err(|_| "invalid transition property name")?;
    if [
        "initial",
        "inherit",
        "unset",
        "revert",
        "revert-layer",
        "default",
    ]
    .iter()
    .any(|keyword| name.eq_ignore_ascii_case(keyword))
    {
        return Err("invalid transition property name");
    }
    Ok(if name.starts_with("--") {
        name.to_string()
    } else {
        name.to_ascii_lowercase()
    })
}

fn validate_names(names: &[String]) -> Result<(), &'static str> {
    if names.len() > 1 && names.iter().any(|name| name == "none") {
        return Err("none must be the only transition property");
    }
    Ok(())
}
