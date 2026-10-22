use super::*;

#[derive(Clone)]
struct InputModality {
    pass: u64,
    keyboard: bool,
}

pub(crate) fn keyboard_modality(context: &Context) -> bool {
    let id = Id::unique("ecss.input-modality").with(context.viewport_id());
    let pass = context.cumulative_pass_nr();
    let previous = context.data(|data| data.get_temp::<InputModality>(id));
    if let Some(previous) = &previous
        && previous.pass == pass
    {
        return previous.keyboard;
    }
    let mut keyboard = previous.is_none_or(|previous| previous.keyboard);
    context.input(|input| {
        for event in &input.raw.events {
            match event {
                egui::Event::Key { pressed: true, .. }
                | egui::Event::Text(_)
                | egui::Event::Paste(_) => keyboard = true,
                egui::Event::PointerButton { pressed: true, .. }
                | egui::Event::Touch {
                    phase: egui::TouchPhase::Start,
                    ..
                } => keyboard = false,
                _ => {}
            }
        }
    });
    context.data_mut(|data| data.insert_temp(id, InputModality { pass, keyboard }));
    keyboard
}

fn focus_state(context: &Context, id: Id) -> u8 {
    if !context.input(|input| input.focused) || !context.memory(|memory| memory.has_focus(id)) {
        return 0;
    }
    let visible = keyboard_modality(context) || egui::TextEdit::load_state(context, id).is_some();
    RuleState::FOCUS.bits()
        | if visible {
            RuleState::FOCUS_VISIBLE.bits()
        } else {
            0
        }
}

pub(super) fn initial_state(ui: &Ui, id: Id, enabled: bool) -> u8 {
    keyboard_modality(ui.ctx());
    let (mut state, widget_id) = ui.ctx().data(|data| {
        (
            data.get_temp::<u8>(id.with("state"))
                .unwrap_or(RuleState::ENABLED.bits()),
            data.get_temp::<Id>(id.with("widget-id")),
        )
    });
    state &= !(RuleState::FOCUS.bits() | RuleState::FOCUS_VISIBLE.bits());
    if !enabled {
        return (state & !RuleState::ENABLED.bits()) | RuleState::DISABLED.bits();
    }
    if let Some(widget_id) = widget_id
        && RuleState::ENABLED.matches(state)
    {
        state |= focus_state(ui.ctx(), widget_id);
    }
    state
}

pub(super) fn response_state(response: &Response, id: Id, enabled: bool) -> u8 {
    let mut state = 0;
    if response.hovered() {
        state |= RuleState::HOVERED.bits();
    }
    if response.is_pointer_button_down_on() {
        state |= RuleState::ACTIVE.bits();
    }
    if enabled {
        state |= RuleState::ENABLED.bits();
        if response.enabled() {
            state |= focus_state(&response.ctx, response.id);
        }
    } else {
        state |= RuleState::DISABLED.bits();
    }
    response.ctx.data_mut(|data| {
        data.insert_temp(id.with("state"), state);
        data.insert_temp(id.with("widget-id"), response.id);
    });
    state
}
