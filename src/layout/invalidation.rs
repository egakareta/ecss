use std::sync::Arc;

use egui::{Context, Id};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LayoutInvalidation {
    Geometry,
    Interaction,
    PaintOrder,
}

#[derive(Clone, Default)]
struct Pending {
    pass: u64,
    reasons: Vec<LayoutInvalidation>,
}

pub(super) fn request_relayout(context: &Context, invalidation: LayoutInvalidation) {
    let id = Id::unique(("ecss.relayout", context.viewport_id()));
    let registered = Id::unique("ecss.relayout.callback");
    let pass = context.cumulative_pass_nr();
    let install_callback = context.data_mut(|data| {
        let pending = data.get_temp_mut_or_default::<Pending>(id);
        if pending.pass != pass {
            pending.pass = pass;
            pending.reasons.clear();
        }
        if !pending.reasons.contains(&invalidation) {
            pending.reasons.push(invalidation);
        }
        if data.get_temp::<bool>(registered).unwrap_or(false) {
            false
        } else {
            data.insert_temp(registered, true);
            true
        }
    });
    if install_callback {
        context.on_end_pass("ecss.relayout", Arc::new(|ui| finish_pass(ui.ctx())));
    }
}

#[expect(
    clippy::disallowed_methods,
    reason = "Same-frame layout convergence is the sole production discard boundary."
)]
fn finish_pass(context: &Context) {
    let id = Id::unique(("ecss.relayout", context.viewport_id()));
    let Some(pending) = context.data_mut(|data| data.remove_temp::<Pending>(id)) else {
        return;
    };
    if pending.pass != context.cumulative_pass_nr() {
        return;
    }
    // Early discards suppress later widgets' paint commands, which supply text measurements.
    for invalidation in pending.reasons {
        let reason = match invalidation {
            LayoutInvalidation::Geometry => "CSS selector layout changed",
            LayoutInvalidation::Interaction => "CSS selector interaction state changed",
            LayoutInvalidation::PaintOrder => "CSS paint order changed",
        };
        context.request_discard(reason);
    }
}
