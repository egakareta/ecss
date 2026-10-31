use std::time::Duration;

use egui::{Context, Id, Rect, Ui};

use crate::{animation::ease, layout_style::BoxStyle};

mod config;
mod values;

use config::Config;
pub(crate) use config::{Property, Settings, is_property, parse_property};
use values::{PropertyId, Value};

#[derive(Clone, Debug)]
struct Running {
    from: Value,
    to: Value,
    reversing_start: Value,
    reversing_factor: f64,
    started_at: f64,
    config: Config,
}

impl Running {
    fn progress(&self, now: f64) -> f64 {
        let elapsed = now - self.started_at - self.config.delay;
        if elapsed < 0.0 {
            return 0.0;
        }
        if self.config.duration == 0.0 || elapsed >= self.config.duration {
            return 1.0;
        }
        ease(&self.config.timing, elapsed / self.config.duration, false)
    }

    fn value(&self, now: f64, property: PropertyId) -> Value {
        if self.finished(now) {
            return self.to.clone();
        }
        property.constrain(
            self.from
                .interpolate(&self.to, self.progress(now) as f32)
                .expect("transition endpoints have compatible computed types"),
        )
    }

    fn finished(&self, now: f64) -> bool {
        now >= self.started_at + self.config.delay + self.config.duration
    }

    fn next_repaint(&self, now: f64) -> Duration {
        let start = self.started_at + self.config.delay;
        let seconds = if now < start {
            start - now
        } else {
            (start + self.config.duration - now).min(1.0 / 60.0)
        };
        Duration::from_secs_f64(seconds.clamp(0.000_001, 3600.0))
    }
}

#[derive(Clone, Debug, Default)]
struct State {
    initialized_frame: u64,
    last_frame: u64,
    targets: Vec<Option<Value>>,
    running: Vec<Option<Running>>,
    present: bool,
}

#[derive(Default)]
pub(crate) struct Sample {
    pub(crate) layout_sampled: bool,
    pub(crate) background: bool,
    pub(crate) background_position_x: bool,
    pub(crate) background_position_y: bool,
    color: bool,
    radius: bool,
}

impl Sample {
    pub(crate) fn apply_visuals(&self, ui: &mut Ui, style: &BoxStyle, border: Rect) {
        if !self.color && !self.background && !self.radius {
            return;
        }
        let radii = crate::box_decoration::resolved_radii(style, border)
            .map(|radius| radius.min_elem().round().clamp(0.0, 255.0) as u8);
        let radius = egui::CornerRadius {
            nw: radii[0],
            ne: radii[1],
            se: radii[2],
            sw: radii[3],
        };
        let visuals = &mut ui.style_mut().visuals;
        if self.color {
            visuals.override_text_color = None;
        }
        let widgets = &mut visuals.widgets;
        for widget in [
            &mut widgets.noninteractive,
            &mut widgets.inactive,
            &mut widgets.hovered,
            &mut widgets.active,
            &mut widgets.open,
        ] {
            if self.color {
                widget.fg_stroke.color = style.current_color;
            }
            if self.background {
                widget.bg_fill = style.background_color;
                widget.weak_bg_fill = style.background_color;
            }
            if self.radius {
                widget.corner_radius = radius;
            }
        }
    }
}

pub(crate) fn sample(
    context: &Context,
    id: Id,
    style: &mut BoxStyle,
    units: crate::layout_style::Units<'_>,
) -> Sample {
    let now = context.input(|input| input.time);
    let frame = context.cumulative_frame_nr();
    let state_id = id.with("ecss.transitions");
    let absent = style.layout.display == taffy::Display::None;
    let targets = PropertyId::all()
        .map(|property| property.get(style, units))
        .collect::<Vec<_>>();
    let previous = context.data_mut(|data| data.remove_temp::<State>(state_id));
    let mut state = previous
        .filter(|state| {
            state.present
                && (state.last_frame == frame || state.last_frame.checked_add(1) == Some(frame))
        })
        .unwrap_or_else(|| State {
            initialized_frame: frame,
            last_frame: frame,
            running: vec![None; targets.len()],
            targets: targets.clone(),
            present: !absent,
        });
    let mut sample = Sample::default();
    if absent {
        state.initialized_frame = frame;
    }
    let initializing = state.initialized_frame == frame;
    for (index, property) in PropertyId::all().enumerate() {
        let config = style
            .transitions
            .matching(|name| property.matches(name, style));
        let previous_running = state.running[index].take();
        let was_running = previous_running.is_some();
        let Some(destination) = targets[index].as_ref() else {
            continue;
        };
        let Some(config) = config else {
            continue;
        };
        let changed = state.targets[index].as_ref() != Some(destination);
        let mut running = if initializing { None } else { previous_running };
        if changed && !initializing {
            let from = running
                .as_ref()
                .map(|running| running.value(now, property))
                .or_else(|| state.targets[index].clone());
            running = from
                .filter(|from| from != destination && from.interpolate(destination, 0.0).is_some())
                .and_then(|from| {
                    let mut config = config.clone();
                    let mut reversing_start = from.clone();
                    let mut reversing_factor = 1.0;
                    if let Some(previous) = &running
                        && previous.reversing_start == *destination
                    {
                        reversing_factor = (previous.progress(now) * previous.reversing_factor
                            + 1.0
                            - previous.reversing_factor)
                            .abs()
                            .clamp(0.0, 1.0);
                        config.duration *= reversing_factor;
                        if config.delay < 0.0 {
                            config.delay *= reversing_factor;
                        }
                        reversing_start = previous.to.clone();
                    }
                    (config.duration + config.delay > 0.0).then_some(Running {
                        from,
                        to: destination.clone(),
                        reversing_start,
                        reversing_factor,
                        started_at: now,
                        config,
                    })
                });
        }
        let selected = config.duration + config.delay > 0.0;
        if !absent && (selected || was_running) {
            sample.layout_sampled |=
                property.affects_layout() && (was_running || running.is_some());
            let value = running.as_ref().map_or_else(
                || destination.clone(),
                |running| running.value(now, property),
            );
            property.set(style, &value, units.math);
            match property {
                PropertyId::Color => sample.color = true,
                PropertyId::Background => {
                    sample.background = style.background_set || was_running || changed
                }
                PropertyId::BackgroundPositionX => sample.background_position_x = true,
                PropertyId::BackgroundPositionY => sample.background_position_y = true,
                PropertyId::Radius(_) => {
                    sample.radius |=
                        was_running
                            || changed
                            || style.decorations.radii.iter().any(|radius| {
                                *radius != crate::box_decoration::CornerRadius::default()
                            })
                }
                _ => {}
            }
        }
        if !absent && let Some(running) = running.filter(|running| !running.finished(now)) {
            context.request_repaint_after(running.next_repaint(now));
            state.running[index] = Some(running);
        }
    }
    state.targets = targets;
    state.last_frame = frame;
    state.present = !absent;
    context.data_mut(|data| data.insert_temp(state_id, state));
    sample
}
